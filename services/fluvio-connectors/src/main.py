"""fluvio-connectors — external service connector hub."""
import logging
import uvicorn
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from strawberry.fastapi import GraphQLRouter

from src.graphql import schema
from src.jobs import start_scheduler, stop_scheduler
from src.config import PORT

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-8s  %(name)s  %(message)s",
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("fluvio-connectors starting...")
    start_scheduler()
    yield
    stop_scheduler()
    logger.info("fluvio-connectors stopped")


app = FastAPI(title="fluvio-connectors", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# GraphQL endpoint
graphql_app = GraphQLRouter(schema)
app.include_router(graphql_app, prefix="/graphql")


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.get("/oauth/github/callback")
async def github_callback(code: str, state: str):
    """
    OAuth callback for GitHub.
    In production this redirects to the frontend with the code.
    Frontend then calls connectOAuth mutation.
    """
    return {
        "code":  code,
        "state": state,
        "next":  "call mutation connectOAuth(input: { kind: 'github', code, state })"
    }


@app.get("/oauth/notion/callback")
async def notion_callback(code: str, state: str):
    return {
        "code":  code,
        "state": state,
        "next":  "call mutation connectOAuth(input: { kind: 'notion', code, state })"
    }


@app.get("/oauth/tableau/callback")
async def tableau_callback(code: str, state: str):
    return {
        "code":  code,
        "state": state,
        "next":  "call mutation connectOAuth(input: { kind: 'tableau', code, state })"
    }


if __name__ == "__main__":
    uvicorn.run(
        "src.main:app",
        host="0.0.0.0",
        port=PORT,
        reload=True,
        log_level="info",
    )