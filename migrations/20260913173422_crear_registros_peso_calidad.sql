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
    intentos             INTEGER NOT NULL DEFAULT 0,   -- reenvíos fallidos hasta ahora
    ultimo_error         TEXT,               -- mensaje del último fallo; NULL si no ha fallado
    enviado_en           TEXT,               -- hora de la Pi (ISO-8601 UTC) en que el upstream lo aceptó; NULL si sigue pendiente

    -- Un reintento por timeout trae la misma terna: el INSERT choca aquí y se responde 409.
    UNIQUE (device_id, sesion_arranque, transaccion)
);

-- El reenviador busca lo pendiente en orden de llegada; sin índice recorre toda la tabla cada vez.
CREATE INDEX idx_registros_upstream_recibido ON registros_peso_calidad (upstream, recibido_en);
