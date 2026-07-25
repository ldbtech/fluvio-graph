"""Phase 20 — Intent disambiguation.

If a user message is ambiguous (vague verbs like "analyze", "look at",
"understand") and the workspace has known tables, ask exactly one precise
clarifying question grounded in what's actually available.

Returns None if the intent is clear enough to proceed without clarification.
The question is cheap to generate locally — no LLM call needed.
"""

from __future__ import annotations

import re

# Verbs that indicate the user hasn't decided what kind of analysis they want
_AMBIGUOUS_PATTERNS = re.compile(
    r"\b(analyze|analyse|understand|look at|explore|report on|show me|check|review|examine)\b",
    re.IGNORECASE,
)

# Signals that indicate sufficient specificity — skip clarification even if verb is ambiguous
_SPECIFIC_SIGNALS = re.compile(
    r"\b(revenue|trend|segmentation|cohort|churn|kpi|dashboard|tableau|powerbi|pdf|"
    r"monthly|weekly|daily|join|aggregate|count|sum|average|growth|forecast|deploy)\b",
    re.IGNORECASE,
)


def needs_clarification(
    message: str,
    table_names: list[str],
    connector_kinds: list[str],
) -> str | None:
    """Return a clarifying question or None if the intent is clear.

    Args:
        message: the raw user message
        table_names: tables known from the workspace connector (e.g. ["orders", "customers"])
        connector_kinds: active connector types (e.g. ["postgres", "tableau"])
    """
    if not _AMBIGUOUS_PATTERNS.search(message):
        return None  # clear intent verb
    if _SPECIFIC_SIGNALS.search(message):
        return None  # user was specific about what they want

    # Build a grounded question using the actual tables
    if table_names:
        table_list = ", ".join(f"`{t}`" for t in table_names[:5])
        options = _infer_options(table_names)
        if options:
            return (
                f"I can see you have {table_list} tables available. "
                f"What are you looking to understand — {options}?"
            )
        return (
            f"I can see you have {table_list} tables available. "
            f"Could you be more specific? For example: revenue trends, customer segmentation, or product performance?"
        )

    if connector_kinds:
        kinds = ", ".join(connector_kinds)
        return (
            f"You have {kinds} connectors active. "
            f"What would you like to analyze — data trends, aggregates, or a dashboard report?"
        )

    return None  # not enough workspace context to ask a grounded question


def _infer_options(table_names: list[str]) -> str:
    """Generate 2-3 analysis options inferred from table names."""
    names_lower = [t.lower() for t in table_names]
    options: list[str] = []

    if any(t in names_lower for t in ("orders", "bookings", "sales", "transactions", "revenue")):
        options.append("revenue trends")
    if any(t in names_lower for t in ("users", "customers", "members", "accounts")):
        options.append("customer segmentation")
    if any(t in names_lower for t in ("products", "items", "inventory", "sku")):
        options.append("product performance")
    if any(t in names_lower for t in ("events", "sessions", "clicks", "pageviews")):
        options.append("user engagement")
    if any(t in names_lower for t in ("employees", "staff", "hr", "payroll")):
        options.append("workforce metrics")

    if not options:
        return ""
    if len(options) == 1:
        return options[0]
    return ", ".join(options[:-1]) + f", or {options[-1]}"
