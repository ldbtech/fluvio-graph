# Findings during library-first restructure

Bugs and oddities found while restructuring. Per the ground rules, fixes land in
their own commits, separate from move/refactor commits.

## Fixed (own commits)

1. **Workspace did not compile at baseline.** `NodeKind::Capability` was added
   to `crates/fluvio-types/src/graph/enums.rs` but
   `services/fluvio-ingestion/src/client/graph_client.rs::kind_to_gql` was never
   updated, so `cargo check --workspace` failed with E0004 (non-exhaustive
   match). Fixed by mapping `Capability => "CAPABILITY"` (matches the
   async-graphql SCREAMING_SNAKE_CASE wire value of `GqlNodeKind::Capability`).

## Noted (not fixed here)

- **No repo-level LICENSE file exists** despite the "added license" commit
  (abed98e) — that commit only brought in `node_modules` licence files. The BSL
  licence text + Additional Use Grant must be added in Phase 7 (§12 of the
  restructure plan). `[workspace.package] license` is intentionally left unset
  until then.
- **`node_modules` directories are committed to git** under `docs/`,
  `enterprise/auth-adapter`, `enterprise/token-service`, and
  `services/fluvio-auth` (hundreds of vendored files). Should be gitignored and
  removed from tracking in a dedicated commit.
- **Local data/log/scratch files live in the repo root** (`fluvio_surreal.db`,
  `fluvio_surreal_data/`, `fluvio_surreal_collab_data/`, `history.txt`,
  `.logs/`, `scratch/`, `.DS_Store` files) — candidates for gitignore cleanup.
- **§11.2 no-transport check nuance:** with `--no-default-features`, none of our
  crates depend on `axum`/`async-graphql` — but `axum` still appears in
  `cargo tree -p fluvio-graph` transitively via `surrealdb → tonic → axum`
  (the SurrealDB client's own gRPC internals). The CI guardrail should assert
  (a) zero `async-graphql` anywhere and (b) no `fluvio-*` crate on any inverse
  path to `axum` (`cargo tree -i axum` shows only the surrealdb/tonic chain),
  rather than a naive "axum absent from the tree".
- **`fluvio-common::config` and `::tracing` are provably dead — and they are the
  only reason the §14 "no `env::var` under `crates/`" box is not literally
  ticked.** Nothing outside the crate calls `load_env`, `require_var`, `var_or`,
  `require_port`, or `init_tracing`; every binary reads `dotenvy` /
  `tracing_subscriber` directly in its own `main.rs`. Both modules now sit
  behind an `env` feature that **no crate in the workspace enables** (the five
  server crates enable only `server`), so they never compile in any build here.
  The rule's intent is met — no library path reads the environment — but the
  literal grep still finds them.

  **Owner's call, deliberately not made here:** delete both modules (they are
  dead, and plan §15 says libraries must not read env or install tracing
  subscribers), or adopt them in the `servers/*` binaries and delete the
  duplicated setup in each `main.rs`. Until then, the CI guardrail in §11.3
  must be written as "no `env::var` in library code that is not behind the
  `env` feature", not a bare `grep -rn "env::var" crates/`.
  The same applies to §11.2 and `fluvio-common`'s optional, `server`-gated
  `axum` dependency.
- **`fluvio-database`'s GraphQL mutation resolver reads `ANTHROPIC_API_KEY` at
  request time** (`src/graphql/mutation.rs:977`). It is inside the server-gated
  `graphql` module so it does not violate the no-env rule, but reading config
  per-request rather than at startup is fragile — worth hoisting into the
  server's config struct later.
- **The five per-service Rust `Dockerfile`s were dead.** `docker-compose.yml`
  builds every Rust service from the shared `Dockerfile.rust` via `target:`, and
  nothing else referenced `services/fluvio-*/Dockerfile`. They are deleted as
  each crate is moved rather than carried along stale.
- **`services/fluvio-ingestion/src/mod.rs` is an orphan** — a `mod.rs` sitting at
  the crate's `src/` root declaring `pub mod graph_client`, which is not
  reachable from `lib.rs` (the real one is `src/client/mod.rs`). Dead file;
  delete when ingestion is moved.
- **The repo root has a dead `src/` tree — 73 tracked Rust files.** The root
  `Cargo.toml` is a pure `[workspace]` manifest with no `[package]` section, so
  nothing under `src/` is ever compiled. It looks like the pre-microservices
  monolith (`app_state.rs`, `authentication/`, `database/`, `graph/`,
  `routes/`, …). **Not deleted here** — it is outside the restructure plan's
  move map, and 73 files is the owner's call, not a packaging refactor's. Decide
  explicitly: delete it, or move it to an `archive/` branch.
- **Two divergent `supergraph.graphql` files** exist: one at the repo root and
  one in `gateway/`. They differ. Only the gateway copy is wired into the Apollo
  Router; the root one appears stale. Confirm and delete the root copy.
- The macOS dev machine's disk hit 100% full during this work; this repo's
  6.6GiB `target/` was cleaned to proceed. Docker Desktop's daemon was also
  non-functional (CLI panic), so the §13 Phase 0 compose smoke test could not
  be run — `cargo check/test --workspace` is the regression oracle instead
  until docker is available.

- `NodeKind::Artifcat` is a preserved typo ("Artifcat" strings exist in
  SurrealDB records) — documented in `fluvio-types`; do not "fix".
- `services/agent-mcp/` is entirely untracked in git at the start of this
  restructure; left untracked (not part of the Rust workspace changes).
- `crates/fluvio-common/src/config.rs` exposes `require_env`/`env_or` helpers
  that read the environment; callers are being migrated to injected config
  structs in Phase 3.
