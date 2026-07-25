"""fluvio-planner: the agent-planner's reusable, transport-free domain layer.

Everything here is import-time environment-free — importing this package never
reads env vars or constructs the service's Settings. Runtime configuration is
injected via `PlannerConfig` (see `fluvio_planner.planner_config`). The FastAPI
service in `services/agent-planner` is one consumer; another process (e.g.
FounderTwin) can embed this package directly, even two differently-configured
planners at once.
"""

from fluvio_planner.plan.orchestrator import generate_plan_context

__all__ = ["generate_plan_context"]
