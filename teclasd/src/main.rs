//! bookos-teclasd: los remapeos de BookOS Teclas en cualquier escritorio.
//!
//! En Plasma, GNOME o X11 ningún cliente puede cambiar una tecla por otra: KWin
//! solo ofrece atajos globales, que lanzan cosas pero no hacen que Copilot sea
//! un Ctrl mantenido. Así que esto va por debajo del escritorio: captura cada
//! teclado físico (`EVIOCGRAB`), traduce y reinyecta por un teclado virtual de
//! uinput. Corre como root porque ni `/dev/uinput` ni `/dev/input/event*` son
//! del usuario.
//!
//! **Contrato** (`org.bookos.Teclas` en `/org/bookos/Teclas`, bus de sistema).
//! Es el subconjunto de `org.bookos.Desktop` que usa la app, con el mismo
//! significado:
//!
//! ```text
//! GetKeyRemaps() -> s
//! ApplyKeyRemaps(s json) -> (b ok, s error)     pide polkit
//! CaptureKey(b armar) -> b                       pide polkit
//! signal KeyCaptured(u evdev, b copilot)         solo a quien armó la captura
//! ```

mod teclas;
mod uinput;

use std::collections::{HashMap, HashSet};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use evdev::raw_stream::{EventStream, RawDevice};
use evdev::{EventSummary, KeyCode};
use tokio::sync::watch;
use zbus::message::Header;
use zbus::names::UniqueName;
use zbus::zvariant::Value;

const NOMBRE: &str = "org.bookos.Teclas";
const RUTA: &str = "/org/bookos/Teclas";
const ACCION_POLKIT: &str = "org.bookos.teclas.configurar";

/// Cada cuánto se buscan teclados nuevos en `/dev/input`. Sin udev ni inotify
/// entre las dependencias, mirar el directorio es barato: solo se abren los
/// nodos que no se habían visto. Un teclado recién enchufado funciona sin
/// remapear durante como mucho este rato.
const SONDEO: Duration = Duration::from_secs(2);

/// Los modificadores no se capturan: son la primera mitad del acorde de
/// Copilot, y al pulsarlos no se sabe si viene algo detrás.
const MODIFICADORES: [u32; 8] = [29, 97, 42, 54, 56, 100, 125, 126];

struct Estado {
    remapeos: teclas::Remapeos,
    /// Quién espera la próxima tecla.
    captura: Option<UniqueName<'static>>,
}

type Compartido = Arc<Mutex<Estado>>;

fn bloquear(estado: &Mutex<Estado>) -> std::sync::MutexGuard<'_, Estado> {
    estado
        .lock()
        .expect("nadie entra en pánico con el estado tomado: no hay código que pueda hacerlo")
}

/// Cambiar teclas o quedarse con la próxima pulsación no puede estar abierto a
/// cualquier proceso del sistema. La acción deja pasar sin contraseña a quien
/// está sentado delante (`allow_active`), que es quien usa la app.
async fn autorizar(conexion: &zbus::Connection, cabecera: &Header<'_>) -> zbus::fdo::Result<()> {
    let remitente = cabecera
        .sender()
        .ok_or_else(|| zbus::fdo::Error::AccessDenied("mensaje sin remitente".into()))?;
    let sujeto = (
        "system-bus-name",
        HashMap::from([("name", Value::from(remitente.as_str()))]),
    );
    // 1 = AllowUserInteraction: si la política pide contraseña, la pregunta el
    // agente de polkit del escritorio.
    let respuesta = conexion
        .call_method(
            Some("org.freedesktop.PolicyKit1"),
            "/org/freedesktop/PolicyKit1/Authority",
            Some("org.freedesktop.PolicyKit1.Authority"),
            "CheckAuthorization",
            &(
                sujeto,
                ACCION_POLKIT,
                HashMap::<&str, &str>::new(),
                1u32,
                "",
            ),
        )
        .await?;
    let (autorizado, _, _): (bool, bool, HashMap<String, String>) =
        respuesta.body().deserialize()?;
    if autorizado {
        Ok(())
    } else {
        Err(zbus::fdo::Error::AccessDenied(
            "polkit no autoriza a cambiar las teclas".into(),
        ))
    }
}

