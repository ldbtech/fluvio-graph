"""GitHub OAuth flow."""
import httpx
from src.config import GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, GITHUB_REDIRECT_URI


def get_auth_url(state: str) -> str:
    """Generate GitHub OAuth authorization URL."""
    params = "&".join([
        f"client_id={GITHUB_CLIENT_ID}",
        f"redirect_uri={GITHUB_REDIRECT_URI}",
        "scope=repo,read:user",
        f"state={state}",
    ])
    return f"https://github.com/login/oauth/authorize?{params}"


async def exchange_code(code: str) -> str:
    """Exchange OAuth code for access token."""
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            "https://github.com/login/oauth/access_token",
            json={
                "client_id":     GITHUB_CLIENT_ID,
                "client_secret": GITHUB_CLIENT_SECRET,
                "code":          code,
                "redirect_uri":  GITHUB_REDIRECT_URI,
            },
            headers={"Accept": "application/json"},
        )
        resp.raise_for_status()
        data = resp.json()

        if "error" in data:
            raise Exception(f"GitHub OAuth error: {data['error_description']}")

        return data["access_token"]


async def get_user_login(access_token: str) -> str:
    """Get the GitHub username for a token."""
    async with httpx.AsyncClient() as client:
        resp = await client.get(
            "https://api.github.com/user",
            headers={
                "Authorization": f"token {access_token}",
                "Accept":        "application/vnd.github.v3+json",
            }
        )
        resp.raise_for_status()
        return resp.json()["login"]