import httpx

class FederationClient:
    def __init__(self, gateway_url: str, headers: dict = None):
        self.gateway_url = gateway_url
        self.headers = headers or {}

    async def query(self, query: str, variables=None, headers=None):
        req_headers = {**self.headers, **(headers or {})}
        async with httpx.AsyncClient() as client:
            response = await client.post(
                self.gateway_url,
                json={
                    "query": query,
                    "variables": variables or {}
                },
                headers=req_headers
            )

            response.raise_for_status()
            return response.json()