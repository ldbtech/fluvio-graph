"""MCP tool discovery + JSON Schema generation (Phase M1).

Maps the existing tool layer onto MCP's flat tool model:

  one MCP tool  ==  one (tool_id, action) pair

The action set is the public async methods on each `<Tool>Runtime` class. Each
action's `inputSchema` is generated from the method signature — Pydantic-typed
parameters become nested object schemas via `model_json_schema()`, scalars are
mapped to their JSON Schema type. No schemas are written by hand.

This module ONLY reads existing tool code; it changes nothing about execution.
"""

from __future__ import annotations

import inspect
import json
import logging
import os
import typing
from dataclasses import dataclass
from importlib import import_module
from typing import Any

from pydantic import BaseModel

logger = logging.getLogger("mcp-schema")

_TOOLS_PKG = "src.tools"
_TOOLS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tools")


@dataclass(frozen=True)
class McpToolSpec:
    mcp_name: str          # e.g. "dashboard_syncer__publish_report"
    tool_id: str           # original manifest id, e.g. "dashboard-syncer"
    action: str            # runtime method name, e.g. "publish_report"
    description: str
    input_schema: dict[str, Any]


# --------------------------------------------------------------------------- #
# Python type annotation -> JSON Schema fragment
# --------------------------------------------------------------------------- #

_PRIMITIVES = {str: "string", int: "integer", float: "number", bool: "boolean"}


def _type_to_schema(annotation: Any) -> dict[str, Any]:
    """Best-effort conversion of a type hint to a JSON Schema fragment."""
    if annotation is inspect.Parameter.empty or annotation is Any:
        return {}

    origin = typing.get_origin(annotation)
    args = typing.get_args(annotation)

    # Optional[X] / Union[...]
    if origin is typing.Union:
        non_none = [a for a in args if a is not type(None)]  # noqa: E721
        if len(non_none) == 1:
            return _type_to_schema(non_none[0])
        return {"anyOf": [_type_to_schema(a) for a in non_none]}

    # Pydantic model -> full nested schema (self-contained with internal $defs)
    if inspect.isclass(annotation) and issubclass(annotation, BaseModel):
        return annotation.model_json_schema()

    # Containers
    if origin in (list, typing.List):
        item = _type_to_schema(args[0]) if args else {}
        return {"type": "array", "items": item}
    if origin in (dict, typing.Dict):
        return {"type": "object"}

    # Primitives
    if annotation in _PRIMITIVES:
        return {"type": _PRIMITIVES[annotation]}

    return {}  # unknown -> accept anything


def _action_input_schema(func: Any) -> dict[str, Any]:
    """Build a JSON Schema object for an action method's parameters."""
    try:
        hints = typing.get_type_hints(func)
    except Exception:
        hints = {}
    sig = inspect.signature(func)

    properties: dict[str, Any] = {}
    required: list[str] = []
    for name, param in sig.parameters.items():
        if name in ("self", "args", "kwargs"):
            continue
        annotation = hints.get(name, param.annotation)
        properties[name] = _type_to_schema(annotation)
        if param.default is inspect.Parameter.empty:
            required.append(name)

    schema: dict[str, Any] = {"type": "object", "properties": properties}
    if required:
        schema["required"] = required
    return schema


# --------------------------------------------------------------------------- #
# Discovery
# --------------------------------------------------------------------------- #

def _load_manifest(tool_dir: str) -> dict[str, Any] | None:
    path = os.path.join(tool_dir, "manifest.json")
    if not os.path.exists(path):
        return None
    try:
        with open(path) as f:
            return json.load(f)
    except Exception as exc:
        logger.error("Bad manifest in %s: %s", tool_dir, exc)
        return None


def _runtime_class(safe_tool_id: str) -> type | None:
    """Import `src.tools.<safe_tool_id>.runtime` and return the `<Pascal>Runtime` class."""
    try:
        module = import_module(f"{_TOOLS_PKG}.{safe_tool_id}.runtime")
    except Exception as exc:
        logger.warning("Skipping tool '%s' — runtime import failed: %s", safe_tool_id, exc)
        return None
    class_name = "".join(w.capitalize() for w in safe_tool_id.split("_")) + "Runtime"
    cls = getattr(module, class_name, None)
    if cls is None:
        logger.warning("Skipping tool '%s' — class '%s' not found", safe_tool_id, class_name)
    return cls


def _is_action(name: str, member: Any) -> bool:
    return inspect.iscoroutinefunction(member) and not name.startswith("_")


def discover_tools() -> list[McpToolSpec]:
    """Scan every tool dir → one McpToolSpec per (tool, public async action)."""
    specs: list[McpToolSpec] = []

    for item in sorted(os.listdir(_TOOLS_DIR)):
        tool_dir = os.path.join(_TOOLS_DIR, item)
        if not os.path.isdir(tool_dir):
            continue
        manifest = _load_manifest(tool_dir)
        if not manifest:
            continue

        tool_id = manifest.get("id") or item
        safe_tool_id = tool_id.replace("-", "_")
        tool_desc = manifest.get("description", "")
        cls = _runtime_class(safe_tool_id)
        if cls is None:
            continue

        for action_name, member in inspect.getmembers(cls, predicate=lambda m: inspect.isfunction(m) or inspect.ismethod(m)):
            if not _is_action(action_name, member):
                continue
            doc = inspect.getdoc(member) or ""
            description = f"[{manifest.get('name', tool_id)}] {action_name}. {tool_desc} {doc}".strip()
            specs.append(
                McpToolSpec(
                    mcp_name=f"{safe_tool_id}__{action_name}",
                    tool_id=tool_id,
                    action=action_name,
                    description=description[:1024],
                    input_schema=_action_input_schema(member),
                )
            )

    logger.info("MCP discovery: %d tool-actions across the toolbox", len(specs))
    return specs
