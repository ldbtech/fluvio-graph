"""GitHub connector — syncs repos into the knowledge graph."""
import logging
from typing import Optional

import httpx
from github import Github, GithubException

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

# Max files per repo to avoid overwhelming the graph
MAX_FILES_PER_REPO = 50


class GitHubConnector(BaseConnector):
    """
    GitHub connector.

    Syncs selected repositories into the knowledge graph.
    Each file becomes a set of nodes (chunked by fluvio-ingestion).

    Also syncs:
      - README as a summary node
      - Recent issues (title + body) as context nodes
      - Recent PRs (title + description) as context nodes
    """

    def __init__(self, access_token: str, owner_id: str):
        super().__init__(access_token, owner_id)
        self.gh = Github(access_token)

    @property
    def kind(self) -> str:
        return "github"

    @property
    def resource_kind(self) -> str:
        return "github_repo"

    async def list_resources(self) -> list[Resource]:
        """List all repos the token has access to."""
        try:
            user  = self.gh.get_user()
            repos = user.get_repos(type="all", sort="updated")

            resources = []
            for repo in repos:
                resources.append(Resource(
                    external_id=  repo.full_name,
                    name=         repo.full_name,
                    description=  repo.description,
                    is_private=   repo.private,
                    meta={
                        "language":      repo.language or "",
                        "stars":         repo.stargazers_count,
                        "default_branch": repo.default_branch,
                    }
                ))

            logger.info(f"GitHub: found {len(resources)} repos")
            return resources

        except GithubException as e:
            logger.error(f"GitHub list_resources failed: {e}")
            raise

    async def sync_resource(
        self,
        resource:     Resource,
        connector_id: str,
        group_id:     Optional[str] = None,
    ) -> SyncResult:
        """Sync one GitHub repo into the knowledge graph."""
        nodes_added = 0
        repo_name   = resource.external_id

        try:
            repo = self.gh.get_repo(repo_name)

            # 1. Sync README as summary
            try:
                readme = repo.get_readme()
                content = readme.decoded_content.decode("utf-8", errors="ignore")
                if content.strip():
                    source_uri = f"github://{repo_name}/README"
                    await ingestion_client.ingest_raw(
                        owner_id=   self.owner_id,
                        text=       f"Repository: {repo_name}\n\n{content}",
                        source_uri= source_uri,
                        domain=     "codebase",
                    )
                    nodes_added += 1
                    logger.debug(f"GitHub: ingested README for {repo_name}")
            except Exception:
                pass  # Repo may have no README

            # 2. Sync code files
            files_synced = 0
            try:
                contents = repo.get_contents("")
                queue    = list(contents)

                while queue and files_synced < MAX_FILES_PER_REPO:
                    item = queue.pop(0)

                    if item.type == "dir":
                        queue.extend(repo.get_contents(item.path))
                        continue

                    ext = "." + item.name.rsplit(".", 1)[-1] if "." in item.name else ""
                    if ext.lower() not in INGESTIBLE_EXTENSIONS:
                        continue
                    if item.size > MAX_FILE_BYTES:
                        continue

                    try:
                        text = item.decoded_content.decode("utf-8", errors="ignore")
                        if not text.strip():
                            continue

                        source_uri = f"github://{repo_name}/{item.path}"
                        await ingestion_client.ingest_raw(
                            owner_id=   self.owner_id,
                            text=       f"File: {item.path}\n\n{text}",
                            source_uri= source_uri,
                            domain=     "codebase",
                        )
                        nodes_added  += 1
                        files_synced += 1

                    except Exception as e:
                        logger.warning(f"GitHub: failed to ingest {item.path}: {e}")

            except Exception as e:
                logger.warning(f"GitHub: failed to list files for {repo_name}: {e}")

            # 3. Sync recent issues
            try:
                issues = repo.get_issues(state="open", sort="updated")
                for issue in list(issues)[:10]:
                    if issue.pull_request:
                        continue  # skip PRs in issues list
                    text = f"Issue #{issue.number}: {issue.title}\n\n{issue.body or ''}"
                    if text.strip():
                        await ingestion_client.ingest_raw(
                            owner_id=   self.owner_id,
                            text=       text,
                            source_uri= f"github://{repo_name}/issues/{issue.number}",
                            domain=     "codebase",
                        )
                        nodes_added += 1
            except Exception:
                pass

            # 4. Sync recent PRs
            try:
                pulls = repo.get_pulls(state="open", sort="updated")
                for pr in list(pulls)[:10]:
                    text = f"PR #{pr.number}: {pr.title}\n\n{pr.body or ''}"
                    if text.strip():
                        await ingestion_client.ingest_raw(
                            owner_id=   self.owner_id,
                            text=       text,
                            source_uri= f"github://{repo_name}/pulls/{pr.number}",
                            domain=     "codebase",
                        )
                        nodes_added += 1
            except Exception:
                pass

            logger.info(f"GitHub: synced {repo_name} → {nodes_added} nodes")
            return SyncResult(external_id=repo_name, nodes_added=nodes_added)

        except Exception as e:
            logger.error(f"GitHub sync_resource failed for {repo_name}: {e}")
            return SyncResult(external_id=repo_name, nodes_added=0, error=str(e))