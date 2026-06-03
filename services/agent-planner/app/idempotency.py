"""Idempotency store for pipeline step execution.

Each step gets a key derived from (job_id, step_index). Before executing,
the worker checks this store. If the key is present, the step was already
completed in a prior attempt and is skipped.

The in-memory implementation is correct for single-process deployments.
Swap `IdempotencyStore` for a Postgres-backed version to survive restarts:
    CREATE TABLE completed_steps (
        key TEXT PRIMARY KEY,
        completed_at TIMESTAMPTZ DEFAULT now()
    );
"""

from __future__ import annotations

import hashlib


class IdempotencyStore:
    def __init__(self) -> None:
        self._done: set[str] = set()

    @staticmethod
    def key(job_id: str, step_index: int) -> str:
        raw = f"{job_id}:{step_index}"
        return hashlib.sha256(raw.encode()).hexdigest()[:32]

    async def already_done(self, key: str) -> bool:
        return key in self._done

    async def mark_done(self, key: str) -> None:
        self._done.add(key)

    async def clear_job(self, job_id: str) -> None:
        """Remove all step keys for a job (e.g. when retrying from scratch)."""
        prefix_candidates = set()
        for i in range(10_000):
            prefix_candidates.add(self.key(job_id, i))
        self._done -= prefix_candidates
