use axum::{Router, routing::post};

use crate::{app::state::AppState, controller::dispositivos::router_eventos};

pub fn routes() -> Router<AppState> {
    Router::new().route("/dispositivos/eventos", post(router_eventos))
}
