"""Phase 19 — Semantic memory: retrieval side.

Queries SurrealDB (via the graph subgraph's `search` resolver) for past
successful deployments that are semantically similar to the current request.
Returns formatted few-shot examples to inject into the step formulation prompt.

The search uses the same BGE-small embeddings already stored in the graph —
no additional embedding infrastructure is needed.
"""

from __future__ import annotations

import json
import logging
from typing import Any

from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")

_SEARCH_QUERY = """
query SearchDeploymentMemory($query: String!, $workspaceId: String) {
  search(query: $query, workspaceId: $workspaceId, config: { similarityTopK: 3, maxZone: 0 }) {
    score
    node {
      id
      sourceText
      metadata { key value }
    }
  }
}
"""


def _parse_deployment_node(scored: dict[str, Any]) -> dict[str, Any] | None:
    node = scored.get("node") or {}
    meta = {m["key"]: m["value"] for m in (node.get("metadata") or [])}
    if meta.get("type") != "deployment_memory":
        return None
    return {
        "score": scored.get("score", 0.0),
        "summary": node.get("sourceText", ""),
        "run_id": meta.get("run_id", ""),
        "tools_used": meta.get("tools_used", ""),
        "step_count": meta.get("step_count", ""),
        "status": meta.get("status", ""),
    }


async def fetch_similar_deployments(
    client: FederationClient,
    query: str,
    workspace_id: str,
    top_k: int = 3,
) -> list[dict[str, Any]]:
    """Return up to top_k past successful deployments similar to query."""
    try:
        resp = await client.query(
            _SEARCH_QUERY,
            variables={"query": query, "workspaceId": workspace_id},
        )
        data = resp.get("data") or resp
        hits = data.get("search") or []
        examples = [_parse_deployment_node(h) for h in hits]
        return [e for e in examples if e and e["status"] == "completed"][:top_k]
    except Exception as exc:
        logger.warning("RAG fetch failed (non-fatal): %s", exc)
        return []


def format_rag_examples(examples: list[dict[str, Any]]) -> str:
    """Format retrieved deployments as few-shot context for the step formulation prompt."""
    if not examples:
        return ""
    parts = ["## Past Successful Deployments (Few-Shot Examples)\n"]
    for i, ex in enumerate(examples, 1):
        parts.append(f"### Example {i} (similarity: {ex['score']:.2f})")
        parts.append(ex["summary"])
        parts.append("")
    return "\n".join(parts)
