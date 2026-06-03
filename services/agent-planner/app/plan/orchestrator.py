import asyncio
import logging

from app.fetch import (
    fetch_connectors_with_resources,
    fetch_knowledge_documents,
    fetch_semantic_nodes,
    fetch_workspace_shares,
    fetch_available_tools,
    fetch_user_profile,
    fetch_company_users,
    fetch_company_teams,
    fetch_team_details,
)
from app.gateway_client.client import FederationClient
from app.plan.markdown.builder import generate_planner_markdown

logger = logging.getLogger("agent-planner")


async def generate_plan_context(
    gateway_url: str,
    user_id: str | None = None,
    group_id: str | None = None,
    workspace_id: str | None = None,
    zone: int = 0,
    domain: str | None = None,
) -> str:
    """
    Query connectors, schemas, and documents via the GraphQL gateway,
    then return a formatted Markdown context plan.
    Includes documents/nodes from the workspace AND personal twins of shared team members.
    """
    logger.info("Initializing FederationClient connecting to: %s", gateway_url)
    headers = {"x-user-id": user_id} if user_id else {}
    client = FederationClient(gateway_url, headers=headers)

    logger.info("Fetching workspace shares...")
    shares = []
    if workspace_id:
        shares = await fetch_workspace_shares(client, workspace_id)

    logger.info("Executing concurrent query fetches for main user context...")
    connectors_data, documents, nodes, tools = await asyncio.gather(
        fetch_connectors_with_resources(client, group_id),
        fetch_knowledge_documents(client, workspace_id, zone),
        fetch_semantic_nodes(client, workspace_id, zone, domain),
        fetch_available_tools(client),
    )

    # Convert documents and nodes to lists to allow mutation/appending
    documents = list(documents)
    nodes = list(nodes)

    # Fetch company IAM and Teams context
    company_users = []
    company_teams = []
    current_user = None

    if user_id:
        current_user = await fetch_user_profile(client, user_id)
        if current_user and current_user.get("companyId"):
            cid = current_user.get("companyId")
            logger.info("User belongs to company: %s. Fetching directory and squads...", cid)
            users_data, teams_data = await asyncio.gather(
                fetch_company_users(client, cid),
                fetch_company_teams(client, cid),
            )
            company_users = users_data

            if teams_data:
                team_tasks = [fetch_team_details(client, t["id"]) for t in teams_data]
                details_list = await asyncio.gather(*team_tasks)
                for t, details in zip(teams_data, details_list):
                    company_teams.append({
                        "team": t,
                        "members": details["members"],
                        "workflows": details["workflows"]
                    })

    # Fetch personal twins of shared team members
    if shares:
        logger.info("Fetching personal twins of shared team members: %s", [s.get("email") for s in shares])
        share_tasks = []
        for share in shares:
            share_user_id = share.get("userId")
            if share_user_id and share_user_id != user_id:
                share_headers = {"x-user-id": share_user_id}
                share_client = FederationClient(gateway_url, headers=share_headers)
                share_tasks.append(fetch_knowledge_documents(share_client, workspace_id=None, zone=zone))
                share_tasks.append(fetch_semantic_nodes(share_client, workspace_id=None, zone=zone, domain=domain))
        
        if share_tasks:
            share_results = await asyncio.gather(*share_tasks)
            # share_results contains alternating list of (docs, nodes) for each shared user
            for i, res in enumerate(share_results):
                if i % 2 == 0:
                    # It's a list of documents. Tag them as shared user twin doc.
                    for doc in res:
                        doc["title"] = f"[Shared Twin] {doc.get('title', '')}"
                        documents.append(doc)
                else:
                    # It's a list of nodes. Tag them.
                    for node in res:
                        node["sourceText"] = f"[Shared Twin Node] {node.get('sourceText', '')}"
                        nodes.append(node)

    logger.info("Fetches completed. Compiling markdown plan...")
    from app.context_budget import ContextBudget
    from app.plan.markdown.builder import generate_planner_markdown_sections

    raw_sections = generate_planner_markdown_sections(
        connectors_data=connectors_data,
        documents=documents,
        nodes=nodes,
        tools=tools,
        users=company_users,
        teams_data=company_teams,
        current_user=current_user,
    )
    return ContextBudget().assemble(raw_sections)

