use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// 2026-01-01T00:00:00Z. Antes de esto el sistema no existía: una hora
/// anterior significa que el RTC se reinició (pila agotada, sin configurar).
const HORA_MINIMA: i64 = 1_767_225_600;

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
    /// Unix epoch en segundos, del RTC del ESP32. Si falta, no es una fecha
    /// válida o es anterior a `HORA_MINIMA`, el registro no deserializa y se responde 422.
    #[serde(deserialize_with = "hora_rtc")]
    pub hora: DateTime<Utc>,
    pub sesion_arranque: i64,
    pub monotonico_us: i64,
    #[serde(flatten)]
    pub evento: Evento,
}

/// Va dentro de serde para que el reenvío con `rechazado` recalcule el mismo motivo.
fn hora_rtc<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
    let hora = chrono::serde::ts_seconds::deserialize(d)?;
    if hora.timestamp() < HORA_MINIMA {
        return Err(serde::de::Error::custom(format!(
            "hora {hora} anterior a 2026-01-01: el RTC parece reiniciado"
        )));
    }
    Ok(hora)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Evento {
    Arranque { version_firmware: String },
    Pesaje { uid_tarjeta: String, peso_kg: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pesaje(hora: Value) -> Value {
        json!({
            "transaccion": 1,
            "hora": hora,
            "sesion_arranque": 1,
            "monotonico_us": 5_000_000,
            "tipo": "pesaje",
            "uid_tarjeta": "04A1B2C3",
            "peso_kg": 12.5
        })
    }

    #[test]
    fn registro_convierte_la_hora_unix() {
        let r: Registro = serde_json::from_value(pesaje(json!(1_789_000_000))).unwrap();
        assert_eq!(r.hora.timestamp(), 1_789_000_000);
    }

    #[test]
    fn registro_rechaza_hora_nula() {
        assert!(serde_json::from_value::<Registro>(pesaje(Value::Null)).is_err());
    }

    #[test]
    fn registro_rechaza_hora_ausente() {
        let mut v = pesaje(json!(0));
        v.as_object_mut().unwrap().remove("hora");
        assert!(serde_json::from_value::<Registro>(v).is_err());
    }

    #[test]
    fn registro_rechaza_hora_de_rtc_reiniciado() {
        // 2000-01-01, la fecha típica de un RTC sin pila
        let e = serde_json::from_value::<Registro>(pesaje(json!(946_684_800))).unwrap_err();
        assert!(e.to_string().contains("RTC parece reiniciado"));
    }

    #[test]
    fn registro_acepta_justo_la_hora_minima() {
        let r: Registro = serde_json::from_value(pesaje(json!(HORA_MINIMA))).unwrap();
        assert_eq!(r.hora, "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn registro_rechaza_un_segundo_antes_de_la_hora_minima() {
        assert!(serde_json::from_value::<Registro>(pesaje(json!(HORA_MINIMA - 1))).is_err());
    }

    #[test]
    fn registro_rechaza_hora_fuera_de_rango() {
        assert!(serde_json::from_value::<Registro>(pesaje(json!(i64::MAX))).is_err());
    }
}
