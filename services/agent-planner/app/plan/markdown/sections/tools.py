from typing import Any
import os


def build_tools_section(tools: list[dict[str, Any]]) -> list[str]:
    """Compile available execution tools into a Markdown section for the agent planner."""
    parts = [
        "### Available Execution Tools",
        "The following backend execution tools are active in the workspace. Twin agents can suggest calling them or building pipelines around them to sync databases, clean data, route streams, or train models.",
    ]

    if not tools:
        parts.append("- No execution tools registered in this workspace.")
        return parts

    for t in tools:
        tool_id = t.get("id", "")
        name = t.get("name", "")
        desc = t.get("description", "")
        cat = t.get("category", "")

        parts.append(f"\n#### Tool: {name} (ID: `{tool_id}`)")
        parts.append(f"- **Category**: {cat}")
        parts.append(f"- **Description**: {desc}")

        params = t.get("parameters", [])
        if params:
            parts.append("- **Parameters**:")
            for p in params:
                p_name = p.get("name", "")
                p_type = p.get("paramType", "string")
                p_desc = p.get("description", "")
                p_req = "Required" if p.get("required") else "Optional"
                p_def = f" (Default: `{p.get('defaultValue')}`)" if p.get("defaultValue") else ""
                parts.append(f"  - `{p_name}` ({p_type}, {p_req}){p_def}: {p_desc}")
        else:
            parts.append("- **Parameters**: None")

        # Dynamically search and load detailed markdown documentation for the tool
        safe_tool_id = tool_id.replace("-", "_")
        md_path = f"/Users/alidaho/Developer/AWS/rust/kg-engine/services/fluvio-tool-builder/src/tools/{safe_tool_id}/{safe_tool_id}.md"
        if not os.path.exists(md_path):
            md_path = f"/Users/alidaho/Developer/AWS/rust/kg-engine/services/fluvio-tool-builder/src/tools/{safe_tool_id}/{tool_id}.md"

        if os.path.exists(md_path):
            try:
                with open(md_path, "r") as f:
                    doc_content = f.read().strip()
                if doc_content:
                    parts.append("- **Detailed Documentation**:")
                    # Indent the markdown to render nicely nested in the list structure
                    indented_doc = "\n".join(f"  {line}" for line in doc_content.split("\n"))
                    parts.append(indented_doc)
            except Exception:
                pass

    return parts

