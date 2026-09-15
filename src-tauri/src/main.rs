//! BookOS Teclas: interfaz para remapear teclas del escritorio BookOS.
//!
//! La app no toca el teclado. Quien valida, guarda y aplica `teclas.conf` es
//! el compositor; aquí solo se habla con él por `org.bookos.Desktop`.

mod escritorio;
mod leyendas;

/// Quita el zoom de página que WebKitGTK hace con el pellizco del touchpad.
///
/// No hay ajuste para eso ni en WebKit ni en Tauri, y la página no se entera:
/// el pellizco lo consume un `GtkGestureZoom` de la propia vista y llega como
/// `setMagnification`, no como `wheel` ni `touch*`, así que ningún
/// `preventDefault` lo frena. Se apaga ese gesto y ningún otro: con fase `None`
/// GTK deja de pasarle eventos (gtkeventcontroller.c, 3.24). Va en
/// `on_page_load` para cubrir también las ventanas que se abran después.
///
/// Copiado de bookos-clock, donde está comprobado.
fn desactivar_zoom_por_pellizco<R: tauri::Runtime>(
    webview: &tauri::Webview<R>,
    _: &tauri::webview::PageLoadPayload<'_>,
) {
    #[cfg(target_os = "linux")]
    let _ = webview.with_webview(|webview| {
        use gtk::glib::translate::from_glib_none;
        use gtk::prelude::*;

        // WebKitGTK guarda ahí el gesto (WebKitWebViewBase.cpp, 2.52.5). No es
        // API pública: si la clave cambia, no se encuentra y el zoom vuelve.
        // Lo guardado es un puntero C a un GObject, no un tipo de Rust.
        let Some(gesto) =
            (unsafe { webview.inner().data::<gtk::ffi::GtkGesture>("wk-view-zoom-gesture") })
        else {
            return;
        };
        let gesto: gtk::Gesture = unsafe { from_glib_none(gesto.as_ptr()) };
        gesto.set_propagation_phase(gtk::PropagationPhase::None);
    });
}

// La paleta la genera BookOS Settings desde el fondo de pantalla y la deja en
// ~/.config/bookos/palette.css. Aquí solo se lee: quien tiñe es la hoja, que
// el cliente bookos-palette.js inyecta al final de <head>.
#[tauri::command]
fn bookos_palette_css() -> String {
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::Path::new(&h).join(".config/bookos/palette.css"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

fn main() {
    tauri::Builder::default()
        .on_page_load(desactivar_zoom_por_pellizco)
        .invoke_handler(tauri::generate_handler![
            bookos_palette_css,
            leyendas::leyendas,
            escritorio::hay_escritorio,
            escritorio::configuracion_escritorio,
            escritorio::obtener_remapeos,
            escritorio::aplicar_remapeos,
            escritorio::capturar_tecla,
            escritorio::cancelar_captura,
        ])
        .setup(|app| {
            use tauri::Manager;
            // La ventana nace oculta para no enseñar un marco vacío mientras
            // carga la página.
            if let Some(ventana) = app.get_webview_window("main") {
                ventana.show()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("la app de Tauri arranca con la configuración empaquetada");
}
