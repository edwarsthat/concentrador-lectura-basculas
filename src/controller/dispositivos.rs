use axum::{body::Bytes, extract::State, http::HeaderMap};
use chrono::DateTime;
use reqwest::StatusCode;

use crate::{
    app::{api_error::ApiError, state::AppState},
    models::{
        anclas_dispositivos::ArranqueValidado,
        eventos::{Evento, EventoEntrante, Registro},
        registro_peso_calidad::PesajeValidado,
    },
    security::validar::firma_validar,
    services,
};

pub async fn router_eventos(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    if !firma_validar(state.config.api_key.as_bytes(), &headers, &body) {
        tracing::warn!("evento con firma invalida o ausente");
        return Err(ApiError::FirmaInvalida);
    }

    let entrante: EventoEntrante = serde_json::from_slice(&body)
        .map_err(|e| ApiError::NoProcesable(format!("cuerpo no reconocido: {e}")))?;

    if entrante.rechazado {
        let motivo = serde_json::from_value::<Registro>(entrante.registro.clone())
            .err()
            .map(|e| e.to_string());

        tracing::warn!(
            device_id = %entrante.device_id,
            registro = %entrante.registro,
            "registro rechazado"
        );
        services::eventos_rechazados::guardar(
            &state.pool,
            &entrante.device_id,
            &entrante.registro,
            motivo,
        )
        .await?;
        return Ok(StatusCode::OK);
    }

    let registro: Registro = serde_json::from_value(entrante.registro)
        .map_err(|e| ApiError::NoProcesable(format!("registro invalido: {e}")))?;

    let hora = registro.hora.and_then(|h| DateTime::from_timestamp(h, 0));
    let transaccion = i64::from(registro.transaccion);

    let nuevo = match registro.evento {
        Evento::Arranque { version_firmware } => {
            let datos = ArranqueValidado {
                device_id: entrante.device_id,
                sesion_arranque: registro.sesion_arranque,
                transaccion,
                monotonico_us: registro.monotonico_us,
                hora,
                version_firmware,
            };
            services::anclas_dispositivos::create_service_registro_anclas_dispositivos(
                &state.pool,
                datos,
            )
            .await?
        }
        Evento::Pesaje {
            uid_tarjeta,
            peso_kg,
        } => {
            let datos = PesajeValidado {
                device_id: entrante.device_id,
                sesion_arranque: registro.sesion_arranque,
                transaccion,
                uid_tarjeta,
                peso_kg,
                hora,
                monotonico_us: registro.monotonico_us,
            };
            services::registro_peso_calidad::crear_pesaje(&state.pool, datos).await?
        }
    };

    // 409 = ya lo teníamos (reintento); la báscula lo da por entregado igual que un 201.
    Ok(if nuevo {
        StatusCode::CREATED
    } else {
        StatusCode::CONFLICT
    })
}
