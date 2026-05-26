from typing import Any

from app.plan.markdown.sections import (
    build_database_section,
    build_documents_section,
    build_graph_nodes_section,
    build_header_section,
    build_semantic_connectors_section,
)


def generate_planner_markdown(
    connectors_data: list[dict[str, Any]],
    documents: list[dict[str, Any]],
    nodes: list[dict[str, Any]],
) -> str:
    """Assemble connector, document, and node data into a Markdown context document."""
    parts: list[str] = []
    parts.extend(build_header_section())
    parts.extend(build_database_section(connectors_data))
    parts.append("\n---")
    parts.extend(build_semantic_connectors_section(connectors_data))
    parts.append("\n---")
    parts.extend(build_documents_section(documents))
    parts.append("\n---")
    parts.extend(build_graph_nodes_section(nodes))
    return "\n".join(parts)
