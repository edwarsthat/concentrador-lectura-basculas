use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::validations::comun::{ValidacionError, en_rango, no_negativo, texto};

/// 2026-01-01T00:00:00Z. Antes de esto el sistema no existía: una hora
/// anterior significa que el RTC se reinició (pila agotada, sin configurar).
const HORA_MINIMA: i64 = 1_767_225_600;

/// El firmware solo registra canastas estables por encima de 5 kg
/// (`PESO_MINIMO_KG`); por debajo de esto es una lectura rota.
const PESO_MINIMO_KG: f64 = 1.0;
/// Techo de cordura: ninguna canasta pesa esto. Ajustar si cambia la báscula.
const PESO_MAXIMO_KG: f64 = 500.0;

/// MAC del ESP32 (`70:4b:ca:99:3f:30`) con margen.
const LARGO_DEVICE_ID: usize = 64;
/// UID NFC en hex: 4, 7 o 10 bytes, o sea 8, 14 o 20 caracteres.
const LARGO_UID: usize = 32;
const LARGO_VERSION: usize = 32;

/// Cuerpo de POST /dispositivos/eventos. El `registro` llega sin tipar porque
/// en un reenvío tras un 422 es justamente el que no pasó la validación, y
/// tiene que poder guardarse tal cual.
///
/// `device_id` y `registro` tienen default para que el sobre deserialice aunque
/// falten: si no, un reenvío con `rechazado` recibiría 422 otra vez y atascaría
/// la cola. En el camino normal, `registro_validado` los rechaza igual (422).
#[derive(Debug, Deserialize)]
pub struct EventoEntrante {
    #[serde(default)]
    pub device_id: String,
    /// Solo viene (en true) cuando es un reenvío después de un 422.
    #[serde(default)]
    pub rechazado: bool,
    #[serde(default)]
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

/// Resultado de `registro_validado`: todo ya recortado y normalizado.
/// El controlador guarda esto, nunca los campos de `EventoEntrante`.
#[derive(Debug)]
pub struct EventoValidado {
    pub device_id: String,
    pub registro: Registro,
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

impl EventoEntrante {
    /// Tipa y valida el registro. Lo usan los dos caminos: el normal responde
    /// 422 con el error, y el reenvío con `rechazado` lo guarda como `motivo`,
    /// así los dos dan siempre el mismo diagnóstico.
    ///
    /// No depende de la hora de la Pi: un reloj desfasado no invalida un pesaje.
    /// Se guardan la hora del ESP32 y la de llegada, y el servidor añade la suya.
    pub fn registro_validado(&self) -> Result<EventoValidado, ValidacionError> {
        let device_id = texto(self.device_id.clone(), LARGO_DEVICE_ID, "device_id")?;

        let registro = Registro::deserialize(&self.registro)
            .map_err(|e| ValidacionError::nuevo(e.to_string()))?
            .validar()?;

        Ok(EventoValidado {
            device_id,
            registro,
        })
    }
}

impl Registro {
    /// Reglas que serde no puede expresar. Solo mira el registro, así que el
    /// mismo registro siempre da el mismo resultado.
    fn validar(mut self) -> Result<Self, ValidacionError> {
        no_negativo(self.sesion_arranque, "sesion_arranque")?;
        no_negativo(self.monotonico_us, "monotonico_us")?;

        match &mut self.evento {
            Evento::Arranque { version_firmware } => {
                *version_firmware = texto(
                    std::mem::take(version_firmware),
                    LARGO_VERSION,
                    "version_firmware",
                )?;
            }
            Evento::Pesaje {
                uid_tarjeta,
                peso_kg,
            } => {
                let uid = texto(std::mem::take(uid_tarjeta), LARGO_UID, "uid_tarjeta")?
                    .to_ascii_uppercase();
                if !uid.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(ValidacionError::nuevo("`uid_tarjeta` debe ser hexadecimal"));
                }
                en_rango(*peso_kg, PESO_MINIMO_KG, PESO_MAXIMO_KG, "peso_kg")?;
                *uid_tarjeta = uid;
            }
        }
        Ok(self)
    }
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
        assert_eq!(
            r.hora,
            "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn registro_rechaza_un_segundo_antes_de_la_hora_minima() {
        assert!(serde_json::from_value::<Registro>(pesaje(json!(HORA_MINIMA - 1))).is_err());
    }

    #[test]
    fn registro_rechaza_hora_fuera_de_rango() {
        assert!(serde_json::from_value::<Registro>(pesaje(json!(i64::MAX))).is_err());
    }

    // ── registro_validado ───────────────────────────

    const HORA_OK: i64 = 1_789_000_000;

    fn entrante(registro: Value) -> EventoEntrante {
        EventoEntrante {
            device_id: "70:4b:ca:99:3f:30".into(),
            rechazado: false,
            registro,
        }
    }

    /// Pesaje válido con un campo cambiado.
    fn pesaje_con(campo: &str, valor: Value) -> EventoEntrante {
        let mut v = pesaje(json!(HORA_OK));
        v[campo] = valor;
        entrante(v)
    }

