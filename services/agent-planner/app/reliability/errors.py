"""Error hierarchy for tool execution.

Transient errors are retryable (network timeouts, 5xx from tool gateway).
Permanent errors are not (schema validation failures, unknown tool_id, 4xx).
"""


class ToolError(Exception):
    """Base class for all tool execution errors."""

    def __init__(self, message: str, tool_id: str = "", action: str = "") -> None:
        super().__init__(message)
        self.tool_id = tool_id
        self.action = action


class TransientToolError(ToolError):
    """Retryable — network hiccup, gateway overload, temporary unavailability."""


class PermanentToolError(ToolError):
    """Non-retryable — bad arguments, unknown tool, schema violation."""


class CircuitOpenError(ToolError):
    """Raised when the circuit breaker for a tool is open."""


# Status strings from executeTool that signal permanent failure
_PERMANENT_STATUSES = frozenset({"validation_error", "not_found", "unauthorized", "bad_request"})
# Status strings that signal transient failure
_TRANSIENT_STATUSES = frozenset({"timeout", "unavailable", "gateway_error", "internal_error"})


def classify_tool_error(tool_id: str, action: str, status: str, output: str) -> ToolError:
    """Convert a failed executeTool response into the correct error subclass."""
    lower_out = (output or "").lower()
    lower_status = (status or "").lower()

    if lower_status in _PERMANENT_STATUSES:
        return PermanentToolError(output or status, tool_id=tool_id, action=action)

    if lower_status in _TRANSIENT_STATUSES:
        return TransientToolError(output or status, tool_id=tool_id, action=action)

    # Heuristic: validation / schema / argument errors → permanent
    permanent_phrases = ("validation", "schema", "unknown tool", "not found", "invalid argument", "400", "401", "403", "404")
    if any(p in lower_out for p in permanent_phrases):
        return PermanentToolError(output or status, tool_id=tool_id, action=action)

    # Default transient so retries happen for ambiguous failures
    return TransientToolError(output or status, tool_id=tool_id, action=action)
