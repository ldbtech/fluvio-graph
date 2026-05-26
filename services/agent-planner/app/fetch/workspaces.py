import logging
from typing import Any

from app.fetch._graphql import extract_data
from app.gateway_client import queries
from app.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_workspace_shares(
    client: FederationClient,
    workspace_id: str,
) -> list[dict[str, Any]]:
    """Fetch shared team members for a workspace."""
    logger.info("Fetching workspace shares for workspace: %s", workspace_id)
    try:
        response = await client.query(
            queries.GET_WORKSPACE_SHARES,
            {"workspaceId": workspace_id},
        )
        return extract_data(response, "workspaceShares") or []
    except Exception as e:
        logger.error("Failed to fetch workspace shares: %s", e)
        return []
