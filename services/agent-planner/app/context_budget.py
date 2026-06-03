"""Budget-aware context assembler.

Fills the context window from highest to lowest priority, truncating or
summarising lower-priority sections once the token budget is used up.

Token counting uses a fast heuristic (chars / 4) so we don't need tiktoken.
Swap `_count_tokens` for a proper tokeniser if accuracy matters.

Priority order (highest first):
    connectors → schemas → tools → teams → documents → graph_nodes

Usage:
    budget = ContextBudget(max_tokens=60_000)
    plan = budget.assemble(sections)   # sections: dict[section_name, markdown_str]
"""

from __future__ import annotations

from dataclasses import dataclass, field

# Rough heuristic: 1 token ≈ 4 characters
_CHARS_PER_TOKEN = 4

PRIORITY_ORDER = ["connectors", "schemas", "tools", "teams", "documents", "graph_nodes"]


def _count_tokens(text: str) -> int:
    return max(1, len(text) // _CHARS_PER_TOKEN)


def _truncate_to_tokens(text: str, max_tokens: int) -> str:
    limit = max_tokens * _CHARS_PER_TOKEN
    if len(text) <= limit:
        return text
    return text[:limit] + "\n\n… [truncated to fit context budget]"


def _compress_section(name: str, content: str, available_tokens: int) -> str:
    """Compress a low-priority section to fit within available_tokens."""
    if _count_tokens(content) <= available_tokens:
        return content

    if name == "documents":
        # Reduce to first line (title) of each document block
        lines = content.split("\n")
        compressed = "\n".join(
            line for line in lines if line.startswith("####") or line.startswith("###")
        )
        compressed += "\n\n_(document bodies omitted to fit context budget)_"
        return _truncate_to_tokens(compressed, available_tokens)

    if name == "graph_nodes":
        # Strip node source text, keep only the first sentence
        lines = content.split("\n")
        compressed = "\n".join(line[:120] for line in lines[:50])
        compressed += "\n\n_(graph nodes truncated to fit context budget)_"
        return _truncate_to_tokens(compressed, available_tokens)

    # Generic fallback: hard truncate
    return _truncate_to_tokens(content, available_tokens)


@dataclass
class ContextBudget:
    max_tokens: int = 60_000
    priority: list[str] = field(default_factory=lambda: list(PRIORITY_ORDER))

    def assemble(self, sections: dict[str, str]) -> str:
        """Return a single markdown string that fits within max_tokens.

        sections: mapping of section_name → markdown content.
        Unknown section names are appended last with whatever budget remains.
        """
        ordered = [s for s in self.priority if s in sections]
        remaining = [s for s in sections if s not in self.priority]

        remaining_budget = self.max_tokens
        parts: list[str] = []

        for name in ordered + remaining:
            content = sections.get(name, "")
            if not content:
                continue
            cost = _count_tokens(content)
            if cost <= remaining_budget:
                parts.append(content)
                remaining_budget -= cost
            elif remaining_budget > 200:
                compressed = _compress_section(name, content, remaining_budget)
                parts.append(compressed)
                remaining_budget -= _count_tokens(compressed)
            # else: drop the section entirely

        return "\n\n---\n\n".join(parts)
