import asyncio
import logging
from typing import Dict, Any, Optional
from src.tools.dbt.contracts import (
    DbtTool,
    DbtExecutionContext
)

logger = logging.getLogger("dbt-runtime")

class DbtRuntime(DbtTool):
    """
    dbt tool implementation. Executes dbt actions inside sandbox containers.
    """

    @staticmethod
    async def _run_dbt(cmd: list, action: str) -> Dict[str, Any]:
        """Execute a dbt CLI command and return the real result — no simulation."""
        try:
            logger.info("Running dbt %s: %s", action, " ".join(cmd))
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
        except Exception as e:
            logger.error("Error executing dbt %s: %s", action, e)
            return {"status": "failed", "error": str(e)}

        output = stdout.decode().strip()
        if proc.returncode != 0:
            err = stderr.decode().strip() or output
            logger.error("dbt %s failed: %s", action, err)
            return {"status": "failed", "command": " ".join(cmd), "error": err}

        return {"status": "success", "command": " ".join(cmd), "output": output}

    async def run_models(
        self,
        context: DbtExecutionContext,
        select: Optional[str] = None,
        exclude: Optional[str] = None
    ) -> Dict[str, Any]:
        if not context.sandbox_id:
            return {"status": "failed", "error": "dbt runs only inside a sandbox container; no sandbox_id provided."}

        container_name = f"fluvio-sandbox-{context.sandbox_id}-dbt"
        cmd = [
            "docker", "exec", container_name,
            "dbt", "run",
            "--project-dir", context.project_dir,
            "--profile", context.profile_name,
            "--target", context.target_name,
        ]
        if select:
            cmd.extend(["--select", select])
        if exclude:
            cmd.extend(["--exclude", exclude])
        return await self._run_dbt(cmd, "run")

    async def test_models(
        self,
        context: DbtExecutionContext,
        select: Optional[str] = None
    ) -> Dict[str, Any]:
        if not context.sandbox_id:
            return {"status": "failed", "error": "dbt runs only inside a sandbox container; no sandbox_id provided."}

        container_name = f"fluvio-sandbox-{context.sandbox_id}-dbt"
        cmd = [
            "docker", "exec", container_name,
            "dbt", "test",
            "--project-dir", context.project_dir,
        ]
        if select:
            cmd.extend(["--select", select])
        return await self._run_dbt(cmd, "test")

    async def compile_project(
        self,
        context: DbtExecutionContext
    ) -> Dict[str, Any]:
        if not context.sandbox_id:
            return {"status": "failed", "error": "dbt runs only inside a sandbox container; no sandbox_id provided."}

        container_name = f"fluvio-sandbox-{context.sandbox_id}-dbt"
        cmd = [
            "docker", "exec", container_name,
            "dbt", "compile",
            "--project-dir", context.project_dir,
        ]
        return await self._run_dbt(cmd, "compile")
