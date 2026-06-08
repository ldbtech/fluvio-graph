import asyncio
import logging
import re
from typing import Dict, List, Any, Optional
from src.tools.data_cleaning.contracts import (
    DataCleaningTool,
    DataCleaningExecutionContext
)

logger = logging.getLogger("data-cleaning-runtime")

class DataCleaningRuntime(DataCleaningTool):
    """
    Data Cleaning tool implementation executing clean queries using psql.
    """

    async def _run_sql(self, context: DataCleaningExecutionContext, sql: str) -> str:
        if context.sandbox_id:
            # Derive the DB name from the connection URL; default to the universal
            # "postgres" database rather than any tenant-specific name.
            db_name = "postgres"
            match = re.search(r"/([^/\?]+)(?:\?|$)", context.database_url or "")
            if match:
                db_name = match.group(1)
            cmd = [
                "docker", "exec", "-i", f"fluvio-sandbox-{context.sandbox_id}-postgres",
                "psql", "-U", "postgres", "-d", db_name,
                "-t", "-A", "-c", sql
            ]
        else:
            cmd = ["psql", context.database_url, "-t", "-A", "-c", sql]
            
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        stdout, stderr = await proc.communicate()
        if proc.returncode != 0:
            raise Exception(f"psql error: {stderr.decode().strip()}")
        return stdout.decode().strip()

    @staticmethod
    def _assert_valid_identifier(name: str, kind: str) -> None:
        if not re.match(r"^[a-zA-Z0-9_]+$", name or ""):
            raise Exception(f"Invalid {kind}: '{name}'")

    # Mutating verbs that must never target the read-only source table.
    _SOURCE_MUTATION_RE = re.compile(
        r"\b(DROP\s+TABLE|TRUNCATE|DELETE\s+FROM|UPDATE|ALTER\s+TABLE|INSERT\s+INTO)\s+"
        r"(?:public\.)?\"?{src}\"?\b",
        re.IGNORECASE,
    )

    def _guard_protects_source(self, statement: str, source_table: str) -> None:
        """Reject any statement that would mutate the raw source table.

        The planner authors these statements, so this is the one hard safety
        rail: cleaning operates on the clone only; the source stays pristine.
        """
        pattern = re.compile(
            self._SOURCE_MUTATION_RE.pattern.replace("{src}", re.escape(source_table)),
            re.IGNORECASE,
        )
        if pattern.search(statement):
            raise Exception(
                f"Refusing statement that would mutate the source table "
                f"'{source_table}'. Operate on the clean table (use the {{table}} "
                f"placeholder) instead."
            )

    async def run_cleaning(
        self,
        context: DataCleaningExecutionContext,
        table_name: str,
        statements: List[str],
        output_table: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Clone the source table, then apply planner-authored SQL to the clone."""
        self._assert_valid_identifier(table_name, "source table name")
        clean_table_name = output_table or f"clean_{table_name}"
        self._assert_valid_identifier(clean_table_name, "output table name")

        try:
            # 1. Verify source exists
            exists_sql = (
                "SELECT count(*) FROM information_schema.tables "
                f"WHERE table_schema = 'public' AND table_name = '{table_name}'"
            )
            if await self._run_sql(context, exists_sql) != "1":
                raise Exception(
                    f"Source table '{table_name}' does not exist in the public schema."
                )

            # 2. Initial row count
            initial_count_str = await self._run_sql(
                context, f"SELECT count(*) FROM {table_name}"
            )
            initial_count = int(initial_count_str) if initial_count_str else 0

            # 3. Clone source → protected output table
            logger.info("Cloning '%s' to '%s'...", table_name, clean_table_name)
            await self._run_sql(
                context,
                f"DROP TABLE IF EXISTS {clean_table_name};\n"
                f"CREATE TABLE {clean_table_name} AS SELECT * FROM {table_name};",
            )

            # 4. Apply planner-authored statements, in order, against the clone
            applied: List[str] = []
            for raw in statements:
                stmt = (raw or "").strip()
                if not stmt:
                    continue
                # Substitute ergonomic placeholders before any safety check.
                stmt = stmt.replace("{table}", clean_table_name).replace(
                    "{source}", table_name
                )
                self._guard_protects_source(stmt, table_name)
                logger.info("Applying cleaning statement: %s", stmt.split("\n")[0][:120])
                await self._run_sql(context, stmt)
                applied.append(stmt)

            # 5. Final row count
            final_count_str = await self._run_sql(
                context, f"SELECT count(*) FROM {clean_table_name}"
            )
            final_count = int(final_count_str) if final_count_str else 0

            return {
                "status": "success",
                "table_name": table_name,
                "output_table": clean_table_name,
                "rows_processed": initial_count,
                "rows_remaining": final_count,
                "rows_purged": initial_count - final_count,
                "statements_applied": len(applied),
            }

        except Exception as e:
            logger.error("Error during run_cleaning: %s", e, exc_info=True)
            return {"status": "failed", "error": str(e)}
