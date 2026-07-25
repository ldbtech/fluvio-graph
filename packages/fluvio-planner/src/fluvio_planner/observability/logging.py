"""Structured JSON logging.

Replaces the plain-text basicConfig with a JSON formatter that emits:
    {"time": "...", "level": "INFO", "service": "agent-planner",
     "trace_id": "abc", "message": "...", ...extra}

Usage:
    from fluvio_planner.observability.logging import get_logger
    logger = get_logger(__name__)
    logger.info("step completed", extra={"tool_id": "spark", "duration_ms": 340})

The trace_id is injected automatically from the ContextVar — no need to
pass it through every call site.

To plug in OpenTelemetry later, replace StructuredFormatter with the
OTel LoggingHandler and set the exporter in configure_logging().
"""

from __future__ import annotations

import json
import logging
import time
from typing import Any

from fluvio_planner.observability.tracing import current_trace_id

SERVICE_NAME = "agent-planner"


class StructuredFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        payload: dict[str, Any] = {
            "time": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(record.created)),
            "level": record.levelname,
            "service": SERVICE_NAME,
            "trace_id": current_trace_id() or "",
            "logger": record.name,
            "message": record.getMessage(),
        }
        # Merge any extra fields passed via logger.info("msg", extra={...})
        for key, val in record.__dict__.items():
            if key not in logging.LogRecord.__dict__ and not key.startswith("_"):
                payload[key] = val
        if record.exc_info:
            payload["exc_info"] = self.formatException(record.exc_info)
        return json.dumps(payload)


def configure_logging(level: int = logging.INFO) -> None:
    handler = logging.StreamHandler()
    handler.setFormatter(StructuredFormatter())
    root = logging.getLogger()
    root.handlers.clear()
    root.addHandler(handler)
    root.setLevel(level)


def get_logger(name: str) -> logging.Logger:
    return logging.getLogger(name)
