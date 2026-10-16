use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::validations::comun::ValidacionError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    CuerpoInvalido(&'static str),

    #[error("base de datos: {0}")]
    Db(#[from] sqlx::Error),

    #[error("{0}")]
    Validacion(#[from] ValidacionError),

    #[error("{0}")]
    NoProcesable(String),

    #[error("firma inválida")]
    FirmaInvalida,
}

#[derive(Serialize)]
struct CuerpoError {
    ok: bool,
    error: &'static str,
    mensaje: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, codigo) = match &self {
            ApiError::CuerpoInvalido(_) => (StatusCode::BAD_REQUEST, "cuerpo_invalido"),
            ApiError::Db(e) => {
                // El detalle va al log, no al ESP32: no le sirve y podría filtrar rutas/SQL.
                tracing::error!(error = %e, "fallo de base de datos");
                (StatusCode::INTERNAL_SERVER_ERROR, "error_interno")
            }
            ApiError::Validacion(_) => (StatusCode::BAD_REQUEST, "validacion"),
            ApiError::NoProcesable(_) => (StatusCode::UNPROCESSABLE_ENTITY, "no_procesable"),
            ApiError::FirmaInvalida => (StatusCode::UNAUTHORIZED, "firma_invalida"),
        };
        let mensaje = match &self {
            ApiError::Db(_) => "error interno, reintentar".to_string(),
            otro => otro.to_string(),
        };
        (
            status,
            Json(CuerpoError {
                ok: false,
                error: codigo,
                mensaje,
            }),
        )
            .into_response()
    }
}
