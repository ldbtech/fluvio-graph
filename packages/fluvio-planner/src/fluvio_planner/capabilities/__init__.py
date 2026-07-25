"""CSP capability layer for agent-planner.

Backs the Capability Synthesis Protocol (CSP) with the knowledge graph:
synthesized capabilities are mirrored into fluvio-graph as `Capability` nodes
(embedded for semantic reuse-first), and the compile-time precedence resolver
queries the graph before asking the LLM to synthesize anything new.

See docs/CSP_KG_INTEGRATION_PLAN.md.
"""

from fluvio_planner.capabilities.graph_store import GraphPlannerStore, mirror_capability_to_graph
from fluvio_planner.capabilities.resolver import find_reusable_capability

__all__ = [
    "GraphPlannerStore",
    "mirror_capability_to_graph",
    "find_reusable_capability",
]
