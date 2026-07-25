from typing import Any

GRAPH_NODE_PREVIEW_LIMIT = 15


def build_graph_nodes_section(nodes: list[dict[str, Any]]) -> list[str]:
    md = ["\n## 4. Graph Nodes Ingestion Telemetry"]
    if not nodes:
        md.append("_No raw node details mapped for schema validation._")
        return md

    md.append(f"Total mapped graph nodes loaded: **{len(nodes):,}**\n")
    md.append("| Node ID | Domain | Kind | Embedded | Zone |")
    md.append("| :--- | :--- | :--- | :--- | :--- |")

    for node in nodes[:GRAPH_NODE_PREVIEW_LIMIT]:
        node_id = node.get("id") or ""
        short_id = f"{node_id[:8]}..." if len(node_id) > 8 else node_id
        embedded = "Yes" if node.get("isEmbedded") else "No"
        md.append(
            f"| `{short_id}` "
            f"| `{node.get('domain')}` "
            f"| `{node.get('kind')}` "
            f"| {embedded} "
            f"| {node.get('zone')} |"
        )

    remaining = len(nodes) - GRAPH_NODE_PREVIEW_LIMIT
    if remaining > 0:
        md.append(f"| ... and {remaining} more nodes ... | | | | |")

    return md
