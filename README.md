# kg-engine

Rust **HTTP service** for building a **multi-domain knowledge graph** from documents and connectors: PDF ingestion, **Gmail** sync, chunk embedding with [fastembed](https://github.com/qdrant/fastembed), **semantic similarity** edges, and **Anthropic Claude** chat over retrieved graph context. A **Next.js workspace UI** in `web/fluvio-ui` is the main way to drive ingestion, graph exploration, and chat (the old CLI has been removed; the binary only starts the API).

---

## What you get

- **REST API (Axum)** on **http://0.0.0.0:8001** — PDF upload, Gmail connect/sync, paged graph endpoints for the UI, chat, and workspace project lifecycle (save/load/archive graphs under `fluvio_graphs/`).
- **Web UI** — `web/fluvio-ui`: connectors, graph view, chat, workspace projects. Run it alongside the Rust server.

---

## Prerequisites

- **Rust** — **1.85 or newer** (`edition = "2024"`). Install via [rustup](https://rustup.rs/), then `rustup update stable`.
- **Anthropic API key** — required for `/chat`. Create a key in the [Anthropic console](https://console.anthropic.com/).
- **Gmail (optional)** — Google OAuth client credentials in **`~/.fluvio/config.json`** (see comments in `src/ingestion_registry/email/auth/oauth.rs` for the expected JSON shape and redirect URI `http://localhost:8001/connect/gmail/callback`).
- **Network (first run)** — the embedding model is downloaded on first use into `.fastembed_cache/` (gitignored).

---

## Configuration

Create a `.env` file in the repository root (gitignored — **do not commit secrets**):

```bash
ANTHROPIC_API_KEY=<your-anthropic-api-key>
```

The server loads `.env` via [dotenvy](https://crates.io/crates/dotenvy) when present. You can also `export ANTHROPIC_API_KEY=...` in your shell.

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

**On-disk layout** — Active workspace graphs live under **`fluvio_graphs/workspace/`** (for example `unified.json`, connector-specific snapshots). Saved projects use **`fluvio_graphs/projects/`**. A legacy **`fluvio_graph.json`** in the process working directory may still be read for migration paths.

---

## Web UI

```bash
cd web/fluvio-ui
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000). The UI expects the API at [http://localhost:8001](http://localhost:8001) by default (`NEXT_PUBLIC_KG_URL` overrides this in `web/fluvio-ui/lib/constants.ts`).

---

## Repository layout (high level)

| Path | Role |
|------|------|
| `src/main.rs` | Starts the HTTP server only. |
| `src/server.rs` | Axum routes: ingest, graph, chat, Gmail, workspace. |
| `src/graph/` | Graph types, embeddings, registry, persistence helpers. |
| `src/ingestion.rs` | Ingestion pipeline wiring. |
| `src/query.rs` | RAG-style retrieval over the graph. |
| `src/ingestion_registry/` | Connectors and document types (e.g. PDF, Gmail). |
| `fluvio_graphs/` | Workspace and project graph JSON (local data; gitignore as appropriate). |
| `web/fluvio-ui/` | Next.js frontend. |

---

## Troubleshooting

- **`ANTHROPIC_API_KEY` errors** — Ensure `.env` exists in the directory you run from, or export the variable.
- **Port 8001 in use** — Stop the other process or change the bind address in `src/server.rs`.
- **Gmail OAuth** — Confirm `~/.fluvio/config.json` exists and redirect URIs match your Google Cloud OAuth client.
- **First embedding run is slow** — Model download; watch the console for fastembed progress.

This codebase is **private / not open source** for now: deep ingest and graph-backed reasoning can be misused against third-party repositories or documents without authorization. The product landing reflects the same stance.
