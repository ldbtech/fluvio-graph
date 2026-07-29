"""Typed, env-free runtime config for the planner's domain layer.

This module deliberately reads *nothing* from the environment and imports
`app.config` *not at all*. Importing it never triggers the pydantic-settings
singleton, so the domain modules that accept a `PlannerConfig` (plan_writer,
orchestrator, harness, mcp_client, worker) stay import-time pure.

The composition root — `app.config.Settings`, the routers, `main.py`, and the
eval CLI — is the only place that turns environment variables into a
`PlannerConfig` (via `Settings.as_planner_config()`) and injects it inward.
That is what lets a single process host two differently-configured planners.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PlannerConfig:
    """The subset of settings the domain layer actually consumes.

    Frozen so an injected config can't be mutated mid-request, and so the same
    instance is safe to share across concurrent tasks.
    """

    graphql_gateway_url: str
    mcp_server_url: str
    # fluvio-database's bare base URL (not /graphql-suffixed) for resolving a
    # per-user BYOK LLM connection via its internal, non-GraphQL route.
    database_service_url: str
    internal_secret: str | None
