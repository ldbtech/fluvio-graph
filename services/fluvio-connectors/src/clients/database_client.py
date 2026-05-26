"""HTTP client for fluvio-database GraphQL."""
import httpx
from typing import Optional
from src.config import DATABASE_SERVICE_URL

class DatabaseClient:
    def __init__(self):
        self.endpoint = DATABASE_SERVICE_URL
        self.client = httpx.AsyncClient(timeout=30.0)
    
    async def post(self, query: str, variables: dict, user_id: str) -> dict:
        resp = await self.client.post(
            self.endpoint,
            json={"query": query, "variables": variables},
            headers={
                "Content-Type": "application/json",
                "x-user-id":    user_id,
            }
        )
        resp.raise_for_status()
        body = resp.json()
        if "errors" in body:
            raise Exception(f"fluvio-database error: {body['errors']}")
        return body["data"]
    
    # ---- Connectors --------------------------------------------------------
    async def create_connector(
        self,
        user_id:      str,
        kind:         str,
        auth_method:  str,
        access_token: str,
        group_id:     Optional[str] = None,
    ) -> dict:
        q = """
        mutation($input: CreateConnectorInput!) {
            createConnector(input: $input) {
                id kind authMethod status groupId createdAt
            }
        }
        """
        data = await self.post(q, {
            "input": {
                "kind":        kind,
                "authMethod":  auth_method,
                "accessToken": access_token,
                "groupId":     group_id,
            }
        }, user_id)
        return data["createConnector"]

    async def get_user_connectors(self, user_id: str, group_id: Optional[str] = None) -> list:
        q = """
        query($groupId: String) {
            getUserConnectors(groupId: $groupId) {
                id kind authMethod status groupId lastSyncAt
            }
        }
        """
        data = await self.post(q, {"groupId": group_id}, user_id)
        return data["getUserConnectors"]
    
    async def update_connector_status(
        self, user_id: str, connector_id: str,
        status: str, error: Optional[str] = None
    ) -> dict:
        q = """
        mutation($connectorId: String!, $status: String!, $error: String) {
            updateConnectorStatus(connectorId: $connectorId, status: $status, error: $error) {
                id status
            }
        }
        """
        data = await self.post(q, {
            "connectorId": connector_id,
            "status":      status,
            "error":       error,
        }, user_id)
        return data["updateConnectorStatus"]
    
    async def mark_synced(self, user_id: str, connector_id: str) -> dict:
        q = """
        mutation($connectorId: String!) {
            markSynced(connectorId: $connectorId) { id lastSyncAt }
        }
        """
        data = await self.post(q, {"connectorId": connector_id}, user_id)
        return data["markSynced"]
    
    # ── Resources ─────────────────────────────────────────────────────────────
    async def upsert_resource(
        self,
        user_id:       str,
        connector_id:  str,
        resource_kind: str,
        external_id:   str,
        name:          str,
        description:   Optional[str] = None,
        meta:          Optional[str] = None,
    ) -> dict:
        q = """
        mutation($input: UpsertResourceInput!) {
            upsertResource(input: $input) {
                id externalId name selected nodeCount
            }
        }
        """
        data = await self.post(q, {
            "input": {
                "connectorId":  connector_id,
                "resourceKind": resource_kind,
                "externalId":   external_id,
                "name":         name,
                "description":  description,
                "meta":         meta,
            }
        }, user_id)
        return data["upsertResource"]
    
    async def get_connector_resources(self, user_id: str, connector_id: str) -> list:
        q = """
        query($connectorId: String!) {
            getConnectorResources(connectorId: $connectorId) {
                id externalId name description selected nodeCount lastSyncAt meta
            }
        }
        """
        data = await self.post(q, {"connectorId": connector_id}, user_id)
        return data["getConnectorResources"]
    
    async def get_selected_resources(self, user_id: str, connector_id: str) -> list:
        q = """
        query($connectorId: String!) {
            getSelectedResources(connectorId: $connectorId) {
                id externalId name selected nodeCount meta
            }
        }
        """
        data = await self.post(q, {"connectorId": connector_id}, user_id)
        return data["getSelectedResources"]
    
    async def select_resources(
        self, user_id: str, connector_id: str, external_ids: list[str]
    ) -> list:
        q = """
        mutation($input: SelectResourcesInput!) {
            selectResources(input: $input) {
                id externalId name selected meta
            }
        }
        """
        data = await self.post(q, {
            "input": {
                "connectorId": connector_id,
                "externalIds": external_ids,
            }
        }, user_id)
        return data["selectResources"]
    
    async def update_resource_sync_stats(
        self, user_id: str, connector_id: str,
        external_id: str, nodes_added: int
    ) -> bool:
        q = """
        mutation($connectorId: String!, $externalId: String!, $nodesAdded: Int!) {
            updateResourceSyncStats(
                connectorId: $connectorId,
                externalId: $externalId,
                nodesAdded: $nodesAdded
            )
        }
        """
        data = await self.post(q, {
            "connectorId": connector_id,
            "externalId":  external_id,
            "nodesAdded":  nodes_added,
        }, user_id)
        return data["updateResourceSyncStats"]
    
    async def get_connector_token(self, user_id: str, connector_id: str) -> dict:
        """Get connector with access token."""
        q = """
        query($connectorId: String!) {
            getConnector(connectorId: $connectorId) {
                id kind accessToken status
            }
        }
        """
        data = await self.post(q, {"connectorId": connector_id}, user_id)
        return data["getConnector"]   # ← return full dict

# Singleton
db_client = DatabaseClient()
