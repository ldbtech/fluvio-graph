"""Chat service — owns the conversation loop, nothing else.

Phases wired in:
  18 — Reflection: after generating a plan, critique it for missing steps / broken deps
  20 — Disambiguation: detect vague intents and ask one grounded clarifying question
"""

from __future__ import annotations

import asyncio
import logging
from pathlib import Path

import httpx
from fastapi import APIRouter, Header, HTTPException
from pydantic import BaseModel

from app.auth import verify_workspace_access
from app.config import settings
from app.fetch import fetch_chat_history, add_chat_message
from app.fetch.connectors import fetch_connectors_with_resources
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

_DEPLOY_KEYWORDS = {"deploy", "execute", "run"}
_CONFIRM_KEYWORDS = {"yes", "please", "go", "proceed", "ok"}


class ChatRequest(BaseModel):
    workspace_id: str
    message: str


class ChatResponse(BaseModel):
    response: str
    intent: str | None = None  # "deploy" | "clarification"


def _is_deploy_intent(message: str) -> bool:
    lower = message.lower().strip()
    has_deploy = any(k in lower for k in _DEPLOY_KEYWORDS)
    has_confirm = any(k in lower for k in _CONFIRM_KEYWORDS) or lower == "deploy"
    return has_deploy and has_confirm


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

    try:
        context_plan = await generate_plan_context(
            gateway_url=settings.graphql_gateway_url,
            user_id=x_user_id,
            workspace_id=body.workspace_id,
        )
    except Exception as exc:
        logger.error("Failed to generate context plan: %s", exc)
        context_plan = ""

    claude_messages = [
        {"role": "user" if m["sender"] == "user" else "assistant", "content": m["content"]}
        for m in history
    ]
    system_prompt = _CHAT_SYSTEM_PROMPT.format(context_plan=context_plan)

    api_key = settings.anthropic_api_key
    if not api_key:
        ai_response = f"ANTHROPIC_API_KEY not configured. Echo: '{body.message}'"
    else:
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
                        "model": "claude-sonnet-4-20250514",
                        "max_tokens": 4096,
                        "system": system_prompt,
                        "messages": claude_messages,
                    },
                    timeout=90.0,
                )
                if resp.status_code != 200:
                    raise HTTPException(status_code=502, detail=f"Anthropic API error: {resp.text}")
                ai_response = resp.json()["content"][0]["text"]
        except HTTPException:
            raise
        except Exception as exc:
            raise HTTPException(status_code=500, detail=f"Error generating LLM response: {exc}")

        # Phase 18 — Reflect on the plan if the response looks like a blueprint
        # Extract just the tools section from context_plan for the reflection check
        tools_context = context_plan[context_plan.find("### Available Execution Tools"):] if "### Available Execution Tools" in context_plan else context_plan[:2000]
        ai_response = await reflect_on_plan(ai_response, tools_context, api_key)

    try:
        await add_chat_message(client, workspace_id=body.workspace_id, sender="ai", content=ai_response)
    except Exception as exc:
        logger.error("Failed to save AI response: %s", exc)

    return ChatResponse(response=ai_response)


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
