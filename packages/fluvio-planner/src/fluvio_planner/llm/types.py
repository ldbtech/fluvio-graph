"""Provider-agnostic types — mirrors the Rust crate's `types.rs`."""

from __future__ import annotations

from dataclasses import dataclass

_DEFAULT_MODELS = {
    "anthropic": "claude-sonnet-4-20250514",
    "openai": "gpt-4o",
    "gemini": "gemini-2.0-flash",
    "ollama": "llama3.1",
}


@dataclass(frozen=True)
class ProviderConfig:
    """Everything needed to make one chat call. Resolved per-request from
    either a user's BYOK connection or a deployment-level fallback — never
    held long-term."""

    provider: str  # one of "anthropic", "openai", "gemini", "ollama"
    api_key: str | None  # None only valid for "ollama"
    base_url: str | None  # required for "ollama", optional override otherwise
    model: str | None  # None -> provider's compiled-in default

    def resolved_model(self) -> str:
        return self.model or _DEFAULT_MODELS.get(self.provider, self.provider)
