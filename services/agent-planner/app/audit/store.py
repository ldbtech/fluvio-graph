"""In-memory audit store.

Interface mirrors what a Postgres implementation would look like — every
method is async and takes the same arguments. To swap to Postgres:
1. Implement the same async methods using asyncpg.
2. Replace `audit_store = InMemoryAuditStore()` at the bottom.

Postgres DDL:
    CREATE TABLE deployment_runs (
        id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'running',
        step_count INT NOT NULL DEFAULT 0,
        failed_step INT,
        started_at TIMESTAMPTZ DEFAULT now(),
        finished_at TIMESTAMPTZ
    );
    CREATE TABLE deployment_steps (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES deployment_runs(id),
        step_index INT NOT NULL,
        tool_id TEXT NOT NULL,
        action TEXT NOT NULL,
        status TEXT NOT NULL,
        duration_ms INT NOT NULL DEFAULT 0,
        error TEXT,
        created_at TIMESTAMPTZ DEFAULT now()
    );
    CREATE INDEX ON deployment_steps(run_id);
    CREATE INDEX ON deployment_runs(workspace_id);
"""

from __future__ import annotations

import time
from typing import Any

from app.audit.models import DeploymentRun, DeploymentStep


class InMemoryAuditStore:
    def __init__(self) -> None:
        self._runs: dict[str, DeploymentRun] = {}

    async def start_run(self, run_id: str, workspace_id: str, step_count: int) -> None:
        self._runs[run_id] = DeploymentRun(
            run_id=run_id,
            workspace_id=workspace_id,
            status="running",
            step_count=step_count,
        )

    async def finish_run(
        self,
        run_id: str,
        status: str,
        failed_step: int | None = None,
    ) -> None:
        run = self._runs.get(run_id)
        if run:
            run.status = status
            run.failed_step = failed_step
            run.finished_at = time.monotonic()

    async def record_step(
        self,
        run_id: str,
        step_index: int,
        tool_id: str,
        action: str,
        status: str,
        duration_ms: int = 0,
        error: str | None = None,
    ) -> None:
        run = self._runs.get(run_id)
        if run:
            run.steps.append(DeploymentStep(
                run_id=run_id,
                step_index=step_index,
                tool_id=tool_id,
                action=action,
                status=status,
                duration_ms=duration_ms,
                error=error,
            ))

    async def get_run(self, run_id: str) -> dict[str, Any] | None:
        run = self._runs.get(run_id)
        return run.to_dict() if run else None

    async def list_runs(self, workspace_id: str) -> list[dict[str, Any]]:
        return [
            r.to_dict()
            for r in self._runs.values()
            if r.workspace_id == workspace_id
        ]


# Module-level singleton — import this everywhere
audit_store = InMemoryAuditStore()
