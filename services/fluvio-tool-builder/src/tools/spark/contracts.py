from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel

# ============================
# Spark Execution Context
# ============================
class SparkExecutionContext(BaseModel):
    """
    Runtime context for connecting to the Spark cluster.
    """
    master_url: str = "local[*]"
    app_name: str = "FluviomeSparkApp"
    environment: str = "local"  # local | dev | prod
    sandbox_id: Optional[str] = None
    # Postgres connection URL used when Spark falls back to the local SQL engine.
    # Supplied by the planner from the active connector — no tenant-specific default.
    database_url: Optional[str] = None

# ============================
# Spark Job Configuration
# ============================
class SparkJobConfig(BaseModel):
    """
    Configuration options for submitting a Spark job.
    """
    main_class: Optional[str] = None
    app_args: List[str] = []
    conf: Dict[str, str] = {}
    executor_memory: Optional[str] = None
    driver_memory: Optional[str] = None

# ============================
# Capability Contract
# ============================
class SparkTool(ABC):
    """
    Defines available Spark operations for the LLM planner.
    """
    name: str = "spark"

    @abstractmethod
    async def execute_sql(
        self,
        context: SparkExecutionContext,
        query: str,
        output_table: str
    ) -> bool:
        """Execute a SQL query against Spark catalog and write output."""
        pass

    @abstractmethod
    async def submit_job(
        self,
        context: SparkExecutionContext,
        jar_or_py_path: str,
        config: SparkJobConfig
    ) -> str:
        """Submit a standalone Spark JAR or PySpark script to the cluster."""
        pass

    @abstractmethod
    async def get_job_status(
        self,
        context: SparkExecutionContext,
        job_id: str
    ) -> Dict[str, Any]:
        """Query the status of a submitted Spark job."""
        pass
