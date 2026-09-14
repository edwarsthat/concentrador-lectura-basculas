use std::net::SocketAddr;

use crate::app::{error::AppError, router::build_router, state::AppState};

pub mod app;
pub mod db;
pub mod security;
pub mod routes;
pub mod models;
pub mod controller;
pub mod services;
pub mod validations;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let state = AppState::new().await?;
    let app = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    tracing::info!("escuchando en http://{}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>()
    ).await?;

    Ok(())
}
