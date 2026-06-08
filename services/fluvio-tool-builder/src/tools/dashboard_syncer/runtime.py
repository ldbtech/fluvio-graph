import asyncio
import datetime
import httpx
import logging
import uuid
from typing import Dict, List, Any, Optional
from src.tools.dashboard_syncer.contracts import (
    DashboardSyncerTool,
    DashboardExecutionContext
)
from src.config import REPORTS_DIR, REPORTS_BASE_URL

logger = logging.getLogger("dashboard-syncer-runtime")


def _derive_db_name(db_url: Optional[str]) -> str:
    """Derive a database name from a connection URL, or a neutral default."""
    from urllib.parse import urlparse
    if db_url:
        try:
            name = urlparse(db_url).path.lstrip("/")
            if name:
                return name
        except Exception:
            pass
    return "report"


def _today() -> str:
    return datetime.date.today().strftime("%B %d, %Y")

class DashboardSyncerRuntime(DashboardSyncerTool):
    """
    BI Dashboard Publisher runtime client supporting PowerBI Service and Tableau Cloud REST APIs.
    """

    async def _get_azure_token(self, context: DashboardExecutionContext) -> Optional[str]:
        if context.client_id and context.client_secret and context.tenant_id:
            url = f"https://login.microsoftonline.com/{context.tenant_id}/oauth2/v2.0/token"
            data = {
                "grant_type": "client_credentials",
                "client_id": context.client_id,
                "client_secret": context.client_secret,
                "scope": "https://analysis.windows.net/powerbi/api/.default"
            }
            logger.info("Requesting Azure Active Directory token for PowerBI access...")
            async with httpx.AsyncClient() as client:
                try:
                    resp = await client.post(url, data=data, timeout=10.0)
                    if resp.status_code == 200:
                        res = resp.json()
                        token = res.get("access_token")
                        logger.info("Successfully acquired Azure AD access token.")
                        return token
                    else:
                        logger.error(f"Azure AD token request failed (status {resp.status_code}): {resp.text}")
                except Exception as e:
                    logger.error(f"Failed to fetch Azure AD token: {e}")
        return context.api_token

    async def _get_tableau_token(self, context: DashboardExecutionContext) -> Optional[str]:
        logger.info(f"Tableau signin requested with Server: {context.tableau_server_url}, Site ID: {context.workspace_id}, Token Name: {context.tableau_token_name}")
        if context.tableau_token_name and context.tableau_token_value:
            server = context.tableau_server_url or "10ax.online.tableau.com"
            server = server.replace("https://", "").replace("http://", "").split("/")[0]
            url = f"https://{server}/api/3.19/auth/signin"
            payload = {
                "credentials": {
                    "personalAccessTokenName": context.tableau_token_name,
                    "personalAccessTokenSecret": context.tableau_token_value,
                    "site": {
                        "contentUrl": context.workspace_id
                    }
                }
            }
            masked_secret = f"{context.tableau_token_value[:6]}...{context.tableau_token_value[-6:]}" if context.tableau_token_value else "None"
            logger.info(f"Signin details - Name: '{context.tableau_token_name}', Secret: '{masked_secret}', URL: '{url}'")
            async with httpx.AsyncClient() as client:
                try:
                    resp = await client.post(
                        url, 
                        json=payload, 
                        headers={"Content-Type": "application/json", "Accept": "application/json"},
                        timeout=10.0
                    )
                    if resp.status_code == 200:
                        res = resp.json()
                        token = res.get("credentials", {}).get("token")
                        logger.info("Successfully acquired Tableau Cloud session token.")
                        return token
                    else:
                        logger.error(f"Tableau signin failed (status {resp.status_code}): {resp.text}")
                except Exception as e:
                    logger.error(f"Failed to sign into Tableau: {e}")
        return context.api_token

    async def publish_report(
        self,
        context: DashboardExecutionContext,
        report_name: str,
        datasource_name: str,
        file_path: Optional[str] = None
    ) -> Dict[str, Any]:
        platform = context.platform.lower().strip()
        workspace_id = context.workspace_id
        report_id = f"rep-{uuid.uuid4().hex[:8]}"
        dataset_id = f"ds-{uuid.uuid4().hex[:8]}"

        logger.info(f"Publishing {report_name} to {platform.upper()} (workspace: {workspace_id})...")

        # Resolve api token
        if platform == "powerbi":
            api_token = await self._get_azure_token(context)
        elif platform == "tableau":
            api_token = await self._get_tableau_token(context)
        else:
            api_token = context.api_token

        if not api_token:
            return {
                "status": "failed",
                "error": (
                    f"No valid {platform} credentials are available, so the report "
                    "could not be published. Connect the BI account (or check the "
                    "credential_ref) and retry; if this persists, please report it."
                ),
            }

        # REAL REST API CALLS — success or honest failure, never a simulated link.
        async with httpx.AsyncClient() as client:
            try:
                if platform == "powerbi":
                    url = f"https://api.powerbi.com/v1.0/myorg/groups/{workspace_id}/imports?datasetDisplayName={report_name}"
                    headers = {
                        "Authorization": f"Bearer {api_token}",
                        "Content-Type": "multipart/form-data"
                    }
                    files = {"file": open(file_path, "rb")} if file_path else {"file": (report_name, b"PBX_RAW_DATA")}
                    resp = await client.post(url, headers=headers, files=files, timeout=10.0)
                    if resp.status_code in [200, 202]:
                        res = resp.json()
                        return {
                            "status": "success",
                            "report_id": res.get("id", report_id),
                            "dataset_id": res.get("datasetId", dataset_id),
                            "web_url": f"https://app.powerbi.com/groups/{workspace_id}/reports/{res.get('id', report_id)}/ReportSection",
                            "details": "Published successfully via PowerBI REST Imports API."
                        }
                    raise Exception(f"PowerBI API returned status {resp.status_code}: {resp.text}")

                elif platform == "tableau":
                    server = context.tableau_server_url or "10ax.online.tableau.com"
                    server = server.replace("https://", "").replace("http://", "").split("/")[0]
                    url = f"https://{server}/api/3.19/sites/{workspace_id}/workbooks"
                    headers = {
                        "X-Tableau-Auth": api_token,
                        "Content-Type": "application/json"
                    }
                    payload = {"workbook": {"name": report_name, "showTabs": "true"}}
                    resp = await client.post(url, headers=headers, json=payload, timeout=10.0)
                    if resp.status_code in [200, 201]:
                        res = resp.json()
                        wb_id = res.get("workbook", {}).get("id", report_id)
                        return {
                            "status": "success",
                            "report_id": wb_id,
                            "web_url": f"https://{server}/#/site/{workspace_id}/workbooks/{wb_id}/views/Dashboard",
                            "details": "Published successfully via Tableau Cloud Publishing API."
                        }
                    raise Exception(f"Tableau API returned status {resp.status_code}: {resp.text}")

                else:
                    return {"status": "failed", "error": f"Unsupported platform: {platform}"}
            except Exception as e:
                logger.error(f"Failed to publish to {platform} via API: {e}")
                return {
                    "status": "failed",
                    "error": (
                        f"Publishing to {platform} failed: {e}. Retry; if this "
                        "persists, please report it to the Fluviome team."
                    ),
                }

    async def trigger_refresh(
        self,
        context: DashboardExecutionContext,
        dataset_id: str
    ) -> bool:
        platform = context.platform.lower().strip()
        workspace_id = context.workspace_id

        logger.info(f"Triggering refresh for {platform.upper()} model (dataset/workbook ID: {dataset_id})...")

        # Resolve api token
        if platform == "powerbi":
            api_token = await self._get_azure_token(context)
        elif platform == "tableau":
            api_token = await self._get_tableau_token(context)
        else:
            api_token = context.api_token

        if not api_token:
            # No simulated success — honestly report we couldn't refresh.
            logger.error(f"No valid {platform} credentials; cannot trigger refresh.")
            return False

        # REAL REST API CALLS only.
        async with httpx.AsyncClient() as client:
            try:
                if platform == "powerbi":
                    url = f"https://api.powerbi.com/v1.0/myorg/groups/{workspace_id}/datasets/{dataset_id}/refreshes"
                    headers = {
                        "Authorization": f"Bearer {api_token}",
                        "Content-Type": "application/json"
                    }
                    resp = await client.post(url, headers=headers, timeout=10.0)
                    return resp.status_code in [200, 202]
                elif platform == "tableau":
                    server = context.tableau_server_url or "10ax.online.tableau.com"
                    server = server.replace("https://", "").replace("http://", "").split("/")[0]
                    url = f"https://{server}/api/3.19/sites/{workspace_id}/workbooks/{dataset_id}/refresh"
                    headers = {
                        "X-Tableau-Auth": api_token,
                        "Content-Type": "application/json"
                    }
                    resp = await client.post(url, headers=headers, timeout=10.0)
                    return resp.status_code in [200, 202]
                else:
                    logger.error(f"Unsupported platform for refresh: {platform}")
                    return False
            except Exception as e:
                logger.error(f"Error calling BI refresh API: {e}")
                return False

    async def get_share_link(
        self,
        context: DashboardExecutionContext,
        report_id: str
    ) -> str:
        platform = context.platform.lower().strip()
        workspace_id = context.workspace_id

        if platform == "powerbi":
            return f"https://app.powerbi.com/groups/{workspace_id}/reports/{report_id}/ReportSection"
        elif platform == "tableau":
            server = context.tableau_server_url or "10ax.online.tableau.com"
            server = server.replace("https://", "").replace("http://", "").split("/")[0]
            return f"https://{server}/#/site/{workspace_id}/home"
        return ""

    @staticmethod
    def _run_chart_code(
        chart_code: List[str],
        db_url: Optional[str],
        target_dir: str,
    ) -> List[str]:
        """Execute planner-authored seaborn/matplotlib code to produce chart PNGs.

        The planner is the brain: it has already cleaned the data and built the
        `*_analytics` tables, and it knows the company (from the knowledge graph).
        Here it authors the actual plotting program — choosing the chart, the
        columns, the styling/branding — and this tool simply runs it and collects
        the figures it saved.

        Each snippet runs in a namespace pre-bound with:
            pd, np, plt, sns   — the usual plotting stack (matplotlib is headless)
            db_url             — the Postgres connection string
            output_dir         — where to save PNGs (savefig here)
            load_df(sql)       — helper returning a DataFrame for a query

        Returns the list of PNG filenames created. Any snippet error propagates so
        the orchestrator can retry — there is no silent fallback.
        """
        import os
        import numpy as np
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import pandas as pd
        import seaborn as sns

        before = set(f for f in os.listdir(target_dir) if f.lower().endswith(".png"))

        conn_holder: Dict[str, Any] = {}

        def load_df(sql: str):
            if "conn" not in conn_holder:
                import psycopg2
                conn_holder["conn"] = psycopg2.connect(db_url)
            return pd.read_sql(sql, conn_holder["conn"])

        sns.set_theme(style="whitegrid")
        namespace = {
            "pd": pd, "np": np, "plt": plt, "sns": sns,
            "db_url": db_url, "output_dir": target_dir, "load_df": load_df,
            "os": os,
        }

        try:
            for i, code in enumerate(chart_code or []):
                if not code or not str(code).strip():
                    continue
                logger.info("Executing planner chart code block %s...", i)
                exec(compile(code, f"<chart_code_{i}>", "exec"), namespace)
                plt.close("all")  # ensure no figure state leaks between blocks
        finally:
            conn = conn_holder.get("conn")
            if conn is not None:
                conn.close()

        after = set(f for f in os.listdir(target_dir) if f.lower().endswith(".png"))
        rendered = sorted(after - before)
        logger.info("Planner chart code produced %s PNG(s): %s", len(rendered), rendered)
        return rendered

    async def generate_pdf_report(
        self,
        context: DashboardExecutionContext,
        report_name: str,
        database_url: Optional[str] = None,
        latex_content: Optional[str] = None,
        chart_code: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        import os
        import subprocess

        logger.info(f"Generating PDF report: {report_name}")

        db_url = database_url
        db_name = _derive_db_name(db_url)

        target_dir = REPORTS_DIR
        os.makedirs(target_dir, exist_ok=True)

        file_basename = f"{db_name}_executive_report"
        tex_path = os.path.join(target_dir, f"{file_basename}.tex")
        pdf_path = os.path.join(target_dir, f"{file_basename}.pdf")

        # The planner authors the whole document. There is no auto-generated
        # fallback: if the LaTeX is missing, fail so the orchestrator retries.
        if not latex_content:
            return {
                "status": "failed",
                "error": (
                    "latex_content is required. Author the full LaTeX document "
                    "(and the chart_code that produces its figures). No auto-report "
                    "fallback exists."
                ),
            }

        # Run the planner-authored seaborn/matplotlib code first, so the LaTeX
        # the planner wrote (with \includegraphics{...}) resolves against real PNGs.
        rendered_charts: List[str] = []
        if chart_code:
            if not db_url:
                return {
                    "status": "failed",
                    "error": "chart_code provided but no database_url to read analytics from.",
                }
            try:
                rendered_charts = self._run_chart_code(chart_code, db_url, target_dir)
            except Exception as e:
                logger.error("Planner chart code failed: %s", e, exc_info=True)
                return {"status": "failed", "error": f"Chart code execution error: {e}"}

        if latex_content:
            logger.info("Custom LaTeX content provided. Compiling directly...")
            try:
                with open(tex_path, "w") as f:
                    f.write(latex_content)
            except Exception as e:
                logger.error(f"Failed to write .tex file: {e}")

            # Compile to a real PDF using whichever LaTeX engine is installed.
            # These are all real compilers producing real PDFs — not a degraded
            # fallback. If none is present, or compilation fails, we fail honestly
            # (no ReportLab approximation that could misrepresent a client report).
            import shutil

            if shutil.which("tectonic"):
                # tectonic is single-pass, self-contained, fetches packages itself.
                last_proc = subprocess.run(
                    ["tectonic", "--outdir", target_dir, "--keep-logs", f"{file_basename}.tex"],
                    cwd=target_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                )
            elif shutil.which("pdflatex") or shutil.which("xelatex"):
                engine = "pdflatex" if shutil.which("pdflatex") else "xelatex"
                last_proc = None
                for _ in range(2):  # two passes for cross-references
                    last_proc = subprocess.run(
                        [engine, "-interaction=nonstopmode", f"{file_basename}.tex"],
                        cwd=target_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    )
            else:
                return {
                    "status": "failed",
                    "error": (
                        "No LaTeX engine (tectonic / pdflatex / xelatex) is installed, "
                        "so the PDF could not be compiled. Please report this to the "
                        "Fluviome team so we can provision LaTeX."
                    ),
                }

            if not os.path.exists(pdf_path):
                err = (last_proc.stderr.decode().strip() if last_proc else "")[:800]
                return {
                    "status": "failed",
                    "error": (
                        "LaTeX compilation failed. Fix the document and retry; if "
                        f"this persists, please report it. Compiler output: {err}"
                    ),
                }

            web_url = f"{REPORTS_BASE_URL}/{file_basename}.pdf"
            return {
                "status": "success",
                "report_name": report_name,
                "web_url": web_url,
                "tex_path": tex_path,
                "pdf_path": pdf_path,
                "charts_rendered": rendered_charts,
                "details": f"Generated PDF report from agent-authored LaTeX with {len(rendered_charts)} planner-authored chart(s).",
            }
