"""MCP server for fluvio-tool-builder (Phase M1).

Exposes every tool-action as an MCP tool over the Streamable HTTP transport,
mounted at `/mcp` on the existing FastAPI app. Both internal callers
(agent-planner, once it becomes an MCP client) and external MCP clients
(Claude Desktop, Cursor, …) use the same endpoint.

This is ADDITIVE: the GraphQL `executeTool` path is untouched. `call_tool` here
delegates to the exact same `registry.execute_tool_action`, so behavior is
identical — only the contract/transport is new (typed `tools/call` instead of
the double-JSON-encoded mutation).
"""

from __future__ import annotations

import json
import logging
from contextlib import asynccontextmanager

import mcp.types as types
from mcp.server.lowlevel import Server
from mcp.server.streamable_http_manager import StreamableHTTPSessionManager

from src.tools.registry import registry
from src.mcp_server.schema import McpToolSpec, discover_tools

logger = logging.getLogger("mcp-server")

# Discover once at import. Re-run discover_tools() if tools are hot-reloaded.
_SPECS: list[McpToolSpec] = discover_tools()
_BY_NAME: dict[str, McpToolSpec] = {s.mcp_name: s for s in _SPECS}

server: Server = Server("fluviome-tools")


@server.list_tools()
async def list_tools() -> list[types.Tool]:
    return [
        types.Tool(
            name=spec.mcp_name,
            description=spec.description,
            inputSchema=spec.input_schema or {"type": "object", "properties": {}},
        )
        for spec in _SPECS
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict | None) -> list[types.ContentBlock]:
    spec = _BY_NAME.get(name)
    if spec is None:
        raise ValueError(f"Unknown tool: {name}")

    # Reuse the exact existing execution core. The registry still wants the
    # legacy {action, arguments(json-string)} envelope, so we build it here —
    # the ugliness is now contained to this one adapter line, not the API.
    inputs_json = json.dumps({"action": spec.action, "arguments": json.dumps(arguments or {})})
    logs: list[str] = []

    result = await registry.execute_tool_action(spec.tool_id, inputs_json, logs)

    payload = {"result": result, "logs": logs}
    is_error = isinstance(result, dict) and result.get("status") == "failed"
    return [
        types.TextContent(
            type="text",
            text=json.dumps(payload, default=str),
        )
    ]
    # Note: returning isError to the client is added in M3 once the planner
    # worker consumes it; for M1 the status travels inside the payload.


# --------------------------------------------------------------------------- #
# Streamable HTTP transport — mounted into the FastAPI app at /mcp
# --------------------------------------------------------------------------- #

session_manager = StreamableHTTPSessionManager(
    app=server,
    event_store=None,
    json_response=True,   # plain JSON responses; simplest for HTTP clients
    stateless=True,       # no server-side session affinity needed for tool calls
)


async def handle_mcp(scope, receive, send) -> None:
    """ASGI entrypoint mounted at /mcp."""
    await session_manager.handle_request(scope, receive, send)


@asynccontextmanager
async def mcp_lifespan():
    """Run the session manager's task group for the app's lifetime."""
    async with session_manager.run():
        logger.info("MCP server ready at /mcp (%d tools)", len(_SPECS))
        yield
