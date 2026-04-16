# kg-engine

Rust service and CLI for building a **multi-domain knowledge graph** from documents (PDF ingestion today), embedding chunks with [fastembed](https://github.com/qdrant/fastembed), wiring **semantic similarity** edges, and answering questions with **Anthropic Claude** using retrieved graph context.

---

## What you get

- **CLI** — ingest PDFs into named domain graphs, chat against a graph, print registry stats. Graph snapshots live under `fluvio_graphs/`.
- **HTTP API** — Axum server on port **8001**: PDF upload, graph JSON, and chat (same RAG-style flow as the CLI).
- **Optional UI** — Next.js app in `web/fluvio-ui` that talks to `http://localhost:8001`.

---

## Prerequisites

- **Rust** — **1.85 or newer** (this crate uses `edition = "2024"`). Install via [rustup](https://rustup.rs/), then `rustup update stable`.
- **Anthropic API key** — required for `chat` and `server`. Create a key in the [Anthropic console](https://console.anthropic.com/).
- **Network (first run)** — the embedding model is downloaded on first use into `.fastembed_cache/` (already listed in `.gitignore`).

---

## Configuration

Create a `.env` file in the repository root (it is gitignored — **do not commit secrets**):

```bash
ANTHROPIC_API_KEY=<your-anthropic-api-key>
```

The program loads `.env` automatically via [dotenvy](https://crates.io/crates/dotenvy) when present. You can also `export ANTHROPIC_API_KEY=...` in your shell.

---

## Build

From the repo root:

```bash
cargo build --release
```

The binary is named **`kg-engine`** (matches the package name). Run it directly:

```bash
./target/release/kg-engine --help   # prints usage (no --help flag; omit subcommand to see help)
```

Or during development:

```bash
cargo run --release -- <subcommand> ...
```

---

## CLI usage

After `cargo build`, replace `cargo run --` with `./target/release/kg-engine` if you prefer.

| Command | Purpose |
|--------|---------|
| `cargo run -- ingest pdf <path-to.pdf>` | Chunk and embed a PDF into the `pdf` graph, wire edges, save under `fluvio_graphs/`. |
| `cargo run -- chat [domain]` | Interactive REPL; `domain` defaults to `pdf`. Requires an ingested graph. |
| `cargo run -- stats` | Print node/edge counts for registered graphs. |
| `cargo run -- server` | Start the REST API on **http://0.0.0.0:8001**. |

**Domains** — The CLI registers `pdf`, `email`, `whatsapp`, `music`, and `codebase`. **PDF ingestion is implemented**; other domains are placeholders until connectors are added.

Example:

```bash
cargo run -- ingest pdf ./samples/paper.pdf
cargo run -- chat pdf
```

---

## HTTP server

```bash
cargo run -- server
```

Server loads or creates **`fluvio_graph.json`** in the current working directory (separate from the CLI’s `fluvio_graphs/*.json` layout).

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/ingest/pdf` | Multipart upload of a PDF file; chunks, embeds, wires edges, persists graph. |
| `GET` | `/graph` | JSON nodes and edges for visualization or debugging. |
| `POST` | `/chat` | JSON body: `question` and `history` (array of `{ "role", "content" }`). Returns `answer` and `sources`. |

CORS is open (`tower-http`) for local UI development.

---

## Optional web UI

```bash
cd web/fluvio-ui
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000). The UI expects the Rust server at [http://localhost:8001](http://localhost:8001) (`KG_URL` in `web/fluvio-ui/app/page.tsx`).

---

## Repository layout (high level)

| Path | Role |
|------|------|
| `src/main.rs` | CLI entry: ingest, chat, stats, server dispatch. |
| `src/server.rs` | Axum routes for ingest, graph, chat. |
| `src/graph/` | Graph types, embeddings, registry. |
| `fluvio_graphs/` | On-disk domain graphs for the CLI (`pdf.json`, `meta.json`, …). |
| `web/fluvio-ui/` | Next.js frontend. |

---

## Troubleshooting

- **`ANTHROPIC_API_KEY` errors** — Ensure `.env` exists in the directory you run from, or export the variable.
- **Port 8001 in use** — Stop the other process or change the bind address in `src/server.rs`.
- **First embedding run is slow** — Model download; watch the console for fastembed progress.

Contributions and issues are welcome on GitHub.
