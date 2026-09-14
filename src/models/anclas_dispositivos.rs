use chrono::{DateTime, TimeDelta, Utc};
use sqlx::FromRow;

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
