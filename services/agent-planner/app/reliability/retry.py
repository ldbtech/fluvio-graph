"""Async retry with exponential backoff.

Usage:
    result = await with_retry(my_coro(), attempts=3, base_delay=1.0)

Only TransientToolError is retried. PermanentToolError and CircuitOpenError
propagate immediately.
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Awaitable
from typing import TypeVar

from app.reliability.errors import PermanentToolError, CircuitOpenError, TransientToolError

logger = logging.getLogger("agent-planner")

T = TypeVar("T")


async def with_retry(
    coro: Awaitable[T],
    *,
    attempts: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    label: str = "",
) -> T:
    """Await coro up to `attempts` times, backing off exponentially on TransientToolError.

    Raises the final exception if all attempts are exhausted.
    PermanentToolError and CircuitOpenError are re-raised immediately without retrying.
    """
    last_exc: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            return await coro  # type: ignore[return-value]
        except (PermanentToolError, CircuitOpenError):
            raise
        except TransientToolError as exc:
            last_exc = exc
            if attempt == attempts:
                break
            delay = min(base_delay * (2 ** (attempt - 1)), max_delay)
            logger.warning(
                "Transient error on attempt %d/%d for %s — retrying in %.1fs: %s",
                attempt, attempts, label or "step", delay, exc,
            )
            await asyncio.sleep(delay)
        except Exception as exc:
            # Treat unknown exceptions as transient by default
            last_exc = exc
            if attempt == attempts:
                break
            delay = min(base_delay * (2 ** (attempt - 1)), max_delay)
            logger.warning(
                "Unknown error on attempt %d/%d for %s — retrying in %.1fs: %s",
                attempt, attempts, label or "step", delay, exc,
            )
            await asyncio.sleep(delay)

    raise last_exc  # type: ignore[misc]
