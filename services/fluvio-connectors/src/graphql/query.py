"""Query resolvers for fluvio-connectors."""
import strawberry
from typing import Optional
from strawberry.types import Info

from src.graphql.types import ConnectorType, DBSchemaResult, ResourceType, SyncJobType
from src.clients import db_client
from src.jobs import job_store

import sys 
from pathlib import Path
_DB_CONNECTOR_PATH = Path(__file__).parent.parent / "database-connectors"
sys.path.insert(0, str(_DB_CONNECTOR_PATH))

from database_types.sql_connector import DBConfig, DatabaseConnector, SchemaFunctions
from .types import DBSchemaTable, DBSchemaResult, DBConnectorInput


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
    
    @strawberry.field
    async def db_schema(
        self,
        info: Info,
        input: "DBConnectorInput",
    ) -> "DBSchemaResult":

        """
            Get a schema for a database - called in wizard step 3 ,
            returns: table names and columns names.
        """
        config = DBConfig(
            dialect   = input.dialect,
            host      = input.host,
            port      = input.port,
            database  = input.database,
            username  = input.username,
            password  = input.password,
        )

        connector  = DatabaseConnector(config)
        schema     = SchemaFunctions(connector.get_engine())
        db_meta    = schema.extract()

        connector.get_engine().dispose()

        tables = [
            DBSchemaTable(
                name            = t.name,
                columns         = [c.name for c in t.columns],
                row_estimate    = 0, 
            ) for t in db_meta.tables
        ]

        return DBSchemaResult(tables = tables) 


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
        meta=         r.get("meta"),
    )