import asyncio
import logging
import uuid
import json
import re
from typing import Dict, Any, Optional
from src.tools.airflow.contracts import (
    AirflowTool,
    AirflowExecutionContext
)

logger = logging.getLogger("airflow-runtime")

class AirflowRuntime(AirflowTool):
    """
    Airflow tool implementation. Triggers and monitors DAGs inside sandbox containers.
    """

    async def trigger_dag(
        self,
        context: AirflowExecutionContext,
        dag_id: str,
        conf: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        if not context.sandbox_id:
            return {
                "status": "failed",
                "error": "Airflow runs only inside a sandbox container; no sandbox_id provided.",
            }

        container_name = f"fluvio-sandbox-{context.sandbox_id}-airflow"
        cmd = ["docker", "exec", container_name, "airflow", "dags", "trigger", dag_id]
        if conf:
            cmd.extend(["--conf", json.dumps(conf)])

        try:
            logger.info(f"Triggering Airflow DAG '{dag_id}' inside sandbox container '{container_name}'...")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
        except Exception as e:
            logger.error(f"Error executing Airflow CLI in container: {e}")
            return {"status": "failed", "dag_id": dag_id, "error": str(e)}

        if proc.returncode != 0:
            err = stderr.decode().strip()
            logger.error(f"Airflow DAG trigger failed: {err}")
            return {"status": "failed", "dag_id": dag_id, "error": err}

        output = stdout.decode().strip()
        logger.info(f"Airflow CLI output: {output}")
        run_id_match = re.search(r"created:\s+([^\s,]+)", output)
        run_id = run_id_match.group(1) if run_id_match else f"manual__{uuid.uuid4()}"
        return {
            "status": "success",
            "dag_id": dag_id,
            "run_id": run_id,
            "state": "queued",
            "output": output,
        }

    async def get_dag_run_status(
        self,
        context: AirflowExecutionContext,
        dag_id: str,
        run_id: str
    ) -> Dict[str, Any]:
        if not context.sandbox_id:
            return {
                "status": "failed",
                "error": "Airflow runs only inside a sandbox container; no sandbox_id provided.",
            }

        container_name = f"fluvio-sandbox-{context.sandbox_id}-airflow"
        cmd = ["docker", "exec", container_name, "airflow", "dags", "state", dag_id, run_id]

        try:
            logger.info(f"Checking state for DAG run '{run_id}' inside sandbox container '{container_name}'...")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
        except Exception as e:
            logger.error(f"Error querying Airflow CLI in container: {e}")
            return {"status": "failed", "dag_id": dag_id, "run_id": run_id, "error": str(e)}

        if proc.returncode != 0:
            err = stderr.decode().strip()
            logger.error(f"Airflow DAG state query failed: {err}")
            return {"status": "failed", "dag_id": dag_id, "run_id": run_id, "error": err}

        return {
            "status": "success",
            "dag_id": dag_id,
            "run_id": run_id,
            "state": stdout.decode().strip(),
        }
