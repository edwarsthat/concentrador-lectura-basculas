use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Un pesaje tal como queda guardado en SQLite. Los campos van agrupados por
/// quién los produce.
#[derive(Debug, FromRow)]
pub struct RegistroPesoCalidad {
    // ── identidad del registro ─────────────────────
    /// Lo genera el servidor al insertar.
    pub id: Uuid,
    /// La terna (device_id, sesion_arranque, transaccion) identifica el evento
    /// en la báscula: un reintento trae la misma y no se duplica.
    pub device_id: String,
    pub sesion_arranque: i64,
    pub transaccion: i64,

    // ── lo que midió y envió el dispositivo ────────
    /// Tarjeta NFC leída junto con el peso (quién / qué se pesó).
    pub uid_tarjeta: String,
    pub peso_kg: f64,
    /// `hora` del ESP32; None si su reloj no era válido.
    pub hora_dispositivo: Option<DateTime<Utc>>,
    /// `esp_timer_get_time()` al pesar. Solo como dato de diagnóstico.
    pub monotonico_us: i64,

    // ── lo que añade el servidor ───────────────────
    /// Hora final del pesaje; hoy es la misma que `hora_dispositivo`.
    pub hora_medicion: Option<DateTime<Utc>>,
    /// Cuándo lo recibió la Raspberry.
    pub recibido_en: DateTime<Utc>,
    /// Estado del reenvío al servidor externo.
    pub upstream: String,
}

/// Lo que trae un evento de pesaje, ya parseado.
#[derive(Debug)]
pub struct PesajeValidado {
    pub device_id: String,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: String,
    pub peso_kg: f64,
    pub hora: DateTime<Utc>,
    pub monotonico_us: i64,
}

impl RegistroPesoCalidad {
    /// Arma la fila. La hora es la del RTC del ESP32, ya validada al parsear.
    pub fn desde_pesaje(datos: PesajeValidado, ahora: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            device_id: datos.device_id,
            sesion_arranque: datos.sesion_arranque,
            transaccion: datos.transaccion,
            uid_tarjeta: datos.uid_tarjeta,
            peso_kg: datos.peso_kg,
            hora_dispositivo: Some(datos.hora),
            monotonico_us: datos.monotonico_us,
            hora_medicion: Some(datos.hora),
            recibido_en: ahora,
            upstream: "pendiente".into(),
        }
    }
}
