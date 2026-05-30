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
