use chrono::Utc;
use sqlx::SqlitePool;

use crate::{
    models::registro_peso_calidad::{PesajeValidado, RegistroPesoCalidad},
    services::anclas_dispositivos::buscar_ancla,
};

/// Guarda el pesaje. Devuelve false si ya existía (reintento).
pub async fn crear_pesaje(pool: &SqlitePool, datos: PesajeValidado) -> Result<bool, sqlx::Error> {
    // Solo hace falta el ancla si el ESP32 no trajo hora.
    let ancla = match datos.hora {
        Some(_) => None,
        None => buscar_ancla(pool, &datos.device_id, datos.sesion_arranque).await?,
    };

    let registro = RegistroPesoCalidad::desde_pesaje(datos, ancla.as_ref(), Utc::now());

    if registro.hora_medicion.is_none() {
        // Se guarda igual: rechazarlo bloquearía la cola de la báscula.
        tracing::warn!(
            device_id = %registro.device_id,
            sesion_arranque = registro.sesion_arranque,
            transaccion = registro.transaccion,
            "pesaje sin hora ni ancla de su sesión"
        );
    }

    let resultado = sqlx::query(
        "INSERT INTO registros_peso_calidad
            (id, device_id, sesion_arranque, transaccion, uid_tarjeta, peso_kg,
             hora_dispositivo, monotonico_us, hora_medicion, recibido_en, upstream)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (device_id, sesion_arranque, transaccion) DO NOTHING",
    )
    // Uuid a secas se guarda como BLOB en SQLite; la columna es TEXT.
    .bind(registro.id.hyphenated())
    .bind(&registro.device_id)
    .bind(registro.sesion_arranque)
    .bind(registro.transaccion)
    .bind(&registro.uid_tarjeta)
    .bind(registro.peso_kg)
    .bind(registro.hora_dispositivo)
    .bind(registro.monotonico_us)
    .bind(registro.hora_medicion)
    .bind(registro.recibido_en)
    .bind(&registro.upstream)
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}
