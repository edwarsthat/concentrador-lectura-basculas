use std::str::FromStr;
use std::time::Duration;

use crate::app::error::AppError;

/// Configuración leída una sola vez al arrancar.
#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub upstream_url: String,
    pub upstream_timeout: Duration,
    pub bind_addr: String,
    pub api_key: String,
    pub confiar_hora_sistema: bool,
    pub upstream_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            upstream_url: required("UPSTREAM_URL")?,
            api_key: required("API_KEY")?,
            upstream_timeout: Duration::from_secs(parsed("UPSTREAM_TIMEOUT_SECS", 10)?),
            bind_addr: optional("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:3000".into()),
            confiar_hora_sistema: parsed("CONFIAR_HORA_SISTEMA", false)?,
            upstream_api_key: required("UPSTREAM_API_KEY")?,
        })
    }
}

/// Obligatoria. Si falta o está vacía -> MissingEnv.
fn required(key: &'static str) -> Result<String, AppError> {
    optional(key).ok_or(AppError::MissingEnv(key))
}

/// Opcional, sin parseo. Nunca falla.
fn optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Opcional con valor por defecto. Si está pero no parsea -> InvalidEnv.
fn parsed<T: FromStr>(key: &'static str, default: T) -> Result<T, AppError> {
    match optional(key) {
        None => Ok(default),
        Some(v) => v.parse().map_err(|_| AppError::InvalidEnv(key)),
    }
}
