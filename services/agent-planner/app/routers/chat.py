"""Chat service — owns the conversation loop, nothing else.

Phases wired in:
  18 — Reflection: after generating a plan, critique it for missing steps / broken deps
  20 — Disambiguation: detect vague intents and ask one grounded clarifying question
"""

from __future__ import annotations

import asyncio
import logging
from pathlib import Path

from fastapi import APIRouter, Header, HTTPException
from pydantic import BaseModel

from app.agent import designate
from app.agent.plan_writer import write_plan
from app.auth import verify_workspace_access
from app.config import settings
from app.fetch import fetch_chat_history, add_chat_message
from app.fetch.connectors import fetch_connectors_with_resources
from app.fetch.documents import fetch_knowledge_documents
from app.fetch.iam import fetch_user_profile
from app.fetch.nodes import fetch_semantic_nodes
from app.gateway_client.client import FederationClient
from app.intent import needs_clarification
from app.plan.orchestrator import generate_plan_context
from app.reflection import reflect_on_plan
from app.schema_inspector import extract_schema_from_resources

logger = logging.getLogger("agent-planner")
router = APIRouter()

_CHAT_SYSTEM_PROMPT = (
    Path(__file__).parent.parent / "prompts" / "chat_system.txt"
).read_text()

import re as _re

# Words that, on their own, mean "stop planning and deploy this".
_DEPLOY_CONFIRM_WORDS = {
    "deploy", "execute", "run", "go", "yes", "please", "proceed", "ok",
    "okay", "confirm", "yep", "yeah", "ship", "it", "lets", "let's", "do",
    "ahead", "now", "sure",
}


class ChatRequest(BaseModel):
    workspace_id: str
    message: str


class ChatResponse(BaseModel):
    response: str
    intent: str | None = None  # "deploy" | "clarification" | "plan"
    # Identity of the designated agent that authored this turn, rendered as a
    # sender badge by the frontend. Kept OUT of `response` so the plan markdown
    # stays clean for the plan editor / deploy compile.
    agent_name: str | None = None
    agent_role: str | None = None


def _is_deploy_intent(message: str) -> bool:
    """True for short confirmation messages like 'go', 'yes', 'deploy', 'run it'.

    We only treat a message as a deploy confirmation when it is composed *only* of
    confirm/deploy words, so a real planning request like 'run a churn report'
    (which merely contains 'run') is not mistaken for a deploy.
    """
    words = _re.findall(r"[a-z']+", message.lower())
    return bool(words) and set(words).issubset(_DEPLOY_CONFIRM_WORDS)


@router.post("/chat", response_model=ChatResponse)
async def chat(
    body: ChatRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> ChatResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await verify_workspace_access(client, body.workspace_id)

    try:
        await add_chat_message(client, workspace_id=body.workspace_id, sender="user", content=body.message)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc))

    if _is_deploy_intent(body.message):
        reply = (
            "I have compiled and verified your integration blueprint. "
            "To deploy the pipeline, please click the **Deploy Plan** button. "
            "This will let you select your target sandbox environment, inspect the required container tools, and execute the deployment."
        )
        try:
            await add_chat_message(client, workspace_id=body.workspace_id, sender="ai", content=reply)
        except Exception as exc:
            logger.error("Failed to save deploy-intent reply: %s", exc)
        return ChatResponse(response=reply, intent="deploy")

    # Phase 20 — Fetch connector metadata for disambiguation check (concurrent with history)
    try:
        history, connectors_data = await asyncio.gather(
            fetch_chat_history(client, body.workspace_id),
            fetch_connectors_with_resources(client, group_id=None),
        )
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Failed to fetch workspace data: {exc}")

    # Phase 20 — Check for ambiguous intent before invoking the full LLM pipeline
    db_schema = extract_schema_from_resources(connectors_data)
    table_names = list(db_schema.keys())
    connector_kinds = [
        entry.get("connector", {}).get("kind", "")
        for entry in connectors_data
        if entry.get("connector")
    ]
    clarification = needs_clarification(body.message, table_names, connector_kinds)
    if clarification and not history:  # only ask once at conversation start
        try:
            await add_chat_message(client, workspace_id=body.workspace_id, sender="ai", content=clarification)
        except Exception as exc:
            logger.error("Failed to save clarification question: %s", exc)
        return ChatResponse(response=clarification, intent="clarification")

    # Assemble the workspace context plan and, in parallel, the company
    # knowledge-graph signal (user profile, graph nodes, reference documents).
    # The KG is ALWAYS consulted before an agent is designated or asks anything.
    try:
        context_plan, user_profile, nodes, documents = await asyncio.gather(
            generate_plan_context(
                gateway_url=settings.graphql_gateway_url,
                user_id=x_user_id,
                workspace_id=body.workspace_id,
            ),
            fetch_user_profile(client, x_user_id),
            fetch_semantic_nodes(client, workspace_id=body.workspace_id),
            fetch_knowledge_documents(client, workspace_id=body.workspace_id),
        )
    except Exception as exc:
        logger.error("Failed to assemble planning context: %s", exc)
        context_plan, user_profile, nodes, documents = "", None, [], []

    # Anchor the agent identity on the FIRST user message so the same agent owns
    # the whole conversation — follow-ups like "go" or "use CSV instead" must not
    # re-roll a different persona. Falls back to the current message at turn one.
    first_user_msg = next(
        (m["content"] for m in history if m["sender"] == "user"),
        body.message,
    )

    session = designate(
        message=first_user_msg,
        workspace_id=body.workspace_id,
        twin_owner_id=x_user_id,
        user_profile=user_profile,
        nodes=nodes,
        documents=documents,
    )

    claude_messages = [
        {"role": "user" if m["sender"] == "user" else "assistant", "content": m["content"]}
        for m in history
    ]

    # The designated agent authors (or revises) the PLAN.md for this turn.
    try:
        plan_md = await write_plan(
            session,
            message=body.message,
            context_plan=context_plan,
            history=claude_messages,
        )
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Error authoring plan: {exc}")

    # Phase 18 — Reflect on the proposed pipeline for missing steps / broken deps.
    api_key = settings.anthropic_api_key
    if api_key:
        tools_context = (
            context_plan[context_plan.find("### Available Execution Tools"):]
            if "### Available Execution Tools" in context_plan
            else context_plan[:2000]
        )
        plan_md = await reflect_on_plan(plan_md, tools_context, api_key)

    # Keep the plan markdown clean; the agent identity travels as structured
    # fields so the UI renders it as a sender badge, not inside the plan body.
    ai_response = plan_md

    try:
        await add_chat_message(client, workspace_id=body.workspace_id, sender="ai", content=ai_response)
    except Exception as exc:
        logger.error("Failed to save AI response: %s", exc)

    return ChatResponse(
        response=ai_response,
        intent="plan",
        agent_name=session.agent.name,
        agent_role=session.agent.role,
    )


@router.get("/history/{workspace_id}")
async def get_history(
    workspace_id: str,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> list[dict]:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await verify_workspace_access(client, workspace_id)
    try:
        return await fetch_chat_history(client, workspace_id)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc))
