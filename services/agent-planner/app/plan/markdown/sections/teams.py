from typing import Any


def build_teams_section(teams_data: list[dict[str, Any]]) -> list[str]:
    """Compile company teams, squads, members, and team workflows into a Markdown section."""
    parts = [
        "### Teams, Squads & Workflows",
        "This section lists the active company divisions/squads, their members, and automated workflows registered under their purview.",
    ]

    if not teams_data:
        parts.append("- No teams or squads registered in this company.")
        return parts

    for t_entry in teams_data:
        team = t_entry.get("team", {})
        members = t_entry.get("members", [])
        workflows = t_entry.get("workflows", [])

        t_name = team.get("name", "N/A")
        t_desc = team.get("description") or "No description provided."
        t_id = team.get("id", "")

        parts.append(f"\n#### Team: {t_name} (ID: `{t_id}`)")
        parts.append(f"- **Description**: {t_desc}")

        # Members list
        if members:
            member_lines = []
            for m in members:
                uid = m.get("userId", "")
                role = m.get("role", "member")
                member_lines.append(f"    - User ID: `{uid}` (Squad Role: `{role}`)")
            parts.append("- **Squad Members**:")
            parts.extend(member_lines)
        else:
            parts.append("- **Squad Members**: None assigned.")

        # Workflows list
        if workflows:
            parts.append("- **Registered Team Workflows**:")
            for wf in workflows:
                wf_name = wf.get("name", "N/A")
                wf_desc = wf.get("description") or "No description."
                wf_enabled = "Enabled" if wf.get("isEnabled") else "Disabled"
                wf_steps = wf.get("steps", "[]")

                parts.append(f"  - **Workflow**: {wf_name} ({wf_enabled})")
                parts.append(f"    - Description: {wf_desc}")
                parts.append(f"    - Blueprint Execution Steps:\n      ```json\n      {wf_steps}\n      ```")
        else:
            parts.append("- **Registered Team Workflows**: None.")

    return parts
