"""Mutation resolvers for fluvio-connectors."""
import asyncio
import logging
import uuid
import strawberry
from typing import Optional
from strawberry.types import Info

from src.graphql.types import (
    ConnectorType, ResourceType, SyncJobType, OAuthUrlType,
    ConnectTokenInput, ConnectOAuthInput, SelectResourcesInput,
)
from src.clients import db_client, ingestion_client
from src.connectors.github import GitHubConnector, get_auth_url as gh_auth_url, exchange_code as gh_exchange
from src.connectors.notion import NotionConnector, get_auth_url as notion_auth_url, exchange_code as notion_exchange
from src.jobs import job_store

logger = logging.getLogger(__name__)


@strawberry.type
class Mutation:

    # ── Connect via token ─────────────────────────────────────────────────────

    @strawberry.mutation
    async def connect_token(
        self,
        info:  Info,
        input: ConnectTokenInput,
    ) -> ConnectorType:
        """Connect a service using a Personal Access Token or API key."""
        user_id = _get_user_id(info)

        # Validate token works before storing
        connector_cls = _get_connector_class(input.kind)
        conn = connector_cls(access_token=input.access_token, owner_id=user_id)
        await conn.list_resources()  # raises if token is invalid

        # Store in fluvio-database
        result = await db_client.create_connector(
            user_id=      user_id,
            kind=         input.kind,
            auth_method=  "token",
            access_token= input.access_token,
            group_id=     input.group_id,
        )

        # Fetch and store available resources
        await _fetch_and_store_resources(conn, result["id"], user_id)

        logger.info(f"Connected {input.kind} via token for user {user_id}")

        return ConnectorType(
            id=           result["id"],
            kind=         result["kind"],
            auth_method=  result["authMethod"],
            status=       result["status"],
            group_id=     result.get("groupId"),
            last_sync_at= None,
        )

    # ── OAuth flow ────────────────────────────────────────────────────────────

    @strawberry.mutation
    async def get_oauth_url(
        self,
        info: Info,
        kind: str,
    ) -> OAuthUrlType:
        """Get the OAuth authorization URL for a service."""
        state = str(uuid.uuid4())

        if kind == "github":
            url = gh_auth_url(state)
        elif kind == "notion":
            url = notion_auth_url(state)
        else:
            raise Exception(f"unsupported connector kind: {kind}")

        return OAuthUrlType(url=url, state=state)

    @strawberry.mutation
    async def connect_oauth(
        self,
        info:  Info,
        input: ConnectOAuthInput,
    ) -> ConnectorType:
        """Complete OAuth flow — exchange code for token and connect."""
        user_id = _get_user_id(info)

        # Exchange code for token
        if input.kind == "github":
            access_token = await gh_exchange(input.code)
        elif input.kind == "notion":
            token_data   = await notion_exchange(input.code)
            access_token = token_data["access_token"]
        else:
            raise Exception(f"unsupported connector kind: {input.kind}")

        # Store connector
        result = await db_client.create_connector(
            user_id=      user_id,
            kind=         input.kind,
            auth_method=  "oauth",
            access_token= access_token,
            group_id=     input.group_id,
        )

        # Fetch and store available resources
        connector_cls = _get_connector_class(input.kind)
        conn = connector_cls(access_token=access_token, owner_id=user_id)
        await _fetch_and_store_resources(conn, result["id"], user_id)

        logger.info(f"Connected {input.kind} via OAuth for user {user_id}")

        return ConnectorType(
            id=           result["id"],
            kind=         result["kind"],
            auth_method=  result["authMethod"],
            status=       result["status"],
            group_id=     result.get("groupId"),
            last_sync_at= None,
        )

    # ── Resource selection ────────────────────────────────────────────────────

    @strawberry.mutation
    async def select_resources(
        self,
        info:  Info,
        input: SelectResourcesInput,
    ) -> list[ResourceType]:
        """Select which repos/pages to sync. All others are deselected."""
        user_id = _get_user_id(info)
        items   = await db_client.select_resources(
            user_id=      user_id,
            connector_id= input.connector_id,
            external_ids= input.external_ids,
        )
        return [ResourceType(
            id=           r["id"],
            external_id=  r["externalId"],
            name=         r["name"],
            description=  r.get("description"),
            selected=     r["selected"],
            node_count=   0,
            last_sync_at= None,
        ) for r in items]

    # ── Sync ──────────────────────────────────────────────────────────────────

    @strawberry.mutation
    async def sync_now(
        self,
        info:         Info,
        connector_id: str,
    ) -> SyncJobType:
        user_id    = _get_user_id(info)
        connectors = await db_client.get_user_connectors(user_id)
        connector  = next((c for c in connectors if c["id"] == connector_id), None)
        if not connector:
            raise Exception("connector not found")

        # We need the token — fetch it from a dedicated endpoint
        # For now store what we have and fetch token separately
        job = job_store.create(
            connector_id=  connector_id,
            owner_id=      user_id,
            kind=          connector["kind"],
        )

        asyncio.create_task(_run_sync(connector_id, user_id, job.id))

        return SyncJobType(
            id=           job.id,
            connector_id= connector_id,
            status=       job.status,
            nodes_added=  0,
            error=        None,
        )

    # ── Disconnect ────────────────────────────────────────────────────────────

    @strawberry.mutation
    async def disconnect(
        self,
        info:         Info,
        connector_id: str,
    ) -> bool:
        """Disconnect a connector and remove all its resources."""
        user_id = _get_user_id(info)
        # fluvio-database cascade deletes resources
        # For now just update status — full delete in next iteration
        await db_client.update_connector_status(
            user_id=      user_id,
            connector_id= connector_id,
            status=       "disconnected",
        )
        return True


