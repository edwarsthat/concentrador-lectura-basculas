use serde::{Deserialize, Serialize};

/// Query de GET /dispositivos/hora?nonce=...
#[derive(Debug, Deserialize)]
pub struct HoraQuery {
    /// Valor aleatorio del ESP32; vuelve en la respuesta firmada para que no
    /// sirva reenviar una respuesta vieja.
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct HoraRespuesta {
    pub hora: i64,
    pub nonce: String,
}
