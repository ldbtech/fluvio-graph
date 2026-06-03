"""Compatibility shim — all real validation is now in app.toolbox.

The static hardcoded registry has been removed. Use toolbox.registry_for()
to get a ToolRegistry scoped to the workspace's active tools, then call
registry.validate_steps() and registry.build_retry_prompt().
"""

from app.toolbox import toolbox, ToolRegistry


def validate_steps(raw_steps: list, active_tool_ids: set[str] | None = None):
    """Validate steps against the live manifest registry.

    Returns (valid_steps, errors). Delegates entirely to ToolRegistry.
    """
    registry = toolbox.registry_for(active_tool_ids)
    return registry.validate_steps(raw_steps)


def build_validation_retry_prompt(errors: list[str], active_tool_ids: set[str] | None = None) -> str:
    registry = toolbox.registry_for(active_tool_ids)
    return registry.build_retry_prompt(errors)
