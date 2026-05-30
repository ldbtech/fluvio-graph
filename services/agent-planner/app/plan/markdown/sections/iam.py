from typing import Any


def build_iam_section(users: list[dict[str, Any]], current_user: dict[str, Any] | None) -> list[str]:
    """Compile company users directory, roles, permissions, policies, and twin manifests into a Markdown section."""
    parts = [
        "### Company Identity & Access Management (IAM)",
        "This section maps out active users in the organization, their platform roles, custom IAM policies, and active AI Twin manifests.",
    ]

    if current_user:
        parts.append(
            f"- **Requesting User**: {current_user.get('displayName', 'Unknown')} "
            f"({current_user.get('email', 'N/A')}) | System Role: `{current_user.get('role', 'member')}`"
        )

    if not users:
        parts.append("- No company user directory returned.")
        return parts

    parts.append("\n#### User Directory & Roles")
    for u in users:
        uid = u.get("id", "")
        name = u.get("displayName", "N/A")
        email = u.get("email", "N/A")
        role = u.get("role", "member")
        policies = u.get("policies", [])
        twin_manifest = u.get("twinManifest")
        assigned_twin_roles = u.get("assignedAgentRoles", [])

        parts.append(f"\n- **User**: {name} ({email}) | Platform Role: `{role}`")
        parts.append(f"  - **User ID**: `{uid}`")

        # Twin status
        has_twin = "Configured" if twin_manifest and twin_manifest.strip() else "Not Configured"
        parts.append(f"  - **AI Twin Manifest Status**: {has_twin}")
        if twin_manifest and twin_manifest.strip():
            # Expose a snippet of their manifest so planner understands their instructions/directives
            manifest_lines = twin_manifest.strip().split("\n")
            preview = "\n".join([f"    > {line}" for line in manifest_lines[:15]])
            parts.append(f"  - **AI Twin Manifest Preview**:\n{preview}")
            if len(manifest_lines) > 15:
                parts.append("    > ... [manifest content continues]")

        # Policies
        if policies:
            parts.append(f"  - **Custom Access Policies**: {', '.join([f'`{p}`' for p in policies])}")
        else:
            parts.append("  - **Custom Access Policies**: None (Standard Role Permissions)")

        # Twin Pre-built roles
        if assigned_twin_roles:
            parts.append(f"  - **Assigned Pre-built Twin Roles**: {', '.join([f'`{r}`' for r in assigned_twin_roles])}")

    return parts
