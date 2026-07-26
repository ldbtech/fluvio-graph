# fluvio-graph — Library-First Restructure (agent handoff)

> **Audience:** coding agents refactoring this repo.
> **Outcome:** one codebase that ships **two distributions** — a batteries-included
> self-host stack for hobbyists/students (gateway + servers, `docker compose up`),
> and a set of **pure Rust/Python libraries** that the commercial FounderTwin
> backend links in-process.
>
> This is a **restructure, not a rewrite.** Almost all logic stays put; what
> changes is *where it lives*, *what it depends on*, and *what it exposes*.
> Behaviour must be identical when you're done — the existing
> `docker compose up` experience must keep working exactly as it does today.

---

## 0. Ground rules for agents

1. **Do not change behaviour.** This is a packaging refactor. If you find a bug,
   note it in `docs/FINDINGS.md` — don't fix it in the same commit.
2. **Work phase by phase** (§13). Each phase must compile, pass tests, and keep
   `docker compose up` working before you start the next.
3. **One crate per commit** during the move phase. Big-bang moves are unreviewable.
4. Use `git mv` so history follows the files.
5. When unsure whether something belongs in a library or a server, apply the
   **golden rule** and the **mechanism-vs-policy rule** in §2.

---

## 1. Why we're doing this

| | **fluvio-graph** (public, BSL) | **FounderTwin** (commercial, private) |
|---|---|---|
| Audience | Hobbyists, students, self-hosters | Paying founders |
| How it's used | `docker compose up` → one gateway endpoint | Links crates **in-process** |
| Needs the gateway? | **Yes — it's the main selling point** | **No** |
| Needs transport in libs? | No (servers provide it) | **No — must not pull axum** |

Today the logic and the transport are welded together, so FounderTwin can't link
the engine without dragging in `axum` + `async-graphql` + a second GraphQL
schema. After this restructure, both distributions are assembled from the same
crates.

---

## 2. The rules that make this work

### 2.1 The golden rule
> **`crates/` and `packages/` MUST NEVER depend on `servers/`, `services/`, or `gateway/`.**

Libraries contain logic. Shells contain transport, config, and wiring.
Dependencies point one way only. CI enforces this (§11).

### 2.2 Mechanism vs policy
> **The engine provides mechanism. FounderTwin provides policy.**

- **Belongs in the engine:** graph storage, embeddings, retrieval, ingestion
  pipelines, **workspace/tenant namespacing** (§9) — hobbyists want multiple
  projects too, so this is a genuine feature, not SaaS leakage.
- **Stays out of the engine:** Twin IAM (groups/policies/allow-deny-approve),
  billing, entitlements, per-user LLM credentials, the mesh's approval UX.
  The engine returns knowledge **with labels**; FounderTwin decides who may see
  it. Two authorization brains would eventually disagree.

### 2.3 Libraries must not read the environment
> **A library never calls `std::env::var`. Config is passed in.**

This is currently violated — see §6.2. The binary reads env and constructs a
config struct; the library accepts it. Otherwise FounderTwin can't run two
differently-configured engine instances in one process, and testing is painful.

---

## 3. Verified current state

Confirmed by inspecting the repo (do not re-derive; just be aware):

- **`crates/`** — `fluvio-types`, `fluvio-common`, `fluvio-auth`, `fluvio-embed`.
  All are pure `lib` targets already. ✅
- **`services/` with BOTH `lib` + `bin`** (already library-shaped — good):
  `fluvio-graph`, `fluvio-database`, `fluvio-twin`, `fluvio-collab`,
  `fluvio-ingestion`.
- **`services/` that are not Rust:**
  - `agent-mcp` — Python (MCP server)
  - `agent-planner` — Python
  - `fluvio-connectors` — Python
  - `fluvio-tool-builder` — Python (has a stray `Cargo.toml`, not in the workspace)
  - `fluvio-gateway` — Apollo Router (`router.yaml`, `supergraph.graphql`)
  - `fluvio-auth` — Node.js
- **`fluvio-graph/src/lib.rs`** exposes:
  `embeddings, graph, graphql, query_context, registry, server, storage`.
  → `graphql` and `server` are **transport** and must become optional.
- **`fluvio-graph` deps include** `axum`, `tower-http`, `async-graphql`,
  `async-graphql-axum` (transport) alongside `surrealdb`, `fastembed`, `ort`
  (domain).
- **Storage is already server-capable:** `storage/surreal.rs` uses
  `surrealdb::engine::any` selected by `SURREAL_URL`, and `docker-compose.yml`
  runs SurrealDB as a server (`http://surrealdb:8000`, `surrealkv://data`).
  **No storage migration is required.** ✅

### 3.1 Two defects found — fix during Phase 1
1. **Duplicate workspace member.** `Cargo.toml` lists `"services/fluvio-collab"`
   **twice**. Remove the duplicate.
