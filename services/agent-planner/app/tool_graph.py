"""Tool capability graph — produces/consumes data-flow relationships.

Action names come from the live toolbox (manifest-driven), not hardcoded here.
The produces/consumes edges and dependency ordering are configuration, not code —
they describe the *data flow* between tools, which the manifests don't encode.

To add a new data-flow relationship:
    Add an entry to _PRODUCES, _CONSUMES, or _DEPENDENCY_CHAIN.
    Action names must match what's in the manifest (toolbox.valid_actions(tool_id)).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from app.toolbox import toolbox, ToolManifest

# ── Data-flow configuration ────────────────────────────────────────────────────
# These express *what data* each tool action produces and consumes.
# Action names are validated against the live toolbox at runtime.

# tool_id/action → list of output patterns
_PRODUCES: dict[str, list[str]] = {
    "data-cleaning/clean_table":              ["clean_{table}"],
    "spark/execute_sql":                      ["{output_table}_analytics"],
    "spark/submit_job":                       ["{output_table}_analytics"],
    "dbt/run_models":                         ["{model}_analytics"],
    "dashboard-syncer/publish_report":        ["dashboard_url"],
    "dashboard-syncer/generate_pdf_report":   ["pdf_url"],
    "kafka/create_topic":                     ["kafka_topic"],
    "terraform/apply_infrastructure":         ["infrastructure"],
}

# tool_id/action → list of input patterns consumed
_CONSUMES: dict[str, list[str]] = {
    "data-cleaning/clean_table":              ["{table}"],
    "spark/execute_sql":                      ["clean_{table}"],
    "spark/submit_job":                       ["clean_{table}"],
    "dbt/run_models":                         ["clean_{table}"],
    "dashboard-syncer/publish_report":        ["*_analytics"],
    "dashboard-syncer/generate_pdf_report":   ["*_analytics"],
    "dashboard-syncer/trigger_refresh":       ["dashboard_url"],
    "airflow/trigger_dag":                    [],
}

# (prerequisite_key, dependent_key) — enforces execution ordering
_DEPENDENCY_CHAIN: list[tuple[str, str]] = [
    ("data-cleaning/clean_table",   "spark/execute_sql"),
    ("data-cleaning/clean_table",   "spark/submit_job"),
    ("data-cleaning/clean_table",   "dbt/run_models"),
    ("spark/execute_sql",           "dashboard-syncer/publish_report"),
    ("spark/execute_sql",           "dashboard-syncer/generate_pdf_report"),
    ("spark/submit_job",            "dashboard-syncer/publish_report"),
    ("dbt/run_models",              "dashboard-syncer/publish_report"),
    ("dbt/run_models",              "dashboard-syncer/generate_pdf_report"),
    ("dashboard-syncer/publish_report", "dashboard-syncer/trigger_refresh"),
]


@dataclass
class ToolCapability:
    tool_id: str
    action: str
    produces: list[str]
    consumes: list[str]
    description: str = ""


@dataclass
class ToolCapabilityGraph:
    capabilities: list[ToolCapability] = field(default_factory=list)

    @classmethod
    def from_active_tools(cls, active_tools: list[dict[str, Any]]) -> "ToolCapabilityGraph":
        """Build a graph from active workspace tools, using manifest-verified action names."""
        active_ids = {t.get("id") for t in active_tools if t.get("id")}
        registry = toolbox.registry_for(active_ids or None)

        caps: list[ToolCapability] = []
        for key, produces in _PRODUCES.items():
            tid, action = key.split("/", 1)
            if tid not in registry:
                continue
            if action not in registry.valid_actions(tid):
                continue  # action doesn't exist in actual manifest
            manifest: ToolManifest | None = toolbox.get(tid)
            caps.append(ToolCapability(
                tool_id=tid,
                action=action,
                produces=produces,
                consumes=_CONSUMES.get(key, []),
                description=manifest.description if manifest else "",
            ))
        return cls(capabilities=caps)

    def derive_pipeline(self, goal_tool_id: str, goal_action: str) -> list[ToolCapability]:
        """Return a topologically ordered list of capabilities needed to reach the goal."""
        goal_key = f"{goal_tool_id}/{goal_action}"

        # Build reverse adjacency: dependent → list of prerequisites
        prereqs_of: dict[str, list[str]] = {}
        for pre, dep in _DEPENDENCY_CHAIN:
            prereqs_of.setdefault(dep, []).append(pre)

        visited: set[str] = set()
        post_order: list[str] = []

        def dfs(node: str) -> None:
            if node in visited:
                return
            visited.add(node)
            for pre in prereqs_of.get(node, []):
                dfs(pre)
            post_order.append(node)

        dfs(goal_key)

        # Map back to ToolCapability objects
        cap_index = {f"{c.tool_id}/{c.action}": c for c in self.capabilities}
        return [cap_index[k] for k in post_order if k in cap_index]

    def format_for_prompt(self) -> str:
        """Markdown section describing the tool dependency graph for the system prompt."""
        if not self.capabilities:
            return ""
        lines = [
            "## Tool Data-Flow Graph\n",
            "Build pipelines by following the produces → consumes chain. "
            "Prerequisites must always run before their dependents.\n",
        ]
        for cap in self.capabilities:
            consumes = ", ".join(f"`{c}`" for c in cap.consumes) or "_(no input)_"
            produces = ", ".join(f"`{p}`" for p in cap.produces) or "_(no output)_"
            lines.append(
                f"- **`{cap.tool_id}/{cap.action}`**: "
                f"consumes {consumes} → produces {produces}"
            )
        active_chain = [
            (p, d) for p, d in _DEPENDENCY_CHAIN
            if any(c.tool_id == p.split("/")[0] for c in self.capabilities)
        ]
        if active_chain:
            lines.append("\n**Required ordering** (run prerequisite before dependent):")
            for pre, dep in active_chain:
                lines.append(f"  - `{pre}` → `{dep}`")
        return "\n".join(lines)
