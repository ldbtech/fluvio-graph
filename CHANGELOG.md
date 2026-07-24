# Changelog

All notable changes to this project are documented here.

The version that matters to consumers is the one on **`fluvio-graph`**, the
facade crate. Internal churn behind the facade is free and unversioned; changes
to the facade's public surface are what get a version bump and an entry below.

Pre-1.0, a breaking change to the facade takes a **minor** bump.

## [Unreleased]

## [0.1.0] — 2026-07-24

First tagged release. This is the library-first restructure: the same engine,
repackaged so it can be linked in-process instead of only run as a server.

### Added
- **`fluvio-graph`** — the facade crate and the only thing external consumers
  should depend on. Curated modules (`storage`, `query`, `embeddings`, `graph`,
  `types`, `ingestion`) plus a `prelude`. The underlying crates are reachable
  only through `#[doc(hidden)] internal`.
- `SurrealConfig`, so storage settings are injected by the caller rather than
  read from the environment inside the library.
- `examples/embedded-consumer` — a downstream-shaped consumer that runs a
  grounded query in-process with no server running.
- `docs/FINDINGS.md` — bugs and dead code found during the restructure, kept
  separate from the refactor itself.

### Changed
- **Libraries moved to `crates/`, transport shells to `servers/`.** The five
  services that were one lib+bin crate each are now a pure library plus a thin
  axum + async-graphql shell:
  `fluvio-graph-core`, `fluvio-ingestion-core`, `fluvio-twin-core`,
  `fluvio-collab-core`, `fluvio-database` in `crates/`; `graph-server`,
  `ingestion-server`, `twin-server`, `collab-server`, `database-server` in
  `servers/`.
- No crate under `crates/` depends on `axum` or `async-graphql` any more
  (`fluvio-common`'s is optional and off by default), so linking the engine no
  longer drags in a web framework or a second GraphQL schema.
- The Apollo Router config moved from `services/fluvio-gateway/` to `gateway/`.
- Binary names are unchanged (`fluvio-graph`, `fluvio-database`, …), so
  `docker compose up` behaves exactly as before.

### Fixed
- The workspace did not compile at baseline: `NodeKind::Capability` was missing
  from ingestion's `kind_to_gql` match.
- `services/fluvio-collab` was listed twice as a workspace member.

### Removed
- `services/fluvio-tool-builder/Cargo.toml`, a stray Rust manifest in a Python
  service that was never a workspace member.
- The five per-service Rust `Dockerfile`s, superseded by the shared
  `Dockerfile.rust` and unreferenced by compose.
- `services/fluvio-ingestion/src/mod.rs`, an orphan module root unreachable
  from `lib.rs`.

[Unreleased]: https://github.com/ldbtech/fluvio-graph/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ldbtech/fluvio-graph/releases/tag/v0.1.0
