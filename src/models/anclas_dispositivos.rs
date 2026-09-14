use chrono::{DateTime, TimeDelta, Utc};
use sqlx::FromRow;
use serde::Deserialize;

use crate::validations::comun::{
    Validar, 
    ValidacionError, 
    no_negativo, requerido, texto, texto_opcional};

/// Ancla de reloj de un arranque del ESP32. El dispositivo no tiene RTC: al
/// encender manda su contador monotónico y la Pi guarda su propia hora en ese
/// instante. Cada medición posterior trae su monotónico y se resuelve contra
/// esta ancla.
#[derive(Debug, FromRow, Clone)]
pub struct AnclaDispositivo {
    pub device_id: String,
    pub sesion_arranque: i64,
    /// `esp_timer_get_time()` del ESP32 en el momento del ancla, en
    /// microsegundos. Es un contador desde el arranque, no una fecha.
    pub monotonico_ancla_us: i64,
    /// Hora de la Pi cuando llegó el ancla.
    pub hora_pi_ancla: DateTime<Utc>,
    pub recibido_en: DateTime<Utc>,
    pub version_firmware: Option<String>,
}

impl AnclaDispositivo {
    /// Reconstruye la hora real de una medición a partir de su contador
    /// monotónico (microsegundos, mismo arranque que esta ancla).
    pub fn resolver(&self, monotonico_medicion_us: i64) -> DateTime<Utc> {
        let transcurrido = TimeDelta::microseconds(monotonico_medicion_us - self.monotonico_ancla_us);
        self.hora_pi_ancla + transcurrido
    }
}

/// Cuerpo del POST /dispositivos/arranque tal como llega. Todo opcional
/// para que la ausencia la reporte nuestra validación y no serde.
#[derive(Debug, Deserialize)]
pub struct ArranqueRequest {
    pub device_id: Option<String>,
    pub sesion_arranque: Option<i64>,
    pub monotonico_ancla_us: Option<i64>,
    pub version_firmware: Option<String>,
}

/// Lo mismo, ya comprobado. Solo lo que aporta el dispositivo.
#[derive(Debug)]
pub struct ArranqueValidado {
    pub device_id: String,
    pub sesion_arranque: i64,
    pub monotonico_ancla_us: i64,
    pub version_firmware: Option<String>,
}

impl Validar for ArranqueRequest {
    type Datos = ArranqueValidado;

    fn validar(self) -> Result<ArranqueValidado, ValidacionError> {
        Ok(ArranqueValidado {
            device_id: texto(requerido(self.device_id, "device_id")?, 64, "device_id")?,
            sesion_arranque: no_negativo(requerido(self.sesion_arranque, "sesion_arranque")?, "sesion_arranque")?,
            monotonico_ancla_us: no_negativo(requerido(self.monotonico_ancla_us, "monotonico_ancla_us")?, "monotonico_ancla_us")?,
            version_firmware: texto_opcional(self.version_firmware, 32, "version_firmware")?,
        })
    }
}

impl AnclaDispositivo {
    /// Arma la fila a partir de lo validado y la hora de la Pi en ese instante.
    pub fn desde_arranque(datos: ArranqueValidado, ahora: DateTime<Utc>) -> Self {
        Self {
            device_id: datos.device_id,
            sesion_arranque: datos.sesion_arranque,
            monotonico_ancla_us: datos.monotonico_ancla_us,
            hora_pi_ancla: ahora,
            recibido_en: ahora,
            version_firmware: datos.version_firmware,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ancla(monotonico_ancla_us: i64) -> AnclaDispositivo {
        AnclaDispositivo {
            device_id: "bascula-1".into(),
            sesion_arranque: 1,
            monotonico_ancla_us,
            hora_pi_ancla: "2026-09-13T10:00:00Z".parse().unwrap(),
            recibido_en: "2026-09-13T10:00:00Z".parse().unwrap(),
            version_firmware: None,
        }
    }

    #[test]
    fn resolver_suma_lo_transcurrido_desde_el_ancla() {
        let a = ancla(5_000_000);
        // 90 s después del ancla, en microsegundos
        let hora = a.resolver(5_000_000 + 90_000_000);
        assert_eq!(hora, "2026-09-13T10:01:30Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn resolver_conserva_los_milisegundos() {
        let a = ancla(0);
        let hora = a.resolver(1_234_000);
        assert_eq!(hora.timestamp_millis(), a.hora_pi_ancla.timestamp_millis() + 1_234);
    }
}
