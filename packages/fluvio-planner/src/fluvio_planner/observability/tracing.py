"""Trace-ID propagation via contextvars.

A trace_id is generated (or extracted from incoming headers) at the edge of
every request and stored in a ContextVar so any code in the call stack can
read it without threading it through function signatures.

FastAPI middleware sets it per request. The job worker sets it per job.

Usage:
    trace_id = current_trace_id()  # read anywhere in the stack
    set_trace_id("abc-123")        # called by middleware / worker
    new_trace_id()                 # generate a fresh one
"""

from __future__ import annotations

import uuid
from contextvars import ContextVar

_trace_id_var: ContextVar[str] = ContextVar("trace_id", default="")


def new_trace_id() -> str:
    tid = uuid.uuid4().hex[:16]
    _trace_id_var.set(tid)
    return tid


def set_trace_id(trace_id: str) -> None:
    _trace_id_var.set(trace_id)


def current_trace_id() -> str:
    return _trace_id_var.get()
