import logging
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_user_profile(client: FederationClient, user_id: str) -> dict[str, Any] | None:
    """Fetch user profile details (role, company_id, policies)."""
    logger.info("Fetching user profile for user_id: %s", user_id)
    try:
        response = await client.query(queries.GET_USER_PROFILE, {"id": user_id})
        return extract_data(response, "getUser")
    except Exception as e:
        logger.error("Failed to fetch user profile: %s", e)
        return None


async def fetch_company_users(client: FederationClient, company_id: str) -> list[dict[str, Any]]:
    """Fetch all users registered under a company for IAM context."""
    logger.info("Fetching company users for company_id: %s", company_id)
    try:
        response = await client.query(queries.GET_COMPANY_USERS, {"companyId": company_id})
        return extract_data(response, "getCompanyUsers") or []
    except Exception as e:
        logger.error("Failed to fetch company users: %s", e)
        return []
