"""Strawberry GraphQL types for fluvio-connectors."""
import strawberry
from typing import Optional


@strawberry.type
class ConnectorType:
    id:           str
    kind:         str
    auth_method:  str
    status:       str
    group_id:     Optional[str]
    last_sync_at: Optional[str]


@strawberry.type
class ResourceType:
    id:            str
    external_id:   str
    name:          str
    description:   Optional[str]
    selected:      bool
    node_count:    int
    last_sync_at:  Optional[str]


@strawberry.type
class SyncJobType:
    id:           str
    connector_id: str
    status:       str
    nodes_added:  int
    error:        Optional[str]


@strawberry.type
class OAuthUrlType:
    url:   str
    state: str


# ── Input types ───────────────────────────────────────────────────────────────

@strawberry.input
class ConnectTokenInput:
    kind:         str        # "github" | "notion"
    access_token: str
    group_id:     Optional[str] = None


@strawberry.input
class ConnectOAuthInput:
    kind:     str            # "github" | "notion"
    code:     str
    state:    str
    group_id: Optional[str] = None


@strawberry.input
class SelectResourcesInput:
    connector_id: str
    external_ids: list[str]  # repos/pages to select — all others deselected

# database-connectors
@strawberry.input
class DBConnectorInput:
    dialect:          str # postgresql | mysql | mssql
    host:             str
    port:             int
    database:         str
    username:         str
    password:         str
    org_id:           str
    group_id:         Optional[str] = None

@strawberry.input
class SyncTablesInput:
    connector_id: str
    org_id:       str
    table_names:  list[str]

    dialect:      str = "postgresql"
    host:         str = "localhost"
    port:         int = 5432
    database:     str = ""
    username:     str = ""
    password:     str = ""

@strawberry.type
class TableSyncResult:
    table:              str
    rows:               int
    columns:            int
    path:               str
    error:              Optional[str] = None

@strawberry.type
class DBSyncResult:
    connector_id:       str
    tables:             list[TableSyncResult]
    total_rows:         int
    status:             str # complete | failed 
    error:              Optional[str] = None 

@strawberry.type
class DBSchemaTable:
    name:        str
    columns:     list[str]
    row_estimate: int

@strawberry.type
class DBSchemaResult:
    tables: list[DBSchemaTable]

@strawberry.type
class ConnectionTestResult:
    success: bool
    message: str
    tables:  int
