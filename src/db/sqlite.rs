use crate::db::error::ConnectError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

pub async fn connect(database_url: &str) -> Result<SqlitePool, ConnectError> {
    let options = SqliteConnectOptions::from_str(database_url)?;

    // `create_if_missing` crea el archivo pero no la carpeta: sin esto, un
    // `sqlite://data/x.db` en una Pi recién instalada falla al abrir.
    if let Some(dir) = options.get_filename().parent() {
        std::fs::create_dir_all(dir)?;
    }

    let options = options
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Las migraciones de ./migrations se embeben en el binario al compilar y se
    // aplican aquí las que falten, así en la Pi no hay que correr sqlx-cli.
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
