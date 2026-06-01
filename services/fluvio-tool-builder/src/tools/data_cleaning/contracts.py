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
    Defines data quality and cleansing operations on database tables.
    """
    name: str = "data-cleaning"

    @abstractmethod
    async def clean_table(
        self,
        context: DataCleaningExecutionContext,
        table_name: str,
        operations: List[str]
    ) -> Dict[str, Any]:
        """
        Cleans and standardizes a source database table.
        Writes the results to a new table named 'clean_<table_name>'.
        Returns a summary dictionary of rows processed, nulls dropped, and operations performed.
        """
        pass
