from pathlib import Path

from pydantic import AliasChoices, Field
from pydantic_settings import BaseSettings, SettingsConfigDict

from fluvio_planner.planner_config import PlannerConfig

# The planner usually runs with CWD = services/agent-planner (no .env there); the
# real secrets live in the repo-root .env. Point at both so config resolves no
# matter how the process is launched. Repo root is four levels up from this file
# (app/ → agent-planner/ → services/ → repo root).
_ROOT_ENV = Path(__file__).resolve().parents[3] / ".env"
_LOCAL_ENV = Path(__file__).resolve().parents[1] / ".env"


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=(_ROOT_ENV, _LOCAL_ENV), extra="ignore")

    port: int = Field(
        default=3007,
        validation_alias=AliasChoices("PORT", "FLUVIO_AGENT_PLANNER_PORT"),
    )
    graphql_gateway_url: str = Field(
        default="http://127.0.0.1:4001",
        validation_alias=AliasChoices("GATEWAY_URL", "GRAPHQL_GATEWAY_URL"),
    )
    tool_builder_tools_dir: str = Field(
        default="",
        validation_alias=AliasChoices("TOOL_BUILDER_TOOLS_DIR"),
        description=(
            "Absolute path to fluvio-tool-builder/src/tools/. "
            "If empty, auto-resolved relative to this file."
        ),
    )
    mcp_server_url: str = Field(
        default="http://127.0.0.1:3008/mcp",
        validation_alias=AliasChoices("MCP_SERVER_URL", "TOOL_BUILDER_MCP_URL"),
        description="fluvio-tool-builder MCP Streamable-HTTP endpoint (tools/list + tools/call).",
    )
    database_service_url: str = Field(
        default="http://127.0.0.1:3005",
        validation_alias=AliasChoices("DATABASE_SERVICE_URL"),
        description="fluvio-database's bare base URL — used for per-user BYOK LLM "
                     "credential resolution via its internal (non-GraphQL) route.",
    )
    internal_secret: str | None = Field(
        default=None,
        validation_alias=AliasChoices("FLUVIOME_INTERNAL_SECRET"),
    )

    def as_planner_config(self) -> PlannerConfig:
        """Project the env-backed settings into the env-free config the domain
        layer accepts. This is the one place env values cross into the domain."""
        return PlannerConfig(
            graphql_gateway_url=self.graphql_gateway_url,
            mcp_server_url=self.mcp_server_url,
            database_service_url=self.database_service_url,
            internal_secret=self.internal_secret,
        )


settings = Settings()
