"""CSP Orchestrator factory for agent-planner.

Builds a Capability Synthesis Protocol orchestrator whose persistence is backed
by the knowledge graph (`GraphPlannerStore`). When the planner hits a goal no
existing MCP tool or graph capability covers, this orchestrator synthesizes a
new general verb, sandbox-tests it, and persists it — locally for instant reuse
and into fluvio-graph for company-wide, semantically-searchable reuse.

Kept separate from the deploy worker: CSP adds a *synthesis* option to compile;
it does not replace the plan→compile→deploy orchestration or its reliability.
"""

from __future__ import annotations

import logging

from app.capabilities.graph_store import GraphPlannerStore
from app.planner_config import PlannerConfig

logger = logging.getLogger("agent-planner")

# How fluvioMe teaches CSP its conventions for generated capability code.
_SYNTHESIS_GUIDANCE = """
You are synthesizing a capability for the fluvioMe data engine.

Conventions for the Python you generate:
- A capability is a GENERAL, reusable verb (e.g. `aggregate_table`, `score_metric`),
  not a one-off task. The specifics (columns, table, metric) come from args.
- Operate over data passed in `args` (e.g. args["rows"], args["columns"]). Do not
  hard-code a single dataset.
- For database / BI / report side effects, prefer calling an existing fluvioMe MCP
  tool (e.g. `database__execute_query`, `dashboard_syncer__generate_pdf_report`)
  rather than re-implementing I/O. List the MCP tools you rely on.
- Plots → return a base64-encoded PNG string. Tables → return list[dict]. Always
  return a JSON-serializable dict.
- Keep it deterministic and side-effect-free unless the goal explicitly asks for a
  write; the deploy worker owns retries, idempotency and rollback.
""".strip()


def build_capability_orchestrator(client, cfg: PlannerConfig, *, planner_dir: str = "planner_caps"):
    """Construct a CSP Orchestrator backed by the knowledge graph.

    `client` is a FederationClient already carrying the owner's x-user-id, used
    to mirror/search capabilities in fluvio-graph. `cfg` is injected by the
    caller (the composition root) so this module reads no config singleton.

    Returns the Orchestrator, or None if CSP / an API key isn't available — the
    planner then simply runs without runtime synthesis.
    """
    if not cfg.anthropic_api_key:
        logger.info("CSP disabled — no ANTHROPIC_API_KEY")
        return None

    try:
        from csp import Orchestrator, AnthropicLLM
    except Exception as exc:
        logger.info("CSP not installed (%s) — capability synthesis disabled", exc)
        return None

    llm = AnthropicLLM(api_key=cfg.anthropic_api_key)

    # planner_dir=None so the Orchestrator skips its own local-only store and
    # seeding; we attach the graph-backed store and replicate the seed/hook.
    app = Orchestrator(
        "fluviome-capabilities",
        llm=llm,
        planner_dir=None,
        synthesis_guidance=_SYNTHESIS_GUIDANCE,
    )

    store = GraphPlannerStore(planner_dir, client=client, mcp_server_url=cfg.mcp_server_url)
    app._store = store
    try:
        for cap in store.load_capabilities():
            app._registry._synthesized[cap.name] = cap
        app._registry.persist_hook = store.save_capability
        logger.info("CSP orchestrator ready (graph-backed capability store)")
    except Exception as exc:
        logger.warning("CSP store seeding failed: %s", exc)

    return app