2. **Inconsistent workspace membership.** `services/fluvio-tool-builder` has a
   `Cargo.toml` but is not a workspace member; `fluvio-connectors`, `agent-mcp`,
   `agent-planner`, `fluvio-gateway`, `services/fluvio-auth` are non-Rust and
   correctly excluded. Either make tool-builder a real member or delete its
   stray `Cargo.toml` (it is a Python service — prefer deleting).

---

## 4. Target structure

```
fluvio-graph/
├── crates/                      # ← RUST LIBRARIES (the product surface)
│   ├── fluvio-types/            # unchanged
│   ├── fluvio-common/           # unchanged
│   ├── fluvio-embed/            # unchanged (fastembed / ort)
│   ├── fluvio-database/         # moved from services/
│   ├── fluvio-graph-core/       # from services/fluvio-graph (lib half)
│   ├── fluvio-ingestion-core/   # from services/fluvio-ingestion (lib half)
│   ├── fluvio-twin-core/        # from services/fluvio-twin (lib half)
│   ├── fluvio-collab-core/      # from services/fluvio-collab (lib half)
│   └── fluvio-graph/            # FACADE: re-exports a curated public API (§6.3)
│
├── servers/                     # ← RUST THIN BINARIES (transport shells)
│   ├── graph-server/            # axum + async-graphql subgraph
│   ├── ingestion-server/
│   ├── twin-server/
│   └── collab-server/
│
├── packages/                    # ← PYTHON LIBRARIES (importable, no web framework)
│   ├── fluvio-planner/          # planner core from services/agent-planner
│   ├── fluvio-tools/            # tool synthesis core from fluvio-tool-builder
│   └── fluvio-connectors-core/  # provider/normalisation logic
│
├── services/                    # ← PYTHON/NODE THIN SHELLS
│   ├── agent-planner/           # FastAPI app importing packages/fluvio-planner
│   ├── agent-mcp/               # MCP server importing packages/fluvio-tools
│   └── fluvio-connectors/       # app importing packages/fluvio-connectors-core
│
├── gateway/                     # Apollo Router config (moved from services/fluvio-gateway)
├── examples/                    # adoption driver — students copy from here
├── sdk/                         # thin Python/TS clients (MIT, see §12)
├── enterprise/                  # unchanged (licence gate / token-service)
├── docs/
└── docker-compose.yml           # unchanged UX: one command, whole stack
```

**Naming convention:** `*-core` = library, no transport. The unsuffixed
`fluvio-graph` becomes the **facade** crate that most consumers depend on.

---

## 5. Move map (mechanical — do exactly this)

| From | To | Notes |
|---|---|---|
| `crates/fluvio-types` | `crates/fluvio-types` | unchanged |
| `crates/fluvio-common` | `crates/fluvio-common` | unchanged |
| `crates/fluvio-embed` | `crates/fluvio-embed` | unchanged |
| `crates/fluvio-auth` | *(see §8.3)* | decide: retire or keep optional |
| `services/fluvio-database` (lib) | `crates/fluvio-database` | drop the bin unless it has standalone value |
| `services/fluvio-graph` (lib) | `crates/fluvio-graph-core` | **minus** `graphql` + `server` modules |
| `services/fluvio-graph` (bin + `graphql`/`server` modules) | `servers/graph-server` | keeps axum/async-graphql |
| `services/fluvio-ingestion` (lib) | `crates/fluvio-ingestion-core` | |
| `services/fluvio-ingestion` (bin) | `servers/ingestion-server` | |
| `services/fluvio-twin` (lib) | `crates/fluvio-twin-core` | |
| `services/fluvio-twin` (bin) | `servers/twin-server` | |
| `services/fluvio-collab` (lib) | `crates/fluvio-collab-core` | |
| `services/fluvio-collab` (bin) | `servers/collab-server` | |
| `services/fluvio-gateway` | `gateway/` | pure config move |
| `services/agent-planner` (logic) | `packages/fluvio-planner` | keep the app shell in `services/` |
| `services/fluvio-tool-builder` (logic) | `packages/fluvio-tools` | delete its stray `Cargo.toml` |
| `services/fluvio-connectors` (logic) | `packages/fluvio-connectors-core` | |
| `services/agent-mcp` | `services/agent-mcp` | stays a shell; imports `packages/fluvio-tools` |
| `services/fluvio-auth` (Node) | **retire** | FounderTwin uses Supabase; see §8.3 |

---

## 6. Required refactors

### 6.1 Feature-gate all transport
Every `*-core` crate defaults to **domain only**. Transport deps become optional:

