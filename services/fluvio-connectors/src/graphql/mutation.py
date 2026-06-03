"""Mutation resolvers for fluvio-connectors."""
import asyncio
import logging
import uuid
import strawberry
from typing import Optional
from strawberry.types import Info

from src.graphql.types import (
    ConnectionTestResult, ConnectorType, ResourceType, SyncJobType, OAuthUrlType,
    ConnectTokenInput, ConnectOAuthInput, SelectResourcesInput,
)
from src.clients import db_client, ingestion_client
from src.connectors.github import GitHubConnector, get_auth_url as gh_auth_url, exchange_code as gh_exchange
from src.connectors.notion import NotionConnector, get_auth_url as notion_auth_url, exchange_code as notion_exchange
from src.connectors.tableau import TableauConnector, get_auth_url as tableau_auth_url, exchange_code as tableau_exchange
from src.connectors.local_drive.connector import LocalDriveConnector
from src.jobs import job_store

import sys 
import asyncio
from pathlib import Path
_DB_CONNECTOR_PATH = Path(__file__).parent.parent / "database-connectors"
sys.path.insert(0, str(_DB_CONNECTOR_PATH))

from database_types.sql_connector import DBConfig, DatabaseConnector, SchemaFunctions
from storage.local import LocalStorage
from sync import sync_tables
from .types import DBSyncResult, TableSyncResult, ConnectionTestResult, DBConnectorInput, SyncTablesInput


logger = logging.getLogger(__name__)

