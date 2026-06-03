"""Compensating action registry for pipeline rollback.

When a mutable step succeeds, it registers its compensating action here.
If the pipeline later fails, the worker iterates these in reverse order
and fires each compensating action to unwind state.

Example registrations:
    rollback.register(2, "dashboard-syncer", "delete_report", {"report_id": "abc"})
    rollback.register(3, "spark", "drop_table", {"table_name": "revenue_analytics"})
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class RollbackRegistry:
    actions: list[dict[str, Any]] = field(default_factory=list)

    def register(
        self,
        step_index: int,
        tool_id: str,
        action: str,
        arguments: dict[str, Any],
    ) -> None:
        self.actions.append({
            "step_index": step_index,
            "tool_id": tool_id,
            "action": action,
            "arguments": arguments,
        })

    def has_actions(self) -> bool:
        return bool(self.actions)
