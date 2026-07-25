"""Plan compiler — owns workspace context assembly and LLM step generation.

POST /plan/compile  → {steps: [...]} validated JSON array
POST /plan/context  → raw markdown context (existing interface, kept intact)

Phases wired in:
  19 — RAG: past deployment examples injected as few-shot context
  21 — Critique: dynamic active-tool validation + auto-recovery
  22 — Tool graph: produces/consumes context in system prompt
  23 — Schema: exact column names injected; SQL warnings surfaced
"""

from __future__ import annotations

import asyncio
import json
import logging
from pathlib import Path
from typing import Any

import httpx
from fastapi import APIRouter, Header, HTTPException
from pydantic import BaseModel

from app.auth import verify_workspace_access
from app.config import settings
from fluvio_planner.credential_vault import scrub_credentials, CredentialRef, resolve_credentials
from fluvio_planner.fetch import fetch_chat_history
from fluvio_planner.fetch.connectors import fetch_connectors_with_resources
from fluvio_planner.fetch.tools import fetch_available_tools
from fluvio_planner.gateway_client.client import FederationClient
from fluvio_planner.memory.rag import fetch_similar_deployments, format_rag_examples
from fluvio_planner.plan.orchestrator import generate_plan_context
from app.schemas import PlanContextRequest, PlanContextResponse
from fluvio_planner.resources import read_prompt
from fluvio_planner.schema_inspector import extract_schema_from_resources, format_schema_for_prompt, explain_sql
from fluvio_planner.tool_graph import ToolCapabilityGraph
from fluvio_planner.toolbox import toolbox
from fluvio_planner.workspace_config import build_environment_context, resolve_workspace_config

logger = logging.getLogger("agent-planner")
router = APIRouter()

_STEP_PROMPT = read_prompt("step_formulation.txt")


class CompileRequest(BaseModel):
    workspace_id: str
    approved_markdown: str | None = None
    message: str | None = None


class CompileResponse(BaseModel):
    steps: list[dict[str, Any]]


def _parse_claude_json(text: str) -> list[Any]:
    clean = text.strip()
    if "```json" in clean:
        clean = clean.split("```json")[1].split("```")[0].strip()
    elif "```" in clean:
        clean = clean.split("```")[1].split("```")[0].strip()
    return json.loads(clean)


