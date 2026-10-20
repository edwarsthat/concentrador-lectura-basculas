use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,

    #[error("no se pudo conectar a SQLite: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("no se pudieron aplicar las migraciones: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("no se pudo crear la carpeta de la base de datos: {0}")]
    Io(#[from] std::io::Error),
}
