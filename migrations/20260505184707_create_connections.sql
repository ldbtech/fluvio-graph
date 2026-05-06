CREATE TABLE connections (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_a      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_b      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    zone        SMALLINT NOT NULL DEFAULT 1,
    tapped_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_a, user_b)
);
