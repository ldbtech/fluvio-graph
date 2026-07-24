# 2. Open decisions the restructure deliberately did not make

Date: 2026-07-24
Status: **Proposed — these need the owner's call**

The library-first restructure (ADR 0001) stopped after Phase 4 / `v0.1.0`. Each
item below is a genuine fork where the cheap option conflicts with the plan, or
where the change is behavioural rather than packaging. They are written down
rather than guessed at.

## 2.1 Python config injection (blocks Phase 5)

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

## 2.2 `WorkspaceId` is currently optional (Phase 6)

`QueryContext::from_text` and `from_embedding` already take
`workspace_id: Option<&str>`. Plan §9 requires it to be **required** —
"No public API may access data without a `WorkspaceId`" — and mapped to a
SurrealDB namespace/database per workspace rather than a filtered column.

Making it required is a breaking change to the facade, so it wants a minor bump
and a `CHANGELOG` entry. Doing it *before* external consumers pin `v0.1.0` is
much cheaper than after.

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