    fn motivo(e: EventoEntrante) -> String {
        e.registro_validado().unwrap_err().mensaje().to_owned()
    }

    #[test]
    fn valida_un_pesaje_correcto() {
        let r = pesaje_con("peso_kg", json!(22.0))
            .registro_validado()
            .unwrap();
        assert!(matches!(r.registro.evento, Evento::Pesaje { peso_kg, .. } if peso_kg == 22.0));
    }

    #[test]
    fn valida_un_arranque_correcto() {
        let e = entrante(json!({
            "transaccion": 10, "hora": HORA_OK, "sesion_arranque": 53,
            "monotonico_us": 20_471_666, "tipo": "arranque", "version_firmware": "0.1.0"
        }));
        assert!(e.registro_validado().is_ok());
    }

    #[test]
    fn rechaza_peso_fuera_de_rango() {
        assert!(motivo(pesaje_con("peso_kg", json!(-3.0))).contains("peso_kg"));
        assert!(motivo(pesaje_con("peso_kg", json!(0.0))).contains("peso_kg"));
        assert!(motivo(pesaje_con("peso_kg", json!(PESO_MAXIMO_KG + 0.1))).contains("peso_kg"));
    }

    #[test]
    fn rechaza_uid_vacio_o_no_hex() {
        assert!(motivo(pesaje_con("uid_tarjeta", json!("  "))).contains("uid_tarjeta"));
        assert!(motivo(pesaje_con("uid_tarjeta", json!("ZZZZ"))).contains("hexadecimal"));
        let largo = "A".repeat(LARGO_UID + 1);
        assert!(motivo(pesaje_con("uid_tarjeta", json!(largo))).contains("máximo"));
    }

    #[test]
    fn rechaza_contadores_negativos() {
        assert!(motivo(pesaje_con("monotonico_us", json!(-1))).contains("monotonico_us"));
        assert!(motivo(pesaje_con("sesion_arranque", json!(-1))).contains("sesion_arranque"));
    }

    #[test]
    fn acepta_cualquier_hora_posterior_a_la_minima() {
        // No se compara con la hora de la Pi: el operario cruza las tres horas
        // (ESP32, Pi, servidor) si algo no cuadra.
        let adelantada = HORA_OK + 3 * 24 * 3600;
        assert!(
            pesaje_con("hora", json!(adelantada))
                .registro_validado()
                .is_ok()
        );
    }

    #[test]
    fn normaliza_device_id_y_uid() {
        let mut e = pesaje_con("uid_tarjeta", json!(" 04a1b2c3 "));
        e.device_id = " 70:4b:ca:99:3f:30 ".into();
        let v = e.registro_validado().unwrap();
        assert_eq!(v.device_id, "70:4b:ca:99:3f:30");
        assert!(matches!(
            v.registro.evento,
            Evento::Pesaje { ref uid_tarjeta, .. } if uid_tarjeta == "04A1B2C3"
        ));
    }

    #[test]
    fn recorta_version_firmware() {
        let e = entrante(json!({
            "transaccion": 10, "hora": HORA_OK, "sesion_arranque": 53,
            "monotonico_us": 20_471_666, "tipo": "arranque", "version_firmware": " 0.1.0 "
        }));
        let v = e.registro_validado().unwrap();
        assert!(matches!(
            v.registro.evento,
            Evento::Arranque { ref version_firmware } if version_firmware == "0.1.0"
        ));
    }

    #[test]
    fn reenvio_rechazado_sin_device_id_deserializa() {
        let e: EventoEntrante = serde_json::from_value(json!({
            "rechazado": true,
            "registro": pesaje(json!(HORA_OK))
        }))
        .unwrap();
        assert!(e.rechazado);
        assert!(motivo(e).contains("device_id"));
    }

    #[test]
    fn reenvio_rechazado_sin_registro_deserializa() {
        let e: EventoEntrante = serde_json::from_value(json!({
            "device_id": "70:4b:ca:99:3f:30",
            "rechazado": true
        }))
        .unwrap();
        assert!(e.rechazado);
        assert!(e.registro.is_null());
        assert!(e.registro_validado().is_err());
    }

    #[test]
    fn sobre_sin_device_id_sigue_siendo_invalido() {
        let e: EventoEntrante = serde_json::from_value(json!({
            "registro": pesaje(json!(HORA_OK))
        }))
        .unwrap();
        assert!(!e.rechazado);
        assert!(motivo(e).contains("device_id"));
    }

    #[test]
    fn rechaza_device_id_vacio() {
        let mut e = pesaje_con("peso_kg", json!(22.0));
        e.device_id = "".into();
        assert!(motivo(e).contains("device_id"));
    }

    #[test]
    fn el_error_de_serde_tambien_es_motivo() {
        assert!(motivo(pesaje_con("hora", json!(946_684_800))).contains("RTC parece reiniciado"));
    }
}
