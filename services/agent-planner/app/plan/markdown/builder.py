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


def generate_planner_markdown_sections(
    connectors_data: list[dict[str, Any]],
    documents: list[dict[str, Any]],
    nodes: list[dict[str, Any]],
    tools: list[dict[str, Any]],
    users: list[dict[str, Any]],
    teams_data: list[dict[str, Any]],
    current_user: dict[str, Any] | None,
) -> dict[str, str]:
    """Return raw markdown per section — used by ContextBudget.assemble()."""
    header = "\n".join(build_header_section())
    iam = "\n".join(build_iam_section(users, current_user))
    teams = "\n".join(build_teams_section(teams_data))
    schemas = "\n".join(build_database_section(connectors_data))
    connectors = "\n".join(build_semantic_connectors_section(connectors_data))
    tools_sec = "\n".join(build_tools_section(tools))
    documents_sec = "\n".join(build_documents_section(documents))
    graph_nodes = "\n".join(build_graph_nodes_section(nodes))
    return {
        "header": header,
        "connectors": connectors,
        "schemas": schemas,
        "tools": tools_sec,
        "teams": iam + "\n\n" + teams,
        "documents": documents_sec,
        "graph_nodes": graph_nodes,
    }


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
