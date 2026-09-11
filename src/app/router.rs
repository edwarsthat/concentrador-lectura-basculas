use axum::{Router, extract::State, middleware, routing::get};

use crate::{
    app::state::AppState,
    security::rate_limit::{RateLimiter, limitar},
};

// Límite por IP de cada endpoint, en (pico de golpe, ritmo sostenido por minuto).
///
/// El pico es lo que se tolera de una tacada con la cubeta llena; el ritmo es lo
/// que queda una vez agotado. Los dos números conviven: un cliente puede gastar
/// el pico entero y después sigue pasando al ritmo sostenido.
mod limites {
    /// Es el endpoint más barato del servicio y lo consulta el monitoreo, así
    /// que va holgado; el tope está para que no sirva de ping flood gratis.
    pub const HEALTH: (u32, u32) = (60, 60);
}
/// Cada endpoint estrena su propia cubeta: gastar el cupo de `/health` no puede
/// dejar a nadie fuera de `/login`.
macro_rules! rate_limit {
    ($limite:expr) => {{
        let (pico, por_minuto) = $limite;
        middleware::from_fn_with_state(RateLimiter::por_minuto(pico, por_minuto), limitar)
    }};
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health).layer(rate_limit!(limites::HEALTH)))
        .with_state(state)
}

async fn health(State(_): State<AppState>) -> &'static str {
    "ok"
}
