"""Notion connector — syncs pages and databases into the knowledge graph."""
import logging
from typing import Optional

from notion_client import AsyncClient

from src.connectors.base import BaseConnector, Resource, SyncResult
from src.clients.ingestion_client import ingestion_client

logger = logging.getLogger(__name__)

MAX_BLOCKS_PER_PAGE = 200


class NotionConnector(BaseConnector):
    """
    Notion connector.

    Syncs selected pages and databases into the knowledge graph.
    Each page becomes nodes (chunked by fluvio-ingestion).
    Database rows become individual nodes.
    """

    def __init__(self, access_token: str, owner_id: str):
        super().__init__(access_token, owner_id)
        self.notion = AsyncClient(auth=access_token)

    @property
    def kind(self) -> str:
        return "notion"

    @property
    def resource_kind(self) -> str:
        return "notion_page"

    async def list_resources(self) -> list[Resource]:
        """List all pages and databases the integration has access to."""
        try:
            results   = []
            has_more  = True
            cursor    = None

            while has_more:
                params = {"page_size": 100}
                if cursor:
                    params["start_cursor"] = cursor

                resp     = await self.notion.search(**params)
                has_more = resp.get("has_more", False)
                cursor   = resp.get("next_cursor")

                for item in resp.get("results", []):
                    obj_type = item.get("object")
                    item_id  = item.get("id", "").replace("-", "")

                    if obj_type == "page":
                        # Get title from page properties
                        title = _extract_page_title(item)
                        results.append(Resource(
                            external_id= item_id,
                            name=        title or f"Page {item_id[:8]}",
                            description= None,
                            meta=        {"type": "page", "url": item.get("url", "")},
                        ))

                    elif obj_type == "database":
                        title = _extract_db_title(item)
                        results.append(Resource(
                            external_id= item_id,
                            name=        title or f"Database {item_id[:8]}",
                            description= None,
                            meta=        {"type": "database", "url": item.get("url", "")},
                        ))

            logger.info(f"Notion: found {len(results)} pages/databases")
            return results

        except Exception as e:
            logger.error(f"Notion list_resources failed: {e}")
            raise

    async def sync_resource(
        self,
        resource:     Resource,
        connector_id: str,
        group_id:     Optional[str] = None,
    ) -> SyncResult:
        """Sync one Notion page or database."""
        nodes_added = 0
        resource_id = resource.external_id

        try:
            obj_type = resource.meta.get("type", "page")

            if obj_type == "page":
                nodes_added = await self._sync_page(resource_id, resource.name)
            elif obj_type == "database":
                nodes_added = await self._sync_database(resource_id, resource.name)

            logger.info(f"Notion: synced {resource.name} → {nodes_added} nodes")
            return SyncResult(external_id=resource_id, nodes_added=nodes_added)

        except Exception as e:
            logger.error(f"Notion sync_resource failed for {resource_id}: {e}")
            return SyncResult(external_id=resource_id, nodes_added=0, error=str(e))

    async def _sync_page(self, page_id: str, title: str) -> int:
        """Extract text from a Notion page and ingest it."""
        try:
            blocks    = await self._get_all_blocks(page_id)
            text      = _blocks_to_text(blocks)

            if not text.strip():
                return 0

            await ingestion_client.ingest_raw(
                owner_id=   self.owner_id,
                text=       f"Notion Page: {title}\n\n{text}",
                source_uri= f"notion://{page_id}",
                domain=     "custom",
            )
            return 1

        except Exception as e:
            logger.warning(f"Notion _sync_page {page_id} failed: {e}")
            return 0

    async def _sync_database(self, db_id: str, title: str) -> int:
        """Sync all rows of a Notion database as individual nodes."""
        nodes = 0
        try:
            has_more = True
            cursor   = None

            while has_more:
                params = {"database_id": db_id, "page_size": 100}
                if cursor:
                    params["start_cursor"] = cursor

                resp     = await self.notion.databases.query(**params)
                has_more = resp.get("has_more", False)
                cursor   = resp.get("next_cursor")

                for row in resp.get("results", []):
                    row_title = _extract_page_title(row) or f"Row {row['id'][:8]}"
                    row_text  = _properties_to_text(row.get("properties", {}))

                    if not row_text.strip():
                        continue

                    await ingestion_client.ingest_raw(
                        owner_id=   self.owner_id,
                        text=       f"Database: {title}\nRow: {row_title}\n\n{row_text}",
                        source_uri= f"notion://{db_id}/{row['id'].replace('-', '')}",
                        domain=     "custom",
                    )
                    nodes += 1

        except Exception as e:
            logger.warning(f"Notion _sync_database {db_id} failed: {e}")

        return nodes

    async def _get_all_blocks(self, block_id: str, depth: int = 0) -> list:
        """Recursively fetch all blocks for a page."""
        if depth > 3:
            return []

        blocks  = []
        has_more = True
        cursor  = None

        while has_more and len(blocks) < MAX_BLOCKS_PER_PAGE:
            params = {"block_id": block_id}
            if cursor:
                params["start_cursor"] = cursor

            resp     = await self.notion.blocks.children.list(**params)
            has_more = resp.get("has_more", False)
            cursor   = resp.get("next_cursor")

            for block in resp.get("results", []):
                blocks.append(block)
                if block.get("has_children"):
                    children = await self._get_all_blocks(block["id"], depth + 1)
                    blocks.extend(children)

        return blocks


# ── Text extraction helpers ────────────────────────────────────────────────────

def _extract_page_title(page: dict) -> str:
    props = page.get("properties", {})
    for key in ["Name", "Title", "title", "name"]:
        if key in props:
            rich = props[key].get("title", [])
            return "".join(t.get("plain_text", "") for t in rich)
    return ""

def _extract_db_title(db: dict) -> str:
    title_list = db.get("title", [])
    return "".join(t.get("plain_text", "") for t in title_list)

def _blocks_to_text(blocks: list) -> str:
    lines = []
    for block in blocks:
        btype   = block.get("type", "")
        content = block.get(btype, {})
        rich    = content.get("rich_text", [])
        text    = "".join(t.get("plain_text", "") for t in rich)
        if text.strip():
            lines.append(text)
    return "\n\n".join(lines)

def _properties_to_text(props: dict) -> str:
    lines = []
    for key, val in props.items():
        ptype = val.get("type", "")
        text  = ""
        if ptype == "title":
            text = "".join(t.get("plain_text", "") for t in val.get("title", []))
        elif ptype == "rich_text":
            text = "".join(t.get("plain_text", "") for t in val.get("rich_text", []))
        elif ptype == "select":
            s = val.get("select")
            text = s.get("name", "") if s else ""
        elif ptype == "multi_select":
            text = ", ".join(s.get("name", "") for s in val.get("multi_select", []))
        elif ptype == "number":
            n = val.get("number")
            text = str(n) if n is not None else ""
        elif ptype == "checkbox":
            text = str(val.get("checkbox", ""))
        elif ptype == "date":
            d = val.get("date")
            text = d.get("start", "") if d else ""

        if text.strip():
            lines.append(f"{key}: {text}")

    return "\n".join(lines)