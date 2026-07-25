"""GraphPlannerStore — backs CSP's capability persistence with fluvio-graph.

CSP's `Orchestrator` persists synthesized capabilities through a `PlannerStore`
(`save_capability` / `load_capabilities`). We subclass it so every synthesized
capability is ALSO mirrored into the knowledge graph as a `Capability` node —
embedded server-side (BGE-small, 384-dim) so it becomes semantically searchable
for reuse-first across the whole company brain.

Local disk persistence (CSP's default) is kept intact via `super()`, so the
running process still reuses capabilities with zero LLM calls; the graph adds
durability + the semantic index the compile-time resolver queries.

The mirror is best-effort: a graph hiccup never breaks synthesis.
"""

from __future__ import annotations

import asyncio
import json
import logging
import uuid
from typing import Any

logger = logging.getLogger("agent-planner")

# CSP is an optional dependency — import lazily so the planner still boots
# (without the capability layer) if csp isn't installed yet.
try:
    from csp.orchestrator.planner_store import PlannerStore
    from csp.orchestrator.capability import SynthesizedCapability
    _CSP_AVAILABLE = True
except Exception:  # pragma: no cover - csp not installed
    PlannerStore = object  # type: ignore
    SynthesizedCapability = Any  # type: ignore
    _CSP_AVAILABLE = False


_UPSERT_CAPABILITY = """
mutation UpsertCapability($input: GqlCapabilityInput!) {
  upsertCapability(input: $input) { id }
}
"""

# A lightweight node representing an MCP tool, so capabilities can edge to it.
_UPSERT_NODE = """
mutation UpsertNode($input: GqlNodeInput!) {
  upsertNode(input: $input) { id }
}
"""

# Capability —USES_TOOL→ tool, Capability —READS→ data node.
_UPSERT_EDGE = """
mutation UpsertEdge($input: GqlEdgeInput!) {
  upsertEdge(input: $input) { from to label }
}
"""


def _tool_node_id(tool_name: str) -> str:
    """Stable UUID for an MCP tool node — idempotent across capabilities."""
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"mcp://tool/{tool_name}"))


def detect_mcp_tools(code: str, catalog: list[str], declared: str = "") -> list[str]:
    """Which MCP tools a capability uses = declared ∪ catalog names found in code."""
    tools: set[str] = set()
    for t in (declared or "").split(","):
        t = t.strip()
        if t:
            tools.add(t)
    if code:
        for name in catalog:
            if name and name in code:
                tools.add(name)
    return sorted(tools)


async def _wire_tool_edges(client, cap_node_id: str, tools: list[str]) -> None:
    """Create a Tool node + Capability—USES_TOOL→Tool edge for each tool."""
    for tool in tools:
        tool_id = _tool_node_id(tool)
        try:
            await client.query(_UPSERT_NODE, variables={"input": {
                "id":         tool_id,
                "domain":     "CUSTOM",
                "sourceUri":  f"mcp://tool/{tool}",
                "sourceText": tool,
                "kind":       "ENTITY",
                "metadata":   [{"key": "mcp_tool", "value": tool}],
            }})
            await client.query(_UPSERT_EDGE, variables={"input": {
                "from":  cap_node_id,
                "to":    tool_id,
                "label": "USES_TOOL",
            }})
        except Exception as exc:
            logger.warning("Failed to wire USES_TOOL edge to %r: %s", tool, exc)


def _capability_spec_text(cap: "SynthesizedCapability") -> str:
    """The text we embed for semantic reuse — name + description + tags.

    Kept rich so a natural-language goal ("rank customers by churn risk")
    matches the capability that serves it ("score_churn_risk").
    """
    parts = [cap.name]
    if getattr(cap, "description", ""):
        parts.append(cap.description)
    tags = getattr(cap, "tags", []) or []
    if tags:
        parts.append("tags: " + ", ".join(tags))
    return "\n".join(parts)


def _build_capability_input(cap: "SynthesizedCapability", mcp_tools: list[str]) -> dict[str, Any]:
    tags = getattr(cap, "tags", []) or []
    params_schema = getattr(cap, "params_schema", {}) or {}
    signature = ", ".join(params_schema.keys()) if params_schema else ""
    return {
        "name":      cap.name,
        "spec":      _capability_spec_text(cap),
        "code":      getattr(cap, "code", "") or "",
        "signature": f"run({signature})" if signature else "run(args)",
        "mcpTools":  ",".join(mcp_tools),
        "tags":      ",".join(tags),
        "status":    "synthesized",
        "specJson":  json.dumps(getattr(cap, "spec", {}) or {}),
    }


async def mirror_capability_to_graph(client, cap: "SynthesizedCapability", mcp_server_url: str) -> None:
    """Upsert a synthesized capability into the graph, and wire USES_TOOL edges
    to every MCP tool it calls (Phase C5 — the capability becomes self-describing).

    `mcp_server_url` is injected by the caller (via GraphPlannerStore) so this
    module never reads config from the environment."""
    from app.capabilities.mcp_client import list_tool_names

    code = getattr(cap, "code", "") or ""
    catalog = await list_tool_names(mcp_server_url)  # best-effort; [] if MCP server is down
    tools = detect_mcp_tools(code, catalog)

    try:
        resp = await client.query(
            _UPSERT_CAPABILITY,
            variables={"input": _build_capability_input(cap, tools)},
        )
        cap_id = (resp.get("data") or resp).get("upsertCapability", {}).get("id")
        logger.info("Mirrored capability %r to knowledge graph (id=%s)", cap.name, cap_id)
    except Exception as exc:
        logger.warning("Capability graph mirror failed for %r: %s", getattr(cap, "name", "?"), exc)
        return

    if cap_id and tools:
        await _wire_tool_edges(client, cap_id, tools)
        logger.info("Wired %d USES_TOOL edge(s) for %r: %s", len(tools), cap.name, ", ".join(tools))


class GraphPlannerStore(PlannerStore):  # type: ignore[misc]
    """A PlannerStore that also mirrors capabilities into the knowledge graph.

    Parameters
    ----------
    root:            local planner_dir (kept for full-spec reload + readable .py)
    client:          a FederationClient already carrying the owner's x-user-id header
    mcp_server_url:  injected MCP endpoint, threaded to the graph-mirror step so
                     this module reads no config singleton
    """

    def __init__(self, root: str, *, client, mcp_server_url: str) -> None:
        if not _CSP_AVAILABLE:
            raise RuntimeError("csp is not installed — cannot use GraphPlannerStore")
        super().__init__(root)
        self._client = client
        self._mcp_server_url = mcp_server_url

    def save_capability(self, cap: "SynthesizedCapability") -> None:  # type: ignore[override]
        # 1. Keep CSP's local persistence (full spec + runnable .py).
        super().save_capability(cap)
        # 2. Best-effort mirror to the graph. save_capability is sync but is
        #    invoked from CSP's async executor, so schedule on the running loop;
        #    fall back to a fresh loop if somehow called synchronously.
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(mirror_capability_to_graph(self._client, cap, self._mcp_server_url))
        except RuntimeError:
            try:
                asyncio.run(mirror_capability_to_graph(self._client, cap, self._mcp_server_url))
            except Exception as exc:  # pragma: no cover
                logger.warning("capability mirror (sync) failed: %s", exc)
