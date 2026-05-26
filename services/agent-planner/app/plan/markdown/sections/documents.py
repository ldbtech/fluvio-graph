from typing import Any


def build_documents_section(documents: list[dict[str, Any]]) -> list[str]:
    md = ["\n## 3. Grounded Context Documents (Company Brain & Twin Nodes)"]
    if not documents:
        md.append("_No semantic document excerpts fetched in the active context._")
        return md

    for idx, doc in enumerate(documents, 1):
        md.append(f"\n### Document {idx}: {doc.get('title')}")
        md.append(f"- **UUID**: `{doc.get('id')}`")
        md.append(f"- **Domain/Source**: `{doc.get('domain')}` ({doc.get('kind')})")
        md.append("- **Cognitive Excerpt**:")
        md.append(f"  > {doc.get('excerpt')}")
        md.append(f"- **Ingestion Trust Zone**: Level {doc.get('zone')}")

    return md
