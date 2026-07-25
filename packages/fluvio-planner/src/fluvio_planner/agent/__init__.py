"""Agent designation — turns a raw chat request into a *named agent* that owns
the session end to end.

Flow (see routers/chat.py):
    1.  Company knowledge-graph context is assembled first, every time.
    2.  `designate()` matches the task to a role and picks an agent identity
        (name, voice, personality) from the roster.
    3.  The designated agent authors a PLAN.md as its first act, grounded in the
        company context and the real workspace schema.
    4.  The user reviews / edits / approves the plan; on "go" the existing step
        formulation + execute pipeline runs.

The digital twin (fluvio-twin) is the meta-orchestrator above this: it spawns
agents on the user's behalf and briefs the user on all active sessions. Each
agent here is the twin's "junior developer" for one specific task.
"""

from fluvio_planner.agent.profiles import AGENT_POOL, AgentProfile, select_agent
from fluvio_planner.agent.designation import AgentSession, designate

__all__ = [
    "AGENT_POOL",
    "AgentProfile",
    "select_agent",
    "AgentSession",
    "designate",
]