# ── Background sync task ─────────────────────────────────────────────────────

async def _run_sync(connector_id: str, user_id: str, job_id: str):
    """Background task — syncs all selected resources for a connector."""
    job_store.update_status(job_id, "running")
    total_nodes = 0

    try:
        # Get connector WITH access token
        connector_data = await db_client.get_connector_token(user_id, connector_id)
        if not connector_data:
            job_store.fail(job_id, "connector not found")
            return

        access_token = connector_data["accessToken"]  # ← fixed
        kind         = connector_data["kind"]

        # Mark as syncing
        await db_client.update_connector_status(user_id, connector_id, "syncing")

        # Get selected resources
        selected = await db_client.get_selected_resources(user_id, connector_id)
        if not selected:
            job_store.finish(job_id, 0)
            await db_client.mark_synced(user_id, connector_id)
            return

        # Build connector instance
        connector_cls = _get_connector_class(kind)
        conn = connector_cls(
            access_token= access_token,
            owner_id=     user_id,
        )

        # Sync each selected resource
        for resource_data in selected:
            from src.connectors.base import Resource
            resource = Resource(
                external_id= resource_data["externalId"],
                name=        resource_data["name"],
            )
            result = await conn.sync_resource(
                resource=     resource,
                connector_id= connector_id,
            )
            if result.success and result.nodes_added > 0:
                await db_client.update_resource_sync_stats(
                    user_id=      user_id,
                    connector_id= connector_id,
                    external_id=  result.external_id,
                    nodes_added=  result.nodes_added,
                )
                total_nodes += result.nodes_added

        # Mark complete
        await db_client.mark_synced(user_id, connector_id)
        job_store.finish(job_id, total_nodes)
        logger.info(f"Sync complete: connector={connector_id} nodes={total_nodes}")

    except Exception as e:
        logger.error(f"Sync failed: connector={connector_id} error={e}")
        job_store.fail(job_id, str(e))
        try:
            await db_client.update_connector_status(
                user_id, connector_id, "error", str(e)
            )
        except Exception:
            pass

# ── Helpers ───────────────────────────────────────────────────────────────────

def _get_user_id(info: Info) -> str:
    user_id = info.context["request"].headers.get("x-user-id", "")
    if not user_id:
        raise Exception("x-user-id header missing")
    return user_id

def _get_connector_class(kind: str):
    if kind == "github":
        return GitHubConnector
    elif kind == "notion":
        return NotionConnector
    raise Exception(f"unsupported connector kind: {kind}")

async def _fetch_and_store_resources(connector, connector_id: str, user_id: str):
    """Fetch resource list from external service and store in fluvio-database."""
    try:
        resources = await connector.list_resources()
        for r in resources:
            await db_client.upsert_resource(
                user_id=       user_id,
                connector_id=  connector_id,
                resource_kind= connector.resource_kind,
                external_id=   r.external_id,
                name=          r.name,
                description=   r.description,
            )
        logger.info(f"Stored {len(resources)} resources for connector {connector_id}")
    except Exception as e:
        logger.warning(f"Failed to fetch resources: {e}")