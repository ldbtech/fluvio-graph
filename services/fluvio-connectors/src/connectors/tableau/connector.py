"""Tableau Cloud Connector implementation."""
import logging
from src.connectors.base import BaseConnector, Resource, SyncResult

logger = logging.getLogger("tableau-connector")

class TableauConnector(BaseConnector):
    """
    Simulated Tableau connector mapping workbooks to syncable resources.
    """

    async def list_resources(self) -> list[Resource]:
        """List mock Tableau workbooks."""
        return [
            Resource(
                external_id="rep-vowayage-executive",
                name="Vowayage Executive Performance Dashboard",
                description="Aggregated signup growth, membership tier performance, and bookings revenue analysis.",
                meta={"server": "10ax.online.tableau.com", "site": "httpsfluviomecom"}
            ),
            Resource(
                external_id="rep-marketing-leads",
                name="Marketing Leads Overview",
                description="Weekly email campaign signups, conversion rates, and acquisition costs.",
                meta={"server": "10ax.online.tableau.com", "site": "httpsfluviomecom"}
            )
        ]

    async def sync_resource(
        self,
        resource: Resource,
        connector_id: str,
        group_id: str = None
    ) -> SyncResult:
        """Mock sync of a workbook resource."""
        logger.info(f"Syncing Tableau workbook resource: {resource.name} ({resource.external_id})")
        return SyncResult(
            external_id=resource.external_id,
            nodes_added=15
        )

    @property
    def kind(self) -> str:
        return "tableau"

    @property
    def resource_kind(self) -> str:
        return "tableau_workbook"
