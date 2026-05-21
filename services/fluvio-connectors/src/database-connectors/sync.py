# database-connectors/sync.py
# Ties schema + connector + convert + storage together.
# One function: connect to DB, fetch selected tables, save CSVs.

import asyncio
from datetime import datetime, UTC
from pathlib import Path

from database_types.sql_connector import DBConfig, DatabaseConnector, SchemaFunctions
from convert.to_csv import to_csv
from storage.local import LocalStorage


async def sync_tables(
    org_id:        str,
    connector_id:  str,
    config:        DBConfig,
    table_names:   list[str],       # tables user selected in wizard
    storage:       LocalStorage,
) -> dict:
    """
    Full sync flow:
      1. Connect to DB
      2. For each selected table: fetch all rows
      3. Convert to CSV
      4. Save to s3/ folder
      5. Return summary

    Read-only. Never writes to the source DB.
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

        # Column names to fetch (all non-sensitive by default for now)
        columns = [col.name for col in table_meta.columns]

        # Fetch rows — read only
        rows = fetch_rows(engine, table_name, columns)

        # Convert to CSV bytes
        csv_result = to_csv(
            table=   table_name,
            rows=    rows,
            columns= columns,
        )

        # Save to s3/ folder
        path = storage.save_snapshot(
            org_id=       org_id,
            connector_id= connector_id,
            table=        table_name,
            content=      csv_result.content,
            filename=     csv_result.filename,
            timestamp=    datetime.now(UTC),
        )

        # Save metadata
        storage.save_metadata(
            org_id=       org_id,
            connector_id= connector_id,
            table=        table_name,
            meta={
                "table":      table_name,
                "row_count":  csv_result.row_count,
                "columns":    csv_result.columns,
                "synced_at":  datetime.now(UTC).isoformat(),
                "source_db":  config.dialect,
                "snapshot":   str(path),
            }
        )

        results[table_name] = {
            "rows":     csv_result.row_count,
            "columns":  len(csv_result.columns),
            "path":     str(path),
        }

        print(f"  ✓ {table_name}: {csv_result.row_count} rows → {path.name}")

    engine.dispose()
    return results


def fetch_rows(engine, table_name: str, columns: list[str]) -> list[dict]:
    """Fetch all rows from a table. Read-only SELECT."""
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

    print("Starting sync...\n")

    results = asyncio.run(sync_tables(
        org_id=       "org-7eceeae5",
        connector_id= "connector-fluvio-collab",
        config=       config,
        table_names=  ["users", "groups", "group_members"],
        storage=      store,
    ))

    print(f"\nSync complete:")
    for table, info in results.items():
        if "error" in info:
            print(f"  ✗ {table}: {info['error']}")
        else:
            print(f"  {table}: {info['rows']} rows, saved to {info['path']}")