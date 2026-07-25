# 2. Open decisions the restructure deliberately did not make

Date: 2026-07-24
Status: **Proposed — these need the owner's call**

The library-first restructure (ADR 0001) stopped after Phase 4 / `v0.1.0`. Each
item below is a genuine fork where the cheap option conflicts with the plan, or
where the change is behavioural rather than packaging. They are written down
rather than guessed at.

## 2.1 Python config injection (blocks Phase 5)

**Done — Option A landed.** A frozen, env-free `PlannerConfig`
(`app/planner_config.py`) is now injected into the five domain modules;
`Settings.as_planner_config()` is the single place env values cross into the
domain. `config.py` stays in the service. Verified: importing any of the domain
modules (plan_writer, mcp_client, orchestrator, harness, worker) no longer
constructs the settings singleton — `app.config` never enters `sys.modules` as a
side effect, transitively either. The file *move* into `packages/` (the second
half of Phase 5) is now mechanical and unblocked. Original analysis kept below.

`services/agent-planner/app/config.py` ends with `settings = Settings()` — a
pydantic-settings singleton constructed at **import time**, reading env vars and
two `.env` files whose paths are derived from `__file__` (`parents[3]`).

Ten modules import it. Five are transport (`main.py`, `routers/*`) and five are
domain (`evals/harness.py`, `capabilities/mcp_client.py`,
`capabilities/orchestrator.py`, `agent/plan_writer.py`, `jobs/worker.py`).

Plan §7 says packages must have **no env reads**, so `config.py` cannot simply
move into `packages/fluvio-planner`. (It would also break: `parents[3]` is
relative to the file's location.)

- **Option A (matches the plan):** `config.py` stays in the service. The five
  domain modules take an injected typed config instead of importing a
  singleton. Correct, and the reason the restructure exists — but it changes
  function signatures across the planner's domain layer, which is a behavioural
  refactor, not a move.
- **Option B:** move `config.py` into the package and fix the path derivation.
  Fast, but bakes an env read into a library and leaves the same problem for
  FounderTwin, which needs two differently-configured planners in one process.

**Recommendation: A**, as its own commit *before* the file moves, so the move
stays mechanical and reviewable.

## 2.2 `WorkspaceId` — foundation landed; two forks remain (Phase 6)

**Resolved (Phase 6).** Both forks were decided by the owner and implemented:

- **Fork A → strict + backfill migration.** The read/scoping APIs
  (`get_user_nodes`, `similarity_search[_nodes]`, `QueryContext::from_text/from_embedding`,
  `delete_workspace_nodes`) now take a **required** `&WorkspaceId` instead of
  `Option<&str>`; the `None → workspace_id = NONE` fallback is gone. Pre-tenancy
  nodes are handled by `SurrealStorage::backfill_default_workspace`, an
  idempotent migration that stamps untagged nodes with `default_workspace()`.
  Breaking change to the facade → minor bump (`v0.2.0`).
- **Fork B → hardened metadata filter, NOT namespace-per-workspace.** Empirical
  finding (test `embedded_surrealkv_is_single_connection_per_path`): the embedded
  surrealkv store rejects a second connection to the same path
  (`LOCK is already locked`). Connection/namespace-per-workspace therefore cannot
  work on the embedded backend — a first-class store and the CI test's backend.
  So isolation stays a single-connection metadata filter, now **required** (Fork
  A) and **injection-safe** (§2.6). The per-op-`use_db`-under-a-lock alternative
  was rejected: it serialises all storage access, defeating the multi-tenant goal.

Verified against embedded surrealkv (`cargo test -p fluvio-graph-core`, 6 tests
green) and the facade compiles with the new signatures. The graph-server resolver
wiring was edited but not compiled here (async-graphql build exceeded available
disk); it needs a `cargo build` on a machine with headroom. Original analysis
below.


**Done (uncommitted, awaiting review):**
- `WorkspaceId` newtype added to `fluvio-types` (rejects empty ids; has a named
  `default_workspace()` for single-tenant use) and re-exported from the facade.
- `crates/fluvio-graph-core/tests/workspace_isolation.rs` — §9's acceptance
  oracle, running against an **embedded** `surrealkv://` store so `cargo test`
  proves cross-workspace read isolation with no Docker. Green. Any future change
  to the isolation mechanism must keep it green.

**Fork A — making the argument required is a data migration, not a signature
change.** Today `workspace_id: Option<&str>` with `None` compiles to the filter
`metadata.workspace_id = NONE`, i.e. "only nodes that have no workspace tag."
Every node ingested so far has no `workspace_id` in its metadata, so it lives in
that no-workspace bucket. The moment the argument becomes a required
`WorkspaceId`, those rows match *no* workspace and effectively disappear from
reads until backfilled. So Phase 6 needs a migration (stamp existing nodes with
`default_workspace()`), decided and run against real data — which cannot be
verified here. This is the breaking, behaviour-changing half; it is why the
newtype was *added* but not yet *threaded through* the read APIs.

**Fork B — the isolation mechanism: filter vs namespace, over a shared
connection.** Plan §9 prefers a SurrealDB namespace/database **per workspace**
("namespaces fail safe") over the current metadata filter. But `AppState` holds
a single `Arc<SurrealStorage>` — one `Surreal<Any>` connection shared across all
requests. Namespace-per-workspace therefore forces a design choice with
correctness stakes:
  - **Per-op `use_db(workspace)`** on the shared connection — racy: a concurrent
    request can switch the active database mid-query.
  - **Connection-per-workspace**, cached (e.g. `DashMap<WorkspaceId, Surreal>`)
    — concurrency-safe for remote `ws://`, but multiple embedded `surrealkv://`
    handles to one path may conflict, which would also break the isolation test
    above.
This is a security boundary whose runtime behaviour I cannot exercise here
(Docker is down; the embedded store has its own single-writer constraints), so
the mechanism swap is deliberately left for a decision + a run against real
SurrealDB. The metadata-filter mechanism stays in place until then, now with the
isolation test guarding it.

Either way, threading `WorkspaceId` through the facade's read methods is a
breaking change — minor bump + `CHANGELOG` entry — and far cheaper before
external consumers pin `v0.1.0` than after.

## 2.6 Query layer interpolates all values into SurrealQL strings

**Fixed for the tenancy filter (Phase 6).** `workspace_id` (and `domain` in
`get_user_nodes`) are now passed via `.bind()` rather than `format!`-interpolated,
so a crafted id like `x' OR '1'='1` is treated as an opaque value and cannot
widen the scope — guarded by test `crafted_workspace_id_cannot_escape_the_filter`.
`owner_id`/`zone` remain interpolated (typed `Uuid`/`i16`, not injectable).
Other queries outside the tenancy path may still interpolate typed values;
audit separately. Original note below.


Not specific to tenancy, but adjacent: every storage query builds SurrealQL by
`format!`-interpolating `owner_id`, `domain`, `zone`, and `workspace_id`
directly into the string rather than binding them (there is a code comment that
`.bind()` was avoided for complex types in SurrealDB 3.x). `owner_id`/`zone` are
typed (`Uuid`/`i16`) so low-risk, but `domain` and `workspace_id` originate as
strings from GraphQL input — a `workspace_id` like `x' OR '1'='1` would break
scoping. When Fork B is settled, bind these values (or validate/escape them at
the `WorkspaceId`/`Domain` boundary). Tracked in `docs/FINDINGS.md`.

## 2.3 `fluvio-auth` retirement (plan §8.3)

Two separate things carry the name, and neither is doing work:

- `crates/fluvio-auth` — a **one-line empty stub**. Nothing depends on it. It is
  still a workspace member.
- `services/fluvio-auth` — the Node service. Still built by `docker-compose.yml`.

Plan §8.3 recommends retiring the Node service (self-hosters put their own proxy
in front; `enterprise/` already covers the licence gate). The empty crate should
simply go. Neither was removed here: deleting a compose service changes the
self-host experience, which is the thing the plan says to protect.

## 2.4 The dead root `src/` tree

73 tracked Rust files at the repo root that **nothing compiles** — the root
`Cargo.toml` has no `[package]` section. It appears to be the pre-microservices
monolith. Delete it, or move it to an archive branch. See `docs/FINDINGS.md`.

## 2.5 Licence text does not exist yet (Phase 7)

There is no `LICENSE` file in the repo at all, despite a commit named "added
license" (that commit only vendored `node_modules` licences). Plan §12 wants BSL
plus a generous Additional Use Grant, a Change Date, and MIT for `examples/` and
`sdk/`. `[workspace.package] license` is intentionally left unset until the
owner authors this — licence text is not something to generate on someone's
behalf.
