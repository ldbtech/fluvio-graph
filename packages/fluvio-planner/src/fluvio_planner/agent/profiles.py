"""Agent identity roster.

A fixed pool of agent personas. Each persona has a distinct name, role, voice,
and personality so that — across sessions — agents feel like different people.
The twin (the user's own AI) speaks in the user's voice; designated agents each
speak in their own. The `voice_id` is a forward-looking handle for the TTS layer
(not wired to an engine yet); name + personality already shape the text replies.

Agent selection is deterministic and grounded in the *task*, never random: the
words in the request map to a role, the role maps to a persona. The same kind of
request always summons the same kind of agent, which makes the system legible to
the user ("Aria handles my pipelines").
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


@dataclass(frozen=True)
class AgentProfile:
    name: str
    role: str
    voice_id: str          # handle for the TTS layer; distinct per agent
    personality: str       # shapes tone of the text + audio reply
    # Lowercased keywords in the user's request that summon this role.
    triggers: tuple[str, ...] = field(default=())

    def intro_line(self) -> str:
        """First-person one-liner the agent opens its plan with."""
        return f"Hi, I'm {self.name}, your {self.role} for this session."

    def signature(self) -> str:
        """Header chip shown above the agent's messages in chat."""
        return f"[Agent {self.name} — {self.role.title()}]"


# Ordered by specificity: the first profile whose triggers match wins, so put
# the more specialized roles before the general ones.
AGENT_POOL: tuple[AgentProfile, ...] = (
    AgentProfile(
        name="Marcus",
        role="ml engineer",
        voice_id="voice_marcus_v1",
        personality="curious and experimental; explains trade-offs, flags uncertainty in the data before training",
        triggers=(
            "model", "ml", "machine learning", "predict", "prediction",
            "train", "training", "classifier", "regression", "forecast",
            "churn", "feature engineering", "embedding", "fine-tune",
        ),
    ),
    AgentProfile(
        name="Sage",
        role="platform engineer",
        voice_id="voice_sage_v1",
        personality="direct and infrastructure-focused; thinks in topics, DAGs, schedules and retries",
        triggers=(
            "kafka", "stream", "streaming", "airflow", "dag", "schedule",
            "scheduled", "cron", "real-time", "realtime", "ingest",
            "orchestrate", "queue", "topic", "event",
        ),
    ),
    AgentProfile(
        name="Zephyr",
        role="data architect",
        voice_id="voice_zephyr_v1",
        personality="big-picture and strategic; designs schemas and data contracts before anything runs",
        triggers=(
            "architecture", "architect", "schema design", "data model",
            "design", "migration", "migrate", "warehouse", "lakehouse",
            "normalize the model", "contract",
        ),
    ),
    AgentProfile(
        name="Nova",
        role="data analyst",
        voice_id="voice_nova_v1",
        personality="warm and narrative-driven; turns numbers into a story, leads with the insight",
        triggers=(
            "report", "dashboard", "pdf", "tableau", "powerbi", "chart",
            "visualize", "visualise", "trend", "insight", "kpi", "metric",
            "analyze", "analyse", "summary", "summarize", "presentation",
        ),
    ),
    AgentProfile(
        name="Aria",
        role="data engineer",
        voice_id="voice_aria_v1",
        personality="precise and methodical; cleans and validates data before trusting it, narrates each transform",
        triggers=(
            "pipeline", "etl", "elt", "clean", "cleaning", "transform",
            "deduplicate", "csv", "export", "load", "extract", "join",
            "aggregate", "wrangle", "preprocess", "standardize",
        ),
    ),
)

# Fallback when nothing matches — the data engineer is the safe generalist for an
# unrecognized data task (almost everything starts with getting data in order).
_DEFAULT_AGENT = AGENT_POOL[-1]  # Aria


def select_agent(message: str) -> AgentProfile:
    """Pick the agent whose role best matches the request.

    Scores each profile by how many of its trigger keywords appear in the
    message; the highest score wins, ties broken by pool order (specialized
    roles first). Falls back to the data engineer when nothing matches.
    """
    text = message.lower()

    best: AgentProfile | None = None
    best_score = 0
    for profile in AGENT_POOL:
        score = sum(
            1 for kw in profile.triggers
            if re.search(rf"\b{re.escape(kw)}\b", text)
        )
        if score > best_score:
            best, best_score = profile, score

    return best or _DEFAULT_AGENT
