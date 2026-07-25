import logging
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_available_tools(client: FederationClient) -> list[dict[str, Any]]:
    """Fetch available execution tools from the tool-builder subgraph."""
    logger.info("Fetching available execution tools from gateway...")
    try:
        response = await client.query(queries.GET_AVAILABLE_TOOLS)
        return extract_data(response, "availableTools") or []
    except Exception as e:
        logger.error("Failed to fetch available tools: %s", e)
        return []
