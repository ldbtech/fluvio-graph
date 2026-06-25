"""Reuse-first capability resolver (Phase C4, compile-time).

Before the planner asks the LLM to synthesize anything, it checks the knowledge
graph for an existing capability that already covers the goal. This is CSP's
reuse-first principle, grounded company-wide in the KG instead of a local folder.

Precedence (highest first), enforced by the compile router:
  1. an existing MCP tool covers the goal   → call it (no synthesis)
  2. an existing graph capability covers it  → reuse it (this module)
  3. neither                                 → CSP synthesizes a new verb
"""

from __future__ import annotations

import logging
from typing import Any

logger = logging.getLogger("agent-planner")

_SEARCH_CAPABILITIES = """
query SearchCapabilities($goal: String!, $topK: Int) {
  searchCapabilities(goal: $goal, topK: $topK) {
    score
    node {
      id
      sourceText
      metadata { key value }
    }
  }
}
"""

# Default cosine threshold above which a capability hit is reused vs re-synthesized.
# Calibrated live against BGE-small (384-dim) on real capability nodes:
#   ~0.96  near-exact goal      ·  ~0.91  same-intent rewording
#   ~0.67  loose paraphrase     ·  ~0.41  unrelated goal
# 0.70 reuses strong matches while rejecting unrelated goals. Tune per registry.
DEFAULT_REUSE_THRESHOLD = 0.70


async def find_reusable_capability(
    client,
    goal: str,
    *,
    threshold: float = DEFAULT_REUSE_THRESHOLD,
    top_k: int = 8,
) -> dict[str, Any] | None:
    """Return the best reusable capability for `goal`, or None.

    On a hit, returns a dict: name, score, code, signature, tags, spec_json,
    spec_text — everything needed to reuse without an LLM call.
    """
    try:
        resp = await client.query(_SEARCH_CAPABILITIES, variables={"goal": goal, "topK": top_k})
    except Exception as exc:
        logger.warning("capability search failed: %s", exc)
        return None

    data = resp.get("data") or resp
    hits = data.get("searchCapabilities") or []
    if not hits:
        return None

    top = hits[0]
    score = float(top.get("score") or 0.0)
    if score < threshold:
        logger.info("No capability above reuse threshold (best score %.3f < %.2f)", score, threshold)
        return None

    node = top.get("node") or {}
    meta = {m["key"]: m["value"] for m in (node.get("metadata") or [])}
    logger.info("Reusing capability %r (score %.3f)", meta.get("name"), score)
    return {
        "name":       meta.get("name"),
        "score":      score,
        "code":       meta.get("generated_code", ""),
        "signature":  meta.get("signature", ""),
        "tags":       meta.get("tags", ""),
        "mcp_tools":  meta.get("mcp_tools", ""),
        "spec_json":  meta.get("spec_json"),
        "spec_text":  node.get("sourceText", ""),
    }
