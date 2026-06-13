CREATE TABLE IF NOT EXISTS zakonim (
    id          TEXT        PRIMARY KEY,
    description TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
