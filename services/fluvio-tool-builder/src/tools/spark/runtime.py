import asyncio
import os
import uuid
import logging
from typing import Dict, List, Any, Optional
from src.tools.spark.contracts import (
    SparkTool,
    SparkExecutionContext,
    SparkJobConfig
)

logger = logging.getLogger("spark-runtime")

# Simple in-memory tracker for submitted jobs (simulate driver process IDs)
submitted_jobs: Dict[str, Dict[str, Any]] = {}

class SparkRuntime(SparkTool):
    """
    Spark Tool implementation that executes Spark commands inside a local Docker container.
    """

    async def _ensure_container_running(self, sandbox_id: Optional[str] = None) -> bool:
        """Verify if the Spark container is running, and try starting it if not."""
        container_name = f"fluvio-sandbox-{sandbox_id}-spark" if sandbox_id else "fluvio-spark"
        try:
            # Check if container is running
            proc = await asyncio.create_subprocess_exec(
                "docker", "inspect", "-f", "{{.State.Running}}", container_name,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, _ = await proc.communicate()
            if proc.returncode == 0 and stdout.strip() == b"true":
                return True

            if sandbox_id:
                logger.info(f"Sandbox container {container_name} is not running. Attempting to start it...")
                start_proc = await asyncio.create_subprocess_exec(
                    "docker", "start", container_name,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE
                )
                await start_proc.communicate()
                if start_proc.returncode == 0:
                    await asyncio.sleep(2)
                    return True
                return False

            logger.info("fluvio-spark container is not running. Attempting to start it...")
            # Try running docker-compose up
            compose_dir = os.path.dirname(os.path.abspath(__file__))
            compose_path = os.path.join(compose_dir, "docker", "docker-compose.yaml")
            
            start_proc = await asyncio.create_subprocess_exec(
                "docker", "compose", "-f", compose_path, "up", "-d",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            await start_proc.communicate()
            if start_proc.returncode == 0:
                logger.info("Successfully started fluvio-spark container.")
                # Give it a moment to initialize
                await asyncio.sleep(5)
                return True
        except Exception as e:
            logger.error(f"Error checking/starting docker container: {e}")
        return False

    async def _execute_sql_postgres(self, db_url: str, query: str, output_table: str) -> bool:
        """Execute the analytical SQL directly against Postgres (the local SQL
        engine) and materialize ``output_table`` there — the same store the report
        charts read. Raises on error so the real DB message surfaces honestly."""
        import re as _re
        import psycopg2  # type: ignore

        pg_url = _re.sub(r"^postgres://", "postgresql://", db_url)
        safe_table = _re.sub(r"[^A-Za-z0-9_]", "", output_table) or "metrics"

        def _run() -> int:
            conn = psycopg2.connect(pg_url)
            conn.autocommit = True
            try:
                with conn.cursor() as cur:
                    cur.execute(f'DROP TABLE IF EXISTS "{safe_table}"')
                    cur.execute(f'CREATE TABLE "{safe_table}" AS {query}')
                    cur.execute(f'SELECT COUNT(*) FROM "{safe_table}"')
                    return cur.fetchone()[0]
            finally:
                conn.close()

        rows = await asyncio.to_thread(_run)
        logger.info("Spark(local SQL) wrote %s row(s) to %s.", rows, safe_table)
        return True

    async def execute_sql(
        self,
        context: SparkExecutionContext,
        query: str,
        output_table: str
    ) -> bool:
        # When a Postgres connection is supplied, run against the local SQL engine
        # so the materialized table lands in the queryable warehouse the rest of
        # the pipeline (and report charts) read. This is the documented fallback.
        if context.database_url:
            return await self._execute_sql_postgres(context.database_url, query, output_table)

        await self._ensure_container_running(context.sandbox_id)
        container_name = f"fluvio-sandbox-{context.sandbox_id}-spark" if context.sandbox_id else "fluvio-spark"
        
        # Clean query: escape quotes
        # We run spark-sql utility inside the container
        cmd = [
            "docker", "exec", container_name,
            "spark-sql", "--name", context.app_name,
            "-e", f"{query}; CREATE TABLE IF NOT EXISTS {output_table} AS {query}"
        ]
        
        try:
            logger.info(f"Executing Spark SQL query: {query}")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                logger.info(f"Query executed successfully and written to {output_table}.")
                return True
            # No silent fallback to another engine: fail honestly so the
            # orchestrator retries and, if it persists, the user reports it.
            logger.error(
                "Spark SQL execution failed for output '%s': %s",
                output_table, stderr.decode().strip(),
            )
            return False
        except Exception as e:
            logger.error(f"Exception during Spark SQL query execution: {e}")
            return False

    async def submit_job(
        self,
        context: SparkExecutionContext,
        jar_or_py_path: str,
        config: SparkJobConfig
    ) -> str:
        await self._ensure_container_running(context.sandbox_id)
        container_name = f"fluvio-sandbox-{context.sandbox_id}-spark" if context.sandbox_id else "fluvio-spark"
        
        job_id = f"spark-job-{uuid.uuid4().hex[:8]}"
        filename = os.path.basename(jar_or_py_path)
        dest_in_container = f"/tmp/{filename}"
        
        # 1. Copy local file into container if it exists on host
        if os.path.exists(jar_or_py_path):
            copy_proc = await asyncio.create_subprocess_exec(
                "docker", "cp", jar_or_py_path, f"{container_name}:{dest_in_container}",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            await copy_proc.communicate()
            if copy_proc.returncode != 0:
                logger.warning(f"Could not copy {jar_or_py_path} to container. Running directly using path...")
            else:
                jar_or_py_path = dest_in_container

        # 2. Build spark-submit command
        cmd = [
            "docker", "exec", container_name,
            "spark-submit",
            "--master", context.master_url,
            "--name", context.app_name
        ]
        
        if config.main_class:
            cmd.extend(["--class", config.main_class])
        if config.executor_memory:
            cmd.extend(["--executor-memory", config.executor_memory])
        if config.driver_memory:
            cmd.extend(["--driver-memory", config.driver_memory])
            
        for k, v in config.conf.items():
            cmd.extend(["--conf", f"{k}={v}"])
            
        cmd.append(jar_or_py_path)
        cmd.extend(config.app_args)
        
        # 3. Submit background process
        try:
            logger.info(f"Submitting Spark job: {job_id}")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            # Store process ref to monitor status
            submitted_jobs[job_id] = {
                "process": proc,
                "status": "RUNNING",
                "app_name": context.app_name
            }
            
            # Non-blocking helper task to monitor completion
            async def monitor():
                stdout, stderr = await proc.communicate()
                if proc.returncode == 0:
                    submitted_jobs[job_id]["status"] = "FINISHED"
                else:
                    submitted_jobs[job_id]["status"] = "FAILED"
                    submitted_jobs[job_id]["error"] = stderr.decode()
            
            asyncio.create_task(monitor())
            return job_id
        except Exception as e:
            logger.error(f"Failed to submit Spark job: {e}")
            submitted_jobs[job_id] = {"status": "FAILED", "error": str(e)}
            return job_id

    async def get_job_status(
        self,
        context: SparkExecutionContext,
        job_id: str
    ) -> Dict[str, Any]:
        await self._ensure_container_running(context.sandbox_id)
        
        # Check in memory first
        if job_id in submitted_jobs:
            job_info = submitted_jobs[job_id]
            return {
                "job_id": job_id,
                "status": job_info["status"],
                "app_name": job_info["app_name"],
                "error": job_info.get("error")
            }
            
        # Or query Spark UI REST API as a fallback if master is localhost:8080
        # Since it runs inside docker, we query http://localhost:8080/api/v1/applications
        import httpx
        try:
            async with httpx.AsyncClient() as client:
                resp = await client.get("http://localhost:8080/api/v1/applications", timeout=2.0)
                if resp.status_code == 200:
                    apps = resp.json()
                    for app in apps:
                        if app.get("id") == job_id or app.get("name") == context.app_name:
                            attempts = app.get("attempts", [{}])
                            completed = attempts[0].get("completed", False)
                            return {
                                "job_id": job_id,
                                "status": "FINISHED" if completed else "RUNNING",
                                "app_name": app.get("name"),
                                "duration": attempts[0].get("duration", 0)
                            }
        except Exception:
            pass
            
        return {
            "job_id": job_id,
            "status": "UNKNOWN",
            "message": "Job ID not found in local active tracker or Spark API catalog."
        }
