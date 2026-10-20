use thiserror::Error;

use crate::db::error::ConnectError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] ConnectError),

    #[error("La variable de entorno {0} es obligatoria")]
    MissingEnv(&'static str),

    #[error("La variable de entorno {0} tiene un valor invalido")]
    InvalidEnv(&'static str),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no se pudo construir el cliente HTTP: {0}")]
    Http(#[from] reqwest::Error),
}
