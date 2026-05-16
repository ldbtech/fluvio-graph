# kg-engine

Rust **HTTP service** for building a **multi-domain knowledge graph** from documents and connectors: PDF ingestion, **Gmail** sync, chunk embedding with [fastembed](https://github.com/qdrant/fastembed), **semantic similarity** edges, and **Anthropic Claude** chat over retrieved graph context. The binary only starts the API (the old CLI has been removed).

---

## What you get

- **REST API (Axum)** on **http://0.0.0.0:8001** — PDF upload, Gmail connect/sync, paged graph endpoints, and chat. Durable graph storage is **SurrealDB** (no on-disk JSON snapshots).

---

## Prerequisites

- **Rust** — **1.85 or newer** (`edition = "2024"`). Install via [rustup](https://rustup.rs/), then `rustup update stable`.
- **Anthropic API key** — required for `/chat`. Create a key in the [Anthropic console](https://console.anthropic.com/).
- **SurrealDB** — kg-engine persists graph nodes (for example after `POST /twin/ingest`) to Surreal. By default it connects to **`ws://127.0.0.1:8000`** with namespace **`fluvio`** and database **`graph`** (root/root). Run a SurrealDB server on that port, or set `SURREAL_URL=embedded` in `.env` to use the embedded `surrealkv://./fluvio_surreal_data` store instead (no separate process; the `surreal sql` CLI will not see that data).
- **Gmail (optional)** — Google OAuth client credentials in **`~/.fluvio/config.json`** (see comments in `src/ingestion_registry/email/auth/oauth.rs` for the expected JSON shape and redirect URI `http://localhost:8001/connect/gmail/callback`).
- **Network (first run)** — the embedding model is downloaded on first use into `.fastembed_cache/` (gitignored).

---

## Configuration

Create a **`.env`** file in the repository root (gitignored — **do not commit secrets**). The server loads it via [dotenvy](https://crates.io/crates/dotenvy) when present; you can also `export …` in your shell.

Typical `.env`:

```bash
ANTHROPIC_API_KEY=<your-anthropic-api-key>

# Surreal — optional; omit these to use defaults (ws://127.0.0.1:8000, root/root, fluvio/graph).
SURREAL_URL=ws://127.0.0.1:8000
SURREAL_USER=root
SURREAL_PASS=root
SURREAL_NS=fluvio
SURREAL_DB=graph

# Embedded Surreal only (no server on 8000): use instead of the block above
# SURREAL_URL=embedded
```

**Embedded Surreal only** (no separate Surreal process): set `SURREAL_URL=embedded` and comment out or remove the other `SURREAL_*` lines if you like; the `surreal sql` CLI will not see that data.

---

## Build and run the server

From the repo root:

```bash
cargo build --release
./target/release/kg-engine
```

During development:

```bash
cargo run
```

The process exits immediately if `ANTHROPIC_API_KEY` is not set. On success you should see logs pointing at **http://localhost:8001**.

---

## HTTP API (summary)

CORS is open for local UI development (`tower-http`).

| Area | Method | Path | Notes |
|------|--------|------|--------|
| Ingest | `POST` | `/ingest/pdf` | Multipart PDF upload; chunk, embed, wire edges; persists into the workspace layout. |
| Graph | `GET` | `/graph` | JSON sample (capped); use paging endpoints for large graphs. |
| Graph | `GET` | `/graph/meta` | Metadata for UI. |
| Graph | `GET` | `/graph/nodes` | Paged nodes. |
| Graph | `POST` | `/graph/edges_subset` | Edges for a subset of node IDs. |
| Chat | `POST` | `/chat` | JSON: `question`, `history` as `{ "role", "content" }[]`; returns `answer` and `sources`. |
| Gmail | `GET` | `/connect/gmail` | Start OAuth (JSON or browser redirect with `?redirect=1`). |
| Gmail | `GET` | `/connect/gmail/callback` | OAuth callback. |
| Gmail | `GET` | `/connect/gmail/status` | Connection status. |
| Gmail | `POST` | `/sync/gmail` | Kick off sync (202); poll progress below. |
| Gmail | `GET` | `/sync/gmail/progress` | Sync progress. |
| Workspace | `GET` | `/workspace/projects` | List saved projects. |
| Workspace | `POST` | `/workspace/archive`, `/workspace/load`, `/workspace/delete` | JSON body `{ "id": "<project-id>" }`. |
| Workspace | `POST` | `/workspace/reset` | Clears the in-memory graph and workspace snapshots; **no JSON body**. |

**Durable storage** — All ingested nodes/edges (codebase, PDF, video, email) are persisted into **SurrealDB** via `src/storage/surreal.rs`. The in-memory `IngestionPipeline` graph is a per-process working set only; nothing is snapshotted to JSON. Surreal records are keyed by `owner_id` (Postgres user id) and `zone` so reads can be scoped per user.

---

## Repository layout (high level)

| Path | Role |
|------|------|
| `src/main.rs` | Starts the HTTP server only. |
| `src/server.rs` | Axum routes: ingest, graph, chat, Gmail, workspace. |
| `src/graph/` | Graph types, embeddings, registry. |
| `src/storage/surreal.rs` | SurrealDB persistence (durable nodes/edges per user). |
| `src/ingestion.rs` | Ingestion pipeline wiring (in-RAM working set). |
| `src/query.rs` | RAG-style retrieval over the graph. |
| `src/ingestion_registry/` | Connectors and document types (e.g. PDF, Gmail). |

---

## Troubleshooting

- **`ANTHROPIC_API_KEY` errors** — Ensure `.env` exists in the directory you run from, or export the variable.
- **`SurrealDB connect failed`** — Start Surreal on `127.0.0.1:8000`, or set `SURREAL_URL=embedded` if you intentionally run without a server.
- **`SELECT * FROM nodes` empty in `surreal sql`** — Confirm the CLI uses the same endpoint and `USE NS fluvio DB graph` as kg-engine (see startup log `[SurrealDB] Connected to …`). If you use `SURREAL_URL=embedded`, data is not on the network server.
- **Port 8001 in use** — Stop the other process or change the bind address in `src/server.rs`.
- **Gmail OAuth** — Confirm `~/.fluvio/config.json` exists and redirect URIs match your Google Cloud OAuth client.
- **First embedding run is slow** — Model download; watch the console for fastembed progress.

This codebase is **private / not open source** for now: deep ingest and graph-backed reasoning can be misused against third-party repositories or documents without authorization. The product landing reflects the same stance.
