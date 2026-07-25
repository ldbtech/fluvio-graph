"""Capabilities router — CSP synthesis + knowledge-graph reuse.

POST /capabilities/search      → reuse-first lookup (graph, no LLM)
POST /capabilities/synthesize  → CSP plans the goal; synthesizes + sandboxes a
                                 new capability if none exists, persists it
                                 locally and mirrors it into fluvio-graph.

The deploy worker / compile flow are untouched — this exposes the capability
layer directly so it can be exercised and tested.
"""

from __future__ import annotations

import logging

from fastapi import APIRouter, Header, HTTPException
from pydantic import BaseModel

from app.auth import verify_workspace_access
from app.config import settings
from fluvio_planner.gateway_client.client import FederationClient
from fluvio_planner.capabilities.resolver import find_reusable_capability, DEFAULT_REUSE_THRESHOLD
from fluvio_planner.capabilities.orchestrator import build_capability_orchestrator

logger = logging.getLogger("agent-planner")
router = APIRouter()


class SearchRequest(BaseModel):
    workspace_id: str
    goal: str
    threshold: float | None = None


class SynthesizeRequest(BaseModel):
    workspace_id: str
    goal: str
    ambient: dict | None = None


def _client(x_user_id: str) -> FederationClient:
    return FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})


@router.post("/capabilities/search")
async def search_capabilities(
    body: SearchRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = _client(x_user_id)
    await verify_workspace_access(client, body.workspace_id)

    hit = await find_reusable_capability(
        client, body.goal,
        threshold=body.threshold if body.threshold is not None else DEFAULT_REUSE_THRESHOLD,
    )
    return {"reusable": hit is not None, "capability": hit}


@router.post("/capabilities/synthesize")
async def synthesize_capability(
    body: SynthesizeRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = _client(x_user_id)
    await verify_workspace_access(client, body.workspace_id)

    # 1. Reuse-first: if the graph already has a capability for this goal, return it.
    existing = await find_reusable_capability(client, body.goal)
    if existing:
        return {"synthesized": False, "reused": True, "capability": existing}

    # 2. No match → run CSP. It synthesizes the verb, sandboxes it, persists it
    #    locally and (via GraphPlannerStore) mirrors it into the knowledge graph.
    app = build_capability_orchestrator(client, settings.as_planner_config())
    if app is None:
        raise HTTPException(
            status_code=503,
            detail="Capability synthesis unavailable (csp not installed or ANTHROPIC_API_KEY unset)",
        )

    try:
        result = await app.run_goal(body.goal, ambient=body.ambient or {})
    except Exception as exc:
        logger.error("Capability synthesis failed: %s", exc)
        raise HTTPException(status_code=500, detail=f"Synthesis failed: {exc}")

    return {"synthesized": True, "reused": False, "result": result}
