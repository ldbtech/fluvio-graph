"""Workspace authorization helpers."""

from fastapi import HTTPException

from app.gateway_client.client import FederationClient

_ACCESS_DENIED_PHRASES = ("access denied", "forbidden", "not found", "unauthorized")

# The federated supergraph exposes `myWorkspaces(userId)` (it has no
# `getWorkspace`). Access is verified by listing the caller's workspaces and
# checking membership.
_MY_WORKSPACES = """
query MyWorkspaces($userId: String!) {
  myWorkspaces(userId: $userId) {
    id
  }
}
"""


async def verify_workspace_access(
    client: FederationClient,
    workspace_id: str,
    user_id: str | None = None,
) -> None:
    """Raise HTTP 403 if the caller does not have access to workspace_id.

    `user_id` defaults to the `x-user-id` header carried by the client, so
    existing callers that pass only (client, workspace_id) keep working.
    """
    uid = user_id or client.headers.get("x-user-id")
    if not uid:
        raise HTTPException(status_code=401, detail="x-user-id is required to verify workspace access")

    try:
        resp = await client.query(_MY_WORKSPACES, variables={"userId": uid})
        data = resp.get("data") or resp
        workspaces = data.get("myWorkspaces") or []
        if not any((ws or {}).get("id") == workspace_id for ws in workspaces):
            raise HTTPException(status_code=403, detail="Workspace not found or access denied")
    except HTTPException:
        raise
    except Exception as exc:
        msg = str(exc).lower()
        if any(phrase in msg for phrase in _ACCESS_DENIED_PHRASES):
            raise HTTPException(status_code=403, detail=str(exc))
        raise HTTPException(status_code=500, detail=f"Failed to verify workspace access: {exc}")
