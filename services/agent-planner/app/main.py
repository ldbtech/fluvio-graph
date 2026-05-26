"""agent-planner — HTTP microservice for LLM context plan generation."""

from contextlib import asynccontextmanager

import uvicorn
import httpx
from fastapi import FastAPI, Header, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from app.config import settings
from app.logging_config import setup_logging
from app.plan.orchestrator import generate_plan_context
from app.schemas import PlanContextRequest, PlanContextResponse
from app.fetch import fetch_chat_history, add_chat_message
from app.fetch.chat import GraphQLError
from app.gateway_client.client import FederationClient

logger = setup_logging()


class ChatRequest(BaseModel):
    workspace_id: str
    message: str


class ChatResponse(BaseModel):
    response: str


@asynccontextmanager
async def lifespan(_app: FastAPI):
    logger.info(
        "agent-planner starting on :%s (gateway=%s)",
        settings.port,
        settings.graphql_gateway_url,
    )
    yield
    logger.info("agent-planner stopped")


app = FastAPI(title="agent-planner", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/plan/context", response_model=PlanContextResponse)
async def plan_context(
    body: PlanContextRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> PlanContextResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    plan = await generate_plan_context(
        gateway_url=settings.graphql_gateway_url,
        user_id=x_user_id,
        group_id=body.group_id,
        workspace_id=body.workspace_id,
        zone=body.zone,
        domain=body.domain,
    )
    return PlanContextResponse(plan=plan)


@app.post("/chat", response_model=ChatResponse)
async def chat(
    body: ChatRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> ChatResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    headers = {"x-user-id": x_user_id}
    client = FederationClient(settings.graphql_gateway_url, headers=headers)

    # 1. Add user message to history. This also serves as workspace authorization.
    try:
        await add_chat_message(
            client=client,
            workspace_id=body.workspace_id,
            sender="user",
            content=body.message,
        )
    except GraphQLError as e:
        if "access denied" in str(e).lower() or "forbidden" in str(e).lower() or "not found" in str(e).lower():
            raise HTTPException(status_code=403, detail=str(e))
        raise HTTPException(status_code=500, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

    # 2. Fetch history (which now includes the user's message)
    try:
        history = await fetch_chat_history(client, body.workspace_id)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch chat history: {e}")

    # 3. Generate context plan using the orchestrator
    try:
        context_plan = await generate_plan_context(
            gateway_url=settings.graphql_gateway_url,
            user_id=x_user_id,
            workspace_id=body.workspace_id,
        )
    except Exception as e:
        logger.error("Failed to generate context plan: %s", e)
        context_plan = ""

    # 4. Construct messages list for Claude
    # Map 'ai' -> 'assistant' and 'user' -> 'user'
    claude_messages = []
    for msg in history:
        role = "user" if msg["sender"] == "user" else "assistant"
        claude_messages.append({
            "role": role,
            "content": msg["content"]
        })

    # System prompt combining instructions and the compiled workspace context
    system_prompt = (
        "You are the Fluviome AI Architect, a planning assistant.\n"
        "You help engineers and database administrators design data pipelines, select database schemas to share with AI systems, and route data using Kafka/Spark.\n\n"
        "Below is the active Workspace blueprint context, including active database schemas, semantic connectors, knowledge documents, and shared team twin concepts:\n\n"
        "[CONTEXT_START]\n"
        f"{context_plan}\n"
        "[CONTEXT_END]\n\n"
        "Analyze the context and chat history to answer user queries, suggest pipelines, or help them structure their data."
    )

    # 5. Call Anthropic API
    ai_response_text = ""
    api_key = settings.anthropic_api_key
    if not api_key:
        logger.warning("ANTHROPIC_API_KEY not configured. Falling back to a mock response.")
        ai_response_text = (
            f"Anthropic API key is not configured. Here is your message: '{body.message}'. "
            "Once you add ANTHROPIC_API_KEY to your environment, I will respond using Claude."
        )
    else:
        try:
            async with httpx.AsyncClient() as http_client:
                resp = await http_client.post(
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
                    timeout=30.0,
                )
                if resp.status_code != 200:
                    logger.error("Anthropic API returned status %s: %s", resp.status_code, resp.text)
                    raise HTTPException(status_code=502, detail=f"Anthropic API error: {resp.text}")
                
                resp_json = resp.json()
                ai_response_text = resp_json["content"][0]["text"]
        except httpx.HTTPError as e:
            logger.error("HTTP error calling Anthropic API: %s", e)
            raise HTTPException(status_code=502, detail=f"Failed to communicate with Anthropic API: {e}")
        except Exception as e:
            logger.error("Error communicating with Anthropic API: %s", e)
            raise HTTPException(status_code=500, detail=f"Error generating LLM response: {e}")

    # 6. Add AI message to history
    try:
        await add_chat_message(
            client=client,
            workspace_id=body.workspace_id,
            sender="ai",
            content=ai_response_text,
        )
    except Exception as e:
        logger.error("Failed to save AI response to history: %s", e)

    return ChatResponse(response=ai_response_text)


if __name__ == "__main__":
    uvicorn.run(
        "app.main:app",
        host="0.0.0.0",
        port=settings.port,
        log_level="info",
    )
