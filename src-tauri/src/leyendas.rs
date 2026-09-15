//! Lo que pone cada tecla del dibujo en cada nivel, sacado de xkb.
//!
//! No hay tablas escritas a mano: se compila la distribución con las mismas
//! reglas que usa la sesión (`evdev`, modelo `pc105`) y se pregunta qué
//! carácter da cada tecla sin nada, con Mayús y con AltGr.

use std::collections::HashMap;

use xkbcommon::xkb;

/// Las teclas muertas no escriben nada solas y xkb no les da texto: se dibujan
/// con el acento que ponen, sacado del nombre del keysym.
const MUERTAS: &[(&str, &str)] = &[
    ("dead_grave", "`"),
    ("dead_acute", "´"),
    ("dead_circumflex", "^"),
    ("dead_tilde", "~"),
    ("dead_diaeresis", "¨"),
    ("dead_cedilla", "¸"),
    ("dead_abovering", "°"),
    ("dead_caron", "ˇ"),
    ("dead_breve", "˘"),
    ("dead_ogonek", "˛"),
    ("dead_doubleacute", "˝"),
    ("dead_abovedot", "˙"),
    ("dead_macron", "¯"),
];

/// Las teclas de carácter del dibujo, en evdev: las filas del 1, la Q, la A y
/// la Z, la 43 (junto al Intro) y la 86 (`<`, solo en ISO).
fn teclas() -> impl Iterator<Item = u32> {
    std::iter::once(41)
        .chain(2..=13)
        .chain(16..=27)
        .chain(30..=40)
        .chain([43, 86])
        .chain(44..=53)
}

/// La máscara de un modificador por su nombre. Si el keymap no lo tiene, no se
/// aplica: desplazar por `MOD_INVALID` desbordaría.
fn mascara(mapa: &xkb::Keymap, nombre: &str) -> xkb::ModMask {
    match mapa.mod_get_index(nombre) {
        xkb::MOD_INVALID => 0,
        indice => 1 << indice,
    }
}

fn caracter(estado: &xkb::State, codigo: u32) -> String {
    // xkb numera las teclas como X: evdev más ocho.
    let tecla = xkb::Keycode::new(codigo + 8);
    let texto = estado.key_get_utf8(tecla);
    if !texto.is_empty() && !texto.chars().any(char::is_control) {
        return texto;
    }
    let nombre = xkb::keysym_get_name(estado.key_get_one_sym(tecla));
    MUERTAS
        .iter()
        .find(|(muerta, _)| *muerta == nombre)
        .map(|(_, acento)| (*acento).to_string())
        .unwrap_or_default()
}

/// Evdev → `[normal, Mayús, AltGr]`. `distribucion` es el nombre de xkb, con la
/// variante entre paréntesis si la hay (`es`, `es(cat)`).
#[tauri::command]
pub fn leyendas(distribucion: &str) -> Result<HashMap<u32, [String; 3]>, String> {
    let (disposicion, variante) = match distribucion.split_once('(') {
        Some((disposicion, variante)) => (disposicion, variante.trim_end_matches(')')),
        None => (distribucion, ""),
    };
    let contexto = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let mapa = xkb::Keymap::new_from_names(
        &contexto,
        "evdev",
        "pc105",
        disposicion,
        variante,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or_else(|| format!("xkb no conoce la distribución «{distribucion}»"))?;

    let niveles = [
        0,
        mascara(&mapa, xkb::MOD_NAME_SHIFT),
        mascara(&mapa, xkb::MOD_NAME_ISO_LEVEL3_SHIFT),
    ];
    let mut estado = xkb::State::new(&mapa);
    let mut resultado: HashMap<u32, [String; 3]> =
        teclas().map(|codigo| (codigo, Default::default())).collect();
    for (nivel, modificadores) in niveles.into_iter().enumerate() {
        estado.update_mask(modificadores, 0, 0, 0, 0, 0);
        for (codigo, textos) in resultado.iter_mut() {
            textos[nivel] = caracter(&estado, *codigo);
        }
    }
    // Una tecla sin nivel de AltGr devuelve con Mod5 lo mismo que sin nada: eso
    // no es una leyenda, es que no tiene.
    for textos in resultado.values_mut() {
        if textos[2] == textos[0] {
            textos[2].clear();
        }
    }
    Ok(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn niveles(distribucion: &str, codigo: u32) -> [String; 3] {
        leyendas(distribucion).expect("xkb del sistema conoce la distribución")[&codigo].clone()
    }

    #[test]
    fn el_uno_espanol_tiene_sus_tres_niveles() {
        assert_eq!(niveles("es", 2), ["1", "!", "|"]);
    }

    #[test]
    fn las_letras_espanolas_llevan_su_altgr() {
        assert_eq!(niveles("es", 18), ["e", "E", "€"]);
    }

    #[test]
    fn las_teclas_muertas_se_dibujan_con_su_acento() {
        assert_eq!(niveles("es", 26), ["`", "^", "["]);
    }

    #[test]
    fn en_ee_uu_no_hay_nivel_de_altgr() {
        assert_eq!(niveles("us", 2), ["1", "!", ""]);
    }

    #[test]
    fn una_variante_entre_parentesis_se_entiende() {
        assert!(leyendas("es(cat)").is_ok());
    }

    #[test]
    fn una_distribucion_inexistente_es_un_error() {
        assert!(leyendas("no-existe").is_err());
    }
}
