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

- `NodeKind::Artifcat` is a preserved typo ("Artifcat" strings exist in
  SurrealDB records) — documented in `fluvio-types`; do not "fix".
- `services/agent-mcp/` is entirely untracked in git at the start of this
  restructure; left untracked (not part of the Rust workspace changes).
- `crates/fluvio-common/src/config.rs` exposes `require_env`/`env_or` helpers
  that read the environment; callers are being migrated to injected config
  structs in Phase 3.
