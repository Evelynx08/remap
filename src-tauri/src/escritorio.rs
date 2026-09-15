//! Cliente de `org.bookos.Desktop` para los remapeos de `teclas.conf`.
//!
//! Validar y escribir el fichero es cosa del compositor: aquí no se analiza
//! nada, solo se pasa el JSON de un lado a otro.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedValue, Value};

const DESTINO: &str = "org.bookos.Desktop";
const RUTA: &str = "/org/bookos/Desktop";
const INTERFAZ: &str = "org.bookos.Desktop";

/// Lo que se espera a que se pulse la tecla. Pasado el plazo se desarma la
/// captura: armada y olvidada, el compositor se tragaría la próxima tecla que
/// se pulsara en cualquier ventana.
const ESPERA_CAPTURA: Duration = Duration::from_secs(10);

/// Número de la captura en curso. Una captura cancelada sigue esperando su
/// plazo; al vencer solo desarma si nadie ha empezado otra, o apagaría la nueva.
static CAPTURA: AtomicU64 = AtomicU64::new(0);

static CONEXION: tokio::sync::OnceCell<zbus::Connection> = tokio::sync::OnceCell::const_new();

async fn proxy() -> Result<zbus::Proxy<'static>, String> {
    let conexion = CONEXION
        .get_or_try_init(|| async { zbus::Connection::session().await })
        .await
        .map_err(|e| e.to_string())?;
    zbus::Proxy::new(conexion, DESTINO, RUTA, INTERFAZ)
        .await
        .map_err(|e| e.to_string())
}

/// ¿Hay un escritorio BookOS que sepa remapear? Si no, la interfaz lo dice en
/// vez de fallar en cada acción.
#[tauri::command]
pub async fn hay_escritorio() -> bool {
    let Ok(proxy) = proxy().await else {
        return false;
    };
    let capacidades: zbus::Result<HashMap<String, OwnedValue>> =
        proxy.call("GetCapabilities", &()).await;
    capacidades.is_ok_and(|c| {
        c.get("key_remap")
            .is_some_and(|v| matches!(&**v, Value::Bool(true)))
    })
}

/// La configuración del escritorio (`GetConfig`) tal cual, en JSON. La página
/// solo mira `teclado`, para arrancar con la distribución de la sesión.
#[tauri::command]
pub async fn configuracion_escritorio() -> Result<String, String> {
    proxy()
        .await?
        .call("GetConfig", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn obtener_remapeos() -> Result<String, String> {
    proxy()
        .await?
        .call("GetKeyRemaps", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn aplicar_remapeos(json: String) -> Result<(), String> {
    let (ok, error): (bool, String) = proxy()
        .await?
        .call("ApplyKeyRemaps", &(json,))
        .await
        .map_err(|e| e.to_string())?;
    if ok {
        Ok(())
    } else {
        Err(error)
    }
}

/// Espera la próxima tecla y devuelve su código evdev y si era Copilot.
/// `None` si se agota el plazo.
#[tauri::command]
pub async fn capturar_tecla() -> Result<Option<(u32, bool)>, String> {
    let numero = CAPTURA.fetch_add(1, Ordering::SeqCst) + 1;
    let proxy = proxy().await?;
    // Suscrito antes de armar: si la tecla llega entre las dos llamadas, la
    // señal ya tiene quien la oiga.
    let mut señales = proxy
        .receive_signal("KeyCaptured")
        .await
        .map_err(|e| e.to_string())?;
    let armada: bool = proxy
        .call("CaptureKey", &(true,))
        .await
        .map_err(|e| e.to_string())?;
    if !armada {
        return Err("el escritorio no pudo armar la captura".into());
    }
    match tokio::time::timeout(ESPERA_CAPTURA, señales.next()).await {
        Ok(Some(mensaje)) => mensaje
            .body()
            .deserialize::<(u32, bool)>()
            .map(Some)
            .map_err(|e| e.to_string()),
        Ok(None) => Err("se cerró la conexión con el escritorio".into()),
        Err(_) => {
            if CAPTURA.load(Ordering::SeqCst) == numero {
                desarmar(&proxy).await?;
            }
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn cancelar_captura() -> Result<(), String> {
    CAPTURA.fetch_add(1, Ordering::SeqCst);
    desarmar(&proxy().await?).await
}

async fn desarmar(proxy: &zbus::Proxy<'_>) -> Result<(), String> {
    let _: bool = proxy
        .call("CaptureKey", &(false,))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
