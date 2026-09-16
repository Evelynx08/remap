//! Cliente de los remapeos de `teclas.conf`.
//!
//! En la sesión BookOS los aplica el compositor (`org.bookos.Desktop`, bus de
//! sesión). En cualquier otro escritorio, `bookos-teclasd` (`org.bookos.Teclas`,
//! bus de sistema), que habla el mismo subconjunto de métodos pero solo sabe de
//! teclas. Validar y escribir el fichero es cosa de ellos: aquí no se analiza
//! nada, solo se pasa el JSON de un lado a otro.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::OnceCell;
use zbus::zvariant::{OwnedValue, Value};

/// Lo que se espera a que se pulse la tecla. Pasado el plazo se desarma la
/// captura: armada y olvidada, el compositor se tragaría la próxima tecla que
/// se pulsara en cualquier ventana.
const ESPERA_CAPTURA: Duration = Duration::from_secs(10);

/// Número de la captura en curso. Una captura cancelada sigue esperando su
/// plazo; al vencer solo desarma si nadie ha empezado otra, o apagaría la nueva.
static CAPTURA: AtomicU64 = AtomicU64::new(0);

static SESION: OnceCell<zbus::Connection> = OnceCell::const_new();
static SISTEMA: OnceCell<zbus::Connection> = OnceCell::const_new();
/// Se decide una vez: la sesión no cambia de escritorio con la app abierta.
static MOTOR: OnceCell<Option<Motor>> = OnceCell::const_new();

#[derive(Clone, Copy)]
enum Motor {
    /// El compositor de BookOS.
    Escritorio,
    /// `bookos-teclasd`.
    Sistema,
}

async fn conectar(motor: Motor) -> zbus::Result<zbus::Proxy<'static>> {
    let (conexion, nombre, ruta) = match motor {
        Motor::Escritorio => (
            SESION.get_or_try_init(zbus::Connection::session).await?,
            "org.bookos.Desktop",
            "/org/bookos/Desktop",
        ),
        Motor::Sistema => (
            SISTEMA.get_or_try_init(zbus::Connection::system).await?,
            "org.bookos.Teclas",
            "/org/bookos/Teclas",
        ),
    };
    zbus::Proxy::new(conexion, nombre, ruta, nombre).await
}

/// El compositor manda si está, porque es el único con las funciones de
/// BookOS. Si `bookos-teclasd` también corre, sigue por debajo con lo suyo.
async fn motor() -> Option<Motor> {
    *MOTOR
        .get_or_init(|| async {
            if escritorio_sabe_remapear().await {
                Some(Motor::Escritorio)
            } else if servicio_responde().await {
                Some(Motor::Sistema)
            } else {
                None
            }
        })
        .await
}

async fn escritorio_sabe_remapear() -> bool {
    let Ok(proxy) = conectar(Motor::Escritorio).await else {
        return false;
    };
    let capacidades: zbus::Result<HashMap<String, OwnedValue>> =
        proxy.call("GetCapabilities", &()).await;
    capacidades.is_ok_and(|c| {
        c.get("key_remap")
            .is_some_and(|v| matches!(&**v, Value::Bool(true)))
    })
}

/// Se llama a un método en vez de mirar si el nombre está en el bus: así D-Bus
/// arranca el servicio si está instalado pero parado.
async fn servicio_responde() -> bool {
    let Ok(proxy) = conectar(Motor::Sistema).await else {
        return false;
    };
    proxy
        .call::<_, _, String>("GetKeyRemaps", &())
        .await
        .is_ok()
}

async fn proxy() -> Result<zbus::Proxy<'static>, String> {
    let motor = motor()
        .await
        .ok_or_else(|| "no hay escritorio BookOS ni bookos-teclasd".to_string())?;
    conectar(motor).await.map_err(|e| e.to_string())
}

/// Quién aplica los remapeos: `"escritorio"`, `"sistema"` o nadie. La interfaz
/// lo usa para avisar de que no hay y para no ofrecer lo que `bookos-teclasd`
/// no sabe hacer.
#[tauri::command]
pub async fn motor_remapeo() -> Option<&'static str> {
    motor().await.map(|motor| match motor {
        Motor::Escritorio => "escritorio",
        Motor::Sistema => "sistema",
    })
}

/// La configuración del escritorio (`GetConfig`) tal cual, en JSON. Solo la
/// tiene el compositor de BookOS. La página solo mira `teclado`, para arrancar
/// con la distribución de la sesión.
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
