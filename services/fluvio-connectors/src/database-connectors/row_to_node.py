# database-connectors/row_to_node.py
#
# Converts a database row (dict) into natural language text.
# Suitable for embedding and storage in fluvio-graph
# 
# Input : {id: "abc", "name": "Alice", "email": "ali@co.com", "role":"owner"}
# Output: "User record: name=Alice, email=alic@co.com, role=owner"

import re
from datetime import datetime

# ONLY these are always blocked - truly never semantic
# Passwords, secrets, hashes - no business value, high risk
ALWAYS_BLOCKED = {
    "password", "password_hash", "passwd",
    "secret", "secret_key", "private_key",
    "access_token", "refresh_token", "api_key",
    "credential", "checksum", "token",
}

# Suggested defaults shown pre-checked in the wizard
# User can uncheck to include them
SUGGESTED_SKIP = {
    "id", "firebase_uid", "avatar_url",
    "installed_on", "execution_time",
}

# Columns with timing context — kept by default
# "created_at" tells you when something happened
# "updated_at" tells you if data is stale
TIMING_COLUMNS = {
    "created_at", "updated_at", "joined_at",
    "last_login", "last_sync_at", "reviewed_at",
    "accepted_at", "expires_at", "installed_on",
}

def row_to_text(
    table:            str,
    row:              dict,
    included_columns: list[str] # user-selected columns from wizard
) -> str:
    """
        Convert one DB row to natural language text for KG Embedding

        Only uses columns in included_columns.
        User controls exactly what gets ingested.
        ALWAYS_BLOCKED columns are stripped even if user included them.
    """

    label = _table_label(table)
    parts = []

    for col in included_columns:
        if col.lower() in ALWAYS_BLOCKED:
            continue

        val = row.get(col)
        if val is None or str(val).strip() == "":
            continue
        
        # Format timing columns with context 
        if col in TIMING_COLUMNS:
            parts.append(f"{_col_label(col)}: {_format_date(val)}")
        else:
            parts.append(f"{_col_label(col)}: {_format_value(val)}")
    if not parts:
        return f"{label} (no indexable content)"
    return f"{label} - {', '.join(parts)}"

def row_to_source_uri(
    table:          str,
    row:            dict,
    connector_id:   str,
) -> str:
    """
        Stable depuplication URI for this row.
        Format: db://{connector_id}/{table}/{pk_value}
    """
    pk_value = (
        row.get("id") or 
        row.get(f"{table}_id") or 
        row.get("uuid") or
        str(list(row.values())[0])
    )

    return f"db://{connector_id}/{table}/{pk_value}"

def rows_to_texts(
    table:              str,
    rows:               list[dict],
    included_columns:   list[str],
    connector_id:       str,
) -> list[tuple[str, str]]:
    """ 
        Convert all rows to (text, source_uri) pairs
        Ready for fluvio_ingestion.
    """
    results = []
    for row in rows:
        text = row_to_text(table, row, included_columns)
        uri = row_to_source_uri(table, row, connector_id)
        results.append((text, uri))
    return results

def suggest_skip_columns(columns: list[str]) -> list[str]:
    """
        Return columns we suggest skipping by default in the wizard.
        These appear pre-unchecked in the column selector
        User can check them to include

        returns list of suggested-skip column names. 
    """

    suggestions = []
    for col in columns:
        col_lower = col.lower()
        # Always block
        if col_lower in ALWAYS_BLOCKED:
            suggestions.append(col)
            continue
        # Suggested skip
        if col_lower in SUGGESTED_SKIP:
            suggestions.append(col)
            continue

            # PII patterns — suggest skip, user decides
        pii_patterns = [
            "ssn", "social_security", "dob", "date_of_birth",
            "phone", "address", "zip", "postal",
            "passport", "license", "medical", "diagnosis",
            "salary", "wage", "income", "bank_account",
            "credit_card", "card_number",
        ]

        if any(p in col_lower for p in pii_patterns):
            suggestions.append(col)

        return suggestions

# --- Helpers ------------------------------------------------------
def _table_label(table: str) -> str:
    clean = table.rstrip("s").replace("_", " ")
    return clean.capitalize() + " record"

def _col_label(col: str) -> str:
    return col.replace("_", " ")

def _format_value(v: str) -> str:
    if isinstance(v, bool):
        return "yes" if v else "no"
    if isinstance(v, datetime):
        return v.strftime("%Y-%m-%d")
    return str(v).strip()

def _format_date(v) -> str:
    """ Format timestamp with relative context. """
    if isinstance(v, datetime):
        return v.strftime("%B %d %Y")
    
    # Try parsing string timestamp 
    try:
        dt = datetime.fromisoformat(str(v).replace("Z", "+00:00").split(".")[0])
        return dt.strftime("%B %d %Y")
    except Exception:
        return str(v)

# ── Test ──────────────────────────────────────────────────────────────────────

if __name__ == "__main__":

    # Simulate wizard: user selected these columns (unchecked firebase_uid, avatar_url)
    user_selected_columns = [
        "email", "display_name", "created_at", "updated_at"
    ]

    rows = [
        {
            "id":           "7eceeae5",
            "firebase_uid": "test-owner-001",   # excluded by user
            "email":        "owner@fluvio.ai",
            "display_name": "Alice Owner",
            "avatar_url":   None,               # excluded by user
            "created_at":   "2026-01-15 10:00:00",
            "updated_at":   "2026-05-18 14:31:05",
        },
        {
            "id":           "0b7103e8",
            "firebase_uid": "test-contrib-001",
            "email":        "bob@fluvio.ai",
            "display_name": "Bob Contributor",
            "avatar_url":   None,
            "created_at":   "2026-03-10 09:00:00",
            "updated_at":   "2026-05-18 14:31:06",
        },
    ]

    all_columns = ["id", "firebase_uid", "email", "display_name", "avatar_url", "created_at", "updated_at"]

    print("Suggested columns to skip (pre-unchecked in wizard):")
    suggestions = suggest_skip_columns(all_columns)
    print(f"  {suggestions}\n")

    print("Row → Text (with user-selected columns only):\n")
    for text, uri in rows_to_texts("users", rows, user_selected_columns, "connector-fluvio-collab"):
        print(f"  text: {text}")
        print(f"  uri:  {uri}")
        print()