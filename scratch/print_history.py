import asyncio
import httpx
import json

async def main():
    url = "http://127.0.0.1:4001"
    headers = {
        "x-user-id": "b5d353f2-22c4-4b3f-b534-15b99c296c6f",
        "Content-Type": "application/json"
    }
    query = """
    query PlannerChatHistory($workspaceId: String!) {
      plannerChatHistory(workspaceId: $workspaceId) {
        sender
        content
      }
    }
    """
    variables = {
        "workspaceId": "5173cbc2-30ed-4382-b89c-a5915bc0ab76"
    }
    async with httpx.AsyncClient() as client:
        resp = await client.post(url, json={"query": query, "variables": variables}, headers=headers)
        print(json.dumps(resp.json(), indent=2))

if __name__ == "__main__":
    asyncio.run(main())
