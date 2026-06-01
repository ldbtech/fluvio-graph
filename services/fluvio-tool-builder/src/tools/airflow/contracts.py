from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional
from pydantic import BaseModel

class AirflowExecutionContext(BaseModel):
    """
    Airflow connection parameters and configuration.
    """
    host_url: str = "http://localhost:8080"
    environment: str = "local"
    sandbox_id: Optional[str] = None

class AirflowTool(ABC):
    """
    Defines Apache Airflow workflow orchestration capabilities.
    """
    name: str = "airflow"

    @abstractmethod
    async def trigger_dag(
        self,
        context: AirflowExecutionContext,
        dag_id: str,
        conf: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Triggers execution of an Airflow DAG.
        """
        pass

    @abstractmethod
    async def get_dag_run_status(
        self,
        context: AirflowExecutionContext,
        dag_id: str,
        run_id: str
    ) -> Dict[str, Any]:
        """
        Retrieves the status of a specific DAG run.
        """
        pass
