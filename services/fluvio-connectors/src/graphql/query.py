"""Query resolvers for fluvio-connectors."""
import strawberry
from typing import Optional
from strawberry.types import Info

from src.graphql.types import ConnectorType, ResourceType, SyncJobType
from src.clients import db_client
from src.jobs import job_store


@strawberry.type
class Query:

    @strawberry.field
    async def my_connectors(
        self,
        info:     Info,
        group_id: Optional[str] = None,
    ) -> list[ConnectorType]:
        """List all connectors for the authenticated user."""
        user_id = _get_user_id(info)
        items   = await db_client.get_user_connectors(user_id, group_id)
        return [ConnectorType(**{
            "id":           c["id"],
            "kind":         c["kind"],
            "auth_method":  c["authMethod"],
            "status":       c["status"],
            "group_id":     c.get("groupId"),
            "last_sync_at": c.get("lastSyncAt"),
        }) for c in items]

    @strawberry.field
    async def connector_resources(
        self,
        info:         Info,
        connector_id: str,
    ) -> list[ResourceType]:
        """List all resources (repos, pages) for a connector."""
        user_id = _get_user_id(info)
        items   = await db_client.get_connector_resources(user_id, connector_id)
        return [_to_resource_type(r) for r in items]

    @strawberry.field
    async def sync_job(
        self,
        info:   Info,
        job_id: str,
    ) -> Optional[SyncJobType]:
        """Get sync job status."""
        job = job_store.get(job_id)
        if not job:
            return None
        return SyncJobType(
            id=           job.id,
            connector_id= job.connector_id,
            status=       job.status,
            nodes_added=  job.nodes_added,
            error=        job.error,
        )


# ── Helpers ───────────────────────────────────────────────────────────────────

def _get_user_id(info: Info) -> str:
    user_id = info.context["request"].headers.get("x-user-id", "")
    if not user_id:
        raise Exception("x-user-id header missing")
    return user_id

def _to_resource_type(r: dict) -> ResourceType:
    return ResourceType(
        id=           r["id"],
        external_id=  r["externalId"],
        name=         r["name"],
        description=  r.get("description"),
        selected=     r["selected"],
        node_count=   r["nodeCount"],
        last_sync_at= r.get("lastSyncAt"),
    )