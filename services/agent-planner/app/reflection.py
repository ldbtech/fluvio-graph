"""Phase 18 — Multi-turn plan reflection.

After Claude generates an initial blueprint, pipe it through a second call that
critiques it for missing cleaning steps, broken dependencies, and unregistered
tools. Returns either the original plan (APPROVED) or a revised one.

Only triggers when the response looks like a plan (contains code fences or
phase headers) — casual answers are passed through unchanged.
"""

from __future__ import annotations

import logging
import re
from pathlib import Path

import httpx

logger = logging.getLogger("agent-planner")

_REFLECTION_PROMPT = (
    Path(__file__).parent / "prompts" / "plan_reflection.txt"
).read_text()

# Heuristics that indicate the response is a plan rather than a casual answer
_PLAN_SIGNALS = re.compile(
    r"(```|Phase \d|### Phase|## Phase|tool_id|data-cleaning|execute_sql|run_cleaning)",
    re.IGNORECASE,
)


def _looks_like_plan(text: str) -> bool:
    return bool(_PLAN_SIGNALS.search(text))


async def reflect_on_plan(
    plan: str,
    tools_context: str,
    api_key: str,
    model: str = "claude-sonnet-4-20250514",
) -> str:
    """Run a reflection pass on plan. Returns the (possibly revised) plan text."""
    if not _looks_like_plan(plan):
        return plan  # casual answer — skip reflection

    system = _REFLECTION_PROMPT.format(
        plan=plan,
        tools_context=tools_context or "_(no tool registry context available)_",
    )
    user = "Review the plan above and output APPROVED or a revised plan."

    try:
        async with httpx.AsyncClient() as http:
            resp = await http.post(
                "https://api.anthropic.com/v1/messages",
                headers={
                    "x-api-key": api_key,
                    "anthropic-version": "2023-06-01",
                    "content-type": "application/json",
                },
                json={
                    "model": model,
                    "max_tokens": 4096,
                    "system": system,
                    "messages": [{"role": "user", "content": user}],
                },
                timeout=60.0,
            )
            if resp.status_code != 200:
                logger.warning("Reflection call failed (%d) — returning original plan", resp.status_code)
                return plan
            text = resp.json()["content"][0]["text"].strip()
    except Exception as exc:
        logger.warning("Reflection call error — returning original plan: %s", exc)
        return plan

    if text.upper().startswith("APPROVED"):
        logger.info("Plan reflection: APPROVED")
        return plan

    if text.upper().startswith("REVISED:"):
        logger.info("Plan reflection: REVISED")
        return text

    # Unexpected format — treat as approved to avoid confusion
    logger.warning("Reflection returned unexpected format — treating as APPROVED")
    return plan
