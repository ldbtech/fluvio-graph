from typing import Any


def build_semantic_connectors_section(connectors_data: list[dict[str, Any]]) -> list[str]:
    md = ["\n## 2. Semantic Knowledge Graph Ingest (File & Wiki Sources)"]
    non_db_connectors = [
        c for c in connectors_data 
        if c["connector"].get("kind") not in ["database", "postgresql", "postgres", "mysql"]
        and (c["connector"].get("status") or "").lower() != "disconnected"
    ]

    if not non_db_connectors:
        md.append("_No semantic file or wiki connectors enabled in the knowledge scope._")
        return md

    for idx, entry in enumerate(non_db_connectors, 1):
        conn = entry["connector"]
        resources = entry["resources"]
        kind = (conn.get("kind") or "").upper()
        md.append(f"\n### Semantic Connector {idx}: {conn.get('id')} ({kind})")
        md.append(f"- **Kind**: {conn.get('kind')}")
        md.append(f"- **Sync Status**: {conn.get('status')}")
        md.append(f"- **Last Ingest**: {conn.get('lastSyncAt') or 'Never'}")
        md.append("\n#### Selected Knowledge Folders & Repositories:")

        if not resources:
            md.append("  _No specific directories or wikis selected for graph ingestion._")
            continue

        for res in resources:
            md.append(f"  - **Source Resource**: `{res.get('name')}`")
            md.append(f"    - **External Ref**: `{res.get('externalId')}`")
            md.append(f"    - **Nodes Contributed**: {res.get('nodeCount'):,}")
            if res.get("description"):
                md.append(f"    - **Description**: {res.get('description')}")

    return md
