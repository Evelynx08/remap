//! Remapeo de teclas: `/etc/bookos/teclas.conf`.
//!
//! Copia de `bookos-comp/src/teclas.rs` sin las funciones del escritorio
//! (`accion:` y `cmd:`): fuera de BookOS no hay Launchpad al que llamar, y un
//! comando lanzado desde este servicio correría como root. El formato y los
//! nombres son los mismos para que la app hable igual con los dos; un cambio en
//! la lógica de Copilot hay que llevarlo a ambos sitios.
//!
//! **Copilot no es una tecla.** El firmware manda `Meta` izquierda, `Mayús`
//! izquierda y `F23`, en ese orden, y las suelta al revés. Cuando llega la F23
//! las dos primeras ya han salido por el teclado virtual: se sueltan a mano en
//! ese momento y se tragan sus sueltas físicas.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const RUTA: &str = "/etc/bookos/teclas.conf";

const KEY_LEFTSHIFT: u32 = 42;
const KEY_F23: u32 = 193;
const KEY_LEFTMETA: u32 = 125;
/// `KEY_MAX` de `linux/input-event-codes.h`.
const KEY_MAX: u32 = 0x2ff;

/// Los nombres que ofrece la app, con su código de `input-event-codes.h`.
/// Cualquier otra tecla se escribe como `evdev:N`.
const NOMBRES: &[(&str, u32)] = &[
    ("ctrl", 29),
    ("ctrl_der", 97),
    ("shift", 42),
    ("shift_der", 54),
    ("alt", 56),
    ("altgr", 100),
    ("meta", 125),
    ("meta_der", 126),
    ("esc", 1),
    ("retroceso", 14),
    ("tab", 15),
    ("intro", 28),
    ("espacio", 57),
    ("bloqmayus", 58),
    ("impr", 99),
    ("inicio", 102),
    ("arriba", 103),
    ("repag", 104),
    ("izquierda", 105),
    ("derecha", 106),
    ("fin", 107),
    ("abajo", 108),
    ("avpag", 109),
    ("insert", 110),
    ("supr", 111),
    ("silencio", 113),
    ("bajar_volumen", 114),
    ("subir_volumen", 115),
    ("menu", 127),
    ("siguiente", 163),
    ("reproducir", 164),
    ("anterior", 165),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origen {
    /// El acorde Meta+Mayús+F23 que manda la tecla de Copilot.
    Copilot,
    Evdev(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destino {
    Tecla(u32),
    /// La tecla se traga: no sale por el teclado virtual.
    Nada,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remapeos(Vec<(Origen, Destino)>);

fn tecla_de_texto(texto: &str) -> Result<u32, String> {
    if let Some(n) = texto.strip_prefix("evdev:") {
        let codigo: u32 = n
            .trim()
            .parse()
            .map_err(|_| format!("código evdev no válido: «{texto}»"))?;
        if codigo == 0 || codigo > KEY_MAX {
            return Err(format!("código evdev fuera de rango: {codigo}"));
        }
        return Ok(codigo);
    }
    NOMBRES
        .iter()
        .find(|(nombre, _)| *nombre == texto)
        .map(|(_, codigo)| *codigo)
        .ok_or_else(|| format!("tecla desconocida: «{texto}»"))
}

fn tecla_a_texto(codigo: u32) -> String {
    NOMBRES
        .iter()
        .find(|(_, c)| *c == codigo)
        .map(|(nombre, _)| (*nombre).to_string())
        .unwrap_or_else(|| format!("evdev:{codigo}"))
}

impl Origen {
    fn desde_texto(texto: &str) -> Result<Self, String> {
        match texto {
            "copilot" => Ok(Origen::Copilot),
            otro => tecla_de_texto(otro).map(Origen::Evdev),
        }
    }

    fn a_texto(self) -> String {
        match self {
            Origen::Copilot => "copilot".into(),
            Origen::Evdev(codigo) => tecla_a_texto(codigo),
        }
    }
}

impl Destino {
    fn desde_texto(texto: &str) -> Result<Self, String> {
        if texto == "nada" {
            return Ok(Destino::Nada);
        }
        if texto.starts_with("accion:") || texto.starts_with("cmd:") {
            return Err(format!(
                "«{texto}» solo existe en el escritorio BookOS; aquí una tecla solo puede ser otra tecla o nada"
            ));
        }
        tecla_de_texto(texto).map(Destino::Tecla)
    }

    fn a_texto(self) -> String {
        match self {
            Destino::Tecla(codigo) => tecla_a_texto(codigo),
            Destino::Nada => "nada".into(),
        }
    }
}

/// Una entrada tal como viaja por D-Bus: los mismos textos que en el fichero.
#[derive(Serialize, Deserialize)]
struct EntradaJson {
    origen: String,
    destino: String,
}

impl Remapeos {
    pub fn buscar(&self, origen: Origen) -> Option<Destino> {
        self.0.iter().find(|(o, _)| *o == origen).map(|(_, d)| *d)
    }

    fn validar(entradas: Vec<(Origen, Destino)>) -> Result<Self, String> {
        for (i, (origen, _)) in entradas.iter().enumerate() {
            if entradas[..i].iter().any(|(o, _)| o == origen) {
                return Err(format!("la tecla «{}» está repetida", origen.a_texto()));
            }
        }
        Ok(Remapeos(entradas))
    }

    fn desde_texto(texto: &str) -> Result<Self, String> {
        let entradas = texto
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|linea| {
                let (origen, destino) = linea
                    .split_once('=')
                    .ok_or_else(|| format!("línea sin «=»: «{linea}»"))?;
                Ok((
                    Origen::desde_texto(origen.trim())?,
                    Destino::desde_texto(destino.trim())?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Self::validar(entradas)
    }

    fn a_texto(&self) -> String {
        let mut salida = String::from("# Remapeo de teclas de BookOS: origen = destino.\n");
        for (origen, destino) in &self.0 {
            salida.push_str(&format!("{} = {}\n", origen.a_texto(), destino.a_texto()));
        }
        salida
    }

    /// Sin la comprobación de saltos de línea del compositor: allí protege a
    /// `cmd:`, y aquí cualquier destino con uno ya no es un nombre válido.
    pub fn desde_json(json: &str) -> Result<Self, String> {
        let entradas: Vec<EntradaJson> =
            serde_json::from_str(json).map_err(|e| format!("JSON no válido: {e}"))?;
        let entradas = entradas
            .iter()
            .map(|e| {
                Ok((
                    Origen::desde_texto(e.origen.trim())?,
                    Destino::desde_texto(e.destino.trim())?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Self::validar(entradas)
    }

    pub fn a_json(&self) -> String {
        let entradas: Vec<EntradaJson> = self
            .0
            .iter()
            .map(|(o, d)| EntradaJson {
                origen: o.a_texto(),
                destino: d.a_texto(),
            })
            .collect();
        serde_json::to_string(&entradas).expect("un Vec de cadenas siempre serializa")
    }
}

/// Sin fichero no hay remapeos. Con un fichero roto tampoco, y se dice: aplicar
/// la mitad buena dejaría teclas cambiadas que el usuario no reconoce.
pub fn cargar() -> Remapeos {
    let Ok(texto) = std::fs::read_to_string(RUTA) else {
        return Remapeos::default();
    };
    Remapeos::desde_texto(&texto).unwrap_or_else(|err| {
        eprintln!("{RUTA} no es válido, sin remapeos: {err}");
        Remapeos::default()
    })
}

pub fn guardar(remapeos: &Remapeos) -> std::io::Result<()> {
    let ruta = Path::new(RUTA);
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(ruta, remapeos.a_texto())
}

/// Lo que hay que recordar entre eventos de un teclado para que ninguna tecla
/// se quede trabada.
#[derive(Debug, Default)]
pub struct Traductor {
    /// Teclas físicas pulsadas. Solo importan Meta y Mayús, para reconocer el
    /// acorde de Copilot.
    fisicas: Vec<u32>,
    /// Origen pulsado y el destino con el que se pulsó. La suelta usa este y no
    /// el mapa actual: si la configuración cambia con la tecla abajo, se suelta
    /// lo que se pulsó. `None` es que la pulsación no salió.
    pulsadas: Vec<(u32, Option<u32>)>,
    /// Sueltas físicas que ya no deben salir: las de Meta y Mayús tras Copilot,
    /// que ya se soltaron a mano, y la de una tecla capturada.
    tragar: Vec<u32>,
}

impl Traductor {
    /// ¿Esta pulsación es la F23 del acorde de Copilot?
    pub fn es_copilot(&self, evdev: u32) -> bool {
        evdev == KEY_F23
            && self.fisicas.contains(&KEY_LEFTMETA)
            && self.fisicas.contains(&KEY_LEFTSHIFT)
    }

    /// La pulsación se la quedó la captura de la app: que su suelta tampoco
    /// salga.
    pub fn tragar_suelta(&mut self, evdev: u32) {
        self.tragar.push(evdev);
    }

    /// `(código, pulsada)` que hay que mandar por el teclado virtual.
    pub fn traducir(&mut self, mapa: &Remapeos, evdev: u32, pulsada: bool) -> Vec<(u32, bool)> {
        if pulsada {
            self.fisicas.push(evdev);
        } else {
            self.fisicas.retain(|&c| c != evdev);
            if let Some(i) = self.tragar.iter().position(|&c| c == evdev) {
                self.tragar.swap_remove(i);
                return Vec::new();
            }
            if let Some(i) = self.pulsadas.iter().position(|&(o, _)| o == evdev) {
                let (_, destino) = self.pulsadas.swap_remove(i);
                return destino.map(|d| (d, false)).into_iter().collect();
            }
            return vec![(evdev, false)];
        }

        let copilot = self.es_copilot(evdev);
        let origen = if copilot {
            Origen::Copilot
        } else {
            Origen::Evdev(evdev)
        };
        let Some(destino) = mapa.buscar(origen) else {
            return vec![(evdev, true)];
        };

        let mut eventos = Vec::new();
        if copilot {
            eventos.push((KEY_LEFTMETA, false));
            eventos.push((KEY_LEFTSHIFT, false));
            self.tragar.extend([KEY_LEFTMETA, KEY_LEFTSHIFT]);
        }
        let salida = match destino {
            Destino::Tecla(d) => {
                eventos.push((d, true));
                Some(d)
            }
            Destino::Nada => None,
        };
        self.pulsadas.push((evdev, salida));
        eventos
    }

    /// Qué repetir cuando el kernel autorrepite una tecla física mantenida:
    /// lo que salió al pulsarla, o nada si no salió. Los escritorios con
    /// libinput repiten por su cuenta, pero la consola usa estas.
    pub fn repeticion(&self, evdev: u32) -> Option<u32> {
        if self.tragar.contains(&evdev) {
            return None;
        }
        match self.pulsadas.iter().find(|&&(o, _)| o == evdev) {
            Some(&(_, destino)) => destino,
            None => Some(evdev),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapa(texto: &str) -> Remapeos {
        Remapeos::desde_texto(texto).expect("mapa de prueba válido")
    }

    fn secuencia_copilot(m: &Remapeos, t: &mut Traductor) -> Vec<(u32, bool)> {
        // La secuencia que manda el firmware: 125↓ 42↓ 193↓ 193↑ 42↑ 125↑.
        [
            (125, true),
            (42, true),
            (193, true),
            (193, false),
            (42, false),
            (125, false),
        ]
        .into_iter()
        .flat_map(|(codigo, pulsada)| t.traducir(m, codigo, pulsada))
        .collect()
    }

    #[test]
    fn el_fichero_va_y_vuelve_igual() {
        let original = mapa("# comentario\ncopilot = ctrl_der\nevdev:190 = esc\nsupr = nada\n");
        assert_eq!(
            Remapeos::desde_texto(&original.a_texto()),
            Ok(original.clone())
        );
        assert_eq!(Remapeos::desde_json(&original.a_json()), Ok(original));
    }

    #[test]
    fn un_origen_repetido_se_rechaza() {
        let err = Remapeos::desde_texto("copilot = ctrl\ncopilot = alt\n").unwrap_err();
        assert!(err.contains("repetida"), "{err}");
    }

    #[test]
    fn se_rechazan_nombres_y_codigos_invalidos() {
        assert!(Remapeos::desde_texto("fn = ctrl").is_err());
        assert!(Remapeos::desde_texto("evdev:0 = ctrl").is_err());
        assert!(Remapeos::desde_texto("evdev:768 = ctrl").is_err());
        assert!(Remapeos::desde_json(r#"[{"origen":"copilot","destino":"ctrl\nalt"}]"#).is_err());
    }

    #[test]
    fn las_funciones_de_bookos_se_rechazan() {
        assert!(Remapeos::desde_texto("copilot = accion:launchpad").is_err());
        assert!(Remapeos::desde_json(r#"[{"origen":"menu","destino":"cmd:konsole"}]"#).is_err());
    }

    #[test]
    fn sin_remapeo_las_teclas_pasan_tal_cual() {
        let mut t = Traductor::default();
        let vacio = Remapeos::default();
        assert_eq!(t.traducir(&vacio, 30, true), vec![(30, true)]);
        assert_eq!(t.traducir(&vacio, 30, false), vec![(30, false)]);
    }

    #[test]
    fn copilot_a_ctrl_derecho_suelta_meta_y_mayus_una_sola_vez() {
        let m = mapa("copilot = ctrl_der");
        let mut t = Traductor::default();
        assert_eq!(
            secuencia_copilot(&m, &mut t),
            vec![
                (125, true),
                (42, true),
                (125, false),
                (42, false),
                (97, true),
                (97, false),
            ]
        );
    }

    #[test]
    fn copilot_sin_remapeo_sale_como_llego() {
        let mut t = Traductor::default();
        assert_eq!(
            secuencia_copilot(&Remapeos::default(), &mut t),
            vec![
                (125, true),
                (42, true),
                (193, true),
                (193, false),
                (42, false),
                (125, false),
            ]
        );
    }

    #[test]
    fn f23_sin_el_acorde_no_es_copilot() {
        let m = mapa("copilot = ctrl");
        let mut t = Traductor::default();
        assert_eq!(t.traducir(&m, 193, true), vec![(193, true)]);
    }

    #[test]
    fn copilot_a_nada_no_deja_nada_pulsado() {
        let m = mapa("copilot = nada");
        let mut t = Traductor::default();
        assert_eq!(
            secuencia_copilot(&m, &mut t),
            vec![(125, true), (42, true), (125, false), (42, false)]
        );
    }

    #[test]
    fn se_suelta_el_destino_con_el_que_se_pulso() {
        let mut t = Traductor::default();
        assert_eq!(
            t.traducir(&mapa("menu = ctrl"), 127, true),
            vec![(29, true)]
        );
        // Entre medias se recarga con otro destino.
        assert_eq!(
            t.traducir(&mapa("menu = alt"), 127, false),
            vec![(29, false)]
        );
    }

    #[test]
    fn la_repeticion_repite_lo_que_salio() {
        let m = mapa("menu = ctrl\nsupr = nada");
        let mut t = Traductor::default();
        t.traducir(&m, 127, true);
        t.traducir(&m, 111, true);
        t.traducir(&m, 30, true);
        assert_eq!(t.repeticion(127), Some(29));
        assert_eq!(t.repeticion(111), None);
        assert_eq!(t.repeticion(30), Some(30));
    }

    #[test]
    fn la_suelta_de_una_tecla_capturada_no_sale() {
        let mut t = Traductor::default();
        t.tragar_suelta(190);
        assert!(t.traducir(&Remapeos::default(), 190, false).is_empty());
    }
}
