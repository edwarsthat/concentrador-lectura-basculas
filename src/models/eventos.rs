use serde::Deserialize;
use serde_json::Value;

/// Cuerpo de POST /dispositivos/eventos. El `registro` llega sin tipar porque
/// en un reenvío tras un 422 es justamente el que no pasó la validación, y
/// tiene que poder guardarse tal cual.
#[derive(Debug, Deserialize)]
pub struct EventoEntrante {
    pub device_id: String,
    /// Solo viene (en true) cuando es un reenvío después de un 422.
    #[serde(default)]
    pub rechazado: bool,
    pub registro: Value,
}

/// Registro ya tipado. Los campos comunes a todo evento van aquí y los
/// propios de cada tipo en `evento`, elegidos por el campo `tipo`.
#[derive(Debug, Deserialize)]
pub struct Registro {
    pub transaccion: u32,
    /// Unix epoch en segundos; None si el reloj no tenía hora válida.
    pub hora: Option<i64>,
    pub sesion_arranque: i64,
    pub monotonico_us: i64,
    #[serde(flatten)]
    pub evento: Evento,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Evento {
    Arranque { version_firmware: String },
    Pesaje { uid_tarjeta: String, peso_kg: f64 },
}
