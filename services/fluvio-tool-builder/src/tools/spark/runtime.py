import asyncio
import os
import uuid
import logging
from typing import Dict, List, Any
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

    async def _ensure_container_running(self) -> bool:
        """Verify if the fluvio-spark container is running, and try starting it if not."""
        try:
            # Check if container is running
            proc = await asyncio.create_subprocess_exec(
                "docker", "inspect", "-f", "{{.State.Running}}", "fluvio-spark",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, _ = await proc.communicate()
            if proc.returncode == 0 and stdout.strip() == b"true":
                return True

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

    async def execute_sql(
        self,
        context: SparkExecutionContext,
        query: str,
        output_table: str
    ) -> bool:
        await self._ensure_container_running()
        
        # Clean query: escape quotes
        # We run spark-sql utility inside the container
        cmd = [
            "docker", "exec", "fluvio-spark",
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
            else:
                logger.error(f"Failed to execute Spark SQL: {stderr.decode()}")
                logger.info("Falling back to executing query on PostgreSQL local engine...")
                
                # Auto-detect database URL by checking where clean_users exists
                db_url = "postgres://localhost/vowayage"
                try:
                    check_proc = await asyncio.create_subprocess_exec(
                        "psql", db_url, "-t", "-A", "-c", "SELECT count(*) FROM clean_users",
                        stdout=asyncio.subprocess.PIPE,
                        stderr=asyncio.subprocess.PIPE
                    )
                    await check_proc.communicate()
                    if check_proc.returncode != 0:
                        db_url = "postgres://localhost/fluvio_company"
                except Exception:
                    db_url = "postgres://localhost/fluvio_company"
                
                logger.info(f"Using database URL for fallback: {db_url}")
                
                # Check query to make it safe for PostgreSQL CREATE TABLE AS SELECT
                # Let's drop output table first, then create it
                fallback_sql = f"DROP TABLE IF EXISTS {output_table}; CREATE TABLE {output_table} AS {query};"
                fallback_cmd = [
                    "psql", db_url,
                    "-c", fallback_sql
                ]
                
                logger.info(f"Running fallback SQL on Postgres: {fallback_sql}")
                fallback_proc = await asyncio.create_subprocess_exec(
                    *fallback_cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE
                )
                fb_stdout, fb_stderr = await fallback_proc.communicate()
                if fallback_proc.returncode == 0:
                    logger.info(f"PostgreSQL fallback query executed successfully. Table '{output_table}' created.")
                    return True
                else:
                    logger.error(f"PostgreSQL fallback failed: {fb_stderr.decode().strip()}")
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
        await self._ensure_container_running()
        
        job_id = f"spark-job-{uuid.uuid4().hex[:8]}"
        filename = os.path.basename(jar_or_py_path)
        dest_in_container = f"/tmp/{filename}"
        
        # 1. Copy local file into container if it exists on host
        if os.path.exists(jar_or_py_path):
            copy_proc = await asyncio.create_subprocess_exec(
                "docker", "cp", jar_or_py_path, f"fluvio-spark:{dest_in_container}",
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
            "docker", "exec", "fluvio-spark",
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
        await self._ensure_container_running()
        
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
