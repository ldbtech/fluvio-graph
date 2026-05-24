"""Notion OAuth flow."""
import httpx
import base64
from urllib.parse import urlencode
from src.config import NOTION_CLIENT_ID, NOTION_CLIENT_SECRET, NOTION_REDIRECT_URI


def get_auth_url(state: str) -> str:
    """Generate Notion OAuth authorization URL."""
    params = urlencode({
        "client_id":     NOTION_CLIENT_ID,
        "redirect_uri":  NOTION_REDIRECT_URI,
        "response_type": "code",
        "owner":         "user",
        "state":         state,
    })
    return f"https://api.notion.com/v1/oauth/authorize?{params}"


async def exchange_code(code: str) -> dict:
    """Exchange OAuth code for access token. Returns token + workspace info."""
    credentials = base64.b64encode(
        f"{NOTION_CLIENT_ID}:{NOTION_CLIENT_SECRET}".encode()
    ).decode()

    async with httpx.AsyncClient() as client:
        resp = await client.post(
            "https://api.notion.com/v1/oauth/token",
            json={
                "grant_type":   "authorization_code",
                "code":         code,
                "redirect_uri": NOTION_REDIRECT_URI,
            },
            headers={
                "Authorization": f"Basic {credentials}",
                "Content-Type":  "application/json",
            },
        )
        resp.raise_for_status()
        data = resp.json()

        if "error" in data:
            raise Exception(f"Notion OAuth error: {data['error']}")

        return {
            "access_token":    data["access_token"],
            "workspace_id":    data.get("workspace_id", ""),
            "workspace_name":  data.get("workspace_name", ""),
        }