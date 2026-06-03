"""Per-tool circuit breaker with sliding-window failure counting.

Each tool_id gets its own breaker. State transitions:
    CLOSED  → (failures >= threshold in window) → OPEN
    OPEN    → (reset_timeout elapsed) → HALF_OPEN
    HALF_OPEN → (success) → CLOSED | (failure) → OPEN

Usage:
    breaker = get_breaker("spark")
    async with breaker:
        result = await call_tool(...)
"""

from __future__ import annotations

import asyncio
import time
from collections import deque
from contextlib import asynccontextmanager
from enum import Enum, auto
from typing import AsyncIterator

from app.reliability.errors import CircuitOpenError

_registry: dict[str, "CircuitBreaker"] = {}


class State(Enum):
    CLOSED = auto()
    OPEN = auto()
    HALF_OPEN = auto()


class CircuitBreaker:
    def __init__(
        self,
        tool_id: str,
        failure_threshold: int = 5,
        window_seconds: float = 60.0,
        reset_timeout: float = 30.0,
    ) -> None:
        self.tool_id = tool_id
        self.failure_threshold = failure_threshold
        self.window_seconds = window_seconds
        self.reset_timeout = reset_timeout

        self._state = State.CLOSED
        self._failure_times: deque[float] = deque()
        self._opened_at: float = 0.0
        self._lock = asyncio.Lock()

    @property
    def state(self) -> State:
        return self._state

    def _prune_window(self) -> None:
        cutoff = time.monotonic() - self.window_seconds
        while self._failure_times and self._failure_times[0] < cutoff:
            self._failure_times.popleft()

    def _record_failure(self) -> None:
        self._failure_times.append(time.monotonic())
        self._prune_window()
        if len(self._failure_times) >= self.failure_threshold:
            self._state = State.OPEN
            self._opened_at = time.monotonic()

    def _record_success(self) -> None:
        self._failure_times.clear()
        self._state = State.CLOSED

    def _check(self) -> None:
        if self._state == State.CLOSED:
            return
        if self._state == State.OPEN:
            elapsed = time.monotonic() - self._opened_at
            if elapsed >= self.reset_timeout:
                self._state = State.HALF_OPEN
                return
            raise CircuitOpenError(
                f"Circuit for '{self.tool_id}' is OPEN "
                f"({len(self._failure_times)} failures in {self.window_seconds}s window). "
                f"Resets in {self.reset_timeout - elapsed:.0f}s.",
                tool_id=self.tool_id,
            )
        # HALF_OPEN — allow the probe through

    @asynccontextmanager
    async def __call__(self) -> AsyncIterator[None]:
        async with self._lock:
            self._check()
        try:
            yield
            async with self._lock:
                self._record_success()
        except Exception:
            async with self._lock:
                self._record_failure()
            raise


def get_breaker(tool_id: str) -> CircuitBreaker:
    """Return the shared CircuitBreaker instance for tool_id, creating it if needed."""
    if tool_id not in _registry:
        _registry[tool_id] = CircuitBreaker(tool_id)
    return _registry[tool_id]


def breaker_status() -> dict[str, str]:
    """Return a snapshot of all breaker states — useful for /health detail."""
    return {tid: cb.state.name for tid, cb in _registry.items()}
