-- Ancla de reloj por arranque del ESP32: el dispositivo no tiene RTC, así que
-- al encender reporta su contador monotónico y la Pi apunta su propia hora en
-- ese instante. Con eso se reconstruye la hora real de cada medición.
CREATE TABLE anclas_dispositivos (
    device_id            TEXT NOT NULL,
    sesion_arranque      INTEGER NOT NULL,
    monotonico_ancla_us  INTEGER NOT NULL,   -- esp_timer_get_time() del ESP32, en microsegundos
    hora_pi_ancla        TEXT NOT NULL,      -- hora de la Pi cuando llegó el ancla (ISO-8601 UTC)
    recibido_en          TEXT NOT NULL,
    version_firmware     TEXT,
    PRIMARY KEY (device_id, sesion_arranque)
);
