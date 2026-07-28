"""HTTP client for database-server's internal (non-GraphQL, non-public)
`/internal/resolve-llm-credential` route. Mirrors the Rust crate's
`resolver.rs` — deliberately not GraphQL, see that file's docstring for why.
"""

from __future__ import annotations

import httpx

from fluvio_planner.llm.types import ProviderConfig


async def resolve_provider(
    database_service_url: str,
    user_id: str,
    provider: str | None = None,
    internal_secret: str | None = None,
) -> ProviderConfig | None:
    """Resolves the caller's active LLM connection (or the deployment
    fallback). Returns `None` if nothing is configured anywhere — callers
    should degrade gracefully, matching the existing `api_key is None` UX in
    `plan_writer.write_plan` / `compile.compile_plan`.
    """
    url = f"{database_service_url.rstrip('/')}/internal/resolve-llm-credential"
    headers = {}
    if internal_secret:
        headers["x-internal-auth"] = internal_secret

    async with httpx.AsyncClient(timeout=10.0) as http:
        resp = await http.post(
            url,
            json={"user_id": user_id, "group_id": None, "provider": provider},
            headers=headers,
        )

    if resp.status_code == 404:
        return None
    resp.raise_for_status()

    body = resp.json()
    return ProviderConfig(
        provider=body["provider"],
        api_key=body.get("api_key"),
        base_url=body.get("base_url"),
        model=body.get("model"),
    )
