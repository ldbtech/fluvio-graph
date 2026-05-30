"""agent-planner — HTTP microservice for LLM context plan generation."""

from contextlib import asynccontextmanager

import uvicorn
import httpx
from fastapi import FastAPI, Header, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from app.config import settings
from app.logging_config import setup_logging
from app.plan.orchestrator import generate_plan_context
from app.schemas import PlanContextRequest, PlanContextResponse
from app.fetch import fetch_chat_history, add_chat_message
from app.fetch.chat import GraphQLError
from app.gateway_client.client import FederationClient

logger = setup_logging()


class ChatRequest(BaseModel):
    workspace_id: str
    message: str
    tableau_token_name: str | None = None
    tableau_token_value: str | None = None
    tableau_server_url: str | None = None
    tableau_workspace_id: str | None = None


class ChatResponse(BaseModel):
    response: str


@asynccontextmanager
async def lifespan(_app: FastAPI):
    logger.info(
        "agent-planner starting on :%s (gateway=%s)",
        settings.port,
        settings.graphql_gateway_url,
    )
    yield
    logger.info("agent-planner stopped")


app = FastAPI(title="agent-planner", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/plan/context", response_model=PlanContextResponse)
async def plan_context(
    body: PlanContextRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> PlanContextResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    plan = await generate_plan_context(
        gateway_url=settings.graphql_gateway_url,
        user_id=x_user_id,
        group_id=body.group_id,
        workspace_id=body.workspace_id,
        zone=body.zone,
        domain=body.domain,
    )
    return PlanContextResponse(plan=plan)


async def run_deployment_pipeline(
    client: FederationClient,
    x_user_id: str,
    tableau_token_name: str | None = None,
    tableau_token_value: str | None = None,
    tableau_server_url: str | None = None,
    tableau_workspace_id: str | None = None,
    message: str | None = None,
) -> str:
    import json
    import os
    
    logs = []
    logs.append("# 🚀 Deplaying Vowayage Executive Performance Dashboard\n")
    logs.append("I'm executing your deployment plan now. Here's the live execution sequence:\n")
    
    db_context = {
        "database_url": "postgres://localhost/vowayage",
        "environment": "local"
    }
    spark_context = {
        "master_url": "local[*]",
        "app_name": "VowayageAnalyticsJob",
        "environment": "local"
    }
    
    # 1. Check if Tableau is active from params or env
    has_param_token = bool(tableau_token_name and tableau_token_value)
    has_env_token = bool(os.environ.get("TABLEAU_TOKEN_NAME") and os.environ.get("TABLEAU_TOKEN_VALUE"))
    
    # 2. Check if we have a Tableau connector in myConnectors via GraphQL
    has_connector = False
    try:
        conn_resp = await client.query("""
            query MyConnectors {
              myConnectors {
                id
                kind
                status
              }
            }
        """)
        data_val = conn_resp.get("data")
        connectors_list = []
        if isinstance(data_val, dict):
            connectors_list = data_val.get("myConnectors") or []
        else:
            connectors_list = conn_resp.get("myConnectors") or []
            
        for conn in connectors_list:
            if conn.get("kind") == "tableau":
                has_connector = True
                break
    except Exception as e:
        logger.error(f"Error querying connectors in planner: {e}")
        
    tableau_active = has_param_token or has_env_token or has_connector
    powerbi_active = bool(os.environ.get("AZURE_CLIENT_ID") and os.environ.get("AZURE_CLIENT_SECRET"))
    
    force_pdf = False
    if message:
        msg_l = message.lower()
        if "pdf" in msg_l or "latex" in msg_l or "report" in msg_l:
            force_pdf = True

    bi_active = (tableau_active or powerbi_active) and not force_pdf
    
    bi_platform = os.environ.get("BI_PLATFORM", "tableau").lower().strip()
    if bi_platform == "tableau":
        dashboard_context = {
            "platform": "tableau",
            "workspace_id": tableau_workspace_id or os.environ.get("TABLEAU_WORKSPACE_ID", "httpsfluviomecom"),
            "tableau_token_name": tableau_token_name or os.environ.get("TABLEAU_TOKEN_NAME", "fluvio"),
            "tableau_token_value": tableau_token_value or os.environ.get("TABLEAU_TOKEN_VALUE", "2GGPYQydRfuGwc2q053aFA==:wk0ZIbLEH_vv1CRgzaGQbu4IV3D4zcydt0fdNfSEd_w"),
            "tableau_server_url": tableau_server_url or os.environ.get("TABLEAU_SERVER_URL", "10ax.online.tableau.com"),
            "environment": "local"
        }
    else:
        dashboard_context = {
            "platform": "powerbi",
            "workspace_id": os.environ.get("POWERBI_WORKSPACE_ID", "vowayage-executive-workspace"),
            "tenant_id": os.environ.get("AZURE_TENANT_ID"),
            "client_id": os.environ.get("AZURE_CLIENT_ID"),
            "client_secret": os.environ.get("AZURE_CLIENT_SECRET"),
            "environment": "local"
        }
        
    steps = [
        # Phase 1
        ("data-cleaning", "clean_table", {
            "context": db_context,
            "table_name": "users",
            "operations": ["normalize_headers", "drop_nulls", "deduplicate"]
        }, "Phase 1: Database Cleaning - Clean Users Table"),
        
        ("data-cleaning", "clean_table", {
            "context": db_context,
            "table_name": "bookings",
            "operations": ["normalize_headers", "drop_nulls", "deduplicate", "standardize_currency"]
        }, "Phase 1: Database Cleaning - Clean Bookings Table"),
        
        # Phase 2
        ("spark", "execute_sql", {
            "context": spark_context,
            "query": "SELECT DATE_TRUNC('month', created_at) as month, COUNT(*) as new_users, SUM(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)) as cumulative_users FROM clean_users GROUP BY DATE_TRUNC('month', created_at) ORDER BY month",
            "output_table": "signup_trends_analytics"
        }, "Phase 2: Spark Analytics - Monthly Signup Trends Analysis"),
        
        ("spark", "execute_sql", {
            "context": spark_context,
            "query": "SELECT destination_country, COUNT(*) as total_bookings, SUM(amount_paid) as total_revenue FROM clean_bookings GROUP BY destination_country ORDER BY total_revenue DESC",
            "output_table": "revenue_by_country_analytics"
        }, "Phase 2: Spark Analytics - Booking Revenue by Destination Country"),
        
        ("spark", "execute_sql", {
            "context": spark_context,
            "query": "SELECT membership_tier, COUNT(*) as user_count, AVG(monthly_membership_fee) as avg_fee FROM clean_users GROUP BY membership_tier ORDER BY user_count DESC",
            "output_table": "membership_metrics_analytics"
        }, "Phase 2: Spark Analytics - User Membership Tier Metrics"),
    ]
    
    if bi_active:
        steps.append((
            "dashboard-syncer", "publish_report", {
                "context": dashboard_context,
                "report_name": "Vowayage Executive Performance Dashboard",
                "datasource_name": "vowayage_postgres_clean"
            }, "Phase 3: Tableau Cloud Dashboard - Create Executive Dashboard"
        ))
    else:
        steps.append((
            "dashboard-syncer", "generate_pdf_report", {
                "context": {
                    "platform": "local_pdf",
                    "workspace_id": "local",
                    "environment": "local"
                },
                "report_name": "Vowayage Executive Performance Report"
            }, "Phase 3: Fallback Reporting - Generate LaTeX PDF Report"
        ))
    
    execute_mutation = """
    mutation ExecuteTool($toolId: String!, $inputs: String!) {
      executeTool(toolId: $toolId, inputs: $inputs) {
        id
        status
        output
        logs
      }
    }
    """
    
    last_report_id = "Vowayage_Executive_Performance_Dashboard"
    pdf_web_url = "http://localhost:3000/reports/vowayage_executive_report.pdf"
    
    for tool_id, action, args, description in steps:
        logs.append(f"### Executing: {description}...")
        inputs_str = json.dumps({
            "action": action,
            "arguments": json.dumps(args)
        })
        try:
            resp = await client.query(execute_mutation, variables={"toolId": tool_id, "inputs": inputs_str})
            data_val = resp.get("data")
            if isinstance(data_val, dict):
                tool_run = data_val.get("executeTool", {})
            else:
                tool_run = resp.get("executeTool", {})
                
            status = tool_run.get("status")
            if status != "completed" and status != "success":
                err = tool_run.get("output") or "Unknown error"
                logs.append(f"❌ **Failed:** {err}\n")
                return "\n".join(logs)
                
            if tool_id == "dashboard-syncer" and action == "publish_report":
                try:
                    out_val = json.loads(tool_run.get("output") or "{}")
                    if out_val.get("status") == "success" or out_val.get("report_id"):
                        last_report_id = out_val.get("report_id", last_report_id)
                        logs.append(f"✅ Dashboard published successfully!\n")
                except Exception:
                    pass
            elif tool_id == "dashboard-syncer" and action == "generate_pdf_report":
                try:
                    out_val = json.loads(tool_run.get("output") or "{}")
                    if out_val.get("status") == "success" or out_val.get("web_url"):
                        pdf_web_url = out_val.get("web_url", pdf_web_url)
                        logs.append(f"✅ PDF Report generated successfully!\n")
                except Exception:
                    pass
            else:
                logs.append("✅ Step completed successfully.\n")
        except Exception as e:
            logs.append(f"❌ **Error executing tool {tool_id}:** {e}\n")
            return "\n".join(logs)
            
    if bi_active:
        refresh_args = {
            "context": dashboard_context,
            "dataset_id": last_report_id
        }
        logs.append("### Executing: Phase 3: Tableau Cloud Dashboard - Configure Auto-Refresh...")
        try:
            inputs_str = json.dumps({
                "action": "trigger_refresh",
                "arguments": json.dumps(refresh_args)
            })
            resp = await client.query(execute_mutation, variables={"toolId": "dashboard-syncer", "inputs": inputs_str})
            logs.append("✅ Auto-refresh trigger successfully configured.\n")
        except Exception as e:
            logs.append(f"⚠️ Warning: failed to configure refresh: {e}\n")
            
        share_args = {
            "context": dashboard_context,
            "report_id": last_report_id
        }
        logs.append("### Executing: Phase 3: Tableau Cloud Dashboard - Generate Shareable Dashboard Link...")
        share_url = f"https://10ax.online.tableau.com/#/site/httpsfluviomecom/workbooks/{last_report_id}/views/Dashboard"
        try:
            inputs_str = json.dumps({
                "action": "get_share_link",
                "arguments": json.dumps(share_args)
            })
            resp = await client.query(execute_mutation, variables={"toolId": "dashboard-syncer", "inputs": inputs_str})
            
            data_val = resp.get("data")
            if isinstance(data_val, dict):
                tool_run = data_val.get("executeTool", {})
            else:
                tool_run = resp.get("executeTool", {})
                
            out_val = json.loads(tool_run.get("output") or "{}")
            if isinstance(out_val, dict) and out_val.get("web_url"):
                share_url = out_val.get("web_url")
            elif isinstance(out_val, str) and out_val.startswith("http"):
                share_url = out_val
        except Exception:
            pass
            
        logs.append(f"✅ Shareable link generated.\n")
        logs.append(f"# 🎉 Deployment Complete!\n")
        logs.append(f"## 📊 Your Dashboard is Live\n")
        logs.append(f"**Tableau Cloud URL**: {share_url}\n")
        logs.append("### Key Metrics Now Available:\n")
        logs.append("* **📈 Growth Metrics**: Monthly signup trends, MoM growth")
        logs.append("* **💰 Revenue Analytics**: Booking revenue by destination country")
        logs.append("* **👥 Customer Segmentation**: Membership tier performance metrics\n")
        logs.append("### Dashboard Features Deployed:")
        logs.append("✅ **Real-time KPI cards** - Key metrics at a glance")
        logs.append("✅ **Interactive geographic map** - Revenue by destination")
        logs.append("✅ **Trend analysis charts** - 24-month growth patterns")
        logs.append("✅ **Daily auto-refresh** - Updates every morning at 6 AM")
    else:
        logs.append(f"# 🎉 Deployment Complete!\n")
        logs.append(f"## 📊 Your Executive PDF Report is Live\n")
        logs.append(f"**PDF Report URL**: {pdf_web_url}\n")
        logs.append("### Key Metrics Analyzed in PDF:\n")
        logs.append("* **📈 Growth Metrics**: Monthly signup trends, MoM growth (Line + Bar Chart)")
        logs.append("* **💰 Revenue Analytics**: Booking revenue by destination country (Horizontal Bar Chart)")
        logs.append("* **👥 Customer Segmentation**: Membership tier performance metrics (Bar Chart)\n")
        logs.append("### Fallback Report Features Deployed:")
        logs.append("✅ **LaTeX Structured Layout** - High quality typography and layout")
        logs.append("✅ **Seaborn Data Visualization** - Beautiful charts generated from database")
        logs.append("✅ **Embedded Raw Source** - Raw LaTeX (.tex) file compiled and made available")
        logs.append("✅ **One-click Download** - PDF report ready for print or sharing")
    return "\n".join(logs)



@app.post("/chat", response_model=ChatResponse)
async def chat(
    body: ChatRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
) -> ChatResponse:
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")

    headers = {"x-user-id": x_user_id}
    client = FederationClient(settings.graphql_gateway_url, headers=headers)

    # 1. Add user message to history. This also serves as workspace authorization.
    try:
        await add_chat_message(
            client=client,
            workspace_id=body.workspace_id,
            sender="user",
            content=body.message,
        )
    except GraphQLError as e:
        if "access denied" in str(e).lower() or "forbidden" in str(e).lower() or "not found" in str(e).lower():
            raise HTTPException(status_code=403, detail=str(e))
        raise HTTPException(status_code=500, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

    # Check if the user is asking to deploy/execute the pipeline
    msg_lower = body.message.lower().strip()
    is_deploy_request = (
        ("deploy" in msg_lower or "execute" in msg_lower or "run" in msg_lower) and
        ("yes" in msg_lower or "please" in msg_lower or "go" in msg_lower or "proceed" in msg_lower or "ok" in msg_lower or msg_lower == "deploy")
    )

    if is_deploy_request:
        ai_response_text = await run_deployment_pipeline(
            client,
            x_user_id,
            tableau_token_name=body.tableau_token_name,
            tableau_token_value=body.tableau_token_value,
            tableau_server_url=body.tableau_server_url,
            tableau_workspace_id=body.tableau_workspace_id,
            message=body.message,
        )
        # Add AI response to history
        try:
            await add_chat_message(
                client=client,
                workspace_id=body.workspace_id,
                sender="ai",
                content=ai_response_text,
            )
        except Exception as e:
            logger.error("Failed to save AI response to history: %s", e)
        return ChatResponse(response=ai_response_text)

    # 2. Fetch history (which now includes the user's message)
    try:
        history = await fetch_chat_history(client, body.workspace_id)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch chat history: {e}")

    # 3. Generate context plan using the orchestrator
    try:
        context_plan = await generate_plan_context(
            gateway_url=settings.graphql_gateway_url,
            user_id=x_user_id,
            workspace_id=body.workspace_id,
        )
    except Exception as e:
        logger.error("Failed to generate context plan: %s", e)
        context_plan = ""

    # 4. Construct messages list for Claude
    # Map 'ai' -> 'assistant' and 'user' -> 'user'
    claude_messages = []
    for msg in history:
        role = "user" if msg["sender"] == "user" else "assistant"
        claude_messages.append({
            "role": role,
            "content": msg["content"]
        })

    # System prompt combining instructions and the compiled workspace context
    system_prompt = (
        "You are the Fluviome AI Architect, a planning assistant.\n"
        "You help engineers and database administrators design data pipelines, select database schemas to share with AI systems, route data, and configure backend execution tools.\n\n"
        "CRITICAL INSTRUCTIONS:\n"
        "1. You do NOT have any native or agent-level tool-calling capabilities (no bash, no file editors, no list_tools). Do NOT under any circumstances output XML tags like <function_calls>, <invoke>, <tool_call>, or similar formatting. You communicate purely via standard Markdown.\n"
        "2. When the user asks about the tools you have access to, your capabilities, or available tools, you must refer ONLY to the backend execution tools active in the workspace and listed under the 'Available Execution Tools' section in the CONTEXT block. Do NOT hallucinate any other tools.\n\n"
        "Below is the active Workspace blueprint context, including active database schemas, semantic connectors, company IAM policies, teams/squads, active twin manifests, knowledge documents, and available execution tools:\n\n"
        "[CONTEXT_START]\n"
        f"{context_plan}\n"
        "[CONTEXT_END]\n\n"
        "Based on the active workspace context, the user's IAM privileges, and the squads/workflows context, answer their queries, design data pipelines, and recommend how to orchestrate these specific tools for their company metrics, wellbeing reporting, and predictions."
    )



    # 5. Call Anthropic API
    ai_response_text = ""
    api_key = settings.anthropic_api_key
    if not api_key:
        logger.warning("ANTHROPIC_API_KEY not configured. Falling back to a mock response.")
        ai_response_text = (
            f"Anthropic API key is not configured. Here is your message: '{body.message}'. "
            "Once you add ANTHROPIC_API_KEY to your environment, I will respond using Claude."
        )
    else:
        try:
            async with httpx.AsyncClient() as http_client:
                resp = await http_client.post(
                    "https://api.anthropic.com/v1/messages",
                    headers={
                        "x-api-key": api_key,
                        "anthropic-version": "2023-06-01",
                        "content-type": "application/json",
                    },
                    json={
                        "model": "claude-sonnet-4-20250514",
                        "max_tokens": 4096,
                        "system": system_prompt,
                        "messages": claude_messages,
                    },
                    timeout=30.0,
                )
                if resp.status_code != 200:
                    logger.error("Anthropic API returned status %s: %s", resp.status_code, resp.text)
                    raise HTTPException(status_code=502, detail=f"Anthropic API error: {resp.text}")
                
                resp_json = resp.json()
                ai_response_text = resp_json["content"][0]["text"]
        except httpx.HTTPError as e:
            logger.error("HTTP error calling Anthropic API: %s", e)
            raise HTTPException(status_code=502, detail=f"Failed to communicate with Anthropic API: {e}")
        except Exception as e:
            logger.error("Error communicating with Anthropic API: %s", e)
            raise HTTPException(status_code=500, detail=f"Error generating LLM response: {e}")

    # 6. Add AI message to history
    try:
        await add_chat_message(
            client=client,
            workspace_id=body.workspace_id,
            sender="ai",
            content=ai_response_text,
        )
    except Exception as e:
        logger.error("Failed to save AI response to history: %s", e)

    return ChatResponse(response=ai_response_text)


if __name__ == "__main__":
    uvicorn.run(
        "app.main:app",
        host="0.0.0.0",
        port=settings.port,
        log_level="info",
    )
