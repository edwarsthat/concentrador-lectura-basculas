use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, HeaderName, StatusCode, header},
    response::IntoResponse,
};

use crate::{
    app::{api_error::ApiError, state::AppState},
    models::{
        anclas_dispositivos::ArranqueValidado,
        eventos::{Evento, EventoEntrante, EventoValidado},
        hora::{HoraQuery, HoraRespuesta},
        registro_peso_calidad::PesajeValidado,
    },
    security::validar::{firma_validar, firmar},
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

    let validado = entrante.registro_validado();

    if entrante.rechazado {
        let motivo = validado.err().map(|e| e.to_string());

        tracing::warn!(
            device_id = %entrante.device_id,
            registro = %entrante.registro,
            "registro rechazado"
        );
        services::eventos_rechazados::guardar(
            &state.pool,
            entrante.device_id.trim(),
            &entrante.registro,
            motivo,
        )
        .await?;
        return Ok(StatusCode::OK);
    }

    // 422 y no 400: con 422 la báscula lo reenvía marcado; cualquier otro
    // código lo reintenta para siempre y atasca su cola.
    let EventoValidado {
        device_id,
        registro,
    } = validado.map_err(|e| ApiError::NoProcesable(format!("registro invalido: {e}")))?;

    let transaccion = i64::from(registro.transaccion);

    let nuevo = match registro.evento {
        Evento::Arranque { version_firmware } => {
            let datos = ArranqueValidado {
                device_id,
                sesion_arranque: registro.sesion_arranque,
                transaccion,
                monotonico_us: registro.monotonico_us,
                hora: Some(registro.hora),
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
                device_id,
                sesion_arranque: registro.sesion_arranque,
                transaccion,
                uid_tarjeta,
                peso_kg,
                hora: registro.hora,
                monotonico_us: registro.monotonico_us,
            };
            let nuevo = services::registro_peso_calidad::crear_pesaje(&state.pool, datos).await?;
            if nuevo {
                state.aviso_reenvio.notify_one();
            }
            nuevo
        }
    };

    // 409 = ya existía esa terna. La báscula lo trata igual que un 201 y lo
    // borra de su cola (ver la tabla de respuestas del README del firmware).
    Ok(if nuevo {
        StatusCode::CREATED
    } else {
        StatusCode::CONFLICT
    })
}

pub async fn hora_actual(
    State(state): State<AppState>,
    Query(q): Query<HoraQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let hora = state.reloj.ahora().ok_or(ApiError::HoraNoSincronizada)?;
    let respuesta = HoraRespuesta {
        hora: hora.timestamp(),
        nonce: q.nonce,
    };
    // Se serializa a mano para firmar exactamente los bytes que se envían.
    let cuerpo = serde_json::to_string(&respuesta).expect("HoraRespuesta siempre serializa");
    let firma = firmar(state.config.api_key.as_bytes(), cuerpo.as_bytes());

    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (HeaderName::from_static("x-firma"), firma),
        ],
        cuerpo,
    ))
}
