# fluvio-planner

The reusable, transport-free **domain layer** of the fluvioMe agent-planner —
the "brain" that turns a natural-language ask plus knowledge-graph context into a
reviewable pipeline plan, compiles it to steps, and runs them with reliability
(retry, circuit breaker, idempotency, rollback, audit).

It is split out of `services/agent-planner` the same way the Rust engine was
split into `crates/` + `servers/`: the FastAPI service is now a thin shell that
imports this package.

## Why it's a package

- **Import-time environment-free.** Importing `fluvio_planner` never reads env
  vars or constructs the service's `Settings`. All runtime configuration is
  injected via `PlannerConfig` (`fluvio_planner.planner_config`).
- **Embeddable.** Another process (e.g. FounderTwin) can depend on it directly —
  including two differently-configured planners in one process — without
  standing up the HTTP service.

## Install (editable, from the monorepo)

```bash
pip install -e packages/fluvio-planner            # core
pip install -e "packages/fluvio-planner[synthesis,mcp]"   # + CSP + MCP client
```

The FastAPI service pulls it in automatically via its `requirements.txt`
(`-e ../../packages/fluvio-planner`).

## Layout

`agent/` designation + plan authoring · `plan/` context assembly + markdown ·
`capabilities/` CSP orchestration + graph-backed store · `jobs/` deploy worker ·
`reliability/` retry/breaker/errors · `memory/` deployment RAG · `audit/` run
trail · `fetch/` + `gateway_client/` federation reads · `evals/` prompt harness.

Config is injected, never read here — see
[`docs/adr/0002-open-decisions.md`](../../docs/adr/0002-open-decisions.md) §2.1.
