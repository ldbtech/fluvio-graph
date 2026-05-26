import os
import logging
from typing import Optional
from src.connectors.base import BaseConnector, Resource, SyncResult
from src.clients.ingestion_client import ingestion_client

logger = logging.getLogger(__name__)

# File extensions worth ingesting — code + docs
INGESTIBLE_EXTENSIONS = {
    ".py", ".rs", ".ts", ".tsx", ".js", ".jsx",
    ".go", ".java", ".kt", ".swift",
    ".md", ".mdx", ".txt", ".rst",
    ".toml", ".yaml", ".yml", ".json",
    ".sql", ".graphql", ".proto",
}

# Max file size to ingest (100KB)
MAX_FILE_BYTES = 100_000

# Max files per folder to avoid overwhelming the graph
MAX_FILES_PER_FOLDER = 50

class LocalDriveConnector(BaseConnector):
    """
    Local Drive connector.
    Syncs selected local directory folder files into the knowledge graph.
    """

    @property
    def kind(self) -> str:
        return "local_drive"

    @property
    def resource_kind(self) -> str:
        return "local_folder"

    async def list_resources(self) -> list[Resource]:
        """List the root folder as the available resource."""
        folder_path = self.access_token
        if not os.path.exists(folder_path):
            raise Exception(f"Folder path does not exist: {folder_path}")
        if not os.path.isdir(folder_path):
            raise Exception(f"Path is not a directory: {folder_path}")
        
        name = os.path.basename(os.path.abspath(folder_path)) or folder_path
        return [
            Resource(
                external_id="local_root",
                name=name,
                description=f"Local directory: {folder_path}"
            )
        ]

    async def sync_resource(
        self,
        resource:     Resource,
        connector_id: str,
        group_id:     Optional[str] = None,
    ) -> SyncResult:
        """Sync a local directory's files into the knowledge graph."""
        folder_path = self.access_token
        nodes_added = 0
        files_synced = 0

        if not os.path.exists(folder_path) or not os.path.isdir(folder_path):
            return SyncResult(external_id=resource.external_id, nodes_added=0, error=f"Directory {folder_path} not found or inaccessible")

        try:
            # Walk directory tree
            for root, dirs, files in os.walk(folder_path):
                # Modify dirs in-place to ignore hidden directories (like .git, .env)
                dirs[:] = [d for d in dirs if not d.startswith('.')]
                
                for file in files:
                    if files_synced >= MAX_FILES_PER_FOLDER:
                        break
                    
                    if file.startswith('.'):
                        continue

                    file_path = os.path.join(root, file)
                    rel_path = os.path.relpath(file_path, folder_path)
                    
                    ext = "." + file.rsplit(".", 1)[-1] if "." in file else ""
                    if ext.lower() not in INGESTIBLE_EXTENSIONS:
                        continue
                    
                    try:
                        size = os.path.getsize(file_path)
                        if size > MAX_FILE_BYTES:
                            continue
                        
                        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                            text = f.read()
                        
                        if not text.strip():
                            continue

                        source_uri = f"file://{os.path.abspath(file_path)}"
                        # Inject into the knowledge graph using the ingestion client
                        await ingestion_client.ingest_raw(
                            owner_id=   self.owner_id,
                            text=       f"File: {rel_path}\n\n{text}",
                            source_uri= source_uri,
                            domain=     "codebase",
                        )
                        nodes_added += 1
                        files_synced += 1
                    except Exception as fe:
                        logger.warning(f"LocalDrive: failed to ingest file {file_path}: {fe}")

                if files_synced >= MAX_FILES_PER_FOLDER:
                    break

            logger.info(f"LocalDrive: synced {folder_path} -> {nodes_added} files/nodes")
            return SyncResult(external_id=resource.external_id, nodes_added=nodes_added)

        except Exception as e:
            logger.error(f"LocalDrive sync_resource failed for {folder_path}: {e}")
            return SyncResult(external_id=resource.external_id, nodes_added=0, error=str(e))
