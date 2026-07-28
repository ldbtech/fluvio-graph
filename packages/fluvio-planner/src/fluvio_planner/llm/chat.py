"""Single dispatch surface — every caller goes through `chat` rather than
talking to a provider's wire format directly. No streaming here: none of the
existing call sites in this package stream today.
"""

from __future__ import annotations

import httpx

from fluvio_planner.llm.types import ProviderConfig


def _require_key(cfg: ProviderConfig) -> str:
    if not cfg.api_key or not cfg.api_key.strip():
        raise RuntimeError(f"{cfg.provider} connection is missing an API key")
    return cfg.api_key


async def _chat_anthropic(cfg: ProviderConfig, system: str, messages: list[dict]) -> str:
    api_key = _require_key(cfg)
    async with httpx.AsyncClient(timeout=90.0) as http:
        resp = await http.post(
            "https://api.anthropic.com/v1/messages",
            headers={
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
                "content-type": "application/json",
            },
            json={
                "model": cfg.resolved_model(),
                "max_tokens": 4096,
                "system": system,
                "messages": [m for m in messages if m.get("content", "").strip()],
            },
        )
        if resp.status_code != 200:
            raise RuntimeError(f"Anthropic API error: {resp.text}")
        return resp.json()["content"][0]["text"]


def _to_gemini_role(role: str) -> str:
    return "model" if role == "assistant" else "user"


async def _chat_gemini(cfg: ProviderConfig, system: str, messages: list[dict]) -> str:
    api_key = _require_key(cfg)
    base_url = (cfg.base_url or "https://generativelanguage.googleapis.com").rstrip("/")
    model = cfg.resolved_model()
    url = f"{base_url}/v1beta/models/{model}:generateContent"

    contents = [
        {"role": _to_gemini_role(m["role"]), "parts": [{"text": m["content"]}]}
        for m in messages
        if m.get("content", "").strip()
    ]

    async with httpx.AsyncClient(timeout=90.0) as http:
        resp = await http.post(
            url,
            headers={"x-goog-api-key": api_key, "content-type": "application/json"},
            json={
                "systemInstruction": {"parts": [{"text": system}]},
                "contents": contents,
            },
        )
        if resp.status_code != 200:
            raise RuntimeError(f"Gemini API error: {resp.text}")
        body = resp.json()
        return body["candidates"][0]["content"]["parts"][0]["text"]


async def _chat_openai_compat(cfg: ProviderConfig, system: str, messages: list[dict]) -> str:
    if cfg.base_url:
        base_url = cfg.base_url.rstrip("/")
    elif cfg.provider == "openai":
        base_url = "https://api.openai.com/v1"
    else:
        # Ollama and similar local endpoints commonly run alongside this engine.
        base_url = "http://localhost:11434/v1"

    payload_messages = [{"role": "system", "content": system}]
    payload_messages.extend(m for m in messages if m.get("content", "").strip())

    headers = {"content-type": "application/json"}
    if cfg.api_key and cfg.api_key.strip():
        headers["authorization"] = f"Bearer {cfg.api_key}"

    async with httpx.AsyncClient(timeout=90.0) as http:
        resp = await http.post(
            f"{base_url}/chat/completions",
            headers=headers,
            json={"model": cfg.resolved_model(), "messages": payload_messages},
        )
        if resp.status_code != 200:
            raise RuntimeError(f"OpenAI-compatible endpoint error: {resp.text}")
        return resp.json()["choices"][0]["message"]["content"]


async def chat(cfg: ProviderConfig, system: str, messages: list[dict]) -> str:
    """`messages` is Anthropic-style: `[{"role": ..., "content": ...}, ...]`."""
    if cfg.provider == "anthropic":
        return await _chat_anthropic(cfg, system, messages)
    if cfg.provider == "gemini":
        return await _chat_gemini(cfg, system, messages)
    if cfg.provider in ("openai", "ollama"):
        return await _chat_openai_compat(cfg, system, messages)
    raise ValueError(f"unknown LLM provider: {cfg.provider}")
