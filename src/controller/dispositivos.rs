use axum::{Json, extract::State, http::HeaderMap};
use reqwest::StatusCode;

use crate::{
    app::{api_error::ApiError, state::AppState},
    models::anclas_dispositivos::ArranqueRequest,
    services,
    validations::comun::Validar,
};

pub async fn create_registro_anclas_dispositivos(
    State(state): State<AppState>,
    Json(body): Json<ArranqueRequest>,
) -> Result<StatusCode, ApiError> {
    let datos = body.validar()?;
    services::anclas_dispositivos::create_service_registro_anclas_dispositivos(&state.pool, datos)
        .await?;
    Ok(StatusCode::CREATED)
}

pub async fn router_eventos(
    State(_state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, ApiError> {
    tracing::info!(?headers, "headers del evento");
    tracing::info!(%body, "cuerpo del evento");
    Ok(StatusCode::OK)
}
