"""Minimal MCP client for agent-planner (Phase C5 / MCP M3 foundation).

Connects to the fluvio-tool-builder MCP server (Streamable HTTP at /mcp) to:
  - list_tool_names()  → the live tool catalog (used to detect which MCP tools a
                         capability calls, so we can wire USES_TOOL edges)
  - call_tool(name, args) → execute a tool (the substrate capabilities run on)

Best-effort and lazy: if `mcp` isn't installed or the server is down, callers
get an empty catalog / a raised error and the planner degrades gracefully.
"""

from __future__ import annotations

import logging
from typing import Any

from app.config import settings

logger = logging.getLogger("agent-planner")


async def list_tools() -> list[dict[str, Any]]:
    """Return full MCP tool specs [{name, description, inputSchema}], or []."""
    try:
        from mcp import ClientSession
        from mcp.client.streamable_http import streamablehttp_client
    except Exception:
        logger.info("mcp client not installed — tool catalog unavailable")
        return []

    try:
        async with streamablehttp_client(settings.mcp_server_url) as (read, write, _):
            async with ClientSession(read, write) as session:
                await session.initialize()
                tools = await session.list_tools()
                return [
                    {"name": t.name, "description": t.description or "", "inputSchema": t.inputSchema or {}}
                    for t in tools.tools
                ]
    except Exception as exc:
        logger.warning("MCP tools/list failed (%s): %s", settings.mcp_server_url, exc)
        return []


async def list_tool_names() -> list[str]:
    """Return just the MCP tool names, or [] if unavailable."""
    return [t["name"] for t in await list_tools()]


def format_tools_for_prompt(tools: list[dict[str, Any]]) -> str:
    """Render MCP tool specs as a compact prompt section (M2)."""
    if not tools:
        return ""
    import json
    lines = ["### MCP Tools (live — name = <tool>__<action>, call with the inputSchema args)"]
    for t in tools:
        required = (t.get("inputSchema") or {}).get("required", [])
        props = list((t.get("inputSchema") or {}).get("properties", {}).keys())
        lines.append(
            f"- `{t['name']}` — {t['description'][:140]}\n"
            f"    args: {', '.join(props) or '(none)'}"
            + (f" · required: {', '.join(required)}" if required else "")
        )
    return "\n".join(lines)


async def call_tool(name: str, arguments: dict[str, Any]) -> Any:
    """Invoke an MCP tool and return its parsed result payload."""
    from mcp import ClientSession
    from mcp.client.streamable_http import streamablehttp_client

    async with streamablehttp_client(settings.mcp_server_url) as (read, write, _):
        async with ClientSession(read, write) as session:
            await session.initialize()
            res = await session.call_tool(name, arguments)
            if res.content and getattr(res.content[0], "text", None):
                import json
                try:
                    return json.loads(res.content[0].text)
                except Exception:
                    return res.content[0].text
            return None
