"""Plan writer — the designated agent's first act.

Given a session (agent identity + company context) and the assembled workspace
context, ask Claude to author the PLAN.md the user will review. The agent's name,
role, and personality are injected into the prompt so the plan reads like it came
from that specific person.
"""

from __future__ import annotations

import logging
from pathlib import Path

from fluvio_planner.agent.designation import AgentSession
from fluvio_planner.llm import ProviderConfig, chat as llm_chat

logger = logging.getLogger("agent-planner")

_PLAN_WRITER_PROMPT = (
    Path(__file__).parent.parent / "prompts" / "plan_writer.txt"
).read_text()


async def write_plan(
    session: AgentSession,
    message: str,
    context_plan: str,
    history: list[dict] | None = None,
    *,
    provider_config: ProviderConfig | None,
) -> str:
    """Author (or revise) the PLAN.md for this session and return it as Markdown.

    `history` is the prior chat turns in Anthropic message form
    (`{"role": ..., "content": ...}`). Passing it lets the same agent revise its
    plan when the user edits it or answers a question, instead of starting over.
    The returned string already carries the agent's voice; the caller prepends
    the agent signature chip and saves it as the AI chat message.

    `provider_config` is injected by the caller (the composition root) rather
    than read from a config singleton, so this module never touches the
    environment — it resolves the user's connected LLM provider, or this
    deployment's fallback.
    """
    agent = session.agent
    system_prompt = _PLAN_WRITER_PROMPT.format(
        agent_name=agent.name,
        agent_role=agent.role,
        agent_personality=agent.personality,
        company_context=session.company_context,
        context_plan=context_plan or "(no workspace context could be assembled)",
    )

    if not provider_config:
        # Degrade gracefully: still introduce the agent and echo the ask so the
        # conversation can continue without a provider configured.
        return (
            f"{agent.intro_line()}\n\n"
            f"_(No LLM provider configured — connect one, or set a deployment "
            f"fallback key, to author the full plan.)_\n\n"
            f"You asked: \"{message}\""
        )

    # Replay the conversation so plan revisions build on what came before; the
    # current message is already the last turn in `history` when present.
    messages = list(history) if history else [{"role": "user", "content": message}]

    try:
        return await llm_chat(provider_config, system_prompt, messages)
    except Exception as exc:
        logger.error("Plan authoring failed for session %s: %s", session.session_id, exc)
        raise
