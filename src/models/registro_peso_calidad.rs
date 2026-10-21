use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::{Uuid, fmt::Hyphenated};

/// Un pesaje tal como queda guardado en SQLite. Los campos van agrupados por
/// quién los produce.
#[derive(Debug, FromRow)]
pub struct RegistroPesoCalidad {
    // ── identidad del registro ─────────────────────
    /// Lo genera el servidor al insertar. La columna es TEXT y sqlx lee `Uuid`
    /// como BLOB, así que se decodifica como texto y se convierte.
    #[sqlx(try_from = "Hyphenated")]
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
    /// `hora` del RTC del ESP32. El evento ya la exige, así que hoy siempre viene;
    /// la columna admite NULL por las filas anteriores a ese cambio.
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
    /// Reenvíos fallidos hasta ahora.
    pub intentos: i64,
    /// Mensaje del último fallo de reenvío; None si no ha fallado.
    pub ultimo_error: Option<String>,
    /// Cuándo lo aceptó el servidor externo; None mientras siga pendiente.
    pub enviado_en: Option<DateTime<Utc>>,
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
            intentos: 0,
            ultimo_error: None,
            enviado_en: None,
        }
    }
}

/// Cuerpo de POST /api/v1/pesajes hacia la nube: el contrato de la API.
/// Va aparte de `RegistroPesoCalidad` para que una columna nueva en SQLite no
/// cambie sin querer lo que recibe la nube, y para no mandar el estado interno
/// del reenvío (`upstream`, `intentos`, `ultimo_error`, `enviado_en`).
///
/// Toma prestados los textos del registro: se arma, se serializa y se descarta.
#[derive(Debug, Serialize)]
pub struct PesajeNube<'a> {
    /// La nube deduplica por aquí: un reenvío tras un corte trae el mismo y responde 409.
    pub id: Uuid,
    pub device_id: &'a str,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: &'a str,
    pub peso_kg: f64,
    pub hora_medicion: Option<DateTime<Utc>>,
    pub hora_dispositivo: Option<DateTime<Utc>>,
    pub recibido_en: DateTime<Utc>,
}

impl<'a> From<&'a RegistroPesoCalidad> for PesajeNube<'a> {
    fn from(r: &'a RegistroPesoCalidad) -> Self {
        Self {
            id: r.id,
            device_id: &r.device_id,
            sesion_arranque: r.sesion_arranque,
            transaccion: r.transaccion,
            uid_tarjeta: &r.uid_tarjeta,
            peso_kg: r.peso_kg,
            hora_medicion: r.hora_medicion,
            hora_dispositivo: r.hora_dispositivo,
            recibido_en: r.recibido_en,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pesaje_nube_serializa_el_contrato_sin_estado_interno() {
        let hora: DateTime<Utc> = "2026-10-20T14:03:11Z".parse().unwrap();
        let datos = PesajeValidado {
            device_id: "70:4b:ca:99:3f:30".into(),
            sesion_arranque: 53,
            transaccion: 128,
            uid_tarjeta: "04A1B2C3".into(),
            peso_kg: 22.4,
            hora,
            monotonico_us: 5_000_000,
        };
        let registro = RegistroPesoCalidad::desde_pesaje(datos, hora);

        let v = serde_json::to_value(PesajeNube::from(&registro)).unwrap();

        assert_eq!(
            v,
            json!({
                "id": registro.id.hyphenated().to_string(),
                "device_id": "70:4b:ca:99:3f:30",
                "sesion_arranque": 53,
                "transaccion": 128,
                "uid_tarjeta": "04A1B2C3",
                "peso_kg": 22.4,
                "hora_medicion": "2026-10-20T14:03:11Z",
                "hora_dispositivo": "2026-10-20T14:03:11Z",
                "recibido_en": "2026-10-20T14:03:11Z"
            })
        );
    }
}
