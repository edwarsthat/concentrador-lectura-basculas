use axum::{Router, routing::post};

use crate::{app::state::AppState, controller::dispositivos::create_registro_anclas_dispositivos};


pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/dispositivos/arranque", 
            post(create_registro_anclas_dispositivos)
        )
}