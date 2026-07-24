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
- **`fluvio-common::config` and `::tracing` are dead code.** Nothing outside
  the crate calls `load_env`, `require_var`, `var_or`, `require_port`, or
  `init_tracing` — every binary reads `dotenvy`/`tracing_subscriber` directly in
  its own `main.rs`. They are now behind the `env` feature, which no crate
  currently enables. Either adopt them in the `servers/*` binaries or delete
  them; doing so is a behaviour change, so it is deliberately not done here.
- **`fluvio-database`'s GraphQL mutation resolver reads `ANTHROPIC_API_KEY` at
  request time** (`src/graphql/mutation.rs:977`). It is inside the server-gated
  `graphql` module so it does not violate the no-env rule, but reading config
  per-request rather than at startup is fragile — worth hoisting into the
  server's config struct later.
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