@router.post("/plan/compile", response_model=CompileResponse)
async def compile_plan(
    body: CompileRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> CompileResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    api_key = settings.anthropic_api_key
    if not api_key:
        raise HTTPException(status_code=503, detail="ANTHROPIC_API_KEY not configured")

    client = FederationClient(settings.graphql_gateway_url, headers={"x-user-id": x_user_id})
    await verify_workspace_access(client, body.workspace_id)

    # Gather everything concurrently
    cfg, connectors_data, active_tools = await asyncio.gather(
        resolve_workspace_config(client=client, workspace_id=body.workspace_id, message=body.message),
        fetch_connectors_with_resources(client, group_id=None),
        fetch_available_tools(client),
    )

    # Phase 23 — Real schema from connector resources
    db_schema = extract_schema_from_resources(connectors_data)
    schema_section = format_schema_for_prompt(db_schema)

    # Phase 22 — Tool capability graph
    tool_graph = ToolCapabilityGraph.from_active_tools(active_tools)
    tool_graph_section = tool_graph.format_for_prompt()

    # Phase 19 — RAG: retrieve similar past deployments
    rag_query = body.message or body.approved_markdown or "pipeline deployment"
    rag_examples = await fetch_similar_deployments(client, rag_query, body.workspace_id)
    rag_section = format_rag_examples(rag_examples)

    safe_env = scrub_credentials(build_environment_context(cfg, sandbox_id=None))
    context_plan = await generate_plan_context(
        gateway_url=settings.graphql_gateway_url,
        user_id=x_user_id,
        workspace_id=body.workspace_id,
    )

    # Build a ToolRegistry scoped to this workspace's active tools. Must be
    # constructed before the system prompt, which embeds the toolbox section.
    active_tool_ids = {t.get("id") for t in active_tools if t.get("id")}
    registry = toolbox.registry_for(active_tool_ids)

    # Build enriched system prompt — toolbox registry goes in FIRST so Claude
    # opens the toolbox before attempting to plan any steps
    toolbox_section = registry.format_for_prompt()

    system_prompt = _STEP_PROMPT.format(
        environment_context=json.dumps(safe_env, indent=2),
        output_suffix=cfg.default_output_table_suffix,
    )
    # Prepend toolbox so it appears before generic instructions
    system_prompt = toolbox_section + "\n\n" + system_prompt
    if schema_section:
        system_prompt += f"\n\n{schema_section}"
    if tool_graph_section:
        system_prompt += f"\n\n{tool_graph_section}"

    # Phase M2 — feed the live MCP tool schemas to Claude. Best-effort: if the
    # MCP server is reachable, its real inputSchemas augment the static toolbox
    # manifest; if not, the existing toolbox section still drives planning.
    try:
        from fluvio_planner.capabilities.mcp_client import list_tools, format_tools_for_prompt
        mcp_section = format_tools_for_prompt(await list_tools(settings.mcp_server_url))
        if mcp_section:
            system_prompt += f"\n\n{mcp_section}"
    except Exception as exc:
        logger.warning("MCP schema injection skipped: %s", exc)

    # Build user prompt
    if body.approved_markdown:
        user_prompt = (
            "Based on the approved plan and workspace context, generate the pipeline execution steps JSON.\n\n"
            "[APPROVED INTEGRATION PLAN]\n"
            f"{body.approved_markdown}\n\n"
            "[WORKSPACE CONTEXT]\n"
            f"{context_plan}\n"
        )
    else:
        history = await fetch_chat_history(client, body.workspace_id)
        claude_messages = [
            {"role": "user" if m["sender"] == "user" else "assistant", "content": m["content"]}
            for m in history
        ]
        user_prompt = (
            "Based on the conversation history and workspace context, generate the pipeline execution steps JSON.\n\n"
            "[WORKSPACE CONTEXT]\n"
            f"{context_plan}\n\n"
            "[CONVERSATION HISTORY]\n"
            f"{json.dumps(claude_messages, indent=2)}\n"
        )

    if rag_section:
        user_prompt += f"\n\n{rag_section}"

    # Phase C4 — reuse-first: surface an existing graph capability that already
    # covers this goal so the planner prefers reusing it over re-deriving steps.
    goal_text = body.message or body.approved_markdown or ""
    if goal_text:
        try:
            from fluvio_planner.capabilities.resolver import find_reusable_capability
            cap = await find_reusable_capability(client, goal_text)
            if cap:
                user_prompt += (
                    "\n\n[REUSABLE CAPABILITY — prefer this over re-deriving]\n"
                    f"A synthesized capability already covers this goal: "
                    f"`{cap['name']}` ({cap.get('signature', 'run(args)')}), "
                    f"semantic match {cap['score']:.2f}. "
                    f"Tools it uses: {cap.get('mcp_tools') or 'none'}."
                )
        except Exception as exc:
            logger.warning("capability reuse hint skipped: %s", exc)

    user_prompt += "\nOutput ONLY the JSON array. Do not put backticks around it."

    # Resolve a Postgres URL for a SQL step's credential_ref, mirroring how the
    # worker builds it at execution time. Used for real EXPLAIN validation below.
    def _resolve_db_url(step: dict) -> str | None:
        token = step.get("credential_ref")
        if not token:
            return None
        try:
            ref = CredentialRef(ref=token)
            creds = resolve_credentials(ref, cfg.connector_configs)
        except Exception:
            return None
        if ref.connector_kind in ("postgres", "postgresql", "database", "mysql") and creds:
            return (
                f"postgresql://{creds.get('username','')}:{creds.get('password','')}@"
                f"{creds.get('host','localhost')}:{creds.get('port', 5432)}/{creds.get('database','')}"
            )
        return None

    # Validate-and-retry loop — all validation goes through the live toolbox registry
    messages: list[dict] = [{"role": "user", "content": user_prompt}]
    steps: list[dict] = []
    last_error: str = ""

    for attempt in range(1, 4):  # max 3 attempts
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
                        "messages": messages,
                    },
                    timeout=90.0,
                )
                if resp.status_code != 200:
                    raise HTTPException(status_code=502, detail=f"Anthropic API error: {resp.text}")
                raw_text = resp.json()["content"][0]["text"]
        except HTTPException:
            raise
        except Exception as exc:
            raise HTTPException(status_code=500, detail=f"Error calling Anthropic API: {exc}")

        # Parse JSON
        try:
            raw_steps = _parse_claude_json(raw_text)
        except Exception as exc:
            last_error = f"JSON parse error: {exc}"
            logger.warning("Attempt %d: unparseable JSON — %s", attempt, last_error)
            if attempt < 3:
                messages.append({"role": "assistant", "content": raw_text})
                messages.append({"role": "user", "content": f"Your output was not valid JSON: {exc}. Output ONLY a JSON array."})
            continue

        # Validate against the live manifest registry
        validated, errors = registry.validate_steps(raw_steps)
        if errors:
            last_error = "; ".join(errors)
            logger.warning("Attempt %d: toolbox validation failed — %s", attempt, last_error)
            if attempt < 3:
                messages.append({"role": "assistant", "content": raw_text})
                messages.append({"role": "user", "content": registry.build_retry_prompt(errors)})
            continue

        # Real EXPLAIN validation — catch semantic SQL errors (missing joins,
        # wrong table for a column, etc.) now and auto-repair, instead of letting
        # them fail at execution time as a "permanent error."
        explain_errors: list[str] = []
        for i, step in enumerate(validated):
            if step.get("tool_id") not in ("spark", "dbt"):
                continue
            sql = (step.get("arguments") or {}).get("query") or (step.get("arguments") or {}).get("sql")
            if not sql:
                continue
            db_url = _resolve_db_url(step)
            if not db_url:
                continue  # can't EXPLAIN without a DB — fall back to static checks
            ok, msg = await explain_sql(sql, db_url)
            if not ok:
                explain_errors.append(f"Step {i + 1} SQL is invalid (EXPLAIN failed): {msg}")

        if explain_errors:
            last_error = "; ".join(explain_errors)
            logger.warning("Attempt %d: EXPLAIN validation failed — %s", attempt, last_error)
            if attempt < 3:
                messages.append({"role": "assistant", "content": raw_text})
                messages.append({"role": "user", "content": (
                    "The following SQL failed real database EXPLAIN validation. The "
                    "error comes directly from the database. Fix the queries so every "
                    "referenced column resolves — consult the Exact Database Schema "
                    "above to see which table owns each column, and join the tables "
                    "needed to bring those columns into scope. Re-output ONLY the "
                    "corrected JSON array.\n\n"
                    + last_error
                )})
            continue

        steps = validated
        break

    if not steps:
        raise HTTPException(
            status_code=502,
            detail=f"Claude could not produce valid steps after 3 attempts. Last error: {last_error}",
        )

    # Phase 23 — warn on suspicious SQL column references (non-blocking)
    if db_schema:
        for i, step in enumerate(steps):
            sql = (step.get("arguments") or {}).get("query")
            if sql and step.get("tool_id") in ("spark", "dbt"):
                from fluvio_planner.schema_inspector import validate_sql_columns
                source_tables = list(db_schema.keys())
                warnings = validate_sql_columns(sql, db_schema, source_tables)
                if warnings:
                    logger.warning("Step %d SQL warnings: %s", i + 1, "; ".join(warnings))
                    step.setdefault("_warnings", []).extend(warnings)

    logger.info("Compiled %d steps for workspace %s", len(steps), body.workspace_id)
    return CompileResponse(steps=steps)


@router.post("/plan/context", response_model=PlanContextResponse)
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