```toml
# crates/fluvio-graph-core/Cargo.toml
[features]
default = []
server  = ["dep:axum", "dep:async-graphql", "dep:async-graphql-axum", "dep:tower-http"]

[dependencies]
axum               = { workspace = true, optional = true }
async-graphql      = { workspace = true, optional = true }
async-graphql-axum = { workspace = true, optional = true }
tower-http         = { workspace = true, optional = true }
```

`servers/graph-server` depends on `fluvio-graph-core = { features = ["server"] }`.
**Acceptance:** `cargo build -p fluvio-graph-core --no-default-features` succeeds
and `cargo tree -p fluvio-graph-core` shows **no axum and no async-graphql**.

### 6.2 Remove `env::var` from library code
`storage/surreal.rs` currently reads `SURREAL_URL`, `SURREAL_USER`,
`SURREAL_PASS`, `SURREAL_NS`, `SURREAL_DB` **inside the library**. Replace with
an injected config struct:

```rust
// crates/fluvio-graph-core — shape only, not final code
pub struct SurrealConfig {
    pub url: String, pub user: String, pub pass: String,
    pub namespace: String, pub database: String,
}
impl SurrealStore {
    pub async fn connect(cfg: &SurrealConfig) -> Result<Self, Error> { /* ... */ }
}
```
The **binaries** read env and build `SurrealConfig`. Provide
`SurrealConfig::from_env()` in the *server* crate (or behind an `env` feature),
never in the core path. Apply the same treatment to every other `env::var` in
`crates/` — grep for them and fix all.

**Acceptance:** `grep -rn "env::var" crates/` returns nothing (outside of tests
or an explicitly `env`-featured module).

### 6.3 Add the facade crate
`crates/fluvio-graph` re-exports a **curated** public API so you can refactor
internals without breaking consumers:
- Re-export the types users actually need; keep everything else private or
  `#[doc(hidden)]`.
- Provide a `prelude` module.
- This crate is what FounderTwin and the community depend on.

### 6.4 Workspace hygiene
- Remove the duplicate `services/fluvio-collab` member.
- Rewrite `[workspace] members` for the new layout (`crates/*`, `servers/*`).
- Move shared deps into `[workspace.dependencies]`; crates use
  `{ workspace = true }`.
- Set `[workspace.package]` `version`, `license`, `repository`, `rust-version`
  and inherit in each crate.

---

## 7. Python: same principle

Apply library-first to Python too, so FounderTwin can `pip install` the planner
and tool logic instead of copying it:

- **`packages/*`** — importable libraries. No FastAPI, no web framework, no env
  reads, no global state. Pure functions/classes + typed config objects.
- **`services/*`** — thin apps that import a package and add transport (FastAPI
  / MCP server), config, and wiring.
- Each package gets its own `pyproject.toml` and is independently installable
  (path deps locally, published later if you choose).

**Acceptance:** `packages/fluvio-planner` imports cleanly in a bare venv with no
web framework installed.

---

## 8. Distribution & the gateway

### 8.1 What hobbyists get (must not regress)
`docker compose up` → SurrealDB + all `servers/*` subgraphs + Apollo Router at
the gateway port. One endpoint, zero Rust knowledge required. **This is the
adoption funnel — protect it.** Add `examples/` showing: ingest a document, ask a
grounded question, traverse the graph.

### 8.2 What FounderTwin links
Only `crates/`: `fluvio-graph` (facade) → `fluvio-graph-core`,
`fluvio-ingestion-core`, `fluvio-embed`, `fluvio-database`. It never compiles
`servers/`, `gateway/`, or `services/`.

### 8.3 `fluvio-auth` decision
FounderTwin uses Supabase, so **it does not use `fluvio-auth`**. For the public
distribution, either (a) retire the Node `services/fluvio-auth` entirely and let
self-hosters put their own proxy in front, or (b) keep it as an **optional
compose profile**. Recommendation: **(a) retire it** — `enterprise/` already
covers the licence-gate concern, and one fewer runtime is one fewer thing to
maintain. Record the choice in `docs/adr/`.

---

## 9. Multi-tenancy (workspace namespacing) — engine mechanism

FounderTwin is multi-tenant and the engine is currently single-workspace. Per
§2.2 this belongs **in the engine**, and hobbyists benefit too (multiple
projects).

- Add an explicit **`WorkspaceId`** (a.k.a. tenant) parameter threaded through
  every public API — ingestion, retrieval, graph traversal.
- Map it to a **SurrealDB namespace/database per workspace** (strong isolation)
  rather than an owner column with filters. One missed filter = cross-tenant
  leak; namespaces fail safe.
- **No public API may access data without a `WorkspaceId`.** Make it a required
  argument, not an `Option`.
- Do **not** add IAM/policy here — labels and metadata only.

**Acceptance:** an integration test proves workspace A cannot read workspace B's
nodes, vectors, or documents through any public API.

