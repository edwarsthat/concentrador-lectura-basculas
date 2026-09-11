use sqlx::SqlitePool;
use std::sync::Arc;

use crate::app::config::Config;
use crate::app::error::AppError;
use crate::db::sqlite::connect;

/// Lo que comparten todos los handlers. Se clona en cada request,
/// por eso todos los campos son baratos de clonar.
#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
}

impl AppState {
    /// Construye todo lo que la app necesita: lee el entorno,
    /// abre la conexión a la base y devuelve el estado compartido.
    pub async fn new() -> Result<Self, AppError> {
        let config = Config::from_env()?;
        let pool = connect(&config.database_url).await?;

        Ok(Self {
            pool,
            config: Arc::new(config),
        })
    }
}
