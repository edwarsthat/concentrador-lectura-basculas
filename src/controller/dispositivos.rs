use axum::{Json, extract::State};
use reqwest::StatusCode;

use crate::{
    app::{api_error::ApiError, state::AppState}, 
    models::anclas_dispositivos::ArranqueRequest, 
    services, validations::comun::Validar
};

pub async fn create_registro_anclas_dispositivos(
    State(state): State<AppState>,
    Json(body): Json<ArranqueRequest>,
) -> Result<StatusCode, ApiError> {
    let datos = body.validar()?; 
    services::anclas_dispositivos::create_service_registro_anclas_dispositivos(&state.pool, datos).await?;
    Ok(StatusCode::CREATED)
}
