from typing import Optional, List
import strawberry
import uuid
import json
import datetime
from src.graphql.types import GqlToolRun, GqlSandboxStatus, GqlSandboxContainerStatus
from src.graphql.query import tool_runs_store, mock_tools

@strawberry.type
class Mutation:
    @strawberry.mutation(description="Executes a tool with the provided input parameters.")
    async def execute_tool(
        self,
        tool_id: str,
        inputs: str,
    ) -> GqlToolRun:
        from src.tools.registry import registry
        # Resolve tool (either from dynamic registry or mock list)
        dynamic_tools = registry.get_all_tools()
        tool = next((t for t in dynamic_tools if t.id == tool_id), None)
        is_dynamic = tool is not None
        if not tool:
            tool = next((t for t in mock_tools if t.id == tool_id), None)
            
        if not tool:
            raise Exception(f"Tool '{tool_id}' not found.")

        # Parse inputs JSON if possible for logging
        try:
            parsed_inputs = json.loads(inputs)
        except Exception:
            parsed_inputs = {}

        # Draft platform-specific mock logs & output
        started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
        
        logs_lines = [
            f"[{started_at}] Launching tool: {tool.name} ({tool_id})",
            f"[{started_at}] Inputs: {inputs}",
        ]
        
        output_data = {}
        status = "completed"

        if is_dynamic:
            try:
                run_res = await registry.execute_tool_action(tool_id, inputs, logs_lines)
                if run_res.get("status") == "success":
                    output_data = run_res.get("result", {})
                else:
                    status = "failed"
                    output_data = {"error": run_res.get("error")}
            except Exception as e:
                status = "failed"
                output_data = {"error": str(e)}
                logs_lines.append(f"Execution failed: {e}")
        else:
            if tool_id == "data-cleaning":
                table = parsed_inputs.get("table_name", "raw_data")
                ops = parsed_inputs.get("operations", "all")
                logs_lines.extend([
                    f"Connecting to database read-replica for table: {table}...",
                    f"Operations selected for cleansing: [{ops}]",
                    "Running Rule 1: Normalizing column header casing to snake_case...",
                    "Running Rule 2: Dropping rows with null customer email identifiers...",
                    "Running Rule 3: Performing currency exchange rate conversions to base USD...",
                    "Cleansing completed. Rows processed: 1420. Null values purged: 12. Standardized: 100%.",
                    f"Cleansed data successfully written back to clean_{table}."
                ])
                output_data = {
                    "status": "success",
                    "processed_rows": 1420,
                    "purged_nulls": 12,
                    "output_table": f"clean_{table}"
                }
            elif tool_id == "spark-analysis":
                query_str = parsed_inputs.get("query", "SELECT *")
                out_table = parsed_inputs.get("output_table", "metrics")
                logs_lines.extend([
                    "Initializing SparkSession on distributed Yarn cluster...",
                    f"Compiling Spark SQL optimization logical plan for: {query_str[:60]}...",
                    "Spark job submitted. Executing map-reduce tasks across 6 worker nodes...",
                    "Aggregating campaign metrics (SUM spend, SUM clicks, calculated ROAS)...",
                    "Calculated results partition successfully gathered at driver node.",
                    f"Writing 14 aggregate rows back to target schema table: {out_table}."
                ])
                output_data = {
                    "status": "success",
                    "calculated_rows": 14,
                    "average_roas": "3.28x",
                    "target_table": out_table
                }
            elif tool_id == "model-training":
                mtype = parsed_inputs.get("model_type", "xgboost")
                features = parsed_inputs.get("features", "")
                target = parsed_inputs.get("target", "")
                logs_lines.extend([
                    f"Loading feature engineering pipeline for target: {target}...",
                    f"Feature columns: [{features}]",
                    f"Initializing {mtype} classifier model training...",
                    "Standardizing features and performing train-test split (80/20)...",
                    "Fitting decision tree boosting iterations (round 100/500)...",
                    "Fitting decision tree boosting iterations (round 300/500)...",
                    "Training complete. Calculating evaluation suite metrics...",
                    "Evaluation MAPE (Mean Absolute Percentage Error): 7.82%",
                    "R2 score: 0.941",
                    "Serializing model artifact to storage path '/models/marketing_predictor_v1.bin'"
                ])
                output_data = {
                    "status": "success",
                    "mape": 0.0782,
                    "r2_score": 0.941,
                    "model_path": "/models/marketing_predictor_v1.bin"
                }
            elif tool_id == "dashboard-syncer":
                dtype = parsed_inputs.get("dashboard_type", "powerbi")
                dataset = parsed_inputs.get("dataset_name", "roi")
                workspace = parsed_inputs.get("workspace_name", "main")
                logs_lines.extend([
                    f"Authenticating with {dtype} cloud gateway API...",
                    f"Locating workspace: '{workspace}'...",
                    f"Pushing updated datasets schema for: '{dataset}'...",
                    "Refreshing live visuals and cache dashboards...",
                    f"Dashboard refresh triggered successfully in {dtype}."
                ])
                output_data = {
                    "status": "success",
                    "dashboard_type": dtype,
                    "refreshed_at": datetime.datetime.now(datetime.timezone.utc).isoformat()
                }
            else:
                logs_lines.extend([
                    "Executing generic tool operations...",
                    "Operations completed successfully."
                ])
                output_data = {"status": "success"}

        finished_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
        logs_lines.append(f"[{finished_at}] Execution complete.")

        run = GqlToolRun(
            id=str(uuid.uuid4()),
            tool_id=tool_id,
            tool_name=tool.name,
            status=status,
            inputs=inputs,
            output=json.dumps(output_data),
            logs="\n".join(logs_lines),
            started_at=started_at,
            finished_at=finished_at,
            duration_ms=1200 # Simulated 1.2s duration
        )

        tool_runs_store.append(run)
        return run

    @strawberry.mutation(description="Create and start a new containerized sandbox.")
    async def create_sandbox(
        self,
        sandbox_id: str,
        components: Optional[List[str]] = None,
        provider: Optional[str] = "docker"
    ) -> GqlSandboxStatus:
        from src.sandbox.orchestrator import orchestrator
        s = await orchestrator.create_sandbox(sandbox_id, components=components, provider=provider)
        return GqlSandboxStatus(
            sandbox_id=s["sandbox_id"],
            status=s["status"],
            provider=s.get("provider", "docker"),
            cost_hourly=s.get("cost_hourly", 0.0),
            efficiency_score=s.get("efficiency_score", 1.0),
            agent_twin_monitored=s.get("agent_twin_monitored", False),
            containers=[
                GqlSandboxContainerStatus(
                    name=c["name"],
                    component=c["component"],
                    status=c["status"],
                    image=c["image"],
                    ports=c["ports"],
                    arn=c.get("arn"),
                    cost_hourly=c.get("cost_hourly", 0.0),
                    efficiency_score=c.get("efficiency_score", 1.0)
                ) for c in s["containers"]
            ]
        )

    @strawberry.mutation(description="Stop all active containers for a sandbox.")
    async def stop_sandbox(self, sandbox_id: str) -> GqlSandboxStatus:
        from src.sandbox.orchestrator import orchestrator
        s = await orchestrator.stop_sandbox(sandbox_id)
        return GqlSandboxStatus(
            sandbox_id=s["sandbox_id"],
            status=s["status"],
            provider=s.get("provider", "docker"),
            cost_hourly=s.get("cost_hourly", 0.0),
            efficiency_score=s.get("efficiency_score", 1.0),
            agent_twin_monitored=s.get("agent_twin_monitored", False),
            containers=[
                GqlSandboxContainerStatus(
                    name=c["name"],
                    component=c["component"],
                    status=c["status"],
                    image=c["image"],
                    ports=c["ports"],
                    arn=c.get("arn"),
                    cost_hourly=c.get("cost_hourly", 0.0),
                    efficiency_score=c.get("efficiency_score", 1.0)
                ) for c in s["containers"]
            ]
        )

    @strawberry.mutation(description="Clean up and remove all containers and networks for a sandbox.")
    async def clean_sandbox(self, sandbox_id: str) -> bool:
        from src.sandbox.orchestrator import orchestrator
        return await orchestrator.clean_sandbox(sandbox_id)
