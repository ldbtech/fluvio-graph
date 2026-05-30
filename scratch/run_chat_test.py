import httpx
import json

def test_chat():
    url = "http://127.0.0.1:3007/chat"
    headers = {
        "x-user-id": "b5d353f2-22c4-4b3f-b534-15b99c296c6f"
    }
    payload = {
      "workspace_id": "5173cbc2-30ed-4382-b89c-a5915bc0ab76",
      "message": "check the current tools you have access to before going forward with building data pipeline and eventually have a report on our company well being and prediction too"
    }
    
    print("Sending chat request to agent-planner at :3007...")
    try:
        response = httpx.post(url, headers=headers, json=payload, timeout=60.0)
        print(f"Status Code: {response.status_code}")
        if response.status_code == 200:
            res_json = response.json()
            print("\n--- FLUVIO ARCHITECT RESPONSE ---")
            print(res_json.get("response"))
            print("----------------------------------")
        else:
            print(f"Error Response: {response.text}")
    except Exception as e:
        print(f"Failed to connect to agent-planner: {e}")

if __name__ == "__main__":
    test_chat()
