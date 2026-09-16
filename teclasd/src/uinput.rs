//! El teclado virtual por el que salen las teclas ya traducidas.
//!
//! Con ioctls a mano y no con `evdev::uinput`: su builder (0.13.2) no tiene
//! forma de declarar LED, y sin `EV_LED` el escritorio no le manda el estado de
//! Bloq Mayús. Tampoco puede mandárselo al teclado físico, porque con él
//! capturado (`EVIOCGRAB`) el kernel descarta lo que escribe cualquiera que no
//! tenga la captura. El piloto se quedaría apagado siempre.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

use tokio::io::unix::AsyncFd;

/// Con este nombre lo ven libinput y el escritorio. `bookos-teclasd.quirks` lo
/// busca por aquí, y el propio servicio lo usa para no capturarse a sí mismo.
pub const NOMBRE: &str = "BookOS Teclas";

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
pub const EV_LED: u16 = 0x11;
const SYN_REPORT: u16 = 0;
pub const LED_MAX: u16 = 0x0f;
const BUS_VIRTUAL: u16 = 0x06;

/// `_IOW('U', nr, tamaño)` de `<asm-generic/ioctl.h>`.
const fn iow(nr: libc::c_ulong, tamano: usize) -> libc::c_ulong {
    (1 << 30) | ((tamano as libc::c_ulong) << 16) | ((b'U' as libc::c_ulong) << 8) | nr
}

const UI_DEV_CREATE: libc::c_ulong = ((b'U' as libc::c_ulong) << 8) | 1;
const UI_DEV_SETUP: libc::c_ulong = iow(3, std::mem::size_of::<libc::uinput_setup>());
const UI_SET_EVBIT: libc::c_ulong = iow(100, std::mem::size_of::<libc::c_int>());
const UI_SET_KEYBIT: libc::c_ulong = iow(101, std::mem::size_of::<libc::c_int>());
const UI_SET_LEDBIT: libc::c_ulong = iow(105, std::mem::size_of::<libc::c_int>());

/// Todas las teclas menos los botones (`BTN_*`: 0x100–0x15f y 0x2c0–0x2e7).
/// Con ellos libinput tomaría el dispositivo por un ratón o un mando.
fn teclas() -> impl Iterator<Item = u16> {
    (1..0x100).chain(0x160..0x2c0).chain(0x2e8..=0x2ff)
}

fn evento(tipo: u16, codigo: u16, valor: i32) -> libc::input_event {
    // El kernel pone la hora al recibirlo: la de aquí se ignora.
    libc::input_event {
        time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        type_: tipo,
        code: codigo,
        value: valor,
    }
}

pub struct Teclado {
    fd: AsyncFd<File>,
}

impl Teclado {
    pub fn crear() -> io::Result<Self> {
        let archivo = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")?;
        let fd = archivo.as_raw_fd();
        let ioctl = |peticion: libc::c_ulong, argumento: libc::c_ulong| {
            // SAFETY: `fd` es de `archivo`, que vive más que el cierre; las
            // peticiones con argumento entero no leen memoria de nadie.
            if unsafe { libc::ioctl(fd, peticion as _, argumento) } < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        };

        ioctl(UI_SET_EVBIT, EV_KEY.into())?;
        for tecla in teclas() {
            ioctl(UI_SET_KEYBIT, tecla.into())?;
        }
        ioctl(UI_SET_EVBIT, EV_LED.into())?;
        for led in 0..=LED_MAX {
            ioctl(UI_SET_LEDBIT, led.into())?;
        }

        // SAFETY: estructura C solo de enteros: todo ceros es un valor válido,
        // y deja el nombre terminado en nulo.
        let mut ajuste: libc::uinput_setup = unsafe { std::mem::zeroed() };
        ajuste.id.bustype = BUS_VIRTUAL;
        for (destino, byte) in ajuste.name.iter_mut().zip(NOMBRE.bytes()) {
            *destino = byte as libc::c_char;
        }
        // SAFETY: `ajuste` es del tipo que espera UI_DEV_SETUP y vive durante
        // la llamada.
        if unsafe { libc::ioctl(fd, UI_DEV_SETUP as _, &ajuste as *const libc::uinput_setup) } < 0 {
            return Err(io::Error::last_os_error());
        }
        ioctl(UI_DEV_CREATE, 0)?;

        Ok(Teclado {
            fd: AsyncFd::new(archivo)?,
        })
    }

    /// Una tecla: `valor` 0 suelta, 1 pulsa, 2 repite. Con su `SYN_REPORT`
    /// detrás, como la manda un teclado de verdad.
    pub fn enviar(&self, codigo: u32, valor: i32) -> io::Result<()> {
        let codigo =
            u16::try_from(codigo).expect("los códigos evdev caben en u16: KEY_MAX es 0x2ff");
        let eventos = [evento(EV_KEY, codigo, valor), evento(EV_SYN, SYN_REPORT, 0)];
        // SAFETY: `input_event` es repr(C) de enteros sin huecos en 64 bits, así
        // que sus bytes se pueden leer tal cual.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                eventos.as_ptr().cast::<u8>(),
                std::mem::size_of_val(&eventos),
            )
        };
        let mut archivo = self.fd.get_ref();
        archivo.write_all(bytes)
    }

    /// Espera al próximo LED que el escritorio fije en este teclado:
    /// `(led, valor)`.
    pub async fn led(&self) -> io::Result<(u16, i32)> {
        loop {
            let mut guardia = self.fd.readable().await?;
            // SAFETY: como en `crear`, todo ceros es un `input_event` válido.
            let mut leido: libc::input_event = unsafe { std::mem::zeroed() };
            let tamano = std::mem::size_of::<libc::input_event>();
            let resultado = guardia.try_io(|fd| {
                let mut archivo = fd.get_ref();
                // SAFETY: el búfer es exactamente el `input_event` de arriba, y
                // cualquier combinación de bytes es un `input_event` válido.
                archivo.read(unsafe {
                    std::slice::from_raw_parts_mut(
                        (&mut leido as *mut libc::input_event).cast::<u8>(),
                        tamano,
                    )
                })
            });
            match resultado {
                Ok(Ok(n)) if n == tamano => {
                    if leido.type_ == EV_LED && leido.code <= LED_MAX {
                        return Ok((leido.code, leido.value));
                    }
                }
                Ok(Ok(n)) => return Err(io::Error::other(format!("uinput devolvió {n} bytes"))),
                Ok(Err(err)) => return Err(err),
                Err(_sin_datos) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_peticiones_coinciden_con_las_de_linux() {
        // Valores de `<linux/uinput.h>` en x86_64.
        assert_eq!(UI_DEV_CREATE, 0x5501);
        assert_eq!(UI_DEV_SETUP, 0x405c5503);
        assert_eq!(UI_SET_EVBIT, 0x40045564);
        assert_eq!(UI_SET_KEYBIT, 0x40045565);
        assert_eq!(UI_SET_LEDBIT, 0x40045569);
    }
}