---

## 10. Versioning & how FounderTwin consumes it

- **Semver from day one.** Tag `v0.1.0` when Phase 4 completes.
- Breaking changes to the **facade** require a minor bump pre-1.0 and a note in
  `CHANGELOG.md`. Internal churn behind the facade is free.
- FounderTwin pins by tag:
  `fluvio-graph = { git = "...", tag = "v0.1.0", default-features = false }`
- For local co-development, FounderTwin uses a `[patch]` path override — so
  **do not break the crate names**, or that override stops resolving.

---

## 11. CI guardrails (add these — they're what keep the rule true)

1. **Golden-rule check:** fail if anything under `crates/` or `packages/`
   references `servers/`, `services/`, or `gateway/`.
2. **No-transport check:** `cargo tree -p fluvio-graph-core --no-default-features`
   must not contain `axum` or `async-graphql`.
3. **No-env check:** `grep -rn "env::var" crates/` must be empty.
4. **Lean build:** `cargo build --workspace --no-default-features` must succeed.
5. **Full build:** `cargo build --workspace --all-features` must succeed.
6. **Compose smoke test:** boot the stack, ingest a fixture, run one grounded
   query, assert a non-empty grounded answer with citations.

---

## 12. Licensing tuning (do this while you're in here)

BSL protects you from a competitor hosting fluvio-graph as a service — that's
the real threat — but it scares off the students you want. As sole copyright
holder you can tune it freely:
- Write a **generous Additional Use Grant**: explicitly permit personal,
  educational, research, and non-production use. Say it in **plain English at the
  top of the README**, above the licence name.
- Set a **Change Date** (BSL converts to Apache 2.0 after ~2–4 years).
- License **`examples/` and `sdk/` as MIT** so people can copy sample code
  without thinking about it.

---

## 13. Phased plan

Each phase ends green: workspace builds, tests pass, `docker compose up` works.

- **Phase 0 — Baseline.** Record current behaviour: boot compose, run an ingest +
  a grounded query, save the outputs as fixtures. These are your regression
  oracle for every later phase.
- **Phase 1 — Hygiene.** Fix the duplicate workspace member, remove the stray
  `fluvio-tool-builder/Cargo.toml`, normalise `[workspace.package]`/deps. No moves yet.
- **Phase 2 — Feature-gate transport.** Add `server` features to the five
  lib+bin crates *in place*. Binaries build with `--features server`. Prove the
  no-transport check (§11.2) passes before moving any files.
- **Phase 3 — Config injection.** Remove `env::var` from library code (§6.2).
- **Phase 4 — The move.** `git mv` libs → `crates/`, bins → `servers/`, gateway →
  `gateway/`. One crate per commit. Add the facade crate. Tag **`v0.1.0`**.
- **Phase 5 — Python split.** Extract `packages/*` from the Python services;
  services become thin shells.
- **Phase 6 — Tenancy.** Implement `WorkspaceId` + namespace isolation (§9).
- **Phase 7 — Polish.** `examples/`, README rewrite (lead with "what is this /
  quickstart / licence in plain English"), CI guardrails, licence tuning.

---

## 14. Definition of done

- [ ] `cargo build -p fluvio-graph-core --no-default-features` pulls **no** axum
      or async-graphql.
- [ ] No `env::var` anywhere under `crates/`.
- [ ] `crates/` has zero dependencies on `servers/`/`services/`/`gateway/`
      (CI-enforced).
- [ ] `docker compose up` gives the **same** hobbyist experience as before the
      refactor (Phase 0 fixtures still pass).
- [ ] A sample external consumer can add the facade crate by git tag and run a
      grounded query in-process, with no server running.
- [ ] Workspace isolation integration test passes (§9).
- [ ] `v0.1.0` tagged; `CHANGELOG.md` started.
- [ ] README leads with a plain-English licence summary and a quickstart.

---

## 15. What NOT to do

- ❌ Don't put Twin IAM, billing, entitlements, or per-user LLM keys in this repo.
- ❌ Don't delete the gateway — it's the public product's main selling point.
- ❌ Don't let `crates/` read env vars, own global state, or init logging/tracing
  subscribers (that's the binary's job).
- ❌ Don't change public behaviour during the move; refactor and fix in separate
  commits.
- ❌ Don't rename crates casually after `v0.1.0` — FounderTwin's `[patch]`
  override resolves by crate name.
- ❌ Don't merge this repo into the FounderTwin backend. Separate products,
  separate cadences, one public and one private.

---

### Change log
- v1 — library-first restructure plan: `crates/` (libs) + `servers/` (thin bins)
  + `gateway/`, Python `packages/` vs `services/`, transport feature-gating,
  config injection, facade crate, workspace tenancy, CI guardrails, BSL tuning.
