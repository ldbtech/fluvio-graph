# fluvioMe — Headless Data Pipeline & Knowledge Graph Engine

[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.87%2B-orange.svg)](https://rustup.rs/)
[![Docker](https://img.shields.io/badge/Docker-compose%20up-2496ED.svg)](docker-compose.yml)

> **Your digital twin and company brain — in your own infrastructure.**

fluvioMe is an open-source, headless engine for automated data pipelines, knowledge graphs, and self-serve BI reporting (PowerBI, Tableau, PDFs). Empower stakeholders with instant reports without waiting for engineering backlogs.

---

## Table of Contents

- [License](#license)
- [What is fluvioMe?](#what-is-fluviome)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Services](#services)
  - [fluvio-graph](#fluvio-graph)
  - [fluvio-twin](#fluvio-twin)
  - [fluvio-database](#fluvio-database)
  - [fluvio-ingestion](#fluvio-ingestion)
  - [fluvio-collab](#fluvio-collab)
  - [fluvio-connectors](#fluvio-connectors)
  - [fluvio-tool-builder](#fluvio-tool-builder)
  - [agent-planner](#agent-planner)
  - [fluvio-gateway](#fluvio-gateway)
- [GraphQL API](#graphql-api)
- [Configuration](#configuration)
  - [LLM Providers (BYOK)](#llm-providers-byok)
- [Enterprise](#enterprise)
- [Contributing](#contributing)
- [Repository Layout](#repository-layout)

---

## License

```
Business Source License 1.1 (BUSL-1.1)
Change Date: Four years from first tagged release
Change License: Apache License 2.0
```

| Who | Use | Cost |
|-----|-----|------|
| Students, researchers, academics | Any non-commercial purpose | **Free · lifetime · no limits** |
| Non-profit organizations | Any non-commercial purpose | **Free · lifetime · no limits** |
| Open-source projects | Non-commercial | **Free · lifetime · no limits** |
| Personal / hobby projects | Non-commercial | **Free · lifetime · no limits** |
| **Any for-profit company** (any size, any stage) | Any production use | **Requires enterprise license** |

**No trial period. No 24-hour limits. No feature expiry. No nag screens.**
Non-commercial use is free and unlimited forever — no token, no registration required.

After the Change Date, each release automatically converts to **Apache License 2.0** for all use.

> See [`LICENSE`](LICENSE) for the full text.
> To obtain a commercial license: [fluviome.com](https://fluviome.com) or [hello@fluviome.com](mailto:hello@fluviome.com).

---

## What is fluvioMe?

fluvioMe gives your company a **semantic memory** that lives in your own servers:

```
Your data sources (PDFs, DBs, APIs, docs, code, Notion, GitHub)
        ↓
   fluvio-ingestion    chunk · tag · embed
        ↓
   fluvio-graph        SurrealDB knowledge graph — nodes, edges, 384-dim vectors
        ↓
   agent-planner       natural language → pipeline plan → compile → deploy
        ↓
   BI outputs          PowerBI · Tableau · PDF reports · dashboards
```

**Headless** means:

- **No built-in UI** — API-first. Embed via SDK, REST, or the bundled Apollo Sandbox.
- **No auth layer** — bring your own identity provider, or omit entirely for open/internal use.
- **Runs anywhere** — Docker, Kubernetes, EC2, bare metal, your laptop.
- **No vendor lock-in** — all data stays in your SurrealDB and Postgres instances.

---

## Quick Start

### Community (non-commercial — free forever)

```bash
# Prerequisites: Docker + Docker Compose
docker compose up
```

GraphQL API at **http://localhost:4001** (Apollo Sandbox included).
Agent planner REST at **http://localhost:3007**.

### Enterprise (any for-profit use)

```bash
# 1. Get your token at https://fluviome.com
# 2. Add to .env:
echo "FLUVIOME_ENTERPRISE_TOKEN=your-token-here" >> .env
# 3. Start with enterprise gate:
docker compose --profile enterprise up
```

### Local development (without Docker)

```bash
# Prerequisites
# - Rust 1.87+     https://rustup.rs/
# - SurrealDB      surreal start --user root --pass root surrealkv://./fluvio_surreal_data
# - PostgreSQL 16  brew services start postgresql@16
# - Python 3.13    for agent-planner and connectors
# - Rover CLI      https://www.apollographql.com/docs/rover/getting-started

cp .env.example .env          # fill in an LLM provider key at minimum — see
                               # "LLM Providers (BYOK)" in Configuration
bash scripts/dev.sh
```

### Embed as a library (no server, no gateway)

Since `v0.1.0` the engine can be linked **in-process**. Depend on the facade
crate — never on the internal `*-core` crates — and inject config rather than
having the library read the environment:

```toml
[dependencies]
fluvio-graph = { git = "https://github.com/ldbtech/fluvio-graph", tag = "v0.1.0" }
# ingestion (pdf-extract, tokenizers) is on by default; turn it off to read an
# already-built graph with no extra deps:
#   fluvio-graph = { ..., default-features = false }
```

```rust
use std::sync::Arc;
use fluvio_graph::prelude::*;

# async fn run() -> anyhow::Result<()> {
// Config is injected — the library never reads the environment.
let cfg = SurrealConfig { url: "ws://127.0.0.1:8000".into(), ..Default::default() };
let store = Arc::new(SurrealStorage::connect(&cfg).await?);
store.init_schema().await?;

let mut embedder = EmbeddingContext::new()?;          // expensive — hold onto it
let ctx = QueryContext::from_text(
    Uuid::nil(),                                      // owner / tenant id
    "what do we know about churn?",
    &QueryConfig::default(),
    &store,
    &mut embedder,
    None,                                             // Option<WorkspaceId> filter
).await?;
println!("retrieved {} grounded nodes", ctx.node_count);
# Ok(()) }
```

See [`examples/embedded-consumer`](examples/embedded-consumer) for the full,
runnable version. Public surface and versioning are documented in
[`CHANGELOG.md`](CHANGELOG.md); the facade is the only thing that gets a version
bump.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                  Client  (SDK / REST / Apollo Sandbox)           │
└───────────────────────────────┬──────────────────────────────────┘
                                │  x-user-id header  (BYO auth)
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│           Apollo Router  :4001   (GraphQL Supergraph)            │
│      Federates all subgraphs into one unified /graphql schema    │
└────┬──────────┬──────────┬──────────┬──────────┬────────────────┘
     │          │          │          │          │
  :3001      :3002      :3003      :3004      :3005      :3008
fluvio-    fluvio-    fluvio-    fluvio-    fluvio-    fluvio-
graph      twin       collab     ingestion  database   tool-builder
(SurrealDB)(SurrealDB)(SurrealDB)(SurrealDB)(Postgres)  (Python)
  │
BGE-small
embedding model
(in-process, no API)

                            agent-planner  :3007  (Python / FastAPI)
                             ↕  LLM Provider (BYOK — Claude, OpenAI, Gemini, or Ollama)
```

All Rust services are **Apollo Federation 2.5** subgraphs.
The gateway composes them into a single `/graphql` endpoint at `:4001`.

**Enterprise gate** (optional, `:4002`) — validates `FLUVIOME_ENTERPRISE_TOKEN`,
injects `x-fluviome-tier` header. Only starts when the env var is set.

---

## Services

---

### fluvio-graph

**Port:** `3001`
**Language:** Rust — Axum · async-graphql · SurrealDB
**Role:** Knowledge graph storage, embedding, and retrieval — the semantic memory of fluvioMe.

#### What it does

`fluvio-graph` is the core persistence and query engine for all knowledge graph operations. It:

- Stores **nodes** (semantic entities extracted from documents, databases, and connectors) and **edges** (typed relationships between entities) in **SurrealDB**
- Embeds all node text using **BGE-small-EN-v1.5** (384-dimensional vectors) via `fastembed` — loaded in-process on startup, no external embedding API required
- Provides **cosine similarity vector search** over SurrealDB for semantic retrieval (RAG)
- Provides **BFS graph traversal** and a request-scoped **Dijkstra shortest-path** over the knowledge graph
- Supports **cross-user network search** — "who in my team knows about X?"
- Exposes all operations as a **GraphQL subgraph** federated into the Apollo supergraph

#### Data model

```
Node {
  id:          UUID  — content-addressed or randomly generated
  owner_id:    UUID  — which user owns this node
  domain:      Pdf | Email | Whatsapp | Calendar | Codebase | Web | Custom
  kind:        Entity | Topic | Artifcat | Event | Conversation | ExternalRef
               ⚠️  "Artifcat" is a preserved typo in the SurrealDB schema — do not rename
  source_uri:  String — opaque locator (file path, message-id, GitHub URL, …)
  source_text: String — canonical text used for embedding + LLM context windows
  embeddings:  Vec<f32> — 384-dim BGE-small vector (excluded from GQL responses)
  metadata:    HashMap<String, String> — arbitrary key-value tags
  zone:        i16 — 0 = own data, 1 = network / shared
}

Edge {
  id:                       UUID
  from:                     NodeId
  to:                       NodeId
  label:                    String — relationship type (e.g. "mentions", "authored_by")
  token:                    i32    — approximate LLM token cost of traversing this edge
  relationship_probability: f64   — confidence [0.0, 1.0]
}
```

SurrealDB table: `nodes`. Edges are stored as SurrealDB `RELATE` records.

#### GraphQL API

**Queries:**

```graphql
# Fetch a single node by UUID
node(id: String!): GqlNode

# All nodes for the authenticated user, optionally filtered
nodes(
  domain:      String    # "Pdf" | "Email" | "Codebase" | …
  zone:        Int       # 0 = own, 1 = network
  workspaceId: String
): [GqlNode!]!

# Semantic similarity search — embeds query, returns top-K closest nodes
search(
  query:       String!
  config:      GqlQueryConfig
  workspaceId: String
): [GqlScoredNode!]!

# Cross-user network search — "who in my team knows about X?"
# Returns sparse GqlScoredNode with owner_id and node_id in metadata;
# full node content is intentionally omitted for privacy
networkSearch(
  query:   String!
  userIds: [String!]!
  topK:    Int           # default: 20
): [GqlScoredNode!]!

# BFS expansion — all nodes within `depth` hops of a given node
neighbors(id: String!, depth: Int): [GqlNode!]!

# Dijkstra shortest path between two nodes
# Builds a request-scoped in-memory subgraph, dropped after the query
shortestPath(from: String!, to: String!): GqlPath!
```

**Mutations:**

```graphql
# Create or update a node.
# If id is supplied and the node exists, embeddings are preserved when
# the embeddings field is omitted — safe to add metadata without re-embedding.
upsertNode(input: GqlNodeInput!): GqlNode!

# Delete a node by UUID
deleteNode(id: String!): Boolean!

# Create or update a directed edge
upsertEdge(input: GqlEdgeInput!): GqlEdge!

# Stub — reserved for future batch persistence (currently returns {nodesSaved: 0, edgesSaved: 0})
saveGraph(zone: Int): GqlSaveResult!

# Delete all graph data owned by the authenticated user
deleteUserGraph: Boolean!

# Delete all nodes scoped to a specific workspace
deleteWorkspaceNodes(workspaceId: String!): Boolean!
```

**Types:**

```graphql
type GqlNode {
  id:                  String!
  domain:              GqlDomain!
  sourceUri:           String!
  sourceText:          String!
  kind:                GqlNodeKind!
  metadata:            [GqlMetadataEntry!]!
  zone:                Int!
  embeddingDimensions: Int!     # 0 = not yet embedded
  isEmbedded:          Boolean!
  # embeddings vector is intentionally excluded — too large for wire transfer
}

enum GqlDomain { Pdf Email Whatsapp Calendar Codebase Web Custom }

enum GqlNodeKind {
  Entity
  Topic
  Artifcat      # ⚠️ preserved typo — matches SurrealDB records; do not rename
  Event
  Conversation
  ExternalRef
}

type GqlScoredNode {
  node:  GqlNode!
  score: Float!   # cosine similarity [0.0, 1.0]
}

type GqlPath {
  nodes: [GqlNode!]!
  found: Boolean!
}

type GqlSaveResult {
  nodesSaved: Int!
  edgesSaved: Int!
}

input GqlNodeInput {
  id:         String        # omit to create new
  domain:     GqlDomain!
  sourceUri:  String!
  sourceText: String!
  kind:       GqlNodeKind!
  metadata:   [GqlMetadataInput!]
  zone:       Int           # default: 0
  embeddings: [Float!]      # optional; existing embeddings preserved if omitted on update
}

input GqlEdgeInput {
  from:                    String!
  to:                      String!
  label:                   String!
  token:                   Int    # default: 1
  relationshipProbability: Float  # default: 0.9
}

input GqlQueryConfig {
  similarityTopK:   Int  # default: 20
  expansionDepth:   Int  # default: 2
  maxSubgraphNodes: Int  # default: 200
  maxZone:          Int  # 0 = own only, 1 = include network
}
```

#### Authentication

`fluvio-graph` performs **no JWT validation**. The Apollo Router injects the caller's identity as the `x-user-id` header (a UUID) after your auth proxy validates it. Resolvers read it from Axum request extensions.

If the header is missing, resolvers return:
```
x-user-id header missing — request must go through the gateway.
In dev, set 'x-user-id: <uuid>' in GraphiQL headers.
```

In development: open Apollo Sandbox at `http://localhost:4001`, add `{"x-user-id": "your-uuid"}` to the request headers panel.

#### Embedding model

| Detail | Value |
|--------|-------|
| Model | `BGE-small-EN-v1.5` |
| Dimensions | 384 |
| Library | `fastembed` (Rust) |
| Runtime | In-process — no external API, no network call |
| Cache | Downloaded from HuggingFace on first startup; subsequent starts load from local cache |
| Concurrency | `Arc<RwLock<EmbeddingContext>>` — write lock per embed call |

#### SurrealDB environment variables

| Variable | Default | Notes |
|----------|---------|-------|
| `SURREAL_URL` | `ws://127.0.0.1:8000` | Use `embedded` for in-process KV store |
| `SURREAL_USER` | `root` | |
| `SURREAL_PASS` | `root` | |
| `SURREAL_NS` | `fluvio` | |
| `SURREAL_DB` | `graph` | |

`SURREAL_URL=embedded` runs SurrealDB in-process using `surrealkv` — no separate SurrealDB process required. Note: the `surreal sql` CLI cannot inspect embedded data.

#### Source layout

```
services/fluvio-graph/src/
├── main.rs                 entry point — tracing setup, loads .env, calls serve()
├── lib.rs                  re-exports server module
├── server.rs               Axum app — AppState, CORS, user_id middleware, /health, serve()
├── graph.rs                FluvioGraph trait + DomainGraph impl — in-memory graph for Dijkstra/BFS
├── registry.rs             GraphRegistry — multi-graph coordinator (future multi-tenant)
├── query_context.rs        request-scoped in-memory subgraph for path queries
├── embeddings.rs           EmbeddingContext wrapping fastembed BGE-small
├── graphql/
│   ├── mod.rs              build_schema(), graphql_router(), extract_user_id_from_headers()
│   ├── query.rs            QueryRoot: node, nodes, search, networkSearch, neighbors, shortestPath
│   ├── mutation.rs         MutationRoot: upsertNode, deleteNode, upsertEdge, saveGraph,
│   │                         deleteUserGraph, deleteWorkspaceNodes
│   ├── subscription.rs     SubscriptionRoot (reserved for graph event streaming)
│   └── types.rs            GqlNode, GqlEdge, GqlScoredNode, GqlPath, all input types,
│                             enum conversions, GqlNodeKind (with preserved Artifcat typo)
└── storage/
    ├── mod.rs
    ├── surreal.rs          SurrealStorage — connect, init_schema, upsert/get/delete nodes & edges,
    │                         BFS, similarity_search_nodes, network_similarity_search,
    │                         delete_user_graph, delete_workspace_nodes, cosine_sim
    └── cache.rs            reserved — optional in-memory LRU cache
```

#### Running standalone

```bash
# External SurrealDB
PORT=3001 SURREAL_URL=ws://127.0.0.1:8000 cargo run -p fluvio-graph

# Embedded SurrealDB (no separate process)
PORT=3001 SURREAL_URL=embedded cargo run -p fluvio-graph

# Docker (build from workspace root)
docker build -f services/fluvio-graph/Dockerfile -t fluviome/graph .
docker run \
  -e PORT=3001 \
  -e SURREAL_URL=ws://host.docker.internal:8000 \
  -p 3001:3001 \
  fluviome/graph
```

Health: `GET http://localhost:3001/health` → `"ok"`

---

### fluvio-twin

**Port:** `3002`
**Language:** Rust — Axum · async-graphql · SurrealDB
**Role:** Digital twin and workspace management. Each user has a personal workspace that aggregates their knowledge, documents, and pipeline state. Manages workspace creation, member sharing, and workspace-scoped access.

---

### fluvio-database

**Port:** `3005`
**Language:** Rust — Axum · async-graphql · PostgreSQL (sqlx)
**Role:** Relational persistence for users, companies, teams, connectors, and chat history. SurrealDB holds the graph; Postgres holds structured business entities. Also owns the `getUserByFirebaseUid` / `createUser` mutations used during auth sync, and the encrypted per-user LLM provider store (see [LLM Providers (BYOK)](#llm-providers-byok)) — including a non-GraphQL internal route other services use to resolve a decrypted credential, deliberately kept outside the public schema.

---

### fluvio-ingestion

**Port:** `3004`
**Language:** Rust — Axum · async-graphql · SurrealDB
**Role:** Pipeline ingestion subgraph. Receives raw documents and data source payloads, chunks and tags them, then pushes structured nodes into `fluvio-graph`. Handles PDF text extraction, chunking strategy, and source metadata.

---

### fluvio-collab

**Port:** `3003`
**Language:** Rust — Axum · async-graphql · SurrealDB
**Role:** Real-time collaboration — workspace sharing, team graph access, workspace invitations, and approval workflows.

> **Enterprise-gated** — requires `FLUVIOME_ENTERPRISE_TOKEN`. The service starts in community mode without the token but gates collaboration mutations behind the tier check.

---

### fluvio-connectors

**Port:** `3006`
**Language:** Python — FastAPI · Strawberry GraphQL
**Role:** Data source connectors. Bridges external systems into fluvioMe:

| Connector | What it ingests |
|-----------|----------------|
| PostgreSQL / MySQL | Schema sync → knowledge graph nodes |
| GitHub | Repositories, issues, PRs → graph nodes |
| Notion | Pages and databases → graph nodes |
| Row-to-node | Tabular rows → typed nodes with embeddings |

---

### fluvio-tool-builder

**Port:** `3008`
**Language:** Python — FastAPI · Strawberry GraphQL
**Role:** Tool execution engine. Exposes `executeTool(toolId, inputs)` called by the agent-planner worker. Tools are defined as YAML manifests and executed in sandboxed environments.

Built-in tools: `spark` (SQL), `dbt`, `dashboard-syncer` (Tableau / PowerBI), `email-sender`, `pdf-report`, `kafka`, `latex`.

---

### agent-planner

**Port:** `3007`
**Language:** Python — FastAPI
**Role:** AI orchestration layer — natural language → pipeline plan → compile → deploy.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/chat` | POST | Conversation loop → generates pipeline plan Markdown |
| `/history/{workspace_id}` | GET | Chat history for a workspace |
| `/plan/compile` | POST | Compile approved plan Markdown → validated JSON step array |
| `/plan/context` | POST | Raw knowledge graph context assembly |
| `/deploy` | POST | Enqueue pipeline job → returns `job_id` immediately |
| `/jobs/{job_id}/status` | GET | Poll: queued / running / completed / failed |
| `/jobs/{job_id}/stream` | GET | SSE stream of real-time execution logs |
| `/deployments/{workspace_id}` | GET | Deployment audit history |
| `/sandbox/provision` | POST | Provision Docker sandbox |
| `/sandbox/resolve` | POST | Resolve sandbox port mappings |
| `/circuit-breakers` | GET | Per-tool circuit breaker states |
| `/capabilities/search` | POST | Reuse-first lookup — semantic search for an existing synthesized capability covering a goal |
| `/capabilities/synthesize` | POST | CSP synthesizes, sandbox-tests, and persists a new capability if no reusable match exists |
| `/health` | GET | Health check |

Uses whichever LLM provider the calling user has connected (see [LLM Providers (BYOK)](#llm-providers-byok)) for plan generation, reflection, step compilation, and CSP capability synthesis — falling back to a deployment-level key (`ANTHROPIC_API_KEY` etc.) if the user hasn't connected one. `fluvio-twin` and `fluvio-collab` resolve providers the same way; `fluvio-database` owns the encrypted credential store.

Pipeline features: circuit breaker (Phase 7), idempotent step execution (Phase 9), audit trail + rollback (Phase 11), plan reflection (Phase 18), RAG deployment memory (Phase 19), intent disambiguation (Phase 20), tool capability graph (Phase 22), SQL EXPLAIN validation (Phase 23).

---

### fluvio-gateway

**Port:** `4001`
**Tech:** Apollo Router
**Role:** GraphQL federation gateway. Composes all subgraphs into a single unified `/graphql` schema. Propagates `x-user-id` and `x-fluviome-token` headers to all downstream services.

This is the **only public API surface** — the SDK, web UI, and agent-planner all call `:4001`.

Apollo Sandbox available at `http://localhost:4001` for interactive schema exploration.

---

## GraphQL API

Full introspected schema available at `http://localhost:4001` via Apollo Sandbox.

**Common operations:**

```graphql
# Knowledge graph — nodes
query {
  nodes(workspaceId: "ws-id") {
    id sourceText domain kind isEmbedded embeddingDimensions
  }
}

# Semantic search
query {
  search(query: "customer churn analysis", workspaceId: "ws-id") {
    node { id sourceText domain }
    score
  }
}

# Create a node
mutation {
  upsertNode(input: {
    domain: Custom
    sourceUri: "notion://page/abc123"
    sourceText: "Q3 revenue declined 12% due to churn in SMB segment"
    kind: Topic
    metadata: [{ key: "workspace_id", value: "ws-id" }]
  }) { id isEmbedded }
}

# Workspaces
query { myWorkspaces(userId: "uid") { id name } }
mutation { createWorkspace(input: { name: "Q4 Pipeline", userId: "uid" }) { id } }

# Graph traversal
query { neighbors(id: "node-uuid", depth: 2) { id sourceText } }
query { shortestPath(from: "uuid-a", to: "uuid-b") { nodes { id } found } }

# Connect an LLM provider (BYOK) — see "LLM Providers (BYOK)" below
mutation {
  connectLlmProvider(input: { provider: "anthropic", apiKey: "sk-ant-..." }) {
    id provider hasApiKey isDefault
  }
}
query { getUserLlmProviders { id provider hasApiKey isDefault baseUrl defaultModel } }
```

**Agent planner (REST, not GraphQL):**

```bash
# Natural language planning
curl -X POST http://localhost:3007/chat \
  -H "x-user-id: your-uuid" \
  -H "content-type: application/json" \
  -d '{"workspace_id":"ws-id","message":"Run a monthly churn report from the orders table"}'

# Compile plan → steps
curl -X POST http://localhost:3007/plan/compile \
  -H "x-user-id: your-uuid" \
  -H "content-type: application/json" \
  -d '{"workspace_id":"ws-id","approved_markdown":"..."}'

# Deploy
curl -X POST http://localhost:3007/deploy \
  -H "x-user-id: your-uuid" \
  -H "content-type: application/json" \
  -d '{"workspace_id":"ws-id","steps":[...]}'
```

---

## Configuration

Copy `.env.example` to `.env`:

```bash
# ── SurrealDB ─────────────────────────────────────────────────────────────────
SURREAL_URL=ws://127.0.0.1:8000    # or "embedded" for in-process
SURREAL_USER=root
SURREAL_PASS=root
SURREAL_NS=fluvio
SURREAL_DB=graph

# ── PostgreSQL ────────────────────────────────────────────────────────────────
DATABASE_URL=postgresql://fluviome:fluviome@localhost:5432/fluviome

# ── Service ports (all have sensible defaults) ────────────────────────────────
FLUVIO_GRAPH_PORT=3001
FLUVIO_TWIN_PORT=3002
FLUVIO_COLLAB_PORT=3003
FLUVIO_INGESTION_PORT=3004
FLUVIO_DATABASE_PORT=3005
FLUVIO_CONNECTORS_PORT=3006
FLUVIO_AGENT_PLANNER_PORT=3007
FLUVIO_TOOL_BUILDER_PORT=3008

# ── LLM Providers (BYOK) ────────────────────────────────────────────────────
# Set ONE of these and you're done — this is the deployment-wide default,
# used whenever a user hasn't connected their own provider via the
# connectLlmProvider GraphQL mutation. None are required to boot; the engine
# just returns "no LLM provider configured" for AI features until one is set.
# DEFAULT_MODEL is optional for all four — omit it to use the built-in
# default (claude-sonnet-4-20250514 / gpt-4o / gemini-2.0-flash / llama3.1).
ANTHROPIC_API_KEY=                 # sk-ant-...
ANTHROPIC_DEFAULT_MODEL=

OPENAI_API_KEY=                    # sk-...
OPENAI_DEFAULT_MODEL=

GEMINI_API_KEY=
GEMINI_DEFAULT_MODEL=

# Local/self-hosted, no API key or spend — e.g. `ollama serve` running on
# your machine. Under docker-compose, reach the host via
# host.docker.internal (already wired up via extra_hosts on the services
# that make LLM calls); running scripts/dev.sh locally, localhost works.
OLLAMA_BASE_URL=                   # e.g. http://host.docker.internal:11434
OLLAMA_DEFAULT_MODEL=              # must match a model you've pulled, e.g. llama3:latest

# AES-256-GCM key (32 raw bytes, base64) encrypting per-user BYOK credentials
# at rest. Optional — fluvio-database boots without it, but BYOK
# connect/resolve operations error until it's set. Generate with:
#   openssl rand -base64 32
FLUVIOME_CREDENTIAL_KEY=

# Optional shared secret guarding fluvio-database's internal credential-
# resolution route (never exposed through the public gateway). Recommended
# whenever backend ports are reachable beyond localhost — docker-compose.yml
# host-publishes every service's port by default.
FLUVIOME_INTERNAL_SECRET=

# ── Enterprise (omit entirely for community / non-commercial use) ─────────────
FLUVIOME_ENTERPRISE_TOKEN=         # issued at https://fluviome.com
FLUVIOME_PUBLIC_KEY=               # RS256 public key for offline token verification
```

### LLM Providers (BYOK)

**For local dev / getting a deployment running: just set one thing in `.env`.**
No code, no GraphQL call, nothing to run. Pick one provider block from the
`.env.example` snippet above, fill it in, restart the affected services
(`docker compose up -d`, or re-run `scripts/dev.sh`), and every AI feature
(twin chat, collab chat, agent-planner) uses it automatically as the
deployment-wide default. This is the only step a new developer needs.

The cheapest way to try it with zero API spend is a local Ollama model:

```bash
# 1. Install Ollama and pull a model: https://ollama.com
ollama pull llama3

# 2. In .env:
OLLAMA_BASE_URL=http://host.docker.internal:11434   # localhost:11434 for scripts/dev.sh
OLLAMA_DEFAULT_MODEL=llama3:latest                  # must match `ollama list` exactly

# 3. Restart whatever's running, then try it (any endpoint that hits an LLM,
#    e.g. agent-planner's /chat, or the draftTwinRole GraphQL mutation).
```

**Per-user BYOK is a separate, optional layer on top**, for when individual
users (not just the deployment operator) should bring their own key — e.g. a
multi-tenant deployment where each person pays for their own usage. That's a
GraphQL API, not an env var, because it's per-request/per-user state, not
static config:

```graphql
mutation {
  connectLlmProvider(input: { provider: "anthropic", apiKey: "sk-ant-..." }) {
    id provider hasApiKey isDefault
  }
}
# provider: "anthropic" | "openai" | "gemini" | "ollama"
#   - anthropic / openai / gemini require apiKey
#   - ollama requires baseUrl instead (e.g. "http://ollama:11434")
# Optional: defaultModel (override the built-in default), groupId (company-
# brain scope instead of personal)

query { getUserLlmProviders { id provider hasApiKey isDefault baseUrl defaultModel } }
mutation { setDefaultLlmProvider(id: "connection-id") { id isDefault } }
mutation { disconnectLlmProvider(id: "connection-id") }
```

The response never contains the raw key — only `hasApiKey: Boolean`. Keys are
encrypted at rest (AES-256-GCM, `FLUVIOME_CREDENTIAL_KEY`) in a `llm_providers`
table scoped per-user (optionally per-group, mirroring `connectors`). If a
user hasn't connected anything, `fluvio-twin`/`fluvio-collab`/`agent-planner`
fall back to the `.env` default above — so the simple single-provider setup
always keeps working, with per-user BYOK layered on only if/when you need it.

---

## Enterprise

Enterprise tokens are issued at **[fluviome.com](https://fluviome.com)**.

Any for-profit company — at any stage and any size — requires an enterprise license. See [License](#license).

**Enterprise-gated features:**

| Feature | Service |
|---------|---------|
| Real-time collaboration | `fluvio-collab` |
| SSO / SAML / OIDC | enterprise coprocessor |
| Audit logs & compliance exports | `agent-planner` audit store |
| White-label deployments | gateway config |
| Managed cloud hosting | fluviome.com |
| SLA & priority support | fluviome.com |

**How the token works:**

```bash
# 1. Sign up at fluviome.com → subscribe → token emailed to you
# 2. Add to .env:
FLUVIOME_ENTERPRISE_TOKEN=eyJhbGci...

# 3. Community mode (default):
docker compose up

# 4. Enterprise mode (enables gate at :4002):
docker compose --profile enterprise up
```

The token is a **self-contained RS256 JWT** — no internet call required for verification. Your engine verifies it offline using the public key embedded in `FLUVIOME_PUBLIC_KEY`. Tokens are re-issued automatically via Stripe webhook on each renewal. Cancellation lets the existing token expire naturally at its 366-day boundary.

Token payload:

```json
{
  "sub":        "org_<uuid>",
  "org":        "acme-corp",
  "org_name":   "Acme Corp",
  "tier":       "starter | growth | enterprise",
  "features":   ["collaboration", "sso", "audit_logs", "white_label"],
  "stripe_sub": "sub_xxx",
  "iat": ..., "exp": ..., "iss": "https://fluviome.com"
}
```

---

## Contributing

Non-commercial contributions are welcome.

- **Bug reports:** open a GitHub Issue
- **Questions:** [hello@fluviome.com](mailto:hello@fluviome.com)
- **Enterprise enquiries:** [fluviome.com/#contact](https://fluviome.com/#contact)

By contributing you agree your code may be distributed under the BUSL-1.1 license and, after the Change Date, under Apache 2.0.

`CONTRIBUTING.md` coming soon.

---

## Repository Layout

> **Post-restructure layout (v0.1.0).** The engine is now *library-first*: the
> logic lives in `crates/` and can be linked in-process; the network servers in
> `servers/` are thin shells around it. External consumers depend on a single
> facade crate, **`fluvio-graph`**.

```
kg-engine/
├── Cargo.toml                     Rust workspace (resolver = "2")
├── Cargo.lock
├── docker-compose.yml             full stack: community + --profile enterprise
├── CHANGELOG.md                   facade (fluvio-graph) public-surface changes
├── FLUVIOME_PLAN.md               product & architecture plan
│
├── crates/                        library crates — linkable in-process
│   ├── fluvio-graph/              ★ the facade — the ONLY crate consumers depend on
│   ├── fluvio-graph-core/         graph storage · query · embeddings (SurrealDB)
│   ├── fluvio-ingestion-core/     ingestion pipeline: extract · chunk · embed
│   ├── fluvio-twin-core/          digital-twin / workspace logic
│   ├── fluvio-collab-core/        collaboration logic [enterprise]
│   ├── fluvio-database/           relational (Postgres) domain logic
│   ├── fluvio-types/              shared domain types (Node, Edge, WorkspaceId, …)
│   ├── fluvio-embed/              embedding helpers (BGE-small / fastembed)
│   ├── fluvio-common/             shared config-loading · error · tracing helpers
│   └── fluvio-auth/               internal auth stub (not the Node coprocessor)
│
├── servers/                       thin transport shells — one binary per subgraph
│   ├── graph-server/              ← crates/fluvio-graph-core                    :3001
│   ├── twin-server/               ← crates/fluvio-twin-core                     :3002
│   ├── collab-server/             ← crates/fluvio-collab-core   [enterprise]    :3003
│   ├── ingestion-server/          ← crates/fluvio-ingestion-core                :3004
│   └── database-server/           ← crates/fluvio-database                      :3005
│
├── gateway/                       Apollo Router — federates the subgraphs        :4001
│
├── examples/
│   └── embedded-consumer/         runs a grounded query in-process, no server
│
├── services/                      Python + Node services (run as processes)
│   ├── agent-planner/             AI planning — Python, FastAPI                  :3007
│   ├── fluvio-connectors/         data-source connectors — Python, FastAPI      :3006
│   ├── fluvio-tool-builder/       tool execution engine — Python, FastAPI       :3008
│   └── fluvio-auth/               enterprise token coprocessor — Node.js [ent]  :4002
│
├── enterprise/
│   ├── token-service/             Stripe → JWT issuer (Node.js)                 :4003
│   └── auth-adapter/              legacy auth adapter
│
├── docs/
│   ├── adr/                       decision records (0001 layout · 0002 open decisions)
│   └── FINDINGS.md                bugs / dead code found during the restructure
│
└── scripts/
    └── dev.sh                     full local dev stack launcher
```

---

*Licensed under [BUSL-1.1](LICENSE). Free forever for non-commercial use. Enterprise license required for any for-profit use.*
*© fluvioMe — [fluviome.com](https://fluviome.com)*
