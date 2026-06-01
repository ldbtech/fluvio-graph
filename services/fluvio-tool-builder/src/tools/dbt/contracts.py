from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional, List
from pydantic import BaseModel

class DbtExecutionContext(BaseModel):
    """
    dbt execution environment variables and profiles.
    """
    project_dir: str = "/workspace/dbt_project"
    profile_name: str = "default"
    target_name: str = "dev"
    sandbox_id: Optional[str] = None

class DbtTool(ABC):
    """
    Defines dbt CLI data build tool capabilities.
    """
    name: str = "dbt"

    @abstractmethod
    async def run_models(
        self,
        context: DbtExecutionContext,
        select: Optional[str] = None,
        exclude: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Runs dbt models with optional select or exclude filters.
        """
        pass

    @abstractmethod
    async def test_models(
        self,
        context: DbtExecutionContext,
        select: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Runs dbt tests on models.
        """
        pass

    @abstractmethod
    async def compile_project(
        self,
        context: DbtExecutionContext
    ) -> Dict[str, Any]:
        """
        Compiles the dbt project.
        """
        pass
