use axum::http::HeaderMap;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Comprueba que `x-firma` sea el HMAC-SHA256 (en hex) de `cuerpo` con `clave`.
/// Compara en tiempo constante.
pub fn firma_validar(clave: &[u8], headers: &HeaderMap, cuerpo: &[u8]) -> bool {
    let Some(firma_hex) = headers.get("x-firma").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(firma) = hex::decode(firma_hex) else {
        return false;
    };
    let mut mac =
        Hmac::<Sha256>::new_from_slice(clave).expect("HMAC acepta claves de cualquier largo");
    mac.update(cuerpo);
    mac.verify_slice(&firma).is_ok()
}
