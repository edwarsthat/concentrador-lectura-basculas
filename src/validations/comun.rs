use chrono::{DateTime, Utc};
use uuid::Uuid;


#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ValidacionError(String);

impl ValidacionError {
    pub fn nuevo(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    pub fn mensaje(&self) -> &str {
        &self.0
    }
}

pub trait Validar {
    type Datos;

    fn validar(self) -> Result<Self::Datos, ValidacionError>;
}

/// Falla si el campo no vino en el cuerpo. Sirve para cualquier tipo:
/// `requerido(body.peso, "peso")?` devuelve el `f64` ya desenvuelto.
pub fn requerido<T>(valor: Option<T>, campo: &str) -> Result<T, ValidacionError> {
    valor.ok_or_else(|| ValidacionError::nuevo(format!("falta el campo `{campo}`")))
}

/// Parsea un UUID ignorando espacios alrededor.
pub fn uuid(valor: &str, campo: &str) -> Result<Uuid, ValidacionError> {
    Uuid::parse_str(valor.trim())
        .map_err(|_| ValidacionError::nuevo(format!("`{campo}` no es un UUID válido")))
}

/// Parsea una fecha en RFC 3339 / ISO 8601 (`2026-09-13T10:00:00Z` o con
/// offset `-05:00`) y la normaliza a UTC.
pub fn fecha(valor: &str, campo: &str) -> Result<DateTime<Utc>, ValidacionError> {
    DateTime::parse_from_rfc3339(valor.trim())
        .map(|f| f.with_timezone(&Utc))
        .map_err(|_| ValidacionError::nuevo(format!("`{campo}` no es una fecha válida (se espera RFC 3339)")))
}

/// Rechaza valores negativos (contadores, microsegundos, cantidades).
/// Sirve para cualquier numérico: i64, f64, etc.
pub fn no_negativo<T: PartialOrd + Default + Copy>(valor: T, campo: &str) -> Result<T, ValidacionError> {
    if valor < T::default() {
        return Err(ValidacionError::nuevo(format!("`{campo}` no puede ser negativo")));
    }
    Ok(valor)
}

/// Verifica que el valor esté dentro de `[min, max]`, ambos incluidos.
pub fn en_rango<T: PartialOrd + Copy + std::fmt::Display>(
    valor: T,
    min: T,
    max: T,
    campo: &str,
) -> Result<T, ValidacionError> {
    if valor < min || valor > max {
        return Err(ValidacionError::nuevo(format!("`{campo}` debe estar entre {min} y {max}")));
    }
    Ok(valor)
}

/// Texto obligatorio (p. ej. `device_id`, `llave_nfc`): no puede estar vacío
/// ni ser solo espacios. Se recorta y se limita a `largo_max` caracteres.
pub fn texto(valor: String, largo_max: usize, campo: &str) -> Result<String, ValidacionError> {
    texto_opcional(Some(valor), largo_max, campo)?
        .ok_or_else(|| ValidacionError::nuevo(format!("`{campo}` no puede estar vacío")))
}

/// Texto que puede faltar (p. ej. `version_firmware`). Si no viene o viene
/// vacío/solo espacios se trata como ausente (`None`); si viene, se recorta y
/// se limita a `largo_max` caracteres.
pub fn texto_opcional(
    valor: Option<String>,
    largo_max: usize,
    campo: &str,
) -> Result<Option<String>, ValidacionError> {
    let Some(valor) = valor else { return Ok(None) };
    let recortado = valor.trim();
    if recortado.is_empty() {
        return Ok(None);
    }
    if recortado.chars().count() > largo_max {
        return Err(ValidacionError::nuevo(format!(
            "`{campo}` supera el máximo de {largo_max} caracteres"
        )));
    }
    Ok(Some(recortado.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requerido_devuelve_el_valor_si_esta() {
        assert_eq!(requerido(Some(7), "x").unwrap(), 7);
    }

    #[test]
    fn requerido_falla_si_falta() {
        let e = requerido(None::<i64>, "peso").unwrap_err();
        assert_eq!(e.mensaje(), "falta el campo `peso`");
    }

    #[test]
    fn uuid_acepta_valido_con_espacios() {
        let u = uuid("  550e8400-e29b-41d4-a716-446655440000 ", "clave").unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn uuid_rechaza_invalido() {
        let e = uuid("no-es-uuid", "clave_idempotencia").unwrap_err();
        assert_eq!(e.mensaje(), "`clave_idempotencia` no es un UUID válido");
    }

    #[test]
    fn fecha_acepta_utc() {
        let f = fecha("2026-09-13T10:00:00Z", "timestamp_dispositivo").unwrap();
        assert_eq!(f.to_rfc3339(), "2026-09-13T10:00:00+00:00");
    }

    #[test]
    fn fecha_convierte_offset_a_utc() {
        let f = fecha("2026-09-13T05:00:00-05:00", "timestamp_dispositivo").unwrap();
        assert_eq!(f, "2026-09-13T10:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn fecha_conserva_fraccion_de_segundo() {
        let f = fecha("2026-09-13T10:00:00.123Z", "t").unwrap();
        assert_eq!(f.timestamp_subsec_millis(), 123);
    }

    #[test]
    fn fecha_rechaza_formato_sin_hora() {
        let e = fecha("2026-09-13", "timestamp_dispositivo").unwrap_err();
        assert_eq!(
            e.mensaje(),
            "`timestamp_dispositivo` no es una fecha válida (se espera RFC 3339)"
        );
    }

    #[test]
    fn fecha_rechaza_texto_y_fechas_imposibles() {
        assert!(fecha("ayer", "t").is_err());
        assert!(fecha("2026-02-30T10:00:00Z", "t").is_err());
        assert!(fecha("2026-09-13T25:00:00Z", "t").is_err());
    }

    #[test]
    fn no_negativo_acepta_cero_y_positivos() {
        assert_eq!(no_negativo(0i64, "x").unwrap(), 0);
        assert_eq!(no_negativo(5_000_000i64, "x").unwrap(), 5_000_000);
        assert_eq!(no_negativo(12.5f64, "x").unwrap(), 12.5);
    }

    #[test]
    fn no_negativo_rechaza_negativos() {
        let e = no_negativo(-1i64, "monotonico_ancla_us").unwrap_err();
        assert_eq!(e.mensaje(), "`monotonico_ancla_us` no puede ser negativo");
        assert!(no_negativo(-0.1f64, "peso").is_err());
    }

    #[test]
    fn en_rango_acepta_los_extremos() {
        assert_eq!(en_rango(0.0, 0.0, 5000.0, "peso").unwrap(), 0.0);
        assert_eq!(en_rango(5000.0, 0.0, 5000.0, "peso").unwrap(), 5000.0);
        assert_eq!(en_rango(7i64, 1, 10, "x").unwrap(), 7);
    }

    #[test]
    fn en_rango_rechaza_fuera_del_intervalo() {
        let e = en_rango(5000.5, 0.0, 5000.0, "peso").unwrap_err();
        assert_eq!(e.mensaje(), "`peso` debe estar entre 0 y 5000");
        assert!(en_rango(0i64, 1, 10, "x").is_err());
    }

    #[test]
    fn texto_opcional_ausente_o_vacio_es_none() {
        assert_eq!(texto_opcional(None, 32, "version_firmware").unwrap(), None);
        assert_eq!(texto_opcional(Some("".into()), 32, "version_firmware").unwrap(), None);
        assert_eq!(texto_opcional(Some("   ".into()), 32, "version_firmware").unwrap(), None);
    }

    #[test]
    fn texto_opcional_recorta_espacios() {
        let v = texto_opcional(Some("  v1.2.3 ".into()), 32, "version_firmware").unwrap();
        assert_eq!(v.as_deref(), Some("v1.2.3"));
    }

    #[test]
    fn texto_opcional_respeta_largo_max_en_caracteres() {
        // 5 caracteres, aunque en bytes sean más por las tildes
        assert!(texto_opcional(Some("ñañañ".into()), 5, "x").unwrap().is_some());
        let e = texto_opcional(Some("ñañañá".into()), 5, "version_firmware").unwrap_err();
        assert_eq!(e.mensaje(), "`version_firmware` supera el máximo de 5 caracteres");
    }

    #[test]
    fn texto_recorta_y_devuelve_el_valor() {
        assert_eq!(texto("  bascula-1 ".into(), 64, "device_id").unwrap(), "bascula-1");
    }

    #[test]
    fn texto_rechaza_vacio_o_solo_espacios() {
        let e = texto("".into(), 64, "device_id").unwrap_err();
        assert_eq!(e.mensaje(), "`device_id` no puede estar vacío");
        assert!(texto("   ".into(), 64, "device_id").is_err());
    }

    #[test]
    fn texto_respeta_largo_max() {
        let e = texto("bascula-123".into(), 8, "device_id").unwrap_err();
        assert_eq!(e.mensaje(), "`device_id` supera el máximo de 8 caracteres");
    }
}
