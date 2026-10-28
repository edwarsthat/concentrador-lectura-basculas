use reqwest::StatusCode;

use crate::{
    app::state::AppState,
    models::registro_peso_calidad::{PesajeNube, RegistroPesoCalidad},
    reenvio::error::ErrorReenvio,
};

const RUTA_PESAJE: &str = "/pesaje";

/// Tope del motivo que se guarda en `ultimo_error`: si la nube responde una
/// página HTML de error, no queremos meter 50 KB en SQLite.
const LARGO_MOTIVO: usize = 500;

#[derive(Debug, PartialEq)]
pub enum Desenlace {
    Enviado,
    Rechazado(String),
}

pub async fn enviar(
    state: &AppState,
    registro: &RegistroPesoCalidad,
) -> Result<Desenlace, ErrorReenvio> {
    let config = &state.config;
    let url = format!("{}{RUTA_PESAJE}", config.upstream_url.trim_end_matches("/"));

    let respuesta = state
        .http
        .post(&url)
        .bearer_auth(&config.upstream_api_key)
        .json(&PesajeNube::from(registro))
        .send()
        .await?;

    let status = respuesta.status();
    let cuerpo = respuesta.text().await.unwrap_or_default();

    clasificar(status, cuerpo)
}

/// Tabla de decisión de la respuesta. Pura, para poder testearla sin servidor.
fn clasificar(status: StatusCode, cuerpo: String) -> Result<Desenlace, ErrorReenvio> {
    match status {
        // 409 = ya tenía ese `id`: un reenvío tras un corte. Para nosotros es éxito.
        StatusCode::CREATED | StatusCode::OK | StatusCode::CONFLICT => Ok(Desenlace::Enviado),

        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            let motivo: String = cuerpo.chars().take(LARGO_MOTIVO).collect();
            Ok(Desenlace::Rechazado(format!("{status}: {motivo}")))
        }

        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ErrorReenvio::NoAutorizado(status)),

        // 5xx, 429 (demasiadas peticiones), redirecciones raras, 404 por URL mal puesta…
        // todo lo que no entendemos se reintenta: nunca se pierde un pesaje por un código raro.
        otro => Err(ErrorReenvio::Servidor(otro)),
    }
}
