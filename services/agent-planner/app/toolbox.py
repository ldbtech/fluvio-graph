"""Dynamic tool registry loaded from fluvio-tool-builder manifests.

This is the ONLY place that knows what tools exist, what actions they support,
and what their documentation says. Nothing else in the planner hardcodes tool
IDs or action names.

At import time the module walks `fluvio-tool-builder/src/tools/*/manifest.json`,
parses each manifest, and extracts valid action names from the `action` parameter
description (the pattern is: "Action to run: 'action_a', 'action_b'").

The corresponding `.md` file is loaded alongside as human-readable documentation
for injection into the Claude system prompt.

Workflow inside the planner:
    1.  `Toolbox.load()` is called once at startup.
    2.  Before planning, `toolbox.registry_for(fetched_tool_ids)` returns a
        `ToolRegistry` scoped to only the tools active in the current workspace.
    3.  The `ToolRegistry` is passed to step validation and to the system prompt
        formatter — no hardcoded lists anywhere else.

To add a new tool:
    - Drop a `manifest.json` + `.md` in `fluvio-tool-builder/src/tools/<name>/`
    - Redeploy (or restart) — the planner picks it up automatically.
"""

from __future__ import annotations

import json
import logging
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

logger = logging.getLogger("agent-planner")

# Pattern: "Action to run: 'trigger_dag', 'get_dag_run_status'"
_ACTION_RE = re.compile(r"'([a-z_][a-z0-9_]*)'")

# Auto-resolve the tool builder directory relative to this file's location
# (agent-planner/app/ → ../../fluvio-tool-builder/src/tools)
_DEFAULT_TOOLS_DIR = (
    Path(__file__).parent.parent.parent.parent  # kg-engine root
    / "services"
    / "fluvio-tool-builder"
    / "src"
    / "tools"
)


@dataclass
class ToolManifest:
    tool_id: str
    name: str
    description: str
    category: str
    valid_actions: set[str]
    documentation: str        # full .md content
    raw: dict[str, Any]       # original manifest dict

    def actions_summary(self) -> str:
        return ", ".join(f"`{a}`" for a in sorted(self.valid_actions))

    def format_for_prompt(self) -> str:
        """Compact markdown block suitable for inclusion in a system prompt."""
        lines = [
            f"### Tool: {self.name}  (id: `{self.tool_id}`, category: {self.category})",
            f"{self.description}",
            f"**Valid actions**: {self.actions_summary()}",
        ]
        if self.documentation:
            # Include the doc but keep it bounded
            doc_preview = self.documentation[:1500]
            if len(self.documentation) > 1500:
                doc_preview += "\n_(documentation truncated)_"
            lines.append("\n**Documentation**:")
            lines.append(doc_preview)
        return "\n".join(lines)


@dataclass
class ToolRegistry:
    """Scoped view of the toolbox — only the tools active in the current workspace."""
    manifests: dict[str, ToolManifest]   # tool_id → manifest

    def __contains__(self, tool_id: str) -> bool:
        return tool_id in self.manifests

    def valid_actions(self, tool_id: str) -> set[str]:
        m = self.manifests.get(tool_id)
        return m.valid_actions if m else set()

    def all_tool_ids(self) -> set[str]:
        return set(self.manifests.keys())

    def format_for_prompt(self) -> str:
        """Full toolbox description for injection into the step formulation prompt."""
        if not self.manifests:
            return "_No tools active in this workspace._"
        parts = ["## Active Toolbox\n"]
        parts.append(
            "The following tools are registered and active in this workspace. "
            "Use ONLY these tools. Do not invent tool_ids or action names not listed here.\n"
        )
        for manifest in sorted(self.manifests.values(), key=lambda m: m.category):
            parts.append(manifest.format_for_prompt())
            parts.append("")
        return "\n".join(parts)

    def validate_step(self, step: dict[str, Any]) -> list[str]:
        """Return a list of error strings for a single step dict, or empty list if valid."""
        errors: list[str] = []
        tool_id = step.get("tool_id", "")
        action = step.get("action", "")

        if not tool_id:
            errors.append("Missing tool_id")
            return errors
        if tool_id not in self.manifests:
            errors.append(
                f"Unknown tool_id '{tool_id}'. "
                f"Active tools: {sorted(self.all_tool_ids())}"
            )
            return errors
        if not action:
            errors.append(f"Missing action for tool '{tool_id}'")
            return errors
        valid = self.valid_actions(tool_id)
        if action not in valid:
            errors.append(
                f"Action '{action}' not valid for '{tool_id}'. "
                f"Valid: {sorted(valid)}"
            )
        return errors

    def validate_steps(self, steps: list[Any]) -> tuple[list[dict], list[str]]:
        """Validate a list of step dicts. Returns (valid_steps, all_errors)."""
        all_errors: list[str] = []
        for i, step in enumerate(steps):
            if not isinstance(step, dict):
                all_errors.append(f"Step {i}: not a JSON object")
                continue
            errs = self.validate_step(step)
            for e in errs:
                all_errors.append(f"Step {i+1} ({step.get('tool_id','?')}/{step.get('action','?')}): {e}")
        if all_errors:
            return [], all_errors
        return list(steps), []

    def build_retry_prompt(self, errors: list[str]) -> str:
        joined = "\n".join(f"  - {e}" for e in errors)
        return (
            "The steps you generated failed validation against the active toolbox:\n"
            f"{joined}\n\n"
            f"Active tools and their valid actions:\n{self._actions_summary()}\n\n"
            "Regenerate the complete corrected steps JSON array. Output ONLY the JSON array."
        )

    def _actions_summary(self) -> str:
        return "\n".join(
            f"  - `{tid}`: {', '.join(sorted(m.valid_actions))}"
            for tid, m in sorted(self.manifests.items())
        )


