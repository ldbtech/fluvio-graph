import logging
from typing import Any

from app.fetch._graphql import extract_data
from app.gateway_client import queries
from app.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_connectors_with_resources(
    client: FederationClient,
    group_id: str | None = None,
) -> list[dict[str, Any]]:
    """Fetch connectors for the group and their selected resources."""
    logger.info("Fetching connectors for groupId: %s", group_id)
    try:
        response = await client.query(queries.GET_USER_CONNECTORS, {"groupId": group_id})
        connectors = extract_data(response, "getUserConnectors") or []
    except Exception as e:
        logger.error("Failed to fetch user connectors: %s", e)
        return []

    if not connectors:
        logger.warning("No connectors found.")
        return []

    result: list[dict[str, Any]] = []
    for conn in connectors:
        conn_id = conn.get("id")
        logger.info(
            "Fetching selected resources for connector: %s (%s)",
            conn_id,
            conn.get("kind"),
        )
        resources = await _fetch_resources_for_connector(client, conn_id)
        result.append({"connector": conn, "resources": resources})

    return result


async def _fetch_resources_for_connector(
    client: FederationClient,
    connector_id: str | None,
) -> list[dict[str, Any]]:
    try:
        response = await client.query(
            queries.GET_SELECTED_RESOURCES,
            {"connectorId": connector_id},
        )
        return extract_data(response, "getSelectedResources") or []
    except Exception as e:
        logger.error(
            "Failed to fetch selected resources for connector %s: %s",
            connector_id,
            e,
        )
        return []
