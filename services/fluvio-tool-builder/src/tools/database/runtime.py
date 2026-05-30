import asyncio
import json
import logging
import re
from typing import Dict, List, Any
from src.tools.database.contracts import (
    DatabaseTool,
    DatabaseExecutionContext
)

logger = logging.getLogger("database-runtime")

class DatabaseRuntime(DatabaseTool):
    """
    Database SQL Tool implementation using psql command-line client.
    """

    def _is_safe_query(self, query: str) -> bool:
        """
        Enforce read-only constraint by verifying that the query only performs SELECT or WITH CTE operations
        and does not contain modifying SQL keywords.
        """
        q = query.strip().lower()
        
        # Check start of the query
        if not (q.startswith("select") or q.startswith("with")):
            return False
            
        # Prohibited writing/modifying SQL keywords (checking word boundaries)
        forbidden_keywords = [
            r"\binsert\b", r"\bupdate\b", r"\bdelete\b", r"\bdrop\b", r"\balter\b",
            r"\btruncate\b", r"\bcreate\b", r"\bgrant\b", r"\brevoke\b", r"\breplace\b",
            r"\bupsert\b", r"\binto\b"
        ]
        
        for pattern in forbidden_keywords:
            if re.search(pattern, q):
                return False
                
        return True

    async def list_tables(self, context: DatabaseExecutionContext) -> List[str]:
        # Fetch tables from information_schema
        sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name"
        cmd = [
            "psql", context.database_url,
            "-t", "-A", "-c", sql
        ]
        
        try:
            logger.info("Listing database tables...")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                output = stdout.decode().strip()
                if not output:
                    return []
                return [line.strip() for line in output.split("\n") if line.strip()]
            else:
                raise Exception(f"psql error: {stderr.decode().strip()}")
        except Exception as e:
            logger.error(f"Error listing tables: {e}")
            raise Exception(f"Failed to list tables: {e}")

    async def get_table_schema(
        self,
        context: DatabaseExecutionContext,
        table_name: str
    ) -> List[Dict[str, Any]]:
        # Enforce alphanumeric table name to prevent SQL injection
        if not re.match(r"^[a-zA-Z0-9_\.]+$", table_name):
            raise Exception(f"Invalid table name format: '{table_name}'")

        # Query columns metadata and aggregate to JSON
        sql = f"""
        SELECT coalesce(json_agg(t), '[]'::json) FROM (
            SELECT column_name, data_type, is_nullable, column_default
            FROM information_schema.columns
            WHERE table_name = '{table_name}'
            ORDER BY ordinal_position
        ) t
        """
        cmd = [
            "psql", context.database_url,
            "-t", "-A", "-c", sql
        ]
        
        try:
            logger.info(f"Fetching schema for table: {table_name}")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                output = stdout.decode().strip()
                return json.loads(output)
            else:
                raise Exception(f"psql error: {stderr.decode().strip()}")
        except Exception as e:
            logger.error(f"Error fetching table schema: {e}")
            raise Exception(f"Failed to fetch schema for '{table_name}': {e}")

    async def execute_query(
        self,
        context: DatabaseExecutionContext,
        query: str,
        limit: int = 100
    ) -> List[Dict[str, Any]]:
        # 1. Enforce safety checks
        if not self._is_safe_query(query):
            raise Exception("Security violation: Only read-only SELECT and WITH statements are allowed. Modifying queries are blocked.")

        # 2. Clean query and wrap it with json_agg and limit
        cleaned = query.strip().rstrip(";")
        wrapped_sql = f"""
        SELECT coalesce(json_agg(sub), '[]'::json) FROM (
            {cleaned}
            LIMIT {limit}
        ) sub
        """
        
        cmd = [
            "psql", context.database_url,
            "-t", "-A", "-c", wrapped_sql
        ]
        
        try:
            logger.info(f"Executing read-only SQL: {cleaned[:80]}...")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                output = stdout.decode().strip()
                # Empty result might return empty string
                if not output:
                    return []
                return json.loads(output)
            else:
                raise Exception(f"psql error: {stderr.decode().strip()}")
        except Exception as e:
            logger.error(f"Error executing query: {e}")
            raise Exception(f"Database query execution failed: {e}")
