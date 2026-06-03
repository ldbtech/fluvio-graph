"""Phase 19 — Semantic memory: storage side.

After a deployment completes successfully, ingests a structured summary into
the knowledge graph via the ingestion subgraph's `ingestRaw` resolver.

The node is stored with:
  - domain: Custom("deployment_memory")
  - workspace_id: the workspace it ran in
  - metadata: type=deployment_memory, run_id, tools_used, step_count, status
  - source_text: human-readable summary Claude can read as a few-shot example

BGE-small embedding happens in the ingestion pipeline automatically.
"""

from __future__ import annotations

import json
import logging

from app.audit.models import DeploymentRun
from app.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")

_INGEST_RAW_MUTATION = """
mutation StoreDeploymentMemory(
  $text: String!
  $sourceUri: String!
  $domain: String
  $zone: Int
  $workspaceId: String
) {
  ingestRaw(
    text: $text
    sourceUri: $sourceUri
    domain: $domain
    zone: $zone
    workspaceId: $workspaceId
  ) {
    jobId
    status
    message
  }
}
"""


def _build_summary(run: DeploymentRun, steps: list[dict]) -> str:
    """Build a human-readable deployment summary suitable for RAG retrieval."""
    tool_actions = ", ".join(
        f"{s.get('tool_id')}/{s.get('action')}"
        for s in steps
        if s.get("tool_id")
    )
    step_summaries = "\n".join(
        f"  - Step {s.get('step_index', i)+1}: {s.get('tool_id')}/{s.get('action')} "
        f"[{s.get('status')}] {s.get('duration_ms', 0)}ms"
        for i, s in enumerate(steps)
    )
    return (
        f"Deployment run {run.run_id} for workspace {run.workspace_id}.\n"
        f"Status: {run.status}. Steps: {run.step_count}.\n"
        f"Tools used: {tool_actions}.\n"
        f"Step details:\n{step_summaries}"
    )


async def store_deployment_summary(
    client: FederationClient,
    run: DeploymentRun,
) -> None:
    """Ingest a deployment run summary into the knowledge graph for future RAG retrieval."""
    if run.status != "completed":
        return  # only store successful runs

    steps = [s.__dict__ if hasattr(s, "__dict__") else s for s in run.steps]
    tools_used = list({s.get("tool_id", "") for s in steps if s.get("tool_id")})
    summary = _build_summary(run, steps)

    source_uri = f"deployment://{run.workspace_id}/{run.run_id}"

    try:
        await client.query(
            _INGEST_RAW_MUTATION,
            variables={
                "text": summary,
                "sourceUri": source_uri,
                "domain": "deployment_memory",
                "zone": 0,
                "workspaceId": run.workspace_id,
            },
        )
        logger.info("Stored deployment memory for run %s", run.run_id)
    except Exception as exc:
        logger.warning("Failed to store deployment memory (non-fatal): %s", exc)
