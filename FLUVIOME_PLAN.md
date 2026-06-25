# fluvioMe — Headless Engine: Strategic Plan

> **Status:** Planning phase — this document is the brief for the dedicated planning session.

---

## Vision

**fluvioMe** is an open-source, headless engine for:
- Automated data pipelines (ETL/ELT, streaming, scheduled)
- Knowledge graphs (semantic nodes, documents, graph traversal)
- Self-serve BI reporting (PowerBI, Tableau, PDFs)
- AI planning agents (plan → compile → deploy pipelines in natural language)

"Headless" means:
- **No built-in UI** — API-first; consumers embed via SDK or REST
- **No auth layer in OSS** — bring your own identity provider (or nothing)
- **No infra lock-in** — runs as Docker image, Helm chart, EC2 AMI, bare Node/Python process
- **Plugin-first** — every connector, tool, and BI target is a swappable module

**Enterprise tier**: request an API token → monthly billing → unlocks managed hosting,
multi-tenant isolation, SLA, audit logs, SSO coprocessor, and priority support.

---

## Engine Data Flow (verified against code)

fluvioMe gives a company a semantic memory that lives on its own servers:

```
Your data sources (PDFs, DBs, APIs, docs, code, Notion, GitHub)
        │
        ▼
   fluvio-ingestion    extract → chunk (~512 tok, 64 overlap) → tag → embed
        │              BGE-small via fastembed → 384-dim vectors
        │              edge_wirer auto-connects nodes by cosine similarity
        ▼
   fluvio-graph        SurrealDB knowledge graph — nodes, edges, 384-dim vectors
        │              cosine vector search over stored embeddings
        ▼
   agent-planner       natural language → pipeline plan → compile → deploy
        │              /chat → /plan/compile → /deploy (async job queue)
        ▼
   BI outputs          PowerBI · Tableau · PDF reports · dashboards
                       (dashboard-syncer + email-sender tools)
```

