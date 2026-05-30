from typing import Any

from app.plan.markdown.sections import (
    build_database_section,
    build_documents_section,
    build_graph_nodes_section,
    build_header_section,
    build_semantic_connectors_section,
    build_tools_section,
    build_iam_section,
    build_teams_section,
)


def generate_planner_markdown(
    connectors_data: list[dict[str, Any]],
    documents: list[dict[str, Any]],
    nodes: list[dict[str, Any]],
    tools: list[dict[str, Any]],
    users: list[dict[str, Any]],
    teams_data: list[dict[str, Any]],
    current_user: dict[str, Any] | None,
) -> str:
    """Assemble connector, document, node, tool, IAM, and team data into a Markdown context document."""
    parts: list[str] = []
    parts.extend(build_header_section())
    parts.extend(build_iam_section(users, current_user))
    parts.append("\n---")
    parts.extend(build_teams_section(teams_data))
    parts.append("\n---")
    parts.extend(build_database_section(connectors_data))
    parts.append("\n---")
    parts.extend(build_semantic_connectors_section(connectors_data))
    parts.append("\n---")
    parts.extend(build_tools_section(tools))
    parts.append("\n---")
    parts.extend(build_documents_section(documents))
    parts.append("\n---")
    parts.extend(build_graph_nodes_section(nodes))
    return "\n".join(parts)