class Toolbox:
    """Loads and caches all tool manifests from the tool-builder directory."""

    def __init__(self, tools_dir: Path | None = None) -> None:
        self._tools_dir: Path = tools_dir or _DEFAULT_TOOLS_DIR
        self._manifests: dict[str, ToolManifest] = {}
        self._loaded = False

    def load(self) -> None:
        """Walk the tools directory and load every manifest. Safe to call multiple times."""
        if self._loaded:
            return
        if not self._tools_dir.exists():
            logger.warning(
                "Tool builder directory not found: %s — toolbox will be empty",
                self._tools_dir,
            )
            self._loaded = True
            return

        count = 0
        for manifest_path in sorted(self._tools_dir.glob("*/manifest.json")):
            try:
                raw = json.loads(manifest_path.read_text())
                tool_id = raw.get("id", "")
                if not tool_id:
                    logger.warning("manifest.json missing 'id': %s", manifest_path)
                    continue

                # Parse valid action names from the action parameter description
                valid_actions: set[str] = set()
                for param in raw.get("parameters", []):
                    if param.get("name") == "action":
                        valid_actions = set(_ACTION_RE.findall(param.get("description", "")))
                        break

                if not valid_actions:
                    logger.warning("Could not parse action names from %s", manifest_path)

                # Load documentation .md file (same directory, same stem as tool folder)
                tool_dir = manifest_path.parent
                md_files = list(tool_dir.glob("*.md"))
                documentation = md_files[0].read_text() if md_files else ""

                self._manifests[tool_id] = ToolManifest(
                    tool_id=tool_id,
                    name=raw.get("name", tool_id),
                    description=raw.get("description", ""),
                    category=raw.get("category", ""),
                    valid_actions=valid_actions,
                    documentation=documentation,
                    raw=raw,
                )
                count += 1
                logger.debug("Loaded tool manifest: %s (%s)", tool_id, sorted(valid_actions))

            except Exception as exc:
                logger.error("Failed to load manifest %s: %s", manifest_path, exc)

        self._loaded = True
        logger.info("Toolbox loaded %d tools from %s", count, self._tools_dir)

    def all_manifests(self) -> dict[str, ToolManifest]:
        self.load()
        return dict(self._manifests)

    def registry_for(self, active_tool_ids: set[str] | None = None) -> ToolRegistry:
        """Return a ToolRegistry scoped to active_tool_ids.

        If active_tool_ids is None or empty, returns all known tools (useful
        when the workspace hasn't returned a tool list yet).
        """
        self.load()
        if not active_tool_ids:
            return ToolRegistry(manifests=dict(self._manifests))
        scoped = {
            tid: m
            for tid, m in self._manifests.items()
            if tid in active_tool_ids
        }
        unknown = active_tool_ids - set(self._manifests.keys())
        if unknown:
            logger.warning("Workspace has tools not in local manifests: %s", unknown)
        return ToolRegistry(manifests=scoped)

    def get(self, tool_id: str) -> ToolManifest | None:
        self.load()
        return self._manifests.get(tool_id)


# Module-level singleton — import this everywhere
toolbox = Toolbox()
