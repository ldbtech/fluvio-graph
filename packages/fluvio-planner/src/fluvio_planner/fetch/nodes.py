import logging
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_semantic_nodes(
    client: FederationClient,
    workspace_id: str | None = None,
    zone: int = 0,
    domain: str | None = None,
) -> list[dict[str, Any]]:
    """Fetch knowledge graph nodes for the active scope."""
    logger.info("Fetching graph nodes for workspace: %s", workspace_id)
    try:
        response = await client.query(
            queries.GET_NODES,
            {"workspaceId": workspace_id, "zone": zone, "domain": domain},
        )
        return extract_data(response, "nodes") or []
    except Exception as e:
        logger.error("Failed to fetch semantic nodes: %s", e)
        return []
