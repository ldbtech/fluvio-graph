import asyncio
import json
import logging
import re
from typing import Dict, List, Any
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
            db_name = "vowayage"
            match = re.search(r"/([^/\?]+)(?:\?|$)", context.database_url)
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

    async def clean_table(
        self,
        context: DataCleaningExecutionContext,
        table_name: str,
        operations: List[str]
    ) -> Dict[str, Any]:
        # Safety formatting to prevent SQL injection
        if not re.match(r"^[a-zA-Z0-9_]+$", table_name):
            raise Exception(f"Invalid source table name: '{table_name}'")
            
        clean_table_name = f"clean_{table_name}"

        try:
            # 1. Verify source table exists
            exists_sql = f"SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{table_name}'"
            exists_res = await self._run_sql(context, exists_sql)
            if exists_res != "1":
                raise Exception(f"Source table '{table_name}' does not exist in the public schema.")

            # 2. Get initial row count
            count_sql = f"SELECT count(*) FROM {table_name}"
            initial_count_str = await self._run_sql(context, count_sql)
            initial_count = int(initial_count_str) if initial_count_str else 0

            # 3. Drop target clean table if exists and create a clone
            logger.info(f"Cloning table '{table_name}' to '{clean_table_name}'...")
            clone_sql = f"""
            DROP TABLE IF EXISTS {clean_table_name};
            CREATE TABLE {clean_table_name} AS SELECT * FROM {table_name};
            """
            await self._run_sql(context, clone_sql)

            applied_ops = []

            # 4. Apply operations sequentially
            for op in operations:
                op = op.strip().lower()
                
                if op == "normalize_headers":
                    logger.info("Normalizing column headers to lowercase snake_case...")
                    # Fetch all column names in the clean table
                    cols_sql = f"SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{clean_table_name}'"
                    cols_res = await self._run_sql(context, cols_sql)
                    columns = [c.strip() for c in cols_res.split("\n") if c.strip()]
                    
                    rename_queries = []
                    for col in columns:
                        # Clean column header: lowercase, alphanumeric and underscores only
                        normalized = col.lower()
                        normalized = re.sub(r"[^a-z0-9_]", "_", normalized)
                        normalized = re.sub(r"_+", "_", normalized).strip("_")
                        
                        if normalized != col and normalized:
                            rename_queries.append(f'ALTER TABLE {clean_table_name} RENAME COLUMN "{col}" TO "{normalized}";')
                    
                    if rename_queries:
                        await self._run_sql(context, "\n".join(rename_queries))
                    applied_ops.append("normalize_headers")

                elif op == "drop_null_emails" or op == "drop_nulls":
                    logger.info("Purging rows with null values in identifier columns...")
                    # Find if 'email' or 'id' column exists
                    cols_sql = f"SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{clean_table_name}'"
                    cols_res = await self._run_sql(context, cols_sql)
                    columns = [c.strip().lower() for c in cols_res.split("\n") if c.strip()]
                    
                    delete_conditions = []
                    if "email" in columns:
                        delete_conditions.append("email IS NULL OR email = ''")
                    if "id" in columns:
                        delete_conditions.append("id IS NULL")
                    if "uuid" in columns:
                        delete_conditions.append("uuid IS NULL")
                    if "user_id" in columns:
                        delete_conditions.append("user_id IS NULL")
                        
                    if delete_conditions:
                        delete_sql = f"DELETE FROM {clean_table_name} WHERE " + " OR ".join(delete_conditions)
                        await self._run_sql(context, delete_sql)
                        applied_ops.append("drop_null_identifiers")

                elif op == "deduplicate":
                    logger.info("Deduplicating rows...")
                    # Build distinct clone
                    dedup_sql = f"""
                    CREATE TABLE temp_dedup_{table_name} AS SELECT DISTINCT * FROM {clean_table_name};
                    DROP TABLE {clean_table_name};
                    ALTER TABLE temp_dedup_{table_name} RENAME TO {clean_table_name};
                    """
                    await self._run_sql(context, dedup_sql)
                    applied_ops.append("deduplicate")

                elif op == "standardize_currency":
                    logger.info("Standardizing currency fields to numeric base USD...")
                    # Find potential currency columns (types like varchar/text containing symbols)
                    cols_sql = f"SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{clean_table_name}'"
                    cols_res = await self._run_sql(context, cols_sql)
                    
                    currency_updates = []
                    for line in cols_res.split("\n"):
                        if not line.strip():
                            continue
                        parts = line.split("\t")
                        if len(parts) < 2:
                            parts = line.split("|")
                        if len(parts) < 2:
                            parts = [p.strip() for p in line.split() if p.strip()]
                        if len(parts) < 2:
                            continue
                        
                        col = parts[0].strip()
                        dtype = parts[1].strip().lower()
                        
                        col_lower = col.lower()
                        if any(k in col_lower for k in ["spend", "amount", "cost", "revenue", "price", "budget"]):
                            if "char" in dtype or "text" in dtype:
                                # Convert currency string like '$100.50' or '100 EUR' to numeric USD
                                # This handles '$', 'EUR', 'USD' symbols and parses numbers
                                currency_updates.append(f"""
                                UPDATE {clean_table_name} 
                                SET {col} = CASE 
                                    WHEN {col} ~* 'eur' THEN (REGEXP_REPLACE({col}, '[^0-9.]', '', 'g')::numeric * 1.1)::text
                                    ELSE REGEXP_REPLACE({col}, '[^0-9.]', '', 'g')
                                END
                                WHERE {col} IS NOT NULL AND {col} != '';
                                """)
                    
                    if currency_updates:
                        await self._run_sql(context, "\n".join(currency_updates))
                    applied_ops.append("standardize_currency")

            # 5. Get final row count
            final_count_str = await self._run_sql(context, count_sql.replace(table_name, clean_table_name))
            final_count = int(final_count_str) if final_count_str else 0

            return {
                "status": "success",
                "table_name": table_name,
                "output_table": clean_table_name,
                "rows_processed": initial_count,
                "rows_remaining": final_count,
                "rows_purged": initial_count - final_count,
                "operations_applied": applied_ops
            }

        except Exception as e:
            logger.error(f"Error during table cleaning: {e}", exc_info=True)
            return {
                "status": "failed",
                "error": str(e)
            }
