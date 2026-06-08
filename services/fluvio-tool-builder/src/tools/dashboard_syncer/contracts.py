from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel

# ============================
# Execution Context
# ============================
class DashboardExecutionContext(BaseModel):
    """
    Credentials and routing context for BI platforms (PowerBI or Tableau).
    """
    platform: str = "powerbi"  # powerbi | tableau
    workspace_id: str = "main-marketing-workspace"
    api_token: Optional[str] = None
    tenant_id: Optional[str] = None
    client_id: Optional[str] = None
    client_secret: Optional[str] = None
    tableau_token_name: Optional[str] = None
    tableau_token_value: Optional[str] = None
    tableau_server_url: Optional[str] = "10ax.online.tableau.com"
    environment: str = "local"

# ============================
# Capability Contract
# ============================
class DashboardSyncerTool(ABC):
    """
    Defines capabilities to deploy reports and trigger refreshes on BI platforms.
    """
    name: str = "dashboard-syncer"

    @abstractmethod
    async def publish_report(
        self,
        context: DashboardExecutionContext,
        report_name: str,
        datasource_name: str,
        file_path: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Uploads and deploys a report/dashboard (e.g. .pbix or .twbx) to the platform.
        Configures datasource bindings and returns sharing/embed links.
        """
        pass

    @abstractmethod
    async def trigger_refresh(
        self,
        context: DashboardExecutionContext,
        dataset_id: str
    ) -> bool:
        """
        Triggers a data model refresh in PowerBI or Tableau to sync with the database.
        """
        pass

    @abstractmethod
    async def get_share_link(
        self,
        context: DashboardExecutionContext,
        report_id: str
    ) -> str:
        """
        Generates a secure sharing/embed URL for the report to be viewed online.
        """
        pass

    @abstractmethod
    async def generate_pdf_report(
        self,
        context: DashboardExecutionContext,
        report_name: str,
        database_url: Optional[str] = None,
        latex_content: Optional[str] = None,
        chart_code: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """
        Generate a PDF report. The planner is the brain and authors EVERYTHING:
        the full LaTeX document (`latex_content`, required) and the actual
        seaborn/matplotlib plotting program (`chart_code`) that produces its
        figures. The runtime just runs the code and compiles the document — there
        is no auto-generated fallback (on failure the orchestrator retries).

        `chart_code` is a list of Python snippets. Each runs in a namespace
        pre-bound with `pd`, `np`, `plt`, `sns`, `db_url`, `output_dir`, and a
        `load_df(sql)` helper. A snippet reads the cleaned/analytics tables it
        needs and saves one or more PNGs into `output_dir`; reference each saved
        file from the LaTeX via `\\includegraphics{<name>.png}`.

        Raises (returns status="failed") if `latex_content` is missing or if any
        chart snippet errors.
        """
        pass