**Code anchors (don't drift from these):**
- Extractors: `services/fluvio-ingestion/src/extractor/` (pdf, docx, text, codebase, detect)
- Chunking: `pipeline/chunker.rs` — ~512 token chunks, 64 token overlap
- Embedding: `pipeline/embedder.rs` — fastembed `BGESmallENV15`, **384 dimensions**
- Auto edge-wiring: `pipeline/edge_wirer.rs` — pairwise cosine sim above threshold
- Graph storage + search: `services/fluvio-graph/src/storage/surreal.rs` (`embeddings: Vec<f32>`, `cosine_sim`)
- Planning: agent-planner (see `docs/agent-planner-architecture.pptx` for full UML)

**Key insight:** the graph is not a passive vector store — `fluvio-ingestion`
computes pairwise cosine similarity between node embeddings and **auto-wires the
edges**, so the knowledge graph builds its own connective structure on ingest.

---

## Current Codebase (what we're converting FROM)

Monorepo: `~/Developer/AWS/rust/kg-engine/`

### Services (`services/`)

| Service | Tech | Current role | Headless fate |
|---------|------|-------------|---------------|
| `fluvio-auth` | Node/Express | Firebase JWT coprocessor for Apollo Router | **Remove from OSS** — auth is enterprise or BYO |
| `fluvio-gateway` | Apollo Router | GraphQL supergraph federation entry point :4001 | **Keep** — becomes the headless API surface |
| `fluvio-database` | ? | Data persistence subgraph | **Keep** — core engine |
| `fluvio-graph` | ? | Knowledge graph subgraph | **Keep** — core engine |
| `fluvio-connectors` | ? | Data source connectors (Postgres, etc.) | **Keep** — core engine |
| `fluvio-ingestion` | ? | Data pipeline ingestion | **Keep** — core engine |
| `fluvio-tool-builder` | ? | Tool manifest builder + executeTool | **Keep** — core engine |
| `fluvio-twin` | ? | Digital twin / workspace | **Keep** — core engine |
| `fluvio-collab` | ? | Collaboration features | **Review** — may be enterprise |
| `agent-planner` | Python/FastAPI :3007 | LLM planning agent | **Keep** — headline feature |

### Current Auth Flow (to REMOVE from OSS)
```
Client → fluvio-auth :4000 (Firebase JWT verify → x-user-id header injection)
       → Apollo Router :4001 (coprocessor validates header)
       → Subgraphs
```

### Headless Flow (target)
```
Client → Apollo Router :4001 directly (no auth coprocessor)
        OR SDK → wraps Apollo Router calls
        OR Docker/K8s → expose :4001 behind customer's own auth/proxy
```

---

## ✅ LOCKED DECISIONS — DO NOT REVISIT

### License: Business Source License 1.1 (BSL 1.1) — MariaDB Model

**How it works:**
- 100% of source code is public and available on GitHub
- **Free / lifetime** for: students, researchers, non-profits, open-source projects, any non-commercial use
  - No 24-hour limits. No trial periods. No feature expiry. No nag screens. Free forever.
  - "Non-commercial" = not used to generate revenue for a for-profit entity.
- **Any for-profit commercial use requires a paid enterprise license** — no scale threshold, no employee count, no ARR minimum. If you make money with it, you pay.
- After **4 years** from each release date → code converts automatically to **Apache 2.0** (fully open source forever)
- This is legally airtight and battle-tested (MariaDB, HashiCorp Terraform pre-2023, Couchbase)

**The usage line (locked):**
> "Any use by a for-profit organization — regardless of size, revenue, or stage — requires a commercial license. Non-commercial use (students, researchers, non-profits, personal projects) is free and unlimited with no time restrictions."

**What this means for the codebase:**
- Single repo, all services, one license header in every file: `SPDX-License-Identifier: BSL-1.1`
- Add `LICENSE` file (BSL 1.1 text) + `CHANGE_DATE` (4 years from release) + `CHANGE_LICENSE: Apache-2.0`
- No code is ever "enterprise-only" — everything is readable and runnable
- Revenue enforcement is contractual + API token gating, not code forking

### Business Model: BSL + Enterprise Commercial License (LOCKED)

| Who | Cost | Limit |
|-----|------|-------|
| Students, researchers, academics | **Free / lifetime** | None — no token, no expiry, full engine |
| Non-profit organizations | **Free / lifetime** | None |
| Open-source projects | **Free / lifetime** | None |
| Personal / hobby projects | **Free / lifetime** | None (as long as non-commercial) |
| **Any for-profit company** (any stage, any size) | **Paid** | Must get `FLUVIOME_ENTERPRISE_TOKEN` from fluviome.com |

- No revenue threshold, no employee count threshold — **if you're for-profit, you pay**
- Token is generated at **fluviome.com** — self-serve signup → billing → token issued
- Token unlocks: collaboration, SSO, audit logs, managed cloud, multi-tenant, SLA, support
- Pricing tiers: Starter / Growth / Enterprise Custom (TBD by fluviome.com)

### Headless Architecture (LOCKED)
- Apollo Router `:4001` is the **public API entry point** — no auth proxy in front
- `x-user-id` header is passed through as-is — consumers provide their own identity
- `FLUVIOME_ENTERPRISE_TOKEN` in env → enterprise-gate coprocessor starts on `:4002`
- Community mode = no token, no coprocessor, full engine runs, no collaboration/SSO features
- `docker compose up` starts the full community engine
- `docker compose --profile enterprise up` adds the enterprise gate

### fluvio-auth → Repurposed (LOCKED)
- **Removed from community runtime** — no longer starts by default
- Renamed role: **Enterprise Token Coprocessor** (`:4002`)
- Only starts when `FLUVIOME_ENTERPRISE_TOKEN` env var is present
- Will validate JWTs issued by fluviome.com, inject `x-fluviome-tier` header

---

## Key Decisions Remaining

### 1. What Moves to Enterprise License Enforcement?
Candidates for "requires commercial license":
- SSO / SAML / OIDC auth coprocessor (currently `fluvio-auth`)
- Multi-tenant workspace isolation
- Audit logs / compliance exports
- SLA-backed managed hosting
- Custom domain + white-label
- Advanced connector plugins (Salesforce, SAP, Snowflake)
- Priority support + dedicated Slack

### 3. SDK Design (PLANNED — next major work item)

Two SDKs, same API surface:

**`fluviome-js`** (Node/TypeScript)
```typescript
import { FluviomeClient } from '@fluviome/sdk';

const client = new FluviomeClient({
  endpoint: 'http://localhost:4001',   // or https://your-instance.fluviome.com
  userId: 'user-123',                  // your own user identity
  enterpriseToken: process.env.FLUVIOME_ENTERPRISE_TOKEN,  // optional
});

// Natural language → plan → deploy
const plan = await client.planner.chat({ workspaceId, message: 'Run a monthly churn report' });
const steps = await client.planner.compile({ workspaceId, approvedMarkdown: plan.response });
const job = await client.deploy({ workspaceId, steps });
await client.jobs.stream(job.jobId, (line) => console.log(line));

// Direct pipeline control
await client.connectors.list();
await client.graph.query({ workspaceId });
```

**`fluviome-py`** (Python)
```python
from fluviome import FluviomeClient

client = FluviomeClient(
    endpoint="http://localhost:4001",
    user_id="user-123",
    enterprise_token=os.getenv("FLUVIOME_ENTERPRISE_TOKEN"),
)
plan = await client.planner.chat(workspace_id=ws, message="Run ETL on orders table")
steps = await client.planner.compile(workspace_id=ws, approved_markdown=plan.response)
job = await client.deploy(workspace_id=ws, steps=steps)
```

SDK wraps:
- GraphQL gateway `:4001` (all KG, connector, workspace operations)
- agent-planner REST `:3007` (`/chat`, `/plan/compile`, `/deploy`, `/jobs/*`)

### 4. Distribution / Packaging (PLANNED)
- ✅ `docker compose up` — full stack, works today (after Phase A)
- `docker pull fluviome/engine` — single pre-built image (Phase C)
- Helm chart `charts/fluviome/` — K8s production (Phase C)
- `npx fluviome init` — scaffold + quickstart CLI (Phase D)
- EC2 AMI / Lightsail one-click (Phase E)

### 5. What Does the Agent Planner Expose as OSS?
Currently: requires `ANTHROPIC_API_KEY` in `.env`.
Headless option:
- OSS: agent-planner runs but requires user to supply their own LLM key (BYO-LLM)
- Enterprise: managed LLM proxy (rate-limited, billed per token)

---

## Migration Plan (phases)

### Phase A — Strip Auth from Core ✅ DONE
1. ✅ `fluvio-auth` removed from default runtime → repurposed as enterprise gate (:4002)
2. ✅ No coprocessor in `router.yaml` (commented stub ready); router :4001 is direct entry
3. ✅ Subgraphs trust `x-user-id` header; gateway propagates it + `x-fluviome-token`
4. ✅ Enterprise coprocessor only starts when `FLUVIOME_ENTERPRISE_TOKEN` is set
5. ✅ Documented in `/docs` + plan

### Phase B — Headless API Hardening  ◀ NEXT (planning session)
**Identity model (LOCKED): generic `external_id`.**
1. Rename `firebase_uid` → `external_id` across the database subgraph:
   - `services/fluvio-database/src/graphql/types.rs` (fields)
   - `services/fluvio-database/src/graphql/query.rs` (`get_user_by_firebase_uid` → `get_user_by_external_id`)
   - `services/fluvio-database/src/graphql/mutation.rs` (`createUser` input + INSERT/ON CONFLICT)
   - `services/fluvio-database/src/db/users.rs` + `db/queries.rs`
   - SQL migration: `ALTER TABLE users RENAME COLUMN firebase_uid TO external_id;`
2. `agent-planner/app/auth.py` already uses `myWorkspaces` — verify no firebase refs remain
3. The engine accepts ANY opaque user id via `x-user-id`; BYO-auth maps their IdP subject → `external_id`
4. Remove remaining Firebase strings from `fluvio-collab` client types
5. Enterprise JWT issuance already built (token-service)

### Phase B2 — TypeScript SDK (LOCKED: thin client first)
Create `sdk/typescript/` → `@fluviome/sdk`:
- `FluviomeClient({ endpoint, plannerUrl, userId, enterpriseToken })`
- `planner.chat()` / `planner.compile()` / `deploy()` / `jobs.stream()` → wrap :3007 REST
- `graph.search()` / `workspaces.list()` → wrap :4001 GraphQL
- Inject `x-user-id` + optional `x-fluviome-token` on every call
- README with the plan→compile→deploy example (already drafted in /docs)
- Python SDK (`fluviome`) mirrors it afterward

### Phase C — Packaging
1. ✅ `Dockerfile` for every service (5 Rust multi-stage, 3 Python, gateway, enterprise gate) + `docker-compose.yml`
2. ✅ `requirements.txt` for all Python services (connectors, tool-builder, agent-planner)
3. Write Helm chart (`charts/fluviome/`)  ◀ TODO
4. Write `npx fluviome init` scaffold CLI  ◀ TODO
5. Getting-started docs ✅ (the `/docs` page)

### Phase D — Open Source Release
1. Create `fluviome-engine` GitHub org / repo
2. Choose license: Apache 2.0 (permissive) or BSL (delayed open-source, Airbyte/HashiCorp style)
3. CI/CD: GitHub Actions for lint + test + Docker publish
4. Landing page / docs site

### Phase E — Enterprise Tier
1. `fluviome-enterprise` private repo: SSO coprocessor, multi-tenant, billing
2. API token issuance + Stripe integration
3. Managed cloud (ECS/EKS + RDS + managed Postgres)

---

## Files to Examine in Planning Session

```
services/fluvio-auth/src/index.js            ← auth to remove
services/fluvio-gateway/router.yaml          ← coprocessor config to strip
services/fluvio-gateway/supergraph.yaml      ← subgraph topology
services/fluvio-gateway/supergraph.graphql   ← full API surface
agent-planner/app/config.py                  ← settings (port, gateway, API key)
agent-planner/app/auth.py                    ← workspace auth (will simplify)
```

---

## Questions for the Planning Session

1. ~~Which license?~~ **LOCKED: BSL 1.1 → Apache 2.0 after 4 years (MariaDB model)**
2. Do we extract `agent-planner` as a separate OSS project or bundle it?
3. What's the minimal "zero to pipeline in 5 minutes" quickstart demo?
4. Name: **fluvioMe** or **fluviome** or something else for the OSS repo?
5. What's the first enterprise customer profile? (Internal teams? SaaS companies? Data consultancies?)
6. Do we need a `fluviome-cloud` SaaS MVP before OSS launch, or launch OSS first?

---

## Context Files Already Documented
- `docs/agent-planner-architecture.pptx` — full UML of agent-planner (13 slides)
- `docs/MCP_MIGRATION_PLAN.md` — plan to convert tools to MCP (internal + external); decision locked
- `docs/CSP_KG_INTEGRATION_PLAN.md` — CSP × knowledge graph × MCP; capabilities as `Capability` KG nodes, CSP in agent-planner, reuse-first via vector search
- `services/agent-planner/app/**` — full Python/FastAPI agent-planner codebase

## Decisions locked (later sessions)
- **SDK = thin client, NOT the engine.** `fluviome-client` (`@fluviome/client` / `pip install fluviome-client`) is pure TS/Python talking to the running Rust engine over HTTP (`:4001` gateway + `:3007` planner). Does not exist yet — to be scaffolded.
- **`fluviome-core` = different/harder model** (embed the Rust engine in the package via PyO3/maturin or napi-rs/WASM). Deferred; client SDK comes first.
- **MCP migration** (`docs/MCP_MIGRATION_PLAN.md`): make `fluvio-tool-builder` an MCP server, `agent-planner` an MCP client; expose internally + externally. Replaces the double-JSON `executeTool` transport; orchestration/reliability stays.
- **Docker registry:** publish images to GHCR (`ghcr.io/ldbtech/<service>`); compose supports `image:` + `build:`; CI to build/push on release. Not done yet.
