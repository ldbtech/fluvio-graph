"""HTTP client for fluvio-ingestion GraphQL API."""
import httpx
from src.config import INGESTION_SERVICE_URL

class IngestionClient:
    def __init__(self):
        self.endpoint = INGESTION_SERVICE_URL
        self.client = httpx.AsyncClient(timeout=60.0) # we need to figure out a way to handle timeouts better

    async def ingest_raw(
        self, 
        owner_id: str, 
        text: str, 
        source_uri: str, 
        domain: str = "custom") -> dict:

        q = """
            mutation($text: String!, $sourceUri: String!, $domain: String) {
                ingestRaw(text: $text, sourceUri: $sourceUri, domain: $domain) {
                    jobId status message
                }
            }
        """

        response = await self.client.post(
            self.endpoint,
            json={"query": q, "variables": {
                "text":      text,
                "sourceUri": source_uri,
                "domain":    domain,
            }},
            headers={
                "Content-Type": "application/json",
                "x-user-id":    owner_id,
            }        
        )

        response.raise_for_status()
        body = response.json()

        if "errors" in body:
            raise Exception(f"fluvio-ingestion error: {body['errors']}")

        return body["data"]["ingestRaw"]

ingestion_client = IngestionClient()