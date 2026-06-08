from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel

# ============================
# Execution Context
# ============================
class DataCleaningExecutionContext(BaseModel):
    """
    Database connection string and environment details for data cleansing.
    """
    database_url: str = "postgres://localhost/fluvio_company"
    environment: str = "local"
    sandbox_id: Optional[str] = None

# ============================
# Capability Contract
# ============================
class DataCleaningTool(ABC):
    """
    Data quality / cleansing on database tables.

    The planner (the brain) authors WHAT cleaning happens; this tool is a dumb,
    safe executor. The only action is `run_cleaning`, where the planner passes
    its own SQL statements to run against a protected clone of the source table.
    """
    name: str = "data-cleaning"

    @abstractmethod
    async def run_cleaning(
        self,
        context: DataCleaningExecutionContext,
        table_name: str,
        statements: List[str],
        output_table: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        Clone the source table into a protected output table (default
        `clean_<table_name>`) and apply the planner-authored `statements` to it,
        in order. The raw source table is never mutated.

        Statements are authored by the planner against the real schema. Each may
        use the placeholders `{table}` (the output/clean table) and `{source}`
        (the read-only source table); both are substituted before execution. Any
        statement that would mutate the source table is rejected.

        Returns a summary: rows before/after, rows purged, and which statements
        ran (or the error that stopped execution).
        """
        pass
