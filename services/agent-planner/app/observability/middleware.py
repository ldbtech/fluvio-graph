"""FastAPI middleware for trace-ID propagation and structured request logging."""

from __future__ import annotations

import time
from typing import Callable

from fastapi import Request, Response
from starlette.middleware.base import BaseHTTPMiddleware

from app.observability.logging import get_logger
from app.observability.tracing import current_trace_id, new_trace_id, set_trace_id

logger = get_logger("agent-planner.http")

_TRACE_HEADER = "x-trace-id"


class TracingMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next: Callable) -> Response:
        # Honour upstream trace-id if present (e.g. from an API gateway)
        incoming = request.headers.get(_TRACE_HEADER)
        if incoming:
            set_trace_id(incoming)
        else:
            new_trace_id()

        trace_id = current_trace_id()
        start = time.monotonic()
        response = await call_next(request)
        duration_ms = int((time.monotonic() - start) * 1000)

        logger.info(
            "%s %s %d",
            request.method,
            request.url.path,
            response.status_code,
            extra={
                "method": request.method,
                "path": request.url.path,
                "status": response.status_code,
                "duration_ms": duration_ms,
            },
        )

        response.headers[_TRACE_HEADER] = trace_id
        return response
