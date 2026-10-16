-- Ancla de reloj por arranque del ESP32: el dispositivo no tiene RTC, así que
-- al encender reporta su contador monotónico y la Pi apunta su propia hora en
-- ese instante. Con eso se reconstruye la hora real de cada medición.
CREATE TABLE anclas_dispositivos (
    device_id            TEXT NOT NULL,
    sesion_arranque      INTEGER NOT NULL,
    transaccion          INTEGER NOT NULL,   -- nº de evento de la báscula que trajo el arranque
    monotonico_ancla_us  INTEGER NOT NULL,   -- esp_timer_get_time() del ESP32, en microsegundos
    hora_dispositivo     TEXT,               -- `hora` del evento (ISO-8601 UTC); NULL si el reloj no era válido
    hora_pi_ancla        TEXT NOT NULL,      -- hora usada como ancla: hora_dispositivo o, si falta, recibido_en
    ancla_aproximada     INTEGER NOT NULL,   -- 1 si se usó recibido_en: solo es fiable si el arranque llegó sin demora
    recibido_en          TEXT NOT NULL,      -- cuándo llegó a la Pi
    version_firmware     TEXT,
    PRIMARY KEY (device_id, sesion_arranque)
);
