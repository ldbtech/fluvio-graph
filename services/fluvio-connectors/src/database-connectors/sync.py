# database-connectors/sync.py

import asyncio
import httpx
from datetime import datetime, UTC
from pathlib import Path

from database_types.sql_connector import DBConfig, DatabaseConnector, SchemaFunctions
from convert.to_csv import to_csv
from storage.local import LocalStorage
from row_to_node import rows_to_texts


async def ingest_to_kg(
    texts:        list[tuple[str, str]],  # (text, source_uri)
    owner_id:     str,
    ingestion_url: str = "http://localhost:3004/graphql",
) -> int:
    """
    Send rows to fluvio-ingestion as knowledge graph nodes.
    Returns number of nodes created.
    """
    q = """
    mutation($text: String!, $sourceUri: String!, $domain: String) {
        ingestRaw(text: $text, sourceUri: $sourceUri, domain: $domain) {
            jobId status message
        }
    }
    """
    nodes_created = 0

    async with httpx.AsyncClient(timeout=30.0) as client:
        for text, uri in texts:
            try:
                resp = await client.post(
                    ingestion_url,
                    json={"query": q, "variables": {
                        "text":      text,
                        "sourceUri": uri,
                        "domain":    "database",
                    }},
                    headers={
                        "Content-Type": "application/json",
                        "x-user-id":    owner_id,
                    }
                )
                body = resp.json()
                if "errors" not in body:
                    nodes_created += 1
            except Exception as e:
                print(f"  ⚠ ingest failed for {uri}: {e}")

    return nodes_created


async def sync_tables(
    org_id:           str,
    connector_id:     str,
    config:           DBConfig,
    table_names:      list[str],
    storage:          LocalStorage,
    owner_id:         str = "",
    included_columns: dict[str, list[str]] | None = None,
    ingestion_url:    str = "http://localhost:3004/graphql",
) -> dict:
    """
    Full sync:
      1. Connect to DB
      2. For each table: fetch rows
      3. In parallel: save CSV + ingest into KG
      4. Return summary

    included_columns: dict of {table_name: [col1, col2, ...]}
    If None, all non-sensitive columns are used.
    """
    connector = DatabaseConnector(config)
    engine    = connector.get_engine()
    schema    = SchemaFunctions(engine)
    db_meta   = schema.extract()

    results = {}

    for table_name in table_names:
        table_meta = db_meta.get_table(table_name)
        if not table_meta:
            results[table_name] = {"error": f"table '{table_name}' not found"}
            continue

        # All columns from schema
        all_columns = [col.name for col in table_meta.columns]

        # User-selected columns (or all if not specified)
        columns = (
            included_columns.get(table_name, all_columns)
            if included_columns else all_columns
        )

        # Fetch rows
        rows = fetch_rows(engine, table_name, all_columns)

        # Convert rows to (text, uri) pairs for KG
        texts = rows_to_texts(
            table=            table_name,
            rows=             rows,
            included_columns= columns,
            connector_id=     connector_id,
        )

        # Run CSV export + KG ingestion in parallel
        csv_result = to_csv(table=table_name, rows=rows, columns=columns)

        path, nodes_created = await asyncio.gather(
            asyncio.to_thread(
                storage.save_snapshot,
                org_id=       org_id,
                connector_id= connector_id,
                table=        table_name,
                content=      csv_result.content,
                filename=     csv_result.filename,
                timestamp=    datetime.now(UTC),
            ),
            ingest_to_kg(
                texts=         texts,
                owner_id=      owner_id,
                ingestion_url= ingestion_url,
            ) if owner_id else asyncio.coroutine(lambda: 0)(),
        )

        # Save metadata
        storage.save_metadata(
            org_id=       org_id,
            connector_id= connector_id,
            table=        table_name,
            meta={
                "table":         table_name,
                "row_count":     csv_result.row_count,
                "columns":       columns,
                "nodes_created": nodes_created if owner_id else 0,
                "synced_at":     datetime.now(UTC).isoformat(),
                "source_db":     config.dialect,
                "snapshot":      str(path),
            }
        )

        results[table_name] = {
            "rows":          csv_result.row_count,
            "columns":       len(columns),
            "path":          str(path),
            "nodes_created": nodes_created if owner_id else 0,
            "columns_list":  columns,
        }

        print(f"  ✓ {table_name}: {csv_result.row_count} rows → CSV + {nodes_created} KG nodes")

    engine.dispose()
    return results


def fetch_rows(engine, table_name: str, columns: list[str]) -> list[dict]:
    cols = ", ".join(f'"{c}"' for c in columns)
    with engine.connect() as conn:
        from sqlalchemy import text
        result = conn.execute(text(f'SELECT {cols} FROM "{table_name}"'))
        return [dict(row._mapping) for row in result]


# ── Test ──────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import sys
    sys.path.insert(0, str(Path(__file__).parent))

    config = DBConfig(
        dialect=  "postgresql",
        host=     "localhost",
        port=     5432,
        database= "fluvio_collab",
        username= "alidaho",
        password= "",
    )

    store = LocalStorage(base_path="./s3")

    print("Starting sync with KG ingestion...\n")

    results = asyncio.run(sync_tables(
        org_id=       "org-7eceeae5",
        connector_id= "connector-fluvio-collab",
        config=       config,
        table_names=  ["users", "groups"],
        storage=      store,
        owner_id=     "7eceeae5-a8ef-4d61-9e50-c99a955dbd11",
        included_columns= {
            "users":  ["email", "display_name", "created_at"],
            "groups": ["name", "description", "created_at"],
        },
    ))

    print(f"\nSync complete:")
    for table, info in results.items():
        if "error" in info:
            print(f"  ✗ {table}: {info['error']}")
        else:
            print(f"  {table}: {info['rows']} rows, "
                  f"{info['nodes_created']} KG nodes, "
                  f"saved to {Path(info['path']).name}")