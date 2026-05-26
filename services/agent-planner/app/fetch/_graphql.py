from typing import Any

from app.gateway_client.client import FederationClient


def extract_data(response: dict[str, Any], field: str) -> list[dict[str, Any]] | dict[str, Any] | None:
    return response.get("data", {}).get(field)
