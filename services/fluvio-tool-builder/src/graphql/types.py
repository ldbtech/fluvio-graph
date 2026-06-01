import strawberry
from typing import Optional, List

@strawberry.type
class GqlToolParameter:
    name: str
    param_type: str = strawberry.field(name="paramType")
    description: str
    required: bool
    default_value: Optional[str] = strawberry.field(name="defaultValue", default=None)

@strawberry.type
class GqlTool:
    id: str
    name: str
    description: str
    category: str
    is_data_science: bool = strawberry.field(name="isDataScience")
    parameters: List[GqlToolParameter]

@strawberry.type
class GqlToolRun:
    id: str
    tool_id: str = strawberry.field(name="toolId")
    tool_name: str = strawberry.field(name="toolName")
    status: str
    inputs: str
    output: Optional[str] = None
    logs: Optional[str] = None
    started_at: str = strawberry.field(name="startedAt")
    finished_at: Optional[str] = strawberry.field(name="finishedAt", default=None)
    duration_ms: Optional[int] = strawberry.field(name="durationMs", default=None)

@strawberry.type
class GqlSandboxContainerStatus:
    name: str
    component: str
    status: str
    image: str
    ports: List[str]
    arn: Optional[str] = strawberry.field(default=None)
    cost_hourly: Optional[float] = strawberry.field(name="costHourly", default=0.0)
    efficiency_score: Optional[float] = strawberry.field(name="efficiencyScore", default=1.0)

@strawberry.type
class GqlSandboxStatus:
    sandbox_id: str = strawberry.field(name="sandboxId")
    status: str
    containers: List[GqlSandboxContainerStatus]
    provider: Optional[str] = strawberry.field(default="docker")
    cost_hourly: Optional[float] = strawberry.field(name="costHourly", default=0.0)
    efficiency_score: Optional[float] = strawberry.field(name="efficiencyScore", default=1.0)
    agent_twin_monitored: Optional[bool] = strawberry.field(name="agentTwinMonitored", default=False)
