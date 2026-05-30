import asyncio
import sys
import os

# Add agent-planner to path
sys.path.append(os.path.abspath('/Users/alidaho/Developer/AWS/rust/kg-engine/services/agent-planner'))

from app.plan.orchestrator import generate_plan_context

async def main():
    gateway_url = "http://127.0.0.1:4001"
    user_id = "b5d353f2-22c4-4b3f-b534-15b99c296c6f"
    workspace_id = "5173cbc2-30ed-4382-b89c-a5915bc0ab76"
    print("Executing generate_plan_context against Apollo Gateway...")
    try:
        context = await generate_plan_context(gateway_url, user_id=user_id, workspace_id=workspace_id)
        print("\n--- COMPILED PLANNER CONTEXT ---")
        print(context)
        print("--------------------------------")
    except Exception as e:
        print(f"Error during plan context generation: {e}")

if __name__ == "__main__":
    asyncio.run(main())
