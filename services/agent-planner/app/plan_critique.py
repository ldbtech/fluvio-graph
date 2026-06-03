"""Plan critique — validates steps against the live toolbox and workspace-active tools.

Replaces the old hardcoded _KNOWN_ACTIONS / _REQUIRED_ARGS approach.
The toolbox knows what tools exist and what actions they support; this module
adds the workspace-active-tool gate on top (a tool in the registry but not
deployed in this workspace should still be rejected).
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Any

from app.toolbox import toolbox, ToolRegistry

logger = logging.getLogger("agent-planner")


@dataclass
class CritiqueResult:
    valid: bool
    steps: list[dict[str, Any]]
    errors: list[str]


def critique_steps(
    steps: list[dict[str, Any]],
    active_tool_ids: set[str],
) -> CritiqueResult:
    """Validate steps using the live manifest registry scoped to active_tool_ids.

    active_tool_ids: the set of tool IDs the workspace actually has deployed
    (fetched from availableTools query at request time).
    """
    registry: ToolRegistry = toolbox.registry_for(active_tool_ids or None)
    validated, errors = registry.validate_steps(steps)
    if errors:
        return CritiqueResult(valid=False, steps=[], errors=errors)
    return CritiqueResult(valid=True, steps=validated, errors=[])


def build_critique_retry_prompt(errors: list[str], active_tool_ids: set[str] | None = None) -> str:
    registry = toolbox.registry_for(active_tool_ids)
    return registry.build_retry_prompt(errors)
