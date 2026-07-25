"""Deploy job worker.

Runs as a background asyncio task started during app lifespan.
Pulls JobRecords from the store and executes them using the same logic
as the old synchronous /deploy handler, with:
  - retry + circuit breaker (Phase 7)
  - idempotency (Phase 9)
  - audit trail (Phase 11)
  - Phase 19: stores successful deployment summaries into the knowledge graph
  - Phase 23: runs EXPLAIN validation on SQL steps before execution
"""

from __future__ import annotations

import asyncio
import json
import logging
import time

from fluvio_planner.audit.store import audit_store
from fluvio_planner.fetch import add_chat_message
from fluvio_planner.gateway_client.client import FederationClient
from fluvio_planner.planner_config import PlannerConfig
from fluvio_planner.idempotency import IdempotencyStore
from fluvio_planner.jobs.models import JobRecord, JobStatus
from fluvio_planner.jobs.store import next_job
from fluvio_planner.observability.tracing import new_trace_id, set_trace_id
from fluvio_planner.reliability.circuit_breaker import get_breaker
from fluvio_planner.reliability.errors import (
    CircuitOpenError,
    PermanentToolError,
    TransientToolError,
    classify_tool_error,
)
from fluvio_planner.reliability.retry import with_retry
from fluvio_planner.rollback import RollbackRegistry
from fluvio_planner.workspace_config import build_environment_context, resolve_workspace_config
from fluvio_planner.credential_vault import CredentialRef, resolve_credentials

logger = logging.getLogger("agent-planner")

_EXECUTE_MUTATION = """
mutation ExecuteTool($toolId: String!, $inputs: String!) {
  executeTool(toolId: $toolId, inputs: $inputs) {
    id
    status
    output
    logs
  }
}
"""

idempotency = IdempotencyStore()


async def _invoke_via_mcp(tool_id: str, action: str, arguments: dict, mcp_server_url: str) -> dict:
    """Invoke a tool through the MCP server (Phase M3). Returns a tool_run-shaped
    dict {status, output, logs}. Raises on transport failure (→ legacy fallback)."""
    from fluvio_planner.capabilities.mcp_client import call_tool as mcp_call
    name = f"{tool_id.replace('-', '_')}__{action}"
    payload = await mcp_call(name, arguments, mcp_server_url)  # raises if MCP unreachable
    inner = (payload or {}).get("result") or {}
    if inner.get("status") == "success":
        return {
            "status": "success",
            "output": json.dumps(inner.get("result", {})),
            "logs":   "\n".join((payload or {}).get("logs", [])),
        }
    return {"status": "failed", "output": inner.get("error", "MCP tool reported failure")}


async def _invoke_via_graphql(client: FederationClient, tool_id: str, action: str, arguments: dict) -> dict:
    """Legacy path — the double-JSON-encoded executeTool mutation."""
    inputs_str = json.dumps({"action": action, "arguments": json.dumps(arguments)})
    resp = await client.query(_EXECUTE_MUTATION, variables={"toolId": tool_id, "inputs": inputs_str})
    data = resp.get("data") or resp
    return data.get("executeTool") or {}


async def _call_tool(
    client: FederationClient,
    tool_id: str,
    action: str,
    arguments: dict,
    mcp_server_url: str,
) -> dict:
    """Single attempt to run a tool. Phase M3: prefer typed MCP `tools/call`;
    fall back to the legacy `executeTool` mutation only if the MCP server is
    unreachable. A tool that *runs and fails* does NOT fall back (no double-run).
    Raises ToolError on tool failure. Retry / circuit-breaker wrap this unchanged."""
    try:
        tool_run = await _invoke_via_mcp(tool_id, action, arguments, mcp_server_url)
    except Exception as exc:
        logger.info("MCP call unavailable (%s) — falling back to executeTool", exc)
        tool_run = await _invoke_via_graphql(client, tool_id, action, arguments)

    status = tool_run.get("status", "")
    if status not in ("completed", "success"):
        raise classify_tool_error(tool_id, action, status, tool_run.get("output", ""))
    return tool_run


async def _execute_step_with_reliability(
    client: FederationClient,
    job: JobRecord,
    step_index: int,
    step: dict,
    connector_configs: dict,
    rollback: RollbackRegistry,
    mcp_server_url: str,
) -> dict:
    """Execute one step with retry, circuit breaker, and idempotency checks."""
    tool_id: str = step["tool_id"]
    action: str = step["action"]
    arguments: dict = dict(step.get("arguments") or {})

    # Resolve credential_ref at execution time
    ref_token = step.get("credential_ref")
    if ref_token:
        ref = CredentialRef(ref=ref_token)
        creds = resolve_credentials(ref, connector_configs)
        kind = ref.connector_kind
        context = dict(arguments.get("context") or {})
        if kind in ("postgres", "postgresql", "database", "mysql") and creds:
            context["database_url"] = (
                f"postgres://{creds.get('username','')}:{creds.get('password','')}@"
                f"{creds.get('host','localhost')}:{creds.get('port', 5432)}/{creds.get('database','')}"
            )
        elif kind == "tableau" and creds:
            context.update({
                "tableau_token_name": creds.get("tableauTokenName", ""),
                "tableau_token_value": creds.get("tableauTokenValue", ""),
                "tableau_server_url": creds.get("tableauServerUrl", ""),
                "workspace_id": creds.get("tableauWorkspaceId", ""),
                "platform": "tableau",
            })
        elif kind == "powerbi" and creds:
            context.update({
                "tenant_id": creds.get("tenantId", ""),
                "client_id": creds.get("clientId", ""),
                "client_secret": creds.get("clientSecret", ""),
                "workspace_id": creds.get("powerbiWorkspaceId", ""),
                "platform": "powerbi",
            })
        if context:
            arguments["context"] = context

    # Inject the requesting user's identity so the email tool can resolve "me"
    # without the planner ever handling the address.
    if tool_id == "email-sender":
        context = dict(arguments.get("context") or {})
        if not context.get("user_email"):
            try:
                data = await client.query(
                    "query($id: String!){ getUser(id: $id){ email companyEmail } }",
                    variables={"id": job.user_id},
                )
                u = ((data.get("data") or data).get("getUser")) or {}
                if u.get("email"):
                    context["user_email"] = u["email"]
                if u.get("companyEmail"):
                    context["company_email"] = u["companyEmail"]
            except Exception as exc:
                logger.warning("Could not resolve user email for 'me' recipient: %s", exc)
        arguments["context"] = context

    idem_key = idempotency.key(job.job_id, step_index)
    if await idempotency.already_done(idem_key):
        logger.info("Step %d already completed (idempotency hit) — skipping", step_index)
        return {"status": "completed", "output": "{}", "skipped": True}

    # Phase 23 — EXPLAIN validation for SQL steps before execution
    if tool_id in ("spark", "dbt") and action in ("execute_sql", "run"):
        sql = arguments.get("query") or arguments.get("sql")
        db_url = (arguments.get("context") or {}).get("database_url")
        if sql and db_url:
            from fluvio_planner.schema_inspector import explain_sql
            ok, explain_msg = await explain_sql(sql, db_url)
            if not ok:
                raise PermanentToolError(
                    f"SQL EXPLAIN failed (step {step_index}): {explain_msg}",
                    tool_id=tool_id,
                    action=action,
                )
            logger.info("Step %d EXPLAIN OK: %s", step_index, explain_msg)

    breaker = get_breaker(tool_id)
    start = time.monotonic()

    async def attempt():
        async with breaker():
            return await _call_tool(client, tool_id, action, arguments, mcp_server_url)

    tool_run = await with_retry(
        attempt,  # factory: a fresh coroutine is created per attempt
        attempts=3,
        base_delay=1.0,
        label=f"{tool_id}/{action}",
    )

    duration_ms = int((time.monotonic() - start) * 1000)
    await idempotency.mark_done(idem_key)

    # Register a compensating action for mutable steps so rollback can unwind them
    if tool_id in ("dashboard-syncer",) and action in ("publish_report",):
        try:
            report_id = json.loads(tool_run.get("output") or "{}").get("report_id", "")
        except Exception:
            report_id = ""
        rollback.register(step_index, tool_id, "delete_report", {"report_id": report_id})

    await audit_store.record_step(
        run_id=job.job_id,
        step_index=step_index,
        tool_id=tool_id,
        action=action,
        status="completed",
        duration_ms=duration_ms,
    )
    return tool_run


async def execute_job(job: JobRecord, cfg: PlannerConfig) -> None:
    """Main entry point called by the worker loop for a single job.

    `cfg` is injected by the worker loop (ultimately the composition root in
    main.py) so this module reads no config singleton."""
    trace_id = new_trace_id()
    set_trace_id(trace_id)
    job.status = JobStatus.RUNNING
    job.started_at = time.monotonic()
    job.total = len(job.steps)

    await audit_store.start_run(run_id=job.job_id, workspace_id=job.workspace_id, step_count=job.total)

    client = FederationClient(cfg.graphql_gateway_url, headers={
        "x-user-id": job.user_id,
        "x-trace-id": trace_id,
    })

    cfg = await resolve_workspace_config(
        client=client,
        workspace_id=job.workspace_id,
        sandbox_id=job.sandbox_id,
    )
    connector_configs = cfg.connector_configs
    rollback = RollbackRegistry()
    logs: list[str] = [f"# Pipeline job {job.job_id}\nTrace: `{trace_id}`\nSandbox: **{job.sandbox_id}**\n"]

    def emit(line: str) -> None:
        logs.append(line)
        job.emit(line)

    failed_step: int | None = None

    for i, step in enumerate(job.steps):
        description = step.get("description") or f"{step.get('tool_id')}/{step.get('action')}"
        emit(f"### [{i+1}/{job.total}] {description}...")

        try:
            tool_run = await _execute_step_with_reliability(
                client, job, i, step, connector_configs, rollback, cfg.mcp_server_url
            )
            if tool_run.get("skipped"):
                emit(f"⏭️ Skipped (already completed).\n")
            else:
                _append_step_output(emit, step, tool_run)
            job.progress = i + 1

        except CircuitOpenError as exc:
            emit(f"⚡ **Circuit open for `{step.get('tool_id')}`** — skipping step: {exc}\n")
            await audit_store.record_step(
                run_id=job.job_id, step_index=i,
                tool_id=step.get("tool_id", ""), action=step.get("action", ""),
                status="skipped_circuit_open", duration_ms=0,
            )
            # Circuit open = skip the step, continue pipeline
            job.progress = i + 1
            continue

        except PermanentToolError as exc:
            emit(f"❌ **Permanent error at step {i+1}** (`{step.get('tool_id')}`): {exc}\n")
            await audit_store.record_step(
                run_id=job.job_id, step_index=i,
                tool_id=step.get("tool_id", ""), action=step.get("action", ""),
                status="failed", duration_ms=0, error=str(exc),
            )
            failed_step = i
            break

        except Exception as exc:
            emit(f"❌ **Error at step {i+1}** (`{step.get('tool_id')}`): {exc}\n")
            await audit_store.record_step(
                run_id=job.job_id, step_index=i,
                tool_id=step.get("tool_id", ""), action=step.get("action", ""),
                status="failed", duration_ms=0, error=str(exc),
            )
            failed_step = i
            break

    if failed_step is not None:
        job.status = JobStatus.FAILED
        job.error = f"Failed at step {failed_step + 1}"
        job.finished_at = time.monotonic()

        # Attempt rollback of completed mutable steps
        if rollback.has_actions():
            emit("\n### Rolling back completed steps...\n")
            await _run_rollback(client, rollback, emit, cfg.mcp_server_url)

        await audit_store.finish_run(job.job_id, status="failed", failed_step=failed_step)
    else:
        job.status = JobStatus.COMPLETED
        job.finished_at = time.monotonic()
        emit(f"\n# ✅ Pipeline complete — {job.total} steps executed.\n")
        await audit_store.finish_run(job.job_id, status="completed")

        # Phase 19 — Store successful deployment in the knowledge graph for future RAG
        try:
            from fluvio_planner.memory.store import store_deployment_summary
            run_record = await audit_store.get_run(job.job_id)
            if run_record:
                from fluvio_planner.audit.models import DeploymentRun, DeploymentStep
                # Reconstruct minimal DeploymentRun for storage
                dr = DeploymentRun(
                    run_id=run_record["run_id"],
                    workspace_id=run_record["workspace_id"],
                    status=run_record["status"],
                    step_count=run_record["step_count"],
                )
                for s in run_record.get("steps", []):
                    dr.steps.append(DeploymentStep(**{k: v for k, v in s.items() if k != "created_at"}, run_id=dr.run_id, created_at=0))
                await store_deployment_summary(client, dr)
        except Exception as exc:
            logger.warning("Failed to store deployment memory (non-fatal): %s", exc)

    result = "\n".join(logs)
    try:
        await add_chat_message(client, workspace_id=job.workspace_id, sender="ai", content=result)
    except Exception as exc:
        logger.error("Failed to save job logs to chat history: %s", exc)

    job.close_streams()


async def _run_rollback(client: FederationClient, rollback: RollbackRegistry, emit, mcp_server_url: str) -> None:
    for entry in reversed(rollback.actions):
        try:
            await _call_tool(client, entry["tool_id"], entry["action"], entry["arguments"], mcp_server_url)
            emit(f"↩️ Rolled back step {entry['step_index']}: `{entry['tool_id']}/{entry['action']}`\n")
        except Exception as exc:
            emit(f"⚠️ Rollback failed for step {entry['step_index']}: {exc}\n")


def _append_step_output(emit, step: dict, tool_run: dict) -> None:
    tool_id = step.get("tool_id")
    action = step.get("action")
    try:
        out = json.loads(tool_run.get("output") or "{}")
    except Exception:
        out = {}
    if tool_id == "dashboard-syncer" and action == "publish_report":
        url = out.get("web_url") or out.get("report_id", "")
        emit(f"✅ Dashboard published. URL: {url}\n")
    elif tool_id == "dashboard-syncer" and action == "generate_pdf_report":
        emit(f"✅ PDF report generated. URL: {out.get('web_url', '')}\n")
    elif tool_id == "email-sender":
        rcpts = ", ".join(out.get("recipients") or [])
        atts = ", ".join(out.get("attachments") or [])
        extra = f" (attached: {atts})" if atts else ""
        emit(f"📧 Email sent to {rcpts}{extra}.\n")
    else:
        emit("✅ Completed.\n")


async def worker_loop(cfg: PlannerConfig) -> None:
    """Background task — runs forever, consuming jobs from the queue.

    `cfg` is injected by the composition root (main.py) and threaded into each
    job, so nothing under jobs/ reads config from the environment."""
    logger.info("Deploy worker started")
    while True:
        job = await next_job()
        try:
            await execute_job(job, cfg)
        except Exception as exc:
            logger.exception("Unhandled error in job %s: %s", job.job_id, exc)
            job.status = JobStatus.FAILED
            job.error = str(exc)
            job.close_streams()
