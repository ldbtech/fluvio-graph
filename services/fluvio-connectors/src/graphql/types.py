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