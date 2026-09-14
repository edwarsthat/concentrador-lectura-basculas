CREATE TABLE registros_peso_calidad (
    id                   TEXT PRIMARY KEY NOT NULL,
    clave_idempotencia   TEXT NOT NULL UNIQUE,

    device_id            TEXT NOT NULL,
    llave_nfc            TEXT NOT NULL,
    peso                 REAL NOT NULL,
    timestamp_dispositivo TEXT NOT NULL,

    recibido_en          TEXT NOT NULL,
    upstream             TEXT NOT NULL DEFAULT 'pendiente'
);

-- El reenviador busca lo pendiente; sin índice recorre toda la tabla cada vez.
CREATE INDEX idx_registros_upstream ON registros_peso_calidad (upstream);
