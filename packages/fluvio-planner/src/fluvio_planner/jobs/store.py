"""In-memory job store.

Interface is intentionally simple so a Redis-backed implementation can be
dropped in by replacing this module — the worker and router never import
storage internals directly.
"""

from __future__ import annotations

import asyncio
from typing import Any

from fluvio_planner.jobs.models import JobRecord

# job_id → JobRecord
_jobs: dict[str, JobRecord] = {}

# Pending jobs waiting to be picked up by the worker loop
_queue: asyncio.Queue[str] = asyncio.Queue()


def enqueue(record: JobRecord) -> None:
    _jobs[record.job_id] = record
    _queue.put_nowait(record.job_id)


def get_job(job_id: str) -> JobRecord | None:
    return _jobs.get(job_id)


def list_jobs(workspace_id: str) -> list[JobRecord]:
    return [j for j in _jobs.values() if j.workspace_id == workspace_id]


async def next_job() -> JobRecord:
    """Block until a job is available, then return it."""
    while True:
        job_id = await _queue.get()
        job = _jobs.get(job_id)
        if job is not None:
            return job
