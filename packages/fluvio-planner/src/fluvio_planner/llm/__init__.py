"""Multi-provider LLM client — mirrors the Rust `fluvio-llm` crate's shape.

Three wire-format implementations cover four providers: native Anthropic,
native Gemini, and one OpenAI-compatible path serving OpenAI itself plus
Ollama and any other self-hosted model that speaks the same shape.
"""

from fluvio_planner.llm.types import ProviderConfig
from fluvio_planner.llm.chat import chat
from fluvio_planner.llm.resolver import resolve_provider

__all__ = ["ProviderConfig", "chat", "resolve_provider"]
