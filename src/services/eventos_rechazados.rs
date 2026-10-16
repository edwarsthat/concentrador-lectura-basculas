use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;

/// Guarda tal cual un registro que la báscula reenvió tras un 422.
/// Si el reenvío se repite, el UNIQUE (device_id, registro) lo ignora.
pub async fn guardar(
    pool: &SqlitePool,
    device_id: &str,
    registro: &Value,
    motivo: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO eventos_rechazados (device_id, registro, motivo, recibido_en)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (device_id, registro) DO NOTHING",
    )
    .bind(device_id)
    .bind(registro.to_string()) // serde_json::Value -> texto JSON
    .bind(motivo)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}
