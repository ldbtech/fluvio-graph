from pydantic import BaseModel, Field


class PlanContextRequest(BaseModel):
    group_id: str | None = None
    workspace_id: str | None = None
    zone: int = Field(default=0, ge=0)
    domain: str | None = None


class PlanContextResponse(BaseModel):
    plan: str
