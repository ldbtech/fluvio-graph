# 1. Library-first layout: `crates/` + `servers/` + `gateway/`

Date: 2026-07-24
Status: Accepted (implemented, tagged `v0.1.0`)

## Context

Logic and transport were welded together: each Rust service was a single crate
holding both its domain modules and its axum + async-graphql subgraph. A
consumer that wanted to link the engine in-process had to compile a web
framework and a second GraphQL schema it would never serve.

## Decision

Split every service into a pure library under `crates/` and a thin transport
shell under `servers/`.

- `crates/` — libraries only. No transport dependencies, no environment reads,
  no global state, no tracing subscriber installation.
- `servers/` — axum + async-graphql binaries. They own config, env reading, and
  logging setup.
- `gateway/` — Apollo Router config, moved out of `services/`.
- `services/` — now only the Python and Node services.
- `crates/fluvio-graph` — a **facade** re-exporting a curated surface. External
  consumers depend on this one crate; everything behind it is free to change.

Dependencies point one way: `crates/` never references `servers/`, `services/`,
or `gateway/`.

Binary names are unchanged (`fluvio-graph`, `fluvio-database`, …), so
`docker compose up` gives self-hosters exactly the experience it did before.

## Consequences

- An external backend links `fluvio-graph` and gets no axum and no
  async-graphql. (The one `axum` left in a lean `cargo tree` arrives through
  `surrealdb → tonic`, which is SurrealDB's own gRPC internals, not ours.)
- Transport can be replaced or added — gRPC, a CLI — without touching logic.
- The facade is the compatibility boundary. `CHANGELOG.md` tracks it; internal
  `*-core` churn is unversioned.
- Crate names are now load-bearing: a downstream `[patch]` override resolves by
  name, so renaming after `v0.1.0` breaks consumers.
