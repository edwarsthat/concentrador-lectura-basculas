pub mod cliente;
pub mod error;

use std::time::Duration;

use crate::{
    app::state::AppState,
    reenvio::{
        cliente::{Desenlace, enviar},
        error::ErrorReenvio,
    },
    services::registro_peso_calidad::{
        marcar_enviado, marcar_rechazado, registrar_fallo, siguiente_pendiente,
    },
};

const RESPALDO: Duration = Duration::from_secs(60);

async fn reenviar_siguiente(state: &AppState) -> Result<bool, ErrorReenvio> {
    let pendiente = match siguiente_pendiente(&state.pool).await? {
        Some(pendiente) => pendiente,
        None => return Ok(false),
    };

    match enviar(&state, &pendiente).await {
        Ok(Desenlace::Enviado) => {
            marcar_enviado(&state.pool, pendiente.id).await?;
        }

        Ok(Desenlace::Rechazado(motivo)) => {
            marcar_rechazado(&state.pool, pendiente.id, &motivo).await?;
        }

        Err(e) => {
            registrar_fallo(&state.pool, pendiente.id, &e.to_string()).await?;
            return Err(e);
        }
    }

    Ok(true)
}

async fn vaciar_cola(state: &AppState) -> Result<(), ErrorReenvio> {
    while reenviar_siguiente(state).await? {}
    Ok(())
}

pub fn iniciar(state: AppState) {
    tokio::spawn(async move {
        loop {
            match vaciar_cola(&state).await {
                Ok(()) => {
                    tokio::select! {
                        _ = state.aviso_reenvio.notified() => {}
                        _ = tokio::time::sleep(RESPALDO) => {}
                    }
                }
                Err(e) => {
                    // La llave mala no se arregla sola: va como error para que se note
                    // entre los warn de los cortes de red.
                    if e.requiere_intervencion() {
                        tracing::error!(error = %e, "reenvío detenido: revisar configuración");
                    } else {
                        tracing::warn!(error = %e, "no se pudo reenviar, reintento en 10 s");
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    });
}
