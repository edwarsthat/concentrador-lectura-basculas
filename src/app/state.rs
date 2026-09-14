use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;

use crate::app::config::Config;
use crate::app::error::AppError;
use crate::db::sqlite::connect;

/// Lo que comparten todos los handlers. Se clona en cada request,
/// por eso todos los campos son baratos de clonar.
#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
}

impl AppState {
    /// Construye todo lo que la app necesita: lee el entorno,
    /// abre la conexión a la base y devuelve el estado compartido.
    pub async fn new() -> Result<Self, AppError> {
        let config = Config::from_env()?;
        let pool = connect(&config.database_url).await?;
        let http = reqwest::Client::builder()
            .timeout(config.upstream_timeout)
            .connect_timeout(Duration::from_secs(5))
            .user_agent(concat!("concentrador-bascula/", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self {
            pool,
            config: Arc::new(config),
            http
        })
    }
}
