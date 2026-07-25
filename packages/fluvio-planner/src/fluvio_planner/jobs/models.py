"""Job state models for the async deploy queue."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class JobStatus(str, Enum):
    QUEUED = "queued"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class JobRecord:
    job_id: str
    workspace_id: str
    user_id: str
    sandbox_id: str
    steps: list[dict[str, Any]]

    status: JobStatus = JobStatus.QUEUED
    progress: int = 0          # steps completed so far
    total: int = 0             # total steps
    error: str | None = None
    created_at: float = field(default_factory=time.monotonic)
    started_at: float | None = None
    finished_at: float | None = None

    # SSE subscribers: asyncio queues waiting for log lines
    _subscribers: list[Any] = field(default_factory=list, repr=False, compare=False)

    def subscribe(self) -> Any:
        import asyncio
        q: asyncio.Queue[str | None] = asyncio.Queue()
        self._subscribers.append(q)
        return q

    def unsubscribe(self, q: Any) -> None:
        try:
            self._subscribers.remove(q)
        except ValueError:
            pass

    def emit(self, line: str) -> None:
        for q in list(self._subscribers):
            q.put_nowait(line)

    def close_streams(self) -> None:
        """Signal None sentinel to all subscribers so they know the stream ended."""
        for q in list(self._subscribers):
            q.put_nowait(None)
        self._subscribers.clear()
