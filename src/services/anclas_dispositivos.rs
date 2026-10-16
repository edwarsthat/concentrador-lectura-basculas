use chrono::Utc;
use sqlx::SqlitePool;

use crate::models::anclas_dispositivos::{AnclaDispositivo, ArranqueValidado};

/// Guarda el ancla del arranque. Devuelve false si ya existía (reintento).
pub async fn create_service_registro_anclas_dispositivos(
    pool: &SqlitePool,
    datos: ArranqueValidado,
) -> Result<bool, sqlx::Error> {
    let ancla = AnclaDispositivo::desde_arranque(datos, Utc::now());

    // Si el ESP32 reintenta el POST (timeout), la PK (device_id, sesion_arranque)
    // ya existe: se conserva el ancla original.
    let resultado = sqlx::query(
        "INSERT INTO anclas_dispositivos
            (device_id, sesion_arranque, transaccion, monotonico_ancla_us,
             hora_dispositivo, hora_pi_ancla, ancla_aproximada, recibido_en, version_firmware)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (device_id, sesion_arranque) DO NOTHING",
    )
    .bind(&ancla.device_id)
    .bind(ancla.sesion_arranque)
    .bind(ancla.transaccion)
    .bind(ancla.monotonico_ancla_us)
    .bind(ancla.hora_dispositivo)
    .bind(ancla.hora_pi_ancla)
    .bind(ancla.ancla_aproximada)
    .bind(ancla.recibido_en)
    .bind(&ancla.version_firmware)
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}

/// El ancla de una sesión, si su arranque ya llegó.
pub async fn buscar_ancla(
    pool: &SqlitePool,
    device_id: &str,
    sesion_arranque: i64,
) -> Result<Option<AnclaDispositivo>, sqlx::Error> {
    sqlx::query_as::<_, AnclaDispositivo>(
        "SELECT device_id, sesion_arranque, transaccion, monotonico_ancla_us,
                hora_dispositivo, hora_pi_ancla, ancla_aproximada, recibido_en, version_firmware
         FROM anclas_dispositivos
         WHERE device_id = ? AND sesion_arranque = ?",
    )
    .bind(device_id)
    .bind(sesion_arranque)
    .fetch_optional(pool)
    .await
}
