-- Registros que el concentrador respondió con 422 y la báscula reenvió con
-- "rechazado": true. Se guardan tal cual llegaron, sin validar, para
-- revisarlos a mano: justamente son los que no pasaron la validación.
CREATE TABLE eventos_rechazados (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id    TEXT NOT NULL,
    registro     TEXT NOT NULL,      -- JSON crudo del campo `registro`
    motivo       TEXT,               -- por qué no pasó, recalculado al recibir el reenvío
    recibido_en  TEXT NOT NULL,      -- hora de la Pi (ISO-8601 UTC)
    revisado     INTEGER NOT NULL DEFAULT 0,   -- 1 cuando alguien ya lo atendió

    -- Si el reenvío también se reintenta por timeout, llega el mismo JSON:
    -- con ON CONFLICT DO NOTHING no se duplica.
    UNIQUE (device_id, registro)
);
