import logging
import asyncio
from typing import Any

from fluvio_planner.fetch._graphql import extract_data
from fluvio_planner.gateway_client import queries
from fluvio_planner.gateway_client.client import FederationClient

logger = logging.getLogger("agent-planner")


async def fetch_company_teams(client: FederationClient, company_id: str) -> list[dict[str, Any]]:
    """Fetch all teams/squads registered in a company."""
    logger.info("Fetching teams for company_id: %s", company_id)
    try:
        response = await client.query(queries.GET_COMPANY_TEAMS, {"companyId": company_id})
        return extract_data(response, "getCompanyTeams") or []
    except Exception as e:
        logger.error("Failed to fetch company teams: %s", e)
        return []


async def fetch_team_details(client: FederationClient, team_id: str) -> dict[str, Any]:
    """Concurrently fetch members and workflows for a given team."""
    logger.info("Fetching members and workflows for team: %s", team_id)
    try:
        members_task = client.query(queries.GET_TEAM_MEMBERS, {"teamId": team_id})
        workflows_task = client.query(queries.GET_TEAM_WORKFLOWS, {"teamId": team_id})

        members_resp, workflows_resp = await asyncio.gather(members_task, workflows_task)

        members = extract_data(members_resp, "getTeamMembers") or []
        workflows = extract_data(workflows_resp, "getTeamWorkflows") or []

        return {
            "members": members,
            "workflows": workflows
        }
    except Exception as e:
        logger.error("Failed to fetch team details for %s: %s", team_id, e)
        return {
            "members": [],
            "workflows": []
        }