struct Servicio {
    estado: Compartido,
}

#[zbus::interface(name = "org.bookos.Teclas")]
impl Servicio {
    fn get_key_remaps(&self) -> String {
        bloquear(&self.estado).remapeos.a_json()
    }

    async fn apply_key_remaps(
        &self,
        #[zbus(header)] cabecera: Header<'_>,
        #[zbus(connection)] conexion: &zbus::Connection,
        json: String,
    ) -> zbus::fdo::Result<(bool, String)> {
        autorizar(conexion, &cabecera).await?;
        let remapeos = match teclas::Remapeos::desde_json(&json) {
            Ok(remapeos) => remapeos,
            Err(err) => return Ok((false, err)),
        };
        if let Err(err) = teclas::guardar(&remapeos) {
            return Ok((false, format!("no se pudieron guardar los remapeos: {err}")));
        }
        bloquear(&self.estado).remapeos = remapeos;
        Ok((true, String::new()))
    }

    async fn capture_key(
        &self,
        #[zbus(header)] cabecera: Header<'_>,
        #[zbus(connection)] conexion: &zbus::Connection,
        armar: bool,
    ) -> zbus::fdo::Result<bool> {
        autorizar(conexion, &cabecera).await?;
        bloquear(&self.estado).captura = if armar {
            cabecera.sender().map(UniqueName::to_owned)
        } else {
            None
        };
        Ok(true)
    }
}

/// Un teclado de verdad: con letras e Intro. Fuera quedan los nodos que además
/// mueven un puntero (algunos receptores USB lo juntan todo): aquí solo se
/// reinyectan teclas y el puntero dejaría de moverse. Y el teclado virtual, que
/// se capturaría a sí mismo.
fn es_teclado(dispositivo: &RawDevice) -> bool {
    dispositivo.name() != Some(uinput::NOMBRE)
        && dispositivo
            .supported_keys()
            .is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_ENTER))
        && dispositivo.supported_relative_axes().is_none()
        && dispositivo.supported_absolute_axes().is_none()
}

fn enviar_leds(dispositivo: &mut RawDevice, mascara: u16) -> io::Result<()> {
    let eventos: Vec<_> = (0..=uinput::LED_MAX)
        .map(|led| evdev::InputEvent::new(uinput::EV_LED, led, i32::from((mascara >> led) & 1)))
        .collect();
    dispositivo.send_events(&eventos)
}

async fn atender(
    mut flujo: EventStream,
    estado: Compartido,
    salida: Arc<uinput::Teclado>,
    conexion: zbus::Connection,
    mut leds: watch::Receiver<u16>,
) -> io::Result<()> {
    let mut traductor = teclas::Traductor::default();
    // El teclado recién capturado no ha visto los LED que se fijaron antes.
    let inicial = *leds.borrow_and_update();
    enviar_leds(flujo.device_mut(), inicial)?;
    loop {
        tokio::select! {
            evento = flujo.next_event() => {
                let EventSummary::Key(_, codigo, valor) = evento?.destructure() else {
                    continue;
                };
                let evdev = u32::from(codigo.code());
                if valor == 2 {
                    if let Some(destino) = traductor.repeticion(evdev) {
                        salida.enviar(destino, 2)?;
                    }
                    continue;
                }
                let pulsada = valor == 1;
                if pulsada && !MODIFICADORES.contains(&evdev) {
                    let quien = bloquear(&estado).captura.take();
                    if let Some(quien) = quien {
                        traductor.tragar_suelta(evdev);
                        let copilot = traductor.es_copilot(evdev);
                        if let Err(err) = conexion
                            .emit_signal(Some(quien), RUTA, NOMBRE, "KeyCaptured", &(evdev, copilot))
                            .await
                        {
                            eprintln!("no se pudo emitir KeyCaptured: {err}");
                        }
                        continue;
                    }
                }
                let salidas = traductor.traducir(&bloquear(&estado).remapeos, evdev, pulsada);
                for (codigo, pulsada) in salidas {
                    salida.enviar(codigo, i32::from(pulsada))?;
                }
            }
            cambio = leds.changed() => {
                // Sin emisor es que el servicio está terminando.
                if cambio.is_err() {
                    return Ok(());
                }
                let mascara = *leds.borrow_and_update();
                enviar_leds(flujo.device_mut(), mascara)?;
            }
        }
    }
}

