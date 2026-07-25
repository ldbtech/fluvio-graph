"""Workspace-driven configuration resolved per request.

Replaces every hardcoded tenant reference with values derived from the
workspace's active connectors and schema hints.
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass, field
from typing import Literal

from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


@dataclass
class WorkspaceConfig:
    workspace_id: str
    bi_platform: Literal["tableau", "powerbi", "pdf"] | None
    table_hints: list[str]
    sandbox_components: list[str]
    connector_configs: dict[str, dict]
    default_output_table_suffix: str = "_analytics"

    # Resolved DB connection details (used by executor, not exposed to Claude)
    db_host: str | None = None
    db_port: int = 5432
    db_user: str | None = None
    db_pass: str | None = None
    db_name: str | None = None


_CONNECTORS_QUERY = """
query GetUserConnectors {
  getUserConnectors {
    id
    kind
    status
  }
}
"""

_CONNECTOR_DETAIL_QUERY = """
query GetConnector($connectorId: String!) {
  getConnector(connectorId: $connectorId) {
    id
    kind
    accessToken
    status
  }
}
"""


async def resolve_workspace_config(
    client: FederationClient,
    workspace_id: str,
    message: str | None = None,
    sandbox_id: str | None = None,
    sandbox_ports: dict[str, int] | None = None,
) -> WorkspaceConfig:
    """Build a WorkspaceConfig from live connector data for workspace_id."""
    connector_configs: dict[str, dict] = {}
    db_host = db_user = db_pass = db_name = None
    db_port = 5432

    try:
        resp = await client.query(_CONNECTORS_QUERY)
        data = resp.get("data") or resp
        connectors = data.get("getUserConnectors") or []

        for conn in connectors:
            kind_lower = conn.get("kind", "").lower()
            try:
                detail_resp = await client.query(
                    _CONNECTOR_DETAIL_QUERY,
                    variables={"connectorId": conn["id"]},
                )
                detail = (detail_resp.get("data") or {}).get("getConnector") or detail_resp.get("getConnector") or {}
                token = detail.get("accessToken")
                if token:
                    try:
                        connector_configs[kind_lower] = json.loads(token)
                    except Exception:
                        connector_configs[kind_lower] = {"token": token}
            except Exception as exc:
                logger.warning("Could not fetch connector %s detail: %s", conn["id"], exc)

        # Resolve DB credentials from the first database-type connector
        for kind in ("postgres", "postgresql", "database", "mysql"):
            cfg = connector_configs.get(kind)
            if cfg:
                db_host = cfg.get("host")
                db_name = cfg.get("database") or cfg.get("databaseName")
                db_user = cfg.get("username")
                db_pass = cfg.get("password", "")
                break

    except Exception as exc:
        logger.error("Failed to resolve workspace connectors: %s", exc)

    # Remap DB port if running inside a sandbox
    if sandbox_id and sandbox_ports and "postgres:5432/tcp" in sandbox_ports:
        db_port = sandbox_ports["postgres:5432/tcp"]

    db_url_host = "localhost" if sandbox_id else (db_host or "localhost")

    # Determine BI platform from message intent → active connectors (no hardcoded default)
    bi_platform: Literal["tableau", "powerbi", "pdf"] | None = None
    msg_lower = (message or "").lower()

    if "pdf" in msg_lower or "latex" in msg_lower:
        bi_platform = "pdf"
    elif "powerbi" in msg_lower or "power bi" in msg_lower:
        bi_platform = "powerbi"
    elif "tableau" in msg_lower:
        bi_platform = "tableau"
    elif "powerbi" in connector_configs or "power_bi" in connector_configs:
        bi_platform = "powerbi"
    elif "tableau" in connector_configs:
        bi_platform = "tableau"

    # Derive sandbox components from connector presence (not keyword matching on plan text)
    sandbox_components = ["postgres"] if (db_host or connector_configs.get("postgres")) else []
    for extra in ("kafka", "spark", "airflow", "dbt"):
        if extra in connector_configs:
            sandbox_components.append(extra)
    if not sandbox_components:
        sandbox_components = ["postgres"]

    # Table hints from schema knowledge — empty list is fine; executor will surface errors rather than guessing
    table_hints: list[str] = []

    return WorkspaceConfig(
        workspace_id=workspace_id,
        bi_platform=bi_platform,
        table_hints=table_hints,
        sandbox_components=sandbox_components,
        connector_configs=connector_configs,
        db_host=db_url_host,
        db_port=db_port,
        db_user=db_user,
        db_pass=db_pass,
        db_name=db_name,
    )


def build_environment_context(cfg: WorkspaceConfig, sandbox_id: str | None) -> dict:
    """Return the full environment context dict (contains raw credentials — executor only)."""
    db_url = (
        f"postgres://{cfg.db_user}:{cfg.db_pass}@{cfg.db_host}:{cfg.db_port}/{cfg.db_name}"
        if all([cfg.db_user, cfg.db_host, cfg.db_name])
        else None
    )
    return {
        "sandbox_id": sandbox_id,
        "environment": "sandbox" if sandbox_id else "local",
        "ports_map": {},
        "database": {
            "host": cfg.db_host,
            "port": cfg.db_port,
            "username": cfg.db_user,
            "password": cfg.db_pass,
            "database": cfg.db_name,
            "url": db_url,
        },
        "connectors": cfg.connector_configs,
    }
