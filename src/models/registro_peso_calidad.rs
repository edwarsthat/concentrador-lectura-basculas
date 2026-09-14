use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Una lectura de peso tal como la manda un ESP32 y como queda guardada en
/// SQLite. Los campos van agrupados por quién los produce.
#[derive(Debug, FromRow)]
pub struct RegistroPesoCalidad {
    // ── identidad del registro ─────────────────────
    /// Lo genera el servidor al insertar.
    pub id: Uuid,
    /// La manda el dispositivo: si repite el POST (reintento por timeout),
    /// el servidor reconoce la lectura y no la duplica.
    pub clave_idempotencia: Uuid,

    // ── lo que midió y envió el dispositivo ────────
    /// Qué báscula lo mandó.
    pub device_id: String,
    /// Tarjeta NFC leída junto con el peso (quién / qué se pesó).
    pub llave_nfc: String,
    pub peso: f64,
    /// Cuándo lo midió el ESP32, según su propio reloj.
    pub timestamp_dispositivo: DateTime<Utc>,

    // ── lo que añade el servidor ───────────────────
    /// Cuándo lo recibió la Raspberry.
    pub recibido_en: DateTime<Utc>,
    /// Estado del reenvío al servidor externo.
    pub upstream: String,
}
