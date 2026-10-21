use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::registro_peso_calidad::{PesajeValidado, RegistroPesoCalidad};

/// Guarda el pesaje. Devuelve false si ya existía (reintento).
pub async fn crear_pesaje(pool: &SqlitePool, datos: PesajeValidado) -> Result<bool, sqlx::Error> {
    let registro = RegistroPesoCalidad::desde_pesaje(datos, Utc::now());

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

/// El pesaje pendiente más viejo, o None si no queda ninguno por enviar.
/// `rowid` desempata los que llegaron en el mismo instante: es el orden real de inserción.
pub async fn siguiente_pendiente(
    pool: &SqlitePool,
) -> Result<Option<RegistroPesoCalidad>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM registros_peso_calidad
         WHERE upstream = 'pendiente'
         ORDER BY recibido_en, rowid
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

/// Deja constancia de que la nube aceptó el pesaje (201 o 409).
/// Devuelve false si ya no estaba pendiente: no se pisa un estado ya decidido.
/// `intentos` y `ultimo_error` se conservan como historial de lo que costó enviarlo.
pub async fn marcar_enviado(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let resultado = sqlx::query(
        "UPDATE registros_peso_calidad
         SET upstream = 'enviado', enviado_en = ?
         WHERE id = ? AND upstream = 'pendiente'",
    )
    .bind(Utc::now())
    .bind(id.hyphenated())
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}

/// La nube dijo que el dato está mal (400/422): no se vuelve a intentar.
/// El motivo queda en `ultimo_error` para revisarlo a mano.
/// Devuelve false si ya no estaba pendiente, igual que `marcar_enviado`.
pub async fn marcar_rechazado(
    pool: &SqlitePool,
    id: Uuid,
    motivo: &str,
) -> Result<bool, sqlx::Error> {
    let resultado = sqlx::query(
        "UPDATE registros_peso_calidad
         SET upstream = 'rechazado', ultimo_error = ?
         WHERE id = ? AND upstream = 'pendiente'",
    )
    .bind(motivo)
    .bind(id.hyphenated())
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}

/// Anota un intento fallido (red, 5xx, llave mala). El registro sigue
/// pendiente: el fallo es de la conexión, no del dato.
pub async fn registrar_fallo(pool: &SqlitePool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE registros_peso_calidad
         SET intentos = intentos + 1, ultimo_error = ?
         WHERE id = ? AND upstream = 'pendiente'",
    )
    .bind(error)
    .bind(id.hyphenated())
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Base en memoria con las migraciones. Una sola conexión: cada conexión
    /// a `sqlite::memory:` abre una base distinta.
    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn pesaje(transaccion: i64) -> PesajeValidado {
        PesajeValidado {
            device_id: "70:4b:ca:99:3f:30".into(),
            sesion_arranque: 1,
            transaccion,
            uid_tarjeta: "04A1B2C3".into(),
            peso_kg: 12.5,
            hora: "2026-10-20T14:03:11Z".parse().unwrap(),
            monotonico_us: 5_000_000,
        }
    }

    #[tokio::test]
    async fn sin_pendientes_devuelve_none() {
        let pool = pool().await;
        assert!(siguiente_pendiente(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn devuelve_el_mas_viejo_primero() {
        let pool = pool().await;
        for t in [1, 2, 3] {
            crear_pesaje(&pool, pesaje(t)).await.unwrap();
        }
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();
        assert_eq!(r.transaccion, 1);
        assert_eq!(r.upstream, "pendiente");
        assert_eq!(r.intentos, 0);
    }

    #[tokio::test]
    async fn ignora_los_que_ya_no_estan_pendientes() {
        let pool = pool().await;
        for t in [1, 2] {
            crear_pesaje(&pool, pesaje(t)).await.unwrap();
        }
        sqlx::query("UPDATE registros_peso_calidad SET upstream = 'enviado' WHERE transaccion = 1")
            .execute(&pool)
            .await
            .unwrap();
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();
        assert_eq!(r.transaccion, 2);
    }

    #[tokio::test]
    async fn marcar_enviado_cambia_estado_y_pone_la_hora() {
        let pool = pool().await;
        crear_pesaje(&pool, pesaje(1)).await.unwrap();
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();

        assert!(marcar_enviado(&pool, r.id).await.unwrap());

        let (upstream, enviado_en): (String, Option<chrono::DateTime<Utc>>) =
            sqlx::query_as("SELECT upstream, enviado_en FROM registros_peso_calidad WHERE id = ?")
                .bind(r.id.hyphenated())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(upstream, "enviado");
        assert!(enviado_en.is_some());
        assert!(siguiente_pendiente(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn marcar_enviado_dos_veces_no_pisa_el_primero() {
        let pool = pool().await;
        crear_pesaje(&pool, pesaje(1)).await.unwrap();
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();

        assert!(marcar_enviado(&pool, r.id).await.unwrap());
        assert!(!marcar_enviado(&pool, r.id).await.unwrap());
    }

    #[tokio::test]
    async fn marcar_enviado_con_id_inexistente_devuelve_false() {
        let pool = pool().await;
        assert!(!marcar_enviado(&pool, Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    async fn marcar_rechazado_guarda_el_motivo_y_lo_saca_de_la_cola() {
        let pool = pool().await;
        crear_pesaje(&pool, pesaje(1)).await.unwrap();
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();

        assert!(
            marcar_rechazado(&pool, r.id, "422: peso_kg fuera de rango")
                .await
                .unwrap()
        );

        let (upstream, ultimo_error, enviado_en): (String, Option<String>, Option<String>) =
            sqlx::query_as(
                "SELECT upstream, ultimo_error, enviado_en FROM registros_peso_calidad WHERE id = ?",
            )
            .bind(r.id.hyphenated())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(upstream, "rechazado");
        assert_eq!(ultimo_error.as_deref(), Some("422: peso_kg fuera de rango"));
        assert!(enviado_en.is_none());
        assert!(siguiente_pendiente(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn marcar_rechazado_no_pisa_uno_ya_enviado() {
        let pool = pool().await;
        crear_pesaje(&pool, pesaje(1)).await.unwrap();
        let r = siguiente_pendiente(&pool).await.unwrap().unwrap();
        marcar_enviado(&pool, r.id).await.unwrap();

        assert!(!marcar_rechazado(&pool, r.id, "tarde").await.unwrap());
    }
}
