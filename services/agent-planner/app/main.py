"""agent-planner — three focused routers, each owning one concern.

chat     → conversation loop, intent detection
compile  → workspace context assembly, LLM step generation
execute  → sandbox lifecycle, step execution (async job queue)
"""

import asyncio
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.config import settings
from app.observability.logging import configure_logging, get_logger
from app.observability.middleware import TracingMiddleware
from app.routers import chat, compile, execute

configure_logging()
logger = get_logger("agent-planner")


@asynccontextmanager
async def lifespan(_app: FastAPI):
    # Load tool manifests from disk before accepting traffic
    from app.toolbox import toolbox
    toolbox.load()

    logger.info(
        "agent-planner starting",
        extra={"port": settings.port, "gateway": settings.graphql_gateway_url},
    )
    from app.jobs.worker import worker_loop
    worker_task = asyncio.create_task(worker_loop())
    yield
    worker_task.cancel()
    try:
        await worker_task
    except asyncio.CancelledError:
        pass
    logger.info("agent-planner stopped")


app = FastAPI(title="agent-planner", lifespan=lifespan)

app.add_middleware(TracingMiddleware)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health() -> dict:
    from app.reliability.circuit_breaker import breaker_status
    return {"status": "ok", "circuit_breakers": breaker_status()}


app.include_router(chat.router)
app.include_router(compile.router)
app.include_router(execute.router)


if __name__ == "__main__":
    uvicorn.run(
        "app.main:app",
        host="0.0.0.0",
        port=settings.port,
        log_level="info",
    )
