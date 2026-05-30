from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel

# ============================
# Database Execution Context
# ============================
class DatabaseExecutionContext(BaseModel):
    """
    Connection details for database operations.
    """
    database_url: str = "postgres://localhost/fluvio_company"
    environment: str = "local"

# ============================
# Capability Contract
# ============================
class DatabaseTool(ABC):
    """
    Defines operations to discover schemas and run read-only queries.
    """
    name: str = "database"

    @abstractmethod
    async def list_tables(self, context: DatabaseExecutionContext) -> List[str]:
        """List all tables available in the public schema of the database."""
        pass

    @abstractmethod
    async def get_table_schema(
        self,
        context: DatabaseExecutionContext,
        table_name: str
    ) -> List[Dict[str, Any]]:
        """Retrieve column names, types, and nullability for a specific table."""
        pass

    @abstractmethod
    async def execute_query(
        self,
        context: DatabaseExecutionContext,
        query: str,
        limit: int = 100
    ) -> List[Dict[str, Any]]:
        """
        Execute a read-only SELECT query.
        Enforces safety checks to prevent destructive database operations.
        """
        pass
