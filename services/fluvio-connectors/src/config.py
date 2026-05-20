"""Configuration — loaded from .env"""
import os
from dotenv import load_dotenv

load_dotenv()

# Service ports
PORT = int(os.getenv("FLUVIO_CONNECTORS_PORT", "3006"))

# Internal service URLs
DATABASE_SERVICE_URL  = os.getenv("DATABASE_SERVICE_URL",  "http://localhost:3005/graphql")
INGESTION_SERVICE_URL = os.getenv("INGESTION_GRAPHQL_URL", "http://localhost:3004/graphql")
GRAPH_SERVICE_URL     = os.getenv("GRAPH_GRAPHQL_URL",     "http://localhost:3001/graphql")

# GitHub OAuth app credentials
GITHUB_CLIENT_ID     = os.getenv("GITHUB_CLIENT_ID", "")
GITHUB_CLIENT_SECRET = os.getenv("GITHUB_CLIENT_SECRET", "")
GITHUB_REDIRECT_URI  = os.getenv("GITHUB_REDIRECT_URI", "http://localhost:3006/oauth/github/callback")

# Notion OAuth app credentials
NOTION_CLIENT_ID     = os.getenv("NOTION_CLIENT_ID", "")
NOTION_CLIENT_SECRET = os.getenv("NOTION_CLIENT_SECRET", "")
NOTION_REDIRECT_URI  = os.getenv("NOTION_REDIRECT_URI", "http://localhost:3006/oauth/notion/callback")

# Sync schedule — how often to auto-sync connected resources
SYNC_INTERVAL_MINUTES = int(os.getenv("SYNC_INTERVAL_MINUTES", "60"))