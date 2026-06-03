"""Audit trail data models.

Maps to the Postgres schema:
    deployment_runs  (id, workspace_id, job_id, status, started_at, finished_at, step_count, failed_step)
    deployment_steps (id, run_id, step_index, tool_id, action, status, duration_ms, error, created_at)
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any


@dataclass
class DeploymentStep:
    run_id: str
    step_index: int
    tool_id: str
    action: str
    status: str
    duration_ms: int = 0
    error: str | None = None
    created_at: float = field(default_factory=time.monotonic)


@dataclass
class DeploymentRun:
    run_id: str
    workspace_id: str
    status: str = "running"
    step_count: int = 0
    failed_step: int | None = None
    started_at: float = field(default_factory=time.monotonic)
    finished_at: float | None = None
    steps: list[DeploymentStep] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "workspace_id": self.workspace_id,
            "status": self.status,
            "step_count": self.step_count,
            "failed_step": self.failed_step,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "steps": [
                {
                    "step_index": s.step_index,
                    "tool_id": s.tool_id,
                    "action": s.action,
                    "status": s.status,
                    "duration_ms": s.duration_ms,
                    "error": s.error,
                }
                for s in self.steps
            ],
        }
