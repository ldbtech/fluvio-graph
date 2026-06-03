"""Deploy executor — fire-and-forget job queue interface.

POST /deploy                → {job_id} immediately; worker runs in background
GET  /jobs/{job_id}/status  → {status, progress, total, error}
GET  /jobs/{job_id}/stream  → SSE stream of log lines
POST /sandbox/resolve       → resolve sandbox port mapping
POST /sandbox/provision     → create sandbox if missing
GET  /deployments/{workspace_id} → deployment history
"""

from __future__ import annotations

import asyncio
import logging
import uuid
from typing import Any, AsyncIterator

from fastapi import APIRouter, Header, HTTPException
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from app.audit.store import audit_store
from app.auth import verify_workspace_access
from app.config import settings
from app.gateway_client.client import FederationClient
from app.jobs.models import JobRecord, JobStatus
from app.jobs import store as job_store
from app.reliability.circuit_breaker import breaker_status

logger = logging.getLogger("agent-planner")
router = APIRouter()

_SANDBOX_STATUS_QUERY = """
query GetSandboxStatus($sandboxId: String!) {
  getSandboxStatus(sandboxId: $sandboxId) {
    sandboxId
    status
    containers { component ports }
  }
}
"""

_CREATE_SANDBOX_MUTATION = """
mutation CreateSandbox($sandboxId: String!, $components: [String!], $provider: String) {
  createSandbox(sandboxId: $sandboxId, components: $components, provider: $provider) {
    sandboxId
    status
    provider
  }
}
"""


# --------------------------------------------------------------------------- #
# Request / response models
# --------------------------------------------------------------------------- #

class DeployRequest(BaseModel):
    workspace_id: str
    sandbox_id: str | None = None
    steps: list[dict[str, Any]]


class SandboxResolveRequest(BaseModel):
    sandbox_id: str


class SandboxProvisionRequest(BaseModel):
    sandbox_id: str
    components: list[str]
    provider: str = "docker"


# --------------------------------------------------------------------------- #
# Sandbox helpers
# --------------------------------------------------------------------------- #

async def _resolve_sandbox_ports(client: FederationClient, sandbox_id: str) -> dict[str, int]:
    try:
        resp = await client.query(_SANDBOX_STATUS_QUERY, variables={"sandboxId": sandbox_id})
        data = resp.get("data") or resp
        containers = (data.get("getSandboxStatus") or {}).get("containers", [])
        ports_map: dict[str, int] = {}
        for c in containers:
            component = c.get("component")
            for p in (c.get("ports") or []):
                if "->" in p:
                    cport, hport = p.split("->")
                    hport = hport.strip()
                    if hport != "unbound":
                        ports_map[f"{component}:{cport.strip()}"] = int(hport)
        return ports_map
    except Exception as exc:
        logger.error("Failed to resolve sandbox ports for %s: %s", sandbox_id, exc)
        return {}


async def _ensure_sandbox(
    client: FederationClient,
    sandbox_id: str,
    components: list[str],
    provider: str,
) -> None:
    try:
        resp = await client.query(_SANDBOX_STATUS_QUERY, variables={"sandboxId": sandbox_id})
        data = resp.get("data") or resp
        if (data.get("getSandboxStatus") or {}).get("sandboxId"):
            return
    except Exception:
        pass

    logger.info("Sandbox %s not found — provisioning (%s %s)", sandbox_id, provider, components)
    try:
        await client.query(
            _CREATE_SANDBOX_MUTATION,
            variables={"sandboxId": sandbox_id, "components": components, "provider": provider},
        )
    except Exception as exc:
        logger.error("Failed to provision sandbox %s: %s", sandbox_id, exc)


# --------------------------------------------------------------------------- #
# Routes
# --------------------------------------------------------------------------- #

@router.post("/deploy")
async def deploy(
    body: DeployRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await verify_workspace_access(client, body.workspace_id)

    sandbox_id = body.sandbox_id or f"sb-{body.workspace_id}"
    job_id = f"job-{uuid.uuid4().hex[:12]}"

    record = JobRecord(
        job_id=job_id,
        workspace_id=body.workspace_id,
        user_id=x_user_id,
        sandbox_id=sandbox_id,
        steps=body.steps,
    )
    job_store.enqueue(record)
    logger.info("Enqueued job %s for workspace %s (%d steps)", job_id, body.workspace_id, len(body.steps))
    return {"job_id": job_id, "status": "queued", "total": len(body.steps)}


@router.get("/jobs/{job_id}/status")
async def job_status(
    job_id: str,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    record = job_store.get_job(job_id)
    if not record:
        raise HTTPException(status_code=404, detail=f"Job {job_id} not found")
    return {
        "job_id": job_id,
        "status": record.status,
        "progress": record.progress,
        "total": record.total,
        "error": record.error,
    }


@router.get("/jobs/{job_id}/stream")
async def job_stream(
    job_id: str,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> StreamingResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    record = job_store.get_job(job_id)
    if not record:
        raise HTTPException(status_code=404, detail=f"Job {job_id} not found")

    async def _event_stream() -> AsyncIterator[str]:
        # If the job is already done, replay the empty finish signal
        if record.status in (JobStatus.COMPLETED, JobStatus.FAILED):
            yield f"data: [DONE status={record.status}]\n\n"
            return

        q = record.subscribe()
        try:
            while True:
                try:
                    line = await asyncio.wait_for(q.get(), timeout=30.0)
                except asyncio.TimeoutError:
                    yield ": keepalive\n\n"
                    continue
                if line is None:
                    yield f"data: [DONE status={record.status}]\n\n"
                    break
                safe = line.replace("\n", " ")
                yield f"data: {safe}\n\n"
        finally:
            record.unsubscribe(q)

    return StreamingResponse(_event_stream(), media_type="text/event-stream")


@router.get("/deployments/{workspace_id}")
async def deployment_history(
    workspace_id: str,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> list[dict]:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await verify_workspace_access(client, workspace_id)
    return await audit_store.list_runs(workspace_id)


@router.post("/sandbox/resolve")
async def sandbox_resolve(
    body: SandboxResolveRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict[str, int]:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    return await _resolve_sandbox_ports(client, body.sandbox_id)


@router.post("/sandbox/provision")
async def sandbox_provision(
    body: SandboxProvisionRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> dict:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await _ensure_sandbox(client, body.sandbox_id, body.components, body.provider)
    return {"sandbox_id": body.sandbox_id, "status": "provisioned"}


@router.get("/circuit-breakers")
async def circuit_breaker_status() -> dict[str, str]:
    """Expose per-tool circuit breaker states — useful for debugging."""
    return breaker_status()
