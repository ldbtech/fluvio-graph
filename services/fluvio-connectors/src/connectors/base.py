"""Base connector abstract class."""
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional


@dataclass
class Resource:
    """A syncable resource — repo, page, recording, etc."""
    external_id:  str
    name:         str
    description:  Optional[str] = None
    is_private:   bool = False
    meta:         dict = None

    def __post_init__(self):
        if self.meta is None:
            self.meta = {}


@dataclass
class SyncResult:
    external_id: str
    nodes_added: int
    error:       Optional[str] = None

    @property
    def success(self) -> bool:
        return self.error is None


class BaseConnector(ABC):
    """
    Abstract base for all connectors.

    Each connector must implement:
      - list_resources() → list available resources (repos, pages, etc)
      - sync_resource()  → ingest one resource into the knowledge graph
    """

    def __init__(self, access_token: str, owner_id: str):
        self.access_token = access_token
        self.owner_id     = owner_id

    @abstractmethod
    async def list_resources(self) -> list[Resource]:
        """List all available resources for this connector."""
        ...

    @abstractmethod
    async def sync_resource(
        self,
        resource:     Resource,
        connector_id: str,
        group_id:     Optional[str] = None,
    ) -> SyncResult:
        """Sync one resource — ingest its content into the knowledge graph."""
        ...

    @property
    @abstractmethod
    def kind(self) -> str:
        """Connector kind string: 'github' | 'notion' | 'zoom'"""
        ...

    @property
    @abstractmethod
    def resource_kind(self) -> str:
        """Resource kind string: 'github_repo' | 'notion_page' | etc"""
        ...