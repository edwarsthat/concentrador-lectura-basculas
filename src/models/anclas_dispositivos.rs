use chrono::{DateTime, TimeDelta, Utc};
use sqlx::FromRow;

/// Ancla de reloj de un arranque del ESP32. Cada medición posterior de la
/// misma sesión trae su monotónico y se resuelve contra esta ancla.
#[derive(Debug, FromRow, Clone)]
pub struct AnclaDispositivo {
    pub device_id: String,
    pub sesion_arranque: i64,
    /// Nº de evento de la báscula que trajo el arranque.
    pub transaccion: i64,
    /// `esp_timer_get_time()` del ESP32 en el momento del ancla, en
    /// microsegundos. Es un contador desde el arranque, no una fecha.
    pub monotonico_ancla_us: i64,
    /// `hora` que mandó el ESP32; None si su reloj no era válido.
    pub hora_dispositivo: Option<DateTime<Utc>>,
    /// Hora usada como ancla: `hora_dispositivo` o, si falta, `recibido_en`.
    pub hora_pi_ancla: DateTime<Utc>,
    /// true si se usó `recibido_en`: solo es fiable si el arranque llegó sin demora.
    pub ancla_aproximada: bool,
    pub recibido_en: DateTime<Utc>,
    pub version_firmware: Option<String>,
}

impl AnclaDispositivo {
    /// Reconstruye la hora real de una medición a partir de su contador
    /// monotónico (microsegundos, mismo arranque que esta ancla).
    pub fn resolver(&self, monotonico_medicion_us: i64) -> DateTime<Utc> {
        let transcurrido =
            TimeDelta::microseconds(monotonico_medicion_us - self.monotonico_ancla_us);
        self.hora_pi_ancla + transcurrido
    }
}

/// Lo que trae un evento de arranque, ya parseado. Solo lo que aporta el dispositivo.
#[derive(Debug)]
pub struct ArranqueValidado {
    pub device_id: String,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub monotonico_us: i64,
    pub hora: Option<DateTime<Utc>>,
    pub version_firmware: String,
}

impl AnclaDispositivo {
    /// Arma la fila a partir del arranque. El arranque puede llegar horas tarde
    /// por la cola, así que manda la hora del ESP32; `ahora` es el último recurso.
    pub fn desde_arranque(datos: ArranqueValidado, ahora: DateTime<Utc>) -> Self {
        Self {
            device_id: datos.device_id,
            sesion_arranque: datos.sesion_arranque,
            transaccion: datos.transaccion,
            monotonico_ancla_us: datos.monotonico_us,
            hora_dispositivo: datos.hora,
            hora_pi_ancla: datos.hora.unwrap_or(ahora),
            ancla_aproximada: datos.hora.is_none(),
            recibido_en: ahora,
            version_firmware: Some(datos.version_firmware),
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
            transaccion: 1,
            monotonico_ancla_us,
            hora_dispositivo: None,
            hora_pi_ancla: "2026-09-13T10:00:00Z".parse().unwrap(),
            ancla_aproximada: false,
            recibido_en: "2026-09-13T10:00:00Z".parse().unwrap(),
            version_firmware: None,
        }
    }

    fn arranque(hora: Option<DateTime<Utc>>) -> ArranqueValidado {
        ArranqueValidado {
            device_id: "bascula-1".into(),
            sesion_arranque: 1,
            transaccion: 9,
            monotonico_us: 20_000_000,
            hora,
            version_firmware: "0.1.0".into(),
        }
    }

    #[test]
    fn resolver_suma_lo_transcurrido_desde_el_ancla() {
        let a = ancla(5_000_000);
        // 90 s después del ancla, en microsegundos
        let hora = a.resolver(5_000_000 + 90_000_000);
        assert_eq!(
            hora,
            "2026-09-13T10:01:30Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn resolver_conserva_los_milisegundos() {
        let a = ancla(0);
        let hora = a.resolver(1_234_000);
        assert_eq!(
            hora.timestamp_millis(),
            a.hora_pi_ancla.timestamp_millis() + 1_234
        );
    }

    #[test]
    fn desde_arranque_usa_la_hora_del_dispositivo_aunque_llegue_tarde() {
        // Caso real de los logs: arranque de la 01:55 que llegó a las 15:31.
        let hora_esp: DateTime<Utc> = "2026-10-16T01:55:27Z".parse().unwrap();
        let llegada: DateTime<Utc> = "2026-10-16T15:31:48Z".parse().unwrap();
        let a = AnclaDispositivo::desde_arranque(arranque(Some(hora_esp)), llegada);
        assert_eq!(a.hora_pi_ancla, hora_esp);
        assert_eq!(a.hora_dispositivo, Some(hora_esp));
        assert_eq!(a.recibido_en, llegada);
        assert!(!a.ancla_aproximada);
    }

    #[test]
    fn desde_arranque_sin_hora_usa_la_llegada_y_la_marca_aproximada() {
        let llegada: DateTime<Utc> = "2026-10-16T15:31:48Z".parse().unwrap();
        let a = AnclaDispositivo::desde_arranque(arranque(None), llegada);
        assert_eq!(a.hora_pi_ancla, llegada);
        assert_eq!(a.hora_dispositivo, None);
        assert!(a.ancla_aproximada);
    }

    #[test]
    fn desde_arranque_copia_los_datos_del_evento() {
        let a = AnclaDispositivo::desde_arranque(arranque(None), Utc::now());
        assert_eq!(a.transaccion, 9);
        assert_eq!(a.monotonico_ancla_us, 20_000_000);
        assert_eq!(a.version_firmware.as_deref(), Some("0.1.0"));
    }
}
