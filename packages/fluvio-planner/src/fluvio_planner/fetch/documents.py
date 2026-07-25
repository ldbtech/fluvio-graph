import logging
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_knowledge_documents(
    client: FederationClient,
    workspace_id: str | None = None,
    zone: int = 0,
) -> list[dict[str, Any]]:
    """Fetch Twin documents registered as semantic context."""
    logger.info("Fetching documents in workspace: %s, zone: %s", workspace_id, zone)
    try:
        response = await client.query(
            queries.GET_DOCUMENTS,
            {"workspaceId": workspace_id, "zone": zone},
        )
        return extract_data(response, "documents") or []
    except Exception as e:
        logger.error("Failed to fetch knowledge documents: %s", e)
        return []
