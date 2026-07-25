"""Access to the package's bundled prompt templates.

The prompts live with the domain package (they are domain data, loaded via
`Path(__file__)`), so consumers — including the FastAPI shell — read them through
here rather than reaching for a filesystem path that assumes a layout.
"""

from __future__ import annotations

from pathlib import Path

_PROMPTS_DIR = Path(__file__).parent / "prompts"


def read_prompt(name: str) -> str:
    """Return the text of a bundled prompt template, e.g. ``"chat_system.txt"``."""
    return (_PROMPTS_DIR / name).read_text()
