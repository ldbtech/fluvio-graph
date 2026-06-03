from pydantic import AliasChoices, Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", extra="ignore")

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


settings = Settings()
