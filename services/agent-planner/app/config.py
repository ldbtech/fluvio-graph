from pathlib import Path

from pydantic import AliasChoices, Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

from app.planner_config import PlannerConfig

# The planner usually runs with CWD = services/agent-planner (no .env there); the
# real secrets live in the repo-root .env. Point at both so the key resolves no
# matter how the process is launched. Repo root is four levels up from this file
# (app/ → agent-planner/ → services/ → repo root).
_ROOT_ENV = Path(__file__).resolve().parents[3] / ".env"
_LOCAL_ENV = Path(__file__).resolve().parents[1] / ".env"


def _key_from_env_files() -> str | None:
    """Read ANTHROPIC_API_KEY straight from the .env files.

    Used as a fallback when the process environment carries an *empty*
    ANTHROPIC_API_KEY (e.g. a parent shell exported a blank one), which would
    otherwise shadow the real value in .env.
    """
    from dotenv import dotenv_values
    for path in (_LOCAL_ENV, _ROOT_ENV):
        if path.exists():
            val = (dotenv_values(path).get("ANTHROPIC_API_KEY") or "").strip()
            if val:
                return val
    return None


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
    anthropic_api_key: str | None = Field(
        default=None,
        validation_alias=AliasChoices("ANTHROPIC_API_KEY"),
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

    @field_validator("anthropic_api_key", mode="after")
    @classmethod
    def _fallback_blank_key(cls, v: str | None) -> str | None:
        # A blank/whitespace env var must not shadow the real .env value.
        if v and v.strip():
            return v.strip()
        return _key_from_env_files()

    def as_planner_config(self) -> PlannerConfig:
        """Project the env-backed settings into the env-free config the domain
        layer accepts. This is the one place env values cross into the domain."""
        return PlannerConfig(
            anthropic_api_key=self.anthropic_api_key,
            graphql_gateway_url=self.graphql_gateway_url,
            mcp_server_url=self.mcp_server_url,
        )


settings = Settings()
