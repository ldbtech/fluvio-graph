"""Prompt eval harness — run before deploying a changed prompt.

Usage:
    python -m app.evals.harness --prompt step_formulation

Each golden example lives in app/evals/golden/*.json with the shape:
    {
      "workspace_context": "<markdown string>",
      "user_message": "Build a signup-trend pipeline",
      "expected_tool_ids": ["data-cleaning", "spark"],
      "must_not_contain": ["VowayageAnalyticsJob"]
    }

The harness calls the real Anthropic API with the current prompt and
scores each example. A score below PASS_THRESHOLD fails the eval.
"""

from __future__ import annotations

import asyncio
import json
import logging
from pathlib import Path
from typing import Any

import httpx

from app.config import settings

logger = logging.getLogger("agent-planner.evals")

GOLDEN_DIR = Path(__file__).parent / "golden"
PROMPTS_DIR = Path(__file__).parent.parent / "prompts"
PASS_THRESHOLD = 0.8  # fraction of golden examples that must pass


def _load_goldens(prompt_name: str) -> list[dict[str, Any]]:
    pattern = f"{prompt_name}_*.json"
    files = list(GOLDEN_DIR.glob(pattern))
    if not files:
        logger.warning("No golden files found matching %s in %s", pattern, GOLDEN_DIR)
    goldens = []
    for f in files:
        try:
            goldens.append(json.loads(f.read_text()))
        except Exception as exc:
            logger.error("Failed to load golden %s: %s", f, exc)
    return goldens


def _score(response_text: str, golden: dict[str, Any]) -> tuple[bool, str]:
    """Return (passed, reason)."""
    expected_ids = golden.get("expected_tool_ids", [])
    must_not = golden.get("must_not_contain", [])

    # Parse the JSON if possible
    try:
        clean = response_text.strip()
        if "```" in clean:
            clean = clean.split("```")[1].split("```")[0].strip()
        steps = json.loads(clean)
        found_ids = {s.get("tool_id") for s in steps if isinstance(s, dict)}
    except Exception:
        found_ids = set()

    for mid in must_not:
        if mid in response_text:
            return False, f"Response contains forbidden string: {mid!r}"

    for eid in expected_ids:
        if eid not in found_ids:
            return False, f"Expected tool_id {eid!r} not found in steps (got {found_ids})"

    return True, "ok"


async def _call_claude(system: str, user: str) -> str:
    api_key = settings.anthropic_api_key
    if not api_key:
        raise RuntimeError("ANTHROPIC_API_KEY not set — cannot run evals")
    async with httpx.AsyncClient() as http:
        resp = await http.post(
            "https://api.anthropic.com/v1/messages",
            headers={
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
                "content-type": "application/json",
            },
            json={
                "model": "claude-haiku-4-5-20251001",  # cheap model for evals
                "max_tokens": 2048,
                "system": system,
                "messages": [{"role": "user", "content": user}],
            },
            timeout=60.0,
        )
        resp.raise_for_status()
        return resp.json()["content"][0]["text"]


async def run_eval(prompt_name: str) -> dict[str, Any]:
    prompt_file = PROMPTS_DIR / f"{prompt_name}.txt"
    if not prompt_file.exists():
        raise FileNotFoundError(f"Prompt file not found: {prompt_file}")

    prompt_template = prompt_file.read_text()
    goldens = _load_goldens(prompt_name)
    if not goldens:
        return {"prompt": prompt_name, "total": 0, "passed": 0, "score": 1.0, "results": []}

    results = []
    for golden in goldens:
        system = prompt_template.format(
            environment_context=golden.get("environment_context", "{}"),
            output_suffix="_analytics",
            context_plan=golden.get("workspace_context", ""),
        )
        user = golden.get("user_message", "Generate steps.")
        try:
            response = await _call_claude(system, user)
            passed, reason = _score(response, golden)
        except Exception as exc:
            passed, reason = False, str(exc)
            response = ""
        results.append({
            "golden": golden.get("user_message", ""),
            "passed": passed,
            "reason": reason,
        })
        status = "✅" if passed else "❌"
        logger.info("%s %s — %s", status, golden.get("user_message", ""), reason)

    total = len(results)
    passed_count = sum(1 for r in results if r["passed"])
    score = passed_count / total if total else 1.0
    overall = score >= PASS_THRESHOLD
    logger.info(
        "Eval '%s': %d/%d passed (%.0f%%) — %s",
        prompt_name, passed_count, total, score * 100,
        "PASS" if overall else "FAIL",
    )
    return {
        "prompt": prompt_name,
        "total": total,
        "passed": passed_count,
        "score": score,
        "overall": overall,
        "results": results,
    }


if __name__ == "__main__":
    import sys
    name = sys.argv[1] if len(sys.argv) > 1 else "step_formulation"
    result = asyncio.run(run_eval(name))
    print(json.dumps(result, indent=2))
    sys.exit(0 if result.get("overall", True) else 1)