# Singleton storage - same instance used everywhere.
_storage = LocalStorage(base_path=str(
    Path(__file__).parent.parent / "s3"
))


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
        is_db = input.kind in ["postgresql", "mysql", "mongodb", "redis", "snowflake", "bigquery"]
        if not is_db:
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
        if not is_db:
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
        elif kind == "tableau":
            url = tableau_auth_url(state)
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
        elif input.kind == "tableau":
            token_data   = await tableau_exchange(input.code)
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
            meta=         r.get("meta"),
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
    
    # Database Connectors 
    @strawberry.mutation
    async def test_db_connection(
        self,
        info: Info,
        input: "DBConnectorInput",
    ) -> "ConnectionTestResult":
        """
            wizard step 2 - test credentials before saving anything. 
            Returns success + table count.
        """
        try:
            config = DBConfig(
                dialect = input.dialect,
                host = input.host,
                port = input.port,
                database = input.database,
                username = input.username,
                password = input.password,
            )

            connector = DatabaseConnector(config)
            schema = SchemaFunctions(connector.get_engine())
            db_meta = schema.extract()
            connector.get_engine().dispose()

            return ConnectionTestResult(
                success = True,
                message = f"Connected. Found {len(db_meta.tables)} tables",
                tables = len(db_meta.tables),
            )

        except Exception as e:
            return ConnectionTestResult(
                success = True,
                message = str(e),
                tables = 0,
            )
    
    @strawberry.mutation 
    async def sync_db_tables(
        self,
        info: Info,
        input: "SyncTablesInput",
    ) -> "DBSyncResult":
        """
        Wizard Step 7 / manual sync — fetch rows and save CSVs.
        Runs in background, returns immediately with job status.
        """

        user_id = _get_user_id(info)

        # Build config directly from input
        config = DBConfig(
            dialect=  input.dialect,
            host=     input.host,
            port=     input.port,
            database= input.database,
            username= input.username,
            password= input.password,
        )
        
        # Run sync
        try:
            results = await sync_tables(
                org_id = input.org_id,
                connector_id = input.connector_id,
                config = config,
                table_names = input.table_names,
                storage = _storage,
                owner_id = user_id,
            )

            table_results = [
                TableSyncResult(
                    table   = table,
                    rows    = tinfo.get("rows", 0),
                    columns = tinfo.get("columns", 0),
                    path    = tinfo.get("path", ""),
                    error   = tinfo.get("error"),
                    columns_list = tinfo.get("columns_list", []),
                )
                for table, tinfo in results.items()
            ]

            total_rows = sum(
                r.rows for r in table_results if r.error is None
            )
            try:
                await db_client.mark_synced(user_id, input.connector_id)
                
                # Automatically upsert resources and update sync stats for successfully synced tables
                synced_table_names = [res.table for res in table_results if res.error is None]
                if synced_table_names:
                    import json
                    for res in table_results:
                        if res.error is None:
                            meta_json = json.dumps({
                                "columns": res.columns_list,
                                "all_columns": res.columns_list,
                                "database": input.database
                            })
                            await db_client.upsert_resource(
                                user_id=user_id,
                                connector_id=input.connector_id,
                                resource_kind="database_table",
                                external_id=res.table,
                                name=res.table,
                                description=f"Table {res.table} with {res.columns} columns",
                                meta=meta_json,
                            )
                            await db_client.update_resource_sync_stats(
                                user_id=user_id,
                                connector_id=input.connector_id,
                                external_id=res.table,
                                nodes_added=res.rows,
                            )
                    await db_client.select_resources(
                        user_id=user_id,
                        connector_id=input.connector_id,
                        external_ids=synced_table_names,
                    )
            except Exception as e:
                logger.error(f"Failed to update connector resources for {input.connector_id}: {e}")

            return DBSyncResult(
                connector_id = input.connector_id,
                tables = table_results,
                total_rows = total_rows,
                status = "complete",
            )

        except Exception as e:
            logger.exception("sync_db_tables failed")
            return DBSyncResult(
                connector_id    = input.connector_id,
                tables          = [],
                total_rows      = 0,
                status          = "failed",
                error           = str(e),
            )


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

        is_db = kind in ["postgresql", "mysql", "mongodb", "redis", "snowflake", "bigquery"]
        if is_db:
            import json
            creds = json.loads(access_token)
            config = DBConfig(
                dialect=  kind,
                host=     creds.get("host", ""),
                port=     creds.get("port", 5432),
                database= creds.get("database", ""),
                username= creds.get("username", ""),
                password= creds.get("password", ""),
            )
            org_id = "org_fluviome"
            try:
                user_res = await db_client.post(
                    "query($id: String!) { getUser(id: $id) { companyId } }",
                    {"id": user_id},
                    user_id
                )
                if user_res and user_res.get("getUser") and user_res["getUser"].get("companyId"):
                    org_id = user_res["getUser"]["companyId"]
            except Exception as e:
                logger.warning(f"Could not fetch company ID for user {user_id}: {e}")

            table_names = [r["externalId"] for r in selected]
            
            results = await sync_tables(
                org_id = org_id,
                connector_id = connector_id,
                config = config,
                table_names = table_names,
                storage = _storage,
                owner_id = user_id,
            )
            
            synced_table_names = []
            for table_name, tinfo in results.items():
                if "error" not in tinfo:
                    synced_table_names.append(table_name)
                    rows = tinfo.get("rows", 0)
                    cols = tinfo.get("columns", 0)
                    cols_list = tinfo.get("columns_list", [])
                    total_nodes += rows
                    
                    meta_json = json.dumps({
                        "columns": cols_list,
                        "all_columns": cols_list,
                        "database": config.database
                    })
                    
                    await db_client.upsert_resource(
                        user_id=user_id,
                        connector_id=connector_id,
                        resource_kind="database_table",
                        external_id=table_name,
                        name=table_name,
                        description=f"Table {table_name} with {cols} columns",
                        meta=meta_json,
                    )
                    await db_client.update_resource_sync_stats(
                        user_id=user_id,
                        connector_id=connector_id,
                        external_id=table_name,
                        nodes_added=rows,
                    )
            
            if synced_table_names:
                await db_client.select_resources(
                    user_id=user_id,
                    connector_id=connector_id,
                    external_ids=synced_table_names,
                )
        else:
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
        try:
            await db_client.mark_synced(user_id, connector_id)
        except Exception:
            pass
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
    elif kind == "local_drive":
        return LocalDriveConnector
    elif kind == "tableau":
        return TableauConnector
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