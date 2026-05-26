import asyncio
import logging

from app.fetch import (
    fetch_connectors_with_resources,
    fetch_knowledge_documents,
    fetch_semantic_nodes,
    fetch_workspace_shares,
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
    connectors_data, documents, nodes = await asyncio.gather(
        fetch_connectors_with_resources(client, group_id),
        fetch_knowledge_documents(client, workspace_id, zone),
        fetch_semantic_nodes(client, workspace_id, zone, domain),
    )

    # Convert documents and nodes to lists to allow mutation/appending
    documents = list(documents)
    nodes = list(nodes)

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
    return generate_planner_markdown(connectors_data, documents, nodes)

