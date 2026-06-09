"""fluvio-tool-builder — data pipeline tools orchestrator."""
import logging
import uvicorn
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from strawberry.fastapi import GraphQLRouter

from src.graphql import schema
from src.config import PORT
from src.mcp_server.server import handle_mcp, mcp_lifespan

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-8s  %(name)s  %(message)s",
)
logger = logging.getLogger(__name__)

@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("fluvio-tool-builder starting...")
    # The MCP Streamable HTTP transport needs its task group running for the
    # app's lifetime — nest it inside the service lifespan.
    async with mcp_lifespan():
        yield
    logger.info("fluvio-tool-builder stopped")

app = FastAPI(title="fluvio-tool-builder", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# GraphQL endpoint (legacy executeTool path — kept intact through M1–M4)
graphql_app = GraphQLRouter(schema)
app.include_router(graphql_app, prefix="/graphql")

# MCP endpoint (Phase M1) — Streamable HTTP transport for tools/list + tools/call.
# Mounted as a raw ASGI app; both agent-planner (M2/M3) and external MCP
# clients (Claude Desktop, Cursor) connect here.
app.mount("/mcp", handle_mcp)

@app.get("/health")
async def health():
    return {"status": "ok"}

if __name__ == "__main__":
    uvicorn.run(
        "src.main:app",
        host="0.0.0.0",
        port=PORT,
        reload=True,
        log_level="info",
    )
