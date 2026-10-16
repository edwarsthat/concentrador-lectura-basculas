use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    app::state::AppState,
    controller::dispositivos::{hora_actual, router_eventos},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dispositivos/eventos", post(router_eventos))
        .route("/dispositivos/hora", get(hora_actual))
}
