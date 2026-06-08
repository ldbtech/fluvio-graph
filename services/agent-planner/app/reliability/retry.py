"""Async retry with exponential backoff.

Usage:
    result = await with_retry(lambda: my_coro(), attempts=3, base_delay=1.0)

Pass a *factory* (a zero-arg callable that returns a fresh coroutine), NOT a
coroutine object — each attempt must create a new coroutine, otherwise re-awaiting
the same one raises "cannot reuse already awaited coroutine".

Only TransientToolError is retried. PermanentToolError and CircuitOpenError
propagate immediately.
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Awaitable
from typing import Callable, TypeVar, Union

from app.reliability.errors import PermanentToolError, CircuitOpenError, TransientToolError

logger = logging.getLogger("agent-planner")

T = TypeVar("T")


async def with_retry(
    coro_factory: Union[Callable[[], Awaitable[T]], Awaitable[T]],
    *,
    attempts: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    label: str = "",
) -> T:
    """Run a coroutine factory up to `attempts` times, backing off exponentially.

    `coro_factory` should be a zero-arg callable returning a fresh awaitable; a
    new coroutine is created for every attempt so retries are safe. For backward
    compatibility a bare coroutine is accepted but can only be awaited once (no
    real retry possible) and logs a warning.

    Raises the final exception if all attempts are exhausted.
    PermanentToolError and CircuitOpenError are re-raised immediately without retrying.
    """
    is_factory = callable(coro_factory)
    if not is_factory:
        logger.warning(
            "with_retry received a coroutine object, not a factory — retries for "
            "'%s' are disabled. Pass a zero-arg callable instead.", label or "step",
        )

    def _make() -> Awaitable[T]:
        return coro_factory() if is_factory else coro_factory  # type: ignore[operator,return-value]

    last_exc: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            return await _make()  # type: ignore[return-value]
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
