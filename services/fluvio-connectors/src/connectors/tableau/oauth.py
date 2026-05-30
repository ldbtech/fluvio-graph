"""Tableau OAuth flow helper (Simulated Sandbox)."""

def get_auth_url(state: str) -> str:
    """Generate mock Tableau OAuth authorization URL."""
    # Direct redirect back to our callback to simulate user approval
    return f"http://localhost:3006/oauth/tableau/callback?code=mock-tableau-code-{state}&state={state}"


async def exchange_code(code: str) -> dict:
    """Exchange code for mock Tableau session token."""
    return {
        "access_token": "mock-tableau-oauth-token",
        "site_id": "httpsfluviomecom",
        "site_name": "Fluviome Cloud Site",
    }
