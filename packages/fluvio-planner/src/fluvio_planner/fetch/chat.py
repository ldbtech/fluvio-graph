import logging
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


class GraphQLError(Exception):
    """Exception raised when GraphQL response contains errors."""
    def __init__(self, message: str, errors: list[dict[str, Any]]):
        super().__init__(message)
        self.errors = errors


def check_errors(response: dict[str, Any]):
    if "errors" in response and response["errors"]:
        messages = [err.get("message", "Unknown GraphQL error") for err in response["errors"]]
        raise GraphQLError("; ".join(messages), response["errors"])


async def fetch_chat_history(
    client: FederationClient,
    workspace_id: str,
) -> list[dict[str, Any]]:
    """Fetch chat history for a given workspace. Raises GraphQLError if unauthorized."""
    logger.info("Fetching chat history for workspace: %s", workspace_id)
    response = await client.query(
        queries.GET_PLANNER_CHAT_HISTORY,
        {"workspaceId": workspace_id},
    )
    check_errors(response)
    return extract_data(response, "plannerChatHistory") or []


async def add_chat_message(
    client: FederationClient,
    workspace_id: str,
    sender: str,
    content: str,
) -> dict[str, Any]:
    """Add a new chat message to the workspace history."""
    logger.info("Adding chat message to workspace: %s", workspace_id)
    response = await client.query(
        queries.ADD_PLANNER_CHAT_MESSAGE,
        {
            "workspaceId": workspace_id,
            "sender": sender,
            "content": content,
        },
    )
    check_errors(response)
    return extract_data(response, "addPlannerChatMessage") or {}


async def clear_chat_history(
    client: FederationClient,
    workspace_id: str,
) -> bool:
    """Clear chat history for a workspace."""
    logger.info("Clearing chat history for workspace: %s", workspace_id)
    response = await client.query(
        queries.CLEAR_PLANNER_CHAT_HISTORY,
        {"workspaceId": workspace_id},
    )
    check_errors(response)
    return bool(extract_data(response, "clearPlannerChatHistory"))
