CREATE TABLE user_uploads (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,
    file_name     TEXT NOT NULL,
    document_id   TEXT,
    graph_nodes   INTEGER,
    graph_edges   INTEGER,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX user_uploads_user_created ON user_uploads (user_id, created_at DESC);
