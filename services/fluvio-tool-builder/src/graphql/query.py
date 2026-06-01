import strawberry
from typing import Optional, List
from src.graphql.types import GqlTool, GqlToolParameter, GqlToolRun, GqlSandboxStatus, GqlSandboxContainerStatus

# In-memory store for runs (singleton)
tool_runs_store: List[GqlToolRun] = []

# Predefined available tools (mocks)
mock_tools = [
    GqlTool(
        id="data-cleaning",
        name="Data Cleaning Processor",
        description="Cleans, normalizes, and standardizes database tables and columns.",
        category="engineering",
        is_data_science=False,
        parameters=[
            GqlToolParameter(name="table_name", param_type="String", description="Name of database table to clean", required=True),
            GqlToolParameter(name="operations", param_type="String", description="Operations to perform (e.g. normalize_headers,drop_null_emails,standardize_currency)", required=False, default_value="normalize_headers,drop_null_emails,standardize_currency"),
        ]
    ),
    GqlTool(
        id="spark-analysis",
        name="Spark Analyst Engine",
        description="Executes high-volume distributed SQL analytical queries and calculations.",
        category="analytics",
        is_data_science=False,
        parameters=[
            GqlToolParameter(name="query", param_type="String", description="The SQL query to execute in Spark environment", required=True),
            GqlToolParameter(name="output_table", param_type="String", description="Target database table to write the aggregate results to", required=True),
        ]
    ),
    GqlTool(
        id="model-training",
        name="Model Training Orchestrator",
        description="Trains machine learning models (XGBoost, Regression, etc.) on historical features.",
        category="ml",
        is_data_science=True,
        parameters=[
            GqlToolParameter(name="model_type", param_type="String", description="Type of model to train (e.g. xgboost, linear_regression)", required=True, default_value="xgboost"),
            GqlToolParameter(name="features", param_type="String", description="Comma-separated columns to use as features", required=True),
            GqlToolParameter(name="target", param_type="String", description="Column name representing target variable to predict", required=True),
            GqlToolParameter(name="hyperparameters", param_type="String", description="JSON string of hyperparameters (e.g. learning_rate, max_depth)", required=False),
        ]
    ),
    GqlTool(
        id="dashboard-syncer",
        name="Executive Dashboard Publisher",
        description="Syncs aggregate data tables directly to BI dashboard assets (PowerBI or Tableau).",
        category="bi",
        is_data_science=False,
        parameters=[
            GqlToolParameter(name="dashboard_type", param_type="String", description="BI platform target: 'powerbi' or 'tableau'", required=True),
            GqlToolParameter(name="dataset_name", param_type="String", description="Name of the dashboard dataset or workbook to publish", required=True),
            GqlToolParameter(name="workspace_name", param_type="String", description="Name of destination workspace or project folder", required=True),
        ]
    )
]

@strawberry.type
class Query:
    @strawberry.field(description="List all available tools.")
    def available_tools(self) -> List[GqlTool]:
        from src.tools.registry import registry
        dynamic_tools = registry.get_all_tools()
        dynamic_ids = {t.id for t in dynamic_tools}
        
        # Merge: dynamic tools override any mock tools with the same ID
        all_tools = list(dynamic_tools)
        for mock in mock_tools:
            if mock.id not in dynamic_ids:
                all_tools.append(mock)
                
        return all_tools

    @strawberry.field(description="Retrieve the execution history, optionally filtered by tool ID.")
    def tool_execution_history(self, tool_id: Optional[str] = None) -> List[GqlToolRun]:
        if tool_id:
            return [run for run in tool_runs_store if run.tool_id == tool_id]
            
        return tool_runs_store

    @strawberry.field(description="List all active sandboxes.")
    async def list_sandboxes(self) -> List[GqlSandboxStatus]:
        from src.sandbox.orchestrator import orchestrator
        res = await orchestrator.list_sandboxes()
        return [
            GqlSandboxStatus(
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
            ) for s in res
        ]

    @strawberry.field(description="Retrieve the status of a specific sandbox.")
    async def get_sandbox_status(self, sandbox_id: str) -> GqlSandboxStatus:
        from src.sandbox.orchestrator import orchestrator
        s = await orchestrator.get_sandbox_status(sandbox_id)
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
