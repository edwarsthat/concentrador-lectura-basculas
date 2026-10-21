use reqwest::StatusCode;

/// Por qué se cortó el reenvío. Cualquiera de estos deja el registro
/// en 'pendiente' y hace que el reenviador espere antes de reintentar.
#[derive(Debug, thiserror::Error)]
pub enum ErrorReenvio {
    /// Falló SQLite al leer o marcar. Raro, pero no hay que tumbar la tarea.
    #[error("base de datos: {0}")]
    Db(#[from] sqlx::Error),

    /// No hubo respuesta: sin internet, DNS, TLS, timeout.
    #[error("sin conexión con la nube: {0}")]
    Red(#[from] reqwest::Error),

    /// La nube respondió pero no pudo atender: 5xx, 429 o un código inesperado.
    #[error("la nube respondió {0}")]
    Servidor(StatusCode),

    /// 401/403: la UPSTREAM_API_KEY está mal. No se corrige solo; hay que
    /// arreglar el .env. Por eso no se marca como rechazado: si no, se
    /// rechazarían todos los registros de la base.
    #[error("la nube no aceptó la llave ({0}): revisar UPSTREAM_API_KEY")]
    NoAutorizado(StatusCode),
}

impl ErrorReenvio {
    /// Errores que necesitan a una persona, no solo esperar.
    /// El reenviador los loguea como error y no como warn.
    pub fn requiere_intervencion(&self) -> bool {
        matches!(self, ErrorReenvio::NoAutorizado(_))
    }
}
