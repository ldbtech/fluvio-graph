"""Designation — bind a request to a named agent and the company context it
needs to plan well.

The session is intentionally lightweight (an in-memory dataclass, not a DB row):
per the agreed design, the *chat is the plan*. The agent identity is derived from
the request, the company context is assembled from the knowledge graph, and the
resulting `AgentSession` is everything the plan writer needs. Promoting this to a
persisted record (so the twin can enumerate live sessions) is a later step.
"""

from __future__ import annotations

import logging
import uuid
from dataclasses import dataclass, field
from typing import Any

from fluvio_planner.agent.profiles import AgentProfile, select_agent

logger = logging.getLogger("agent-planner")


@dataclass
class AgentSession:
    session_id: str
    agent: AgentProfile
    workspace_id: str
    twin_owner_id: str            # the user/twin this agent reports to
    company_context: str          # distilled KG context (mission, domain, ...)
    status: str = "planning"      # planning → executing → done
    metadata: dict[str, Any] = field(default_factory=dict)


def summarize_company_context(
    user_profile: dict[str, Any] | None,
    nodes: list[dict[str, Any]],
    documents: list[dict[str, Any]],
) -> str:
    """Distill the knowledge-graph signal into a short company brief.

    This is the "always check the company KG first" step: before an agent asks a
    single question, it knows what the company *is*. A healthcare startup and a
    fintech get different clarifying questions from the same request because this
    brief frames everything downstream.

    Kept compact on purpose — it is a framing header for the plan writer, not the
    full context dump (that still flows through `generate_plan_context`).
    """
    lines: list[str] = []

    company = (user_profile or {}).get("company") or {}
    name = company.get("name") or (user_profile or {}).get("companyName")
    mission = company.get("mission") or company.get("description")
    industry = company.get("industry") or company.get("domain")

    if name:
        lines.append(f"Company: {name}")
    if industry:
        lines.append(f"Industry / domain: {industry}")
    if mission:
        lines.append(f"Mission: {mission}")

    role = (user_profile or {}).get("role")
    if role:
        lines.append(f"You are speaking with: a {role}")

    # Pull a few of the most salient graph nodes as domain anchors.
    node_labels = [
        n.get("label") or n.get("name")
        for n in nodes[:8]
        if (n.get("label") or n.get("name"))
    ]
    if node_labels:
        lines.append("Key domain entities in the knowledge graph: " + ", ".join(node_labels))

    doc_titles = [
        d.get("title") or d.get("name")
        for d in documents[:5]
        if (d.get("title") or d.get("name"))
    ]
    if doc_titles:
        lines.append("Reference documents available: " + ", ".join(doc_titles))

    if not lines:
        return (
            "No company profile was found in the knowledge graph for this "
            "workspace. Proceed, but ask the user for any domain context you need."
        )

    return "\n".join(lines)


def designate(
    message: str,
    workspace_id: str,
    twin_owner_id: str,
    user_profile: dict[str, Any] | None,
    nodes: list[dict[str, Any]],
    documents: list[dict[str, Any]],
) -> AgentSession:
    """Match the request to an agent and open a planning session for it."""
    agent = select_agent(message)
    company_context = summarize_company_context(user_profile, nodes, documents)

    session = AgentSession(
        session_id=str(uuid.uuid4()),
        agent=agent,
        workspace_id=workspace_id,
        twin_owner_id=twin_owner_id,
        company_context=company_context,
    )

    logger.info(
        "Designated agent %s (%s) for session %s in workspace %s",
        agent.name, agent.role, session.session_id, workspace_id,
    )
    return session
