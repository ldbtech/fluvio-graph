import asyncio
import httpx
import json

async def test():
    query = """
    mutation ExecuteTool($toolId: String!, $inputs: String!) {
      executeTool(toolId: $toolId, inputs: $inputs) {
        id
        status
        output
        logs
      }
    }
    """
    
    db_context = {
        "database_url": "postgres://localhost/vowayage",
        "environment": "local"
    }
    spark_context = {
        "master_url": "local[*]",
        "app_name": "VowayageAnalyticsJob",
        "environment": "local"
    }
    
    args = {
        "context": spark_context,
        "query": "SELECT DATE_TRUNC('month', created_at) as month, COUNT(*) as new_users, SUM(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)) as cumulative_users FROM clean_users GROUP BY DATE_TRUNC('month', created_at) ORDER BY month",
        "output_table": "signup_trends_analytics"
    }
    
    inputs_str = json.dumps({
        "action": "execute_sql",
        "arguments": json.dumps(args)
    })
    
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            "http://127.0.0.1:3008/graphql",
            json={
                "query": query,
                "variables": {
                    "toolId": "spark",
                    "inputs": inputs_str
                }
            },
            headers={"x-user-id": "2a638162-449f-436c-9a45-a0669f57fe4a"}
        )
        print("Status code:", resp.status_code)
        print("Response:", json.dumps(resp.json(), indent=2))

if __name__ == "__main__":
    asyncio.run(test())
