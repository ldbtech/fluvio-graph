CREATE TABLE cards (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    card_type   TEXT NOT NULL DEFAULT 'nfc',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