async fn vigilar_teclados(
    estado: Compartido,
    salida: Arc<uinput::Teclado>,
    conexion: zbus::Connection,
    leds: watch::Receiver<u16>,
) {
    // Nodos ya mirados, sean teclados o no, para no reabrirlos en cada vuelta.
    // Al desaparecer uno de `/dev/input` se olvida: el número se reutiliza.
    let mut vistos: HashSet<PathBuf> = HashSet::new();
    let mut intervalo = tokio::time::interval(SONDEO);
    loop {
        intervalo.tick().await;
        let Ok(entradas) = std::fs::read_dir("/dev/input") else {
            continue;
        };
        let presentes: HashSet<PathBuf> = entradas
            .flatten()
            .map(|entrada| entrada.path())
            .filter(|ruta| {
                ruta.file_name()
                    .is_some_and(|n| n.as_bytes().starts_with(b"event"))
            })
            .collect();
        vistos.retain(|ruta| presentes.contains(ruta));

        for ruta in presentes {
            if vistos.contains(&ruta) {
                continue;
            }
            let Ok(mut dispositivo) = RawDevice::open(&ruta) else {
                continue;
            };
            if !es_teclado(&dispositivo) {
                vistos.insert(ruta);
                continue;
            }
            // Capturar con una tecla abajo la dejaría pulsada para siempre en
            // el escritorio: su suelta ya no le llegaría. Se prueba en la
            // siguiente vuelta.
            if dispositivo
                .get_key_state()
                .is_ok_and(|teclas| teclas.iter().next().is_some())
            {
                continue;
            }
            vistos.insert(ruta.clone());
            let flujo = match dispositivo
                .grab()
                .and_then(|()| dispositivo.into_event_stream())
            {
                Ok(flujo) => flujo,
                Err(err) => {
                    eprintln!("no se pudo capturar {}: {err}", ruta.display());
                    continue;
                }
            };
            eprintln!("capturado {}", ruta.display());
            let (estado, salida, conexion, leds) = (
                estado.clone(),
                salida.clone(),
                conexion.clone(),
                leds.clone(),
            );
            tokio::spawn(async move {
                // Lo normal es ENODEV al desenchufarlo.
                if let Err(err) = atender(flujo, estado, salida, conexion, leds).await {
                    eprintln!("se dejó de atender {}: {err}", ruta.display());
                }
            });
        }
    }
}

/// Lee los LED que el escritorio fija en el teclado virtual y los deja en
/// `leds` para que cada teclado físico los copie. Solo vuelve si uinput falla,
/// y entonces el servicio no sirve: que lo reinicie systemd.
async fn leer_leds(salida: &uinput::Teclado, leds: watch::Sender<u16>) -> io::Error {
    loop {
        match salida.led().await {
            Ok((led, valor)) => leds.send_modify(|mascara| {
                if valor != 0 {
                    *mascara |= 1 << led;
                } else {
                    *mascara &= !(1 << led);
                }
            }),
            Err(err) => return err,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let salida = Arc::new(uinput::Teclado::crear()?);
    let estado = Arc::new(Mutex::new(Estado {
        remapeos: teclas::cargar(),
        captura: None,
    }));
    let conexion = zbus::connection::Builder::system()?
        .name(NOMBRE)?
        .serve_at(
            RUTA,
            Servicio {
                estado: estado.clone(),
            },
        )?
        .build()
        .await?;
    let (leds_tx, leds_rx) = watch::channel(0);
    tokio::spawn(vigilar_teclados(estado, salida.clone(), conexion, leds_rx));
    Err(leer_leds(&salida, leds_tx).await.into())
}
