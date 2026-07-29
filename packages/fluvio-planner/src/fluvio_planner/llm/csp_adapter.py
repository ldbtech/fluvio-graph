"""Adapts this package's multi-provider `chat()` dispatch to the csp-sdk's
`BaseLLM` interface, so CSP capability synthesis gets the same
anthropic/openai/gemini/ollama + BYOK support as the rest of agent-planner
instead of being hardcoded to `csp.AnthropicLLM`.
"""

from __future__ import annotations

from fluvio_planner.llm.chat import chat
from fluvio_planner.llm.types import ProviderConfig


class FluvioLLM:
    """`csp.BaseLLM` implementation backed by `ProviderConfig` + `chat()`.

    Not a `csp.BaseLLM` subclass at import time — importing `csp` here would
    make this module (and thus the rest of `fluvio_planner.llm`) fail to
    import in processes that don't have the optional `csp-sdk` dependency
    installed. `complete()` still satisfies `BaseLLM`'s abstract signature
    structurally, which is all `Orchestrator` requires.
    """

    __slots__ = ("_cfg",)

    def __init__(self, cfg: ProviderConfig) -> None:
        self._cfg = cfg

    async def complete(
        self,
        messages: list,
        *,
        max_tokens: int = 4096,
        temperature: float = 0.0,
        system: str | None = None,
    ):
        from csp import LLMResponse

        plain_messages = [
            {"role": m.role, "content": m.content} for m in messages if m.role != "system"
        ]
        content = await chat(self._cfg, system or "", plain_messages)
        return LLMResponse(content=content)

    async def complete_once(
        self,
        prompt: str,
        *,
        system: str | None = None,
        max_tokens: int = 4096,
        temperature: float = 0.0,
    ):
        from csp import LLMMessage

        return await self.complete(
            [LLMMessage(role="user", content=prompt)],
            system=system,
            max_tokens=max_tokens,
            temperature=temperature,
        )
