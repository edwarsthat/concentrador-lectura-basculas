-- Pesajes que mandan las básculas por POST /dispositivos/eventos.
CREATE TABLE registros_peso_calidad (
    id                   TEXT PRIMARY KEY NOT NULL,   -- UUID generado por el servidor

    -- ── identidad del evento en la báscula ────────
    device_id            TEXT NOT NULL,
    sesion_arranque      INTEGER NOT NULL,
    transaccion          INTEGER NOT NULL,

    -- ── lo que midió la báscula ───────────────────
    uid_tarjeta          TEXT NOT NULL,
    peso_kg              REAL NOT NULL,
    hora_dispositivo     TEXT,               -- `hora` del ESP32 (ISO-8601 UTC); NULL si su reloj no era válido
    monotonico_us        INTEGER NOT NULL,   -- esp_timer_get_time() al pesar, para resolver contra el ancla

    -- ── lo que añade el servidor ──────────────────
    hora_medicion        TEXT,               -- hora final: hora_dispositivo, o la resuelta con el ancla; NULL si aún no hay ninguna
    recibido_en          TEXT NOT NULL,
    upstream             TEXT NOT NULL DEFAULT 'pendiente',

    -- Un reintento por timeout trae la misma terna: el INSERT choca aquí y se responde 409.
    UNIQUE (device_id, sesion_arranque, transaccion)
);

-- El reenviador busca lo pendiente; sin índice recorre toda la tabla cada vez.
CREATE INDEX idx_registros_upstream ON registros_peso_calidad (upstream);
