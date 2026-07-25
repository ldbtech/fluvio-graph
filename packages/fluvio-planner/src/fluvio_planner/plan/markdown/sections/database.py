import json
from typing import Any


def build_database_section(connectors_data: list[dict[str, Any]]) -> list[str]:
    md = ["\n## 1. Active Data Pipeline Schema Context (Database Sources)"]
    db_connectors = [
        c for c in connectors_data 
        if c["connector"].get("kind") in ["database", "postgresql", "postgres", "mysql"]
        and (c["connector"].get("status") or "").lower() != "disconnected"
    ]

    if not db_connectors:
        md.append("_No active database schema connectors configured in the pipeline scope._")
        return md

    for idx, dc in enumerate(db_connectors, 1):
        conn = dc["connector"]
        resources = dc["resources"]
        md.append(f"\n### Database Connector {idx}: {conn.get('id')}")
        md.append(f"- **Kind**: {conn.get('kind')}")
        md.append(f"- **Auth Method**: {conn.get('authMethod')}")
        md.append(f"- **Ingestion Status**: {conn.get('status')}")
        md.append(f"- **Last Sync**: {conn.get('lastSyncAt') or 'Never'}")
        md.append("\n#### Scoped Schema Resources (Tables):")

        if not resources:
            md.append("  _No specific tables selected for data mapping context._")
            continue

        for res in resources:
            md.extend(_format_table_resource(res))

    return md


def _format_table_resource(res: dict[str, Any]) -> list[str]:
    lines = [
        f"  - **Table**: `{res.get('name')}`",
        f"    - **External ID**: `{res.get('externalId')}`",
        f"    - **Ingested Graph Nodes**: {res.get('nodeCount'):,}",
        f"    - **Last Sync**: {res.get('lastSyncAt') or 'Never'}",
    ]
    if res.get("description"):
        lines.append(f"    - **Metadata Description**: {res.get('description')}")

    meta_str = res.get("meta")
    if meta_str:
        columns_line = _columns_from_meta(meta_str)
        if columns_line:
            lines.append(columns_line)

    return lines


def _columns_from_meta(meta_str: str) -> str | None:
    try:
        columns = json.loads(meta_str).get("columns", [])
    except (json.JSONDecodeError, TypeError):
        return None
    if not columns:
        return None
    return "    - **Columns**: " + ", ".join(f"`{col}`" for col in columns)
