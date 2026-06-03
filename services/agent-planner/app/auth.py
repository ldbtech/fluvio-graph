"""Workspace authorization helpers."""

from fastapi import HTTPException

from app.gateway_client.client import FederationClient

_ACCESS_DENIED_PHRASES = ("access denied", "forbidden", "not found", "unauthorized")

_QUERY = """
query VerifyWorkspaceAccess($workspaceId: String!) {
  getWorkspace(workspaceId: $workspaceId) {
    id
  }
}
"""


async def verify_workspace_access(
    client: FederationClient,
    workspace_id: str,
) -> None:
    """Raise HTTP 403 if the caller does not have access to workspace_id."""
    try:
        resp = await client.query(_QUERY, variables={"workspaceId": workspace_id})
        data = resp.get("data") or resp
        if not (data.get("getWorkspace") or {}).get("id"):
            raise HTTPException(status_code=403, detail="Workspace not found or access denied")
    except HTTPException:
        raise
    except Exception as exc:
        msg = str(exc).lower()
        if any(phrase in msg for phrase in _ACCESS_DENIED_PHRASES):
            raise HTTPException(status_code=403, detail=str(exc))
        raise HTTPException(status_code=500, detail=f"Failed to verify workspace access: {exc}")
