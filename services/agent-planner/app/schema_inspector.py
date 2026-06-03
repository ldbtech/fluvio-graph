"""Phase 23 — Schema-aware SQL generation.

Extracts actual table schemas from connector resources (already fetched during
context assembly) and formats them for injection into the step formulation prompt.

This gives Claude exact column names rather than letting it hallucinate them.

The schema comes from two sources:
1. connector resource `meta` JSON (available without a DB connection) — used for prompt injection
2. Live asyncpg EXPLAIN query (requires DB credentials) — used for pre-execution validation

Both are best-effort: if either fails, execution continues without the validation step.
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any

logger = logging.getLogger("agent-planner")


def extract_schema_from_resources(
    connectors_data: list[dict[str, Any]],
) -> dict[str, list[str]]:
    """Parse table → column list from connector resource metadata.

    Returns: {"users": ["id", "email", "created_at"], "orders": [...], ...}
    """
    schema: dict[str, list[str]] = {}
    for entry in connectors_data:
        for resource in entry.get("resources") or []:
            table_name = resource.get("name") or resource.get("externalId", "")
            if not table_name:
                continue
            meta_str = resource.get("meta")
            if not meta_str:
                continue
            try:
                columns = json.loads(meta_str).get("columns") or []
                if columns:
                    schema[table_name] = columns
            except Exception:
                pass
    return schema


def format_schema_for_prompt(schema: dict[str, list[str]]) -> str:
    """Format the schema as a compact markdown table definition for prompt injection."""
    if not schema:
        return "_(no schema information available)_"
    lines = ["### Exact Database Schema\n"]
    for table, columns in sorted(schema.items()):
        col_list = ", ".join(f"`{c}`" for c in columns)
        lines.append(f"- **`{table}`**: {col_list}")
    lines.append(
        "\nGenerate SQL that references ONLY these exact column names. "
        "Do not invent columns. Do not use `*` in output_table queries — name columns explicitly."
    )
    return "\n".join(lines)


def extract_sql_column_refs(sql: str) -> set[str]:
    """Extract column-like identifiers from a SQL string (heuristic)."""
    # Strip string literals and comments first
    cleaned = re.sub(r"'[^']*'", " ", sql)
    cleaned = re.sub(r"--[^\n]*", " ", cleaned)
    # Keywords to exclude from column detection
    keywords = {
        "select", "from", "where", "join", "on", "group", "by", "order",
        "having", "limit", "offset", "as", "and", "or", "not", "in", "is",
        "null", "true", "false", "inner", "outer", "left", "right", "full",
        "cross", "union", "distinct", "case", "when", "then", "else", "end",
        "sum", "count", "avg", "max", "min", "date_trunc", "over", "partition",
    }
    tokens = set(re.findall(r"\b([a-z_][a-z0-9_]*)\b", cleaned.lower()))
    return tokens - keywords


def validate_sql_columns(
    sql: str,
    schema: dict[str, list[str]],
    source_tables: list[str],
) -> list[str]:
    """Static check: are all column refs in sql present in source_tables' schemas?

    Returns a list of warning strings (empty if all clear).
    This is a fast heuristic — false positives are possible for aliases, CTEs, etc.
    """
    if not schema or not source_tables:
        return []

    known_columns: set[str] = set()
    for table in source_tables:
        known_columns.update(c.lower() for c in schema.get(table, []))
        # Also accept clean_ prefixed version
        known_columns.update(c.lower() for c in schema.get(f"clean_{table}", []))

    if not known_columns:
        return []

    refs = extract_sql_column_refs(sql)
    # Filter out table names themselves and common aggregate aliases
    table_names_lower = {t.lower() for t in source_tables} | {f"clean_{t.lower()}" for t in source_tables}
    suspicious = refs - known_columns - table_names_lower

    warnings = []
    for col in sorted(suspicious):
        if len(col) < 2:
            continue  # too short to be meaningful
        warnings.append(
            f"Column `{col}` not found in schemas for {source_tables}. "
            "Verify this column exists or it may be a CTE alias."
        )
    return warnings


async def explain_sql(
    sql: str,
    db_url: str,
) -> tuple[bool, str]:
    """Run EXPLAIN against the database to catch syntax/column errors before execution.

    Returns (ok, message). ok=False means the SQL is definitely broken.
    Requires asyncpg and a live DB connection — degrades gracefully if unavailable.
    """
    try:
        import asyncpg  # type: ignore
    except ImportError:
        return True, "asyncpg not installed — skipping EXPLAIN"

    if not db_url:
        return True, "no db_url — skipping EXPLAIN"

    try:
        conn = await asyncpg.connect(db_url)
        try:
            await conn.fetch(f"EXPLAIN {sql}")
            return True, "ok"
        except Exception as exc:
            return False, str(exc)
        finally:
            await conn.close()
    except Exception as exc:
        return True, f"EXPLAIN connection failed (non-fatal): {exc}"
