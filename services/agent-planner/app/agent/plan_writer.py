"""Plan writer — the designated agent's first act.

Given a session (agent identity + company context) and the assembled workspace
context, ask Claude to author the PLAN.md the user will review. The agent's name,
role, and personality are injected into the prompt so the plan reads like it came
from that specific person.
"""

from __future__ import annotations

import logging
from pathlib import Path

import httpx

from app.agent.designation import AgentSession

logger = logging.getLogger("agent-planner")

_PLAN_WRITER_PROMPT = (
    Path(__file__).parent.parent / "prompts" / "plan_writer.txt"
).read_text()

# Match the model used elsewhere in the planner chat path.
_MODEL = "claude-sonnet-4-20250514"


async def write_plan(
    session: AgentSession,
    message: str,
    context_plan: str,
    history: list[dict] | None = None,
    *,
    api_key: str | None,
) -> str:
    """Author (or revise) the PLAN.md for this session and return it as Markdown.

    `history` is the prior chat turns in Anthropic message form
    (`{"role": ..., "content": ...}`). Passing it lets the same agent revise its
    plan when the user edits it or answers a question, instead of starting over.
    The returned string already carries the agent's voice; the caller prepends
    the agent signature chip and saves it as the AI chat message.

    `api_key` is injected by the caller (the composition root) rather than read
    from a config singleton, so this module never touches the environment.
    """
    agent = session.agent
    system_prompt = _PLAN_WRITER_PROMPT.format(
        agent_name=agent.name,
        agent_role=agent.role,
        agent_personality=agent.personality,
        company_context=session.company_context,
        context_plan=context_plan or "(no workspace context could be assembled)",
    )

    if not api_key:
        # Degrade gracefully: still introduce the agent and echo the ask so the
        # conversation can continue without a key configured.
        return (
            f"{agent.intro_line()}\n\n"
            f"_(ANTHROPIC_API_KEY not configured — cannot author the full plan yet.)_\n\n"
            f"You asked: \"{message}\""
        )

    # Replay the conversation so plan revisions build on what came before; the
    # current message is already the last turn in `history` when present.
    messages = list(history) if history else [{"role": "user", "content": message}]

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
                    "model": _MODEL,
                    "max_tokens": 4096,
                    "system": system_prompt,
                    "messages": messages,
                },
                timeout=90.0,
            )
            if resp.status_code != 200:
                raise RuntimeError(f"Anthropic API error: {resp.text}")
            return resp.json()["content"][0]["text"]
    except Exception as exc:
        logger.error("Plan authoring failed for session %s: %s", session.session_id, exc)
        raise
