use crate::app::{error::AppError, router::build_router, state::AppState};

pub mod app;
pub mod db;
pub mod security;

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
    let _app = build_router(state);

    todo!("montar el router y arrancar el servidor")
}
