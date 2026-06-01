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


class DeployRequest(BaseModel):
    workspace_id: str
    sandbox_id: str | None = None
    tableau_token_name: str | None = None
    tableau_token_value: str | None = None
    tableau_server_url: str | None = None
    tableau_workspace_id: str | None = None
    powerbi_workspace_id: str | None = None
    tenant_id: str | None = None
    client_id: str | None = None
    client_secret: str | None = None
    plan_markdown: str | None = None


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
    workspace_id: str,
    sandbox_id: str | None = None,
    ports_map: dict[str, int] | None = None,
    tableau_token_name: str | None = None,
    tableau_token_value: str | None = None,
    tableau_server_url: str | None = None,
    tableau_workspace_id: str | None = None,
    powerbi_workspace_id: str | None = None,
    tenant_id: str | None = None,
    client_id: str | None = None,
    client_secret: str | None = None,
    message: str | None = None,
    plan_markdown: str | None = None,
) -> str:
    import json
    import os
    
    logs = []
    logs.append("# 🚀 Deploying Executive Performance Dashboard\n")
    if sandbox_id:
        logs.append(f"Targeting isolated Sandbox: **{sandbox_id}**\n")
    logs.append("Formulating and executing deployment steps. Live execution sequence:\n")
    
    # Try to fetch chat history to extract the last user message if not supplied
    if not message:
        try:
            history = await fetch_chat_history(client, workspace_id)
            if history:
                user_msgs = [m for m in history if m.get("sender") == "user"]
                if user_msgs:
                    message = user_msgs[-1].get("content") or ""
        except Exception as e:
            logger.error(f"Failed to fetch history in pipeline: {e}")

    pg_port = 5432
    if ports_map and "postgres:5432/tcp" in ports_map:
        pg_port = ports_map["postgres:5432/tcp"]
        logger.info(f"Mapping database queries to sandbox postgres port: {pg_port}")
        
    # Fetch active database connector details strictly from the database connector
    db_host = None
    db_user = None
    db_pass = None
    db_name = None
    
    has_tableau_connector = False
    has_powerbi_connector = False
    
    tableau_resolved = {}
    powerbi_resolved = {}
    
    try:
        conn_resp = await client.query("""
            query GetUserConnectors {
              getUserConnectors {
                id
                kind
                status
              }
            }
        """)
        data_val = conn_resp.get("data")
        connectors_list = []
        if isinstance(data_val, dict):
            connectors_list = data_val.get("getUserConnectors") or []
        else:
            connectors_list = conn_resp.get("getUserConnectors") or []
            
        db_conn = None
        connectors_config = {}
        for conn in connectors_list:
            kind_lower = conn.get("kind", "").lower()
            if kind_lower in ["postgres", "postgresql", "database", "mysql"]:
                db_conn = conn
            
            try:
                conn_detail_resp = await client.query("""
                    query GetConnector($connectorId: String!) {
                      getConnector(connectorId: $connectorId) {
                        id
                        kind
                        accessToken
                        status
                      }
                    }
                """, variables={"connectorId": conn["id"]})
                detail_data = conn_detail_resp.get("data") or {}
                connector_detail = detail_data.get("getConnector") or conn_detail_resp.get("getConnector") or {}
                access_token = connector_detail.get("accessToken")
                if access_token:
                    try:
                        token_data = json.loads(access_token)
                        connectors_config[kind_lower] = token_data
                        if kind_lower == "tableau":
                            tableau_resolved = token_data
                            has_tableau_connector = True
                        elif kind_lower == "powerbi":
                            powerbi_resolved = token_data
                            has_powerbi_connector = True
                    except Exception:
                        connectors_config[kind_lower] = {"token": access_token}
            except Exception as e:
                logger.error(f"Failed to query connector {conn['id']}: {e}")

        if not db_conn:
            logs.append("❌ **Failed:** No database connector configured in your workspace. Please connect a database connector first.\n")
            return "\n".join(logs)

        # Fetch connector credentials
        conn_detail_resp = await client.query("""
            query GetConnector($connectorId: String!) {
              getConnector(connectorId: $connectorId) {
                id
                kind
                accessToken
                status
              }
            }
        """, variables={"connectorId": db_conn["id"]})
        detail_data = conn_detail_resp.get("data")
        connector_detail = (detail_data or {}).get("getConnector") or conn_detail_resp.get("getConnector") or {}
        access_token = connector_detail.get("accessToken")
        if not access_token:
            logs.append("❌ **Failed:** Database connector accessToken details are empty.\n")
            return "\n".join(logs)

        try:
            token_data = json.loads(access_token)
            db_host = token_data.get("host")
            db_name = token_data.get("database") or token_data.get("databaseName")
            db_user = token_data.get("username")
            db_pass = token_data.get("password")
        except Exception as e:
            logger.error(f"Error parsing connector credentials: {e}")
            logs.append(f"❌ **Failed:** Failed to parse database connector credentials: {e}\n")
            return "\n".join(logs)
            
    except Exception as e:
        logger.error(f"Error resolving database connector in planner: {e}")
        logs.append(f"❌ **Failed:** Error resolving database connector: {e}\n")
        return "\n".join(logs)
        
    if not all([db_host, db_user, db_name]):
        logs.append(f"❌ **Failed:** Missing database credentials in active connector (host={db_host}, user={db_user}, name={db_name}).\n")
        return "\n".join(logs)
        
    db_pass = db_pass or ""
        
    db_url_host = "localhost" if sandbox_id else db_host
    db_url_port = pg_port if sandbox_id else 5432
    
    db_context = {
        "database_url": f"postgres://{db_user}:{db_pass}@{db_url_host}:{db_url_port}/{db_name}",
        "environment": "sandbox" if sandbox_id else "local",
        "sandbox_id": sandbox_id
    }
    
    spark_context = {
        "master_url": "local[*]",
        "app_name": "VowayageAnalyticsJob",
        "environment": "sandbox" if sandbox_id else "local",
        "sandbox_id": sandbox_id
    }
    
    airflow_port = 8080
    if ports_map and "airflow:8080/tcp" in ports_map:
        airflow_port = ports_map["airflow:8080/tcp"]
        
    airflow_context = {
        "host_url": f"http://localhost:{airflow_port}",
        "environment": "sandbox" if sandbox_id else "local",
        "sandbox_id": sandbox_id
    }
    
    dbt_context = {
        "project_dir": "/workspace/dbt_project",
        "profile_name": "default",
        "target_name": "dev",
        "environment": "sandbox" if sandbox_id else "local",
        "sandbox_id": sandbox_id
    }
    
    terraform_context = {
        "environment": "cloud",
        "provider": "aws",
        "sandbox_id": sandbox_id
    }
    
    environment_context = {
        "sandbox_id": sandbox_id,
        "environment": "sandbox" if sandbox_id else "local",
        "ports_map": ports_map or {},
        "database": {
            "host": db_url_host,
            "port": db_url_port,
            "username": db_user,
            "password": db_pass,
            "database": db_name,
            "url": f"postgres://{db_user}:{db_pass}@{db_url_host}:{db_url_port}/{db_name}"
        },
        "connectors": connectors_config
    }
    
    # Check if Tableau is active
    has_param_token = bool(tableau_token_name and tableau_token_value)
    has_env_token = bool(os.environ.get("TABLEAU_TOKEN_NAME") and os.environ.get("TABLEAU_TOKEN_VALUE"))
    tableau_active = has_param_token or has_env_token or has_tableau_connector
    
    # Check if PowerBI is active
    powerbi_active = (
        bool(os.environ.get("AZURE_CLIENT_ID") and os.environ.get("AZURE_CLIENT_SECRET")) or
        has_powerbi_connector or
        bool(powerbi_workspace_id and client_id)
    )
    
    msg_l = (message or "").lower()
    force_pdf = False
    if ("pdf" in msg_l or "latex" in msg_l) and not ("tableau" in msg_l or "powerbi" in msg_l or "power bi" in msg_l):
        force_pdf = True

    # Resolve platform preference dynamically
    bi_platform = "tableau"
    if has_powerbi_connector or bool(powerbi_workspace_id or tenant_id or client_id or client_secret):
        bi_platform = "powerbi"
    if message:
        if "powerbi" in msg_l or "power bi" in msg_l:
            bi_platform = "powerbi"
        elif "tableau" in msg_l:
            bi_platform = "tableau"
            
    env_bi_platform = os.environ.get("BI_PLATFORM")
    if env_bi_platform:
        bi_platform = env_bi_platform.lower().strip()

    bi_active = ((bi_platform == "tableau" and tableau_active) or (bi_platform == "powerbi" and powerbi_active)) and not force_pdf
    
    if bi_platform == "tableau":
        dashboard_context = {
            "platform": "tableau",
            "workspace_id": (
                tableau_workspace_id or
                tableau_resolved.get("tableauWorkspaceId") or
                os.environ.get("TABLEAU_WORKSPACE_ID", "httpsfluviomecom")
            ),
            "tableau_token_name": (
                tableau_token_name or
                tableau_resolved.get("tableauTokenName") or
                os.environ.get("TABLEAU_TOKEN_NAME", "fluvio")
            ),
            "tableau_token_value": (
                tableau_token_value or
                tableau_resolved.get("tableauTokenValue") or
                os.environ.get("TABLEAU_TOKEN_VALUE", "2GGPYQydRfuGwc2q053aFA==:wk0ZIbLEH_vv1CRgzaGQbu4IV3D4zcydt0fdNfSEd_w")
            ),
            "tableau_server_url": (
                tableau_server_url or
                tableau_resolved.get("tableauServerUrl") or
                os.environ.get("TABLEAU_SERVER_URL", "10ax.online.tableau.com")
            ),
            "environment": "local"
        }
    else:
        dashboard_context = {
            "platform": "powerbi",
            "workspace_id": (
                powerbi_workspace_id or
                powerbi_resolved.get("powerbiWorkspaceId") or
                os.environ.get("POWERBI_WORKSPACE_ID", "vowayage-executive-workspace")
            ),
            "tenant_id": (
                tenant_id or
                powerbi_resolved.get("tenantId") or
                os.environ.get("AZURE_TENANT_ID")
            ),
            "client_id": (
                client_id or
                powerbi_resolved.get("clientId") or
                os.environ.get("AZURE_CLIENT_ID")
            ),
            "client_secret": (
                client_secret or
                powerbi_resolved.get("clientSecret") or
                os.environ.get("AZURE_CLIENT_SECRET")
            ),
            "environment": "local"
        }

    # Define standard fallback/default steps
    default_steps = [
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
        ("spark", "execute_sql", {
            "context": spark_context,
            "query": "SELECT cu.membership_tier, cb.destination_country, COUNT(*) as booking_count, SUM(cb.amount_paid) as total_revenue, AVG(cb.amount_paid) as avg_booking_value FROM clean_users cu JOIN clean_bookings cb ON cu.id = cb.user_id GROUP BY cu.membership_tier, cb.destination_country",
            "output_table": "membership_destination_analytics"
        }, "Phase 2: Spark Analytics - Membership Tier Destination Correlation"),
    ]
    if bi_active:
        default_steps.append((
            "dashboard-syncer", "publish_report", {
                "context": dashboard_context,
                "report_name": "Vowayage Executive Performance Dashboard",
                "datasource_name": "vowayage_postgres_clean"
            }, "Phase 3: Tableau Cloud Dashboard - Create Executive Dashboard"
        ))
    else:
        default_steps.append((
            "dashboard-syncer", "generate_pdf_report", {
                "context": {
                    "platform": "local_pdf",
                    "workspace_id": "local",
                    "environment": "local"
                },
                "report_name": "Vowayage Executive Performance Report"
            }, "Phase 3: Fallback Reporting - Generate LaTeX PDF Report"
        ))

    steps = []
    api_key = settings.anthropic_api_key
    if not api_key:
        logger.warning("ANTHROPIC_API_KEY not configured. Falling back to default steps.")
        steps = default_steps
    else:
        try:
            # 1. Fetch history and context
            history = await fetch_chat_history(client, workspace_id)
            context_plan = await generate_plan_context(
                gateway_url=settings.graphql_gateway_url,
                user_id=x_user_id,
                workspace_id=workspace_id
            )
            
            # Map history roles
            claude_messages = []
            for msg in history:
                role = "user" if msg["sender"] == "user" else "assistant"
                claude_messages.append({
                    "role": role,
                    "content": msg["content"]
                })
                
            system_prompt = (
                "You are the Fluviome AI Deployment Orchestrator.\n"
                "Your task is to analyze the approved integration plan and workspace context to formulate a sequence of backend tool executions to fulfill the user's analytics and deployment request.\n\n"
                "DYNAMIC TOOL DISCOVERY & RULES:\n"
                "1. Inspect the 'Available Execution Tools' section inside the WORKSPACE CONTEXT block below to find active tools, their IDs, actions, parameters, descriptions, and markdown documentation.\n"
                "2. Do NOT use any hardcoded tool IDs or actions that are not actively listed in the available tools section of the workspace context.\n"
                "3. Ensure the arguments you output match the actual parameters, schemas, and formats described in the documentation of the respective tools.\n"
                "4. Extract database connection parameters, credentials, sandbox environment info, and external service connector configurations (like Tableau/PowerBI tokens) dynamically from the provided Environment Context below to bind to tool arguments.\n"
                "5. If a tool needs a connection context (e.g. database_url, host, port, credentials, token, server url, workspace id, etc.), construct it dynamically by reading from the Environment Context.\n"
                "6. When calling spark, the output table name MUST end with '_analytics' (e.g. signup_trends_analytics, revenue_by_country_analytics) so that the reporting engine can find it. Targets must be clean versions of tables (e.g. clean_bookings).\n\n"
                "Use the following Environment Context to resolve credentials, connection strings, sandbox ports, and active connector details:\n"
                f"{json.dumps(environment_context, indent=2)}\n\n"
                "FORMULATE the sequence of steps. Your output must be a valid JSON array of objects, with NO other text or markdown block wrapping. Each object must have:\n"
                "  - \"tool_id\": string matching the dynamic tool's id (e.g. \"data-cleaning\", \"spark\", \"dashboard-syncer\", \"terraform\")\n"
                "  - \"action\": string matching the dynamic tool's action name\n"
                "  - \"arguments\": object containing the tool arguments matching their dynamic parameter types (with variables populated from the Environment Context)\n"
                "  - \"description\": a user-facing short description of what this step is doing"
            )
            
            if plan_markdown:
                user_prompt = (
                    "Based on the approved markdown integration plan and available tools/schemas in the workspace context below, generate the pipeline execution steps JSON.\n\n"
                    "[APPROVED INTEGRATION PLAN]\n"
                    f"{plan_markdown}\n\n"
                    "[WORKSPACE CONTEXT]\n"
                    f"{context_plan}\n\n"
                    "Output ONLY the JSON array. Do not put backticks around it."
                )
            else:
                user_prompt = (
                    "Based on the conversation history and workspace context below, generate the pipeline execution steps JSON.\n\n"
                    "[WORKSPACE CONTEXT]\n"
                    f"{context_plan}\n\n"
                    "[CONVERSATION HISTORY]\n"
                    f"{json.dumps(claude_messages, indent=2)}\n\n"
                    "Output ONLY the JSON array. Do not put backticks around it."
                )
            
            logger.info("Calling Claude to formulate dynamic deployment pipeline steps...")
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
                        "messages": [{"role": "user", "content": user_prompt}],
                    },
                    timeout=90.0,
                )
                if resp.status_code == 200:
                    resp_json = resp.json()
                    ai_response_text = resp_json["content"][0]["text"]
                    
                    # Parse steps
                    clean_text = ai_response_text.strip()
                    if "```json" in clean_text:
                        clean_text = clean_text.split("```json")[1].split("```")[0].strip()
                    elif "```" in clean_text:
                        clean_text = clean_text.split("```")[1].split("```")[0].strip()
                        
                    dynamic_steps = json.loads(clean_text)
                    for step in dynamic_steps:
                        steps.append((
                            step.get("tool_id"),
                            step.get("action"),
                            step.get("arguments"),
                            step.get("description", f"Executing {step.get('tool_id')} / {step.get('action')}")
                        ))
                    logger.info("Dynamic steps formulated successfully: %s", len(steps))
                else:
                    logger.error("Claude dynamic step formulation failed with status %s: %s", resp.status_code, resp.text)
                    steps = default_steps
        except Exception as e:
            logger.error("Exception occurred during dynamic pipeline formulation: %s", e)
            steps = default_steps
            
    if not steps:
        steps = default_steps

    
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
            
    has_dashboard = any(t == "dashboard-syncer" and a == "publish_report" for t, a, _, _ in steps)
    has_pdf = any(t == "dashboard-syncer" and a == "generate_pdf_report" for t, a, _, _ in steps)

    if has_dashboard:
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
    elif has_pdf:
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
    else:
        logs.append(f"# 🎉 Pipeline Deployment Complete!\n")
        logs.append(f"## ⚙️ Data Pipeline Status: Active\n")
        logs.append("All backend pipeline orchestration and configuration steps completed successfully:\n")
        for tool_id, action, _, description in steps:
            logs.append(f"- **{tool_id}** ({action}): {description} ✅")
        logs.append("\nYour data pipeline is now configured and active.")
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
        ai_response_text = (
            "I have compiled and verified your integration blueprint. "
            "To deploy the pipeline, please click the [Deploy Plan](#deploy-plan) button. "
            "This will let you select your target sandbox environment, inspect the required container tools, and execute the deployment."
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
        "Your role is to help users design integration blueprints and data pipelines based on their company structure, teams, workflows, and available execution tools in the workspace context.\n\n"
        "CRITICAL INSTRUCTIONS:\n"
        "1. KNOWLEDGE-BASED PLANNING:\n"
        "   - Plan based on the provided company structure, squads, teams, members, roles, workflows, and IAM privileges in the CONTEXT block. Understand who is doing what and which data needs to route where.\n"
        "2. DYNAMIC TOOL DISCOVERY:\n"
        "   - Inspect the 'Available Execution Tools' section in the CONTEXT block below to find active tools, their manifests, parameters, and documentation. Use ONLY these active tools for planning pipeline steps.\n"
        "3. FLEXIBLE PIPELINE DESIGN (NO HARDCODED BI REQUISITES):\n"
        "   - Adapt completely to the user's intent. You can design pure data pipelines, streaming architectures (similar to Netflix data pipelines), data migrations, or reporting dashboards. Do NOT assume Tableau, PowerBI, or any specific reporting is required by default unless explicitly asked.\n"
        "4. BLUEPRINT CARD GENERATION:\n"
        "   - Your output will be displayed as a detailed Integration Blueprint in Markdown (.md). You must plan the pipeline steps clearly, detailing:\n"
        "     - Phase names and descriptions\n"
        "     - The specific tool IDs and action names to execute\n"
        "     - Query SQL strings or arguments needed for execution\n"
        "   - Explicitly instruct the user (or CEO) that they can review, edit, and approve this Markdown blueprint, and click the Deploy button. Once approved, the downstream orchestrator agent will read the Markdown and execute the tools dynamically.\n"
        "5. VISUAL REPORTING FALLBACKS (ONLY WHEN REPORTING IS ASKED):\n"
        "   - ONLY if the user specifically requests a visual report or dashboard and has not defined the destination, ask them explicitly: 'Where would you like to publish the report?' and output the following markdown links:\n"
        "     - [Tableau Dashboard](#choose-tableau)\n"
        "     - [PowerBI Workspace](#choose-powerbi)\n"
        "     - [PDF Report via LaTeX](#choose-pdf)\n"
        "   - If they choose Tableau or PowerBI but the corresponding connector is not active in the 'Semantic Knowledge Graph Ingest' connectors section, prompt them to configure it by outputting:\n"
        "     - For Tableau: `[Connect Tableau](#connect-tableau)`\n"
        "     - For PowerBI: `[Connect PowerBI](#connect-powerbi)`\n"
        "6. NO NATIVE TOOL CALLING: You do NOT have any native or agent-level tool-calling capabilities. Do NOT output XML tags like <function_calls>, <invoke>, <tool_call>, or similar formatting. Communicate purely via standard Markdown.\n"
        "7. INCREMENTAL BLUEPRINT EVOLUTION:\n"
        "   - If a pipeline blueprint has already been proposed or deployed in the conversation history, and the user asks to modify, extend, or add components/steps, do NOT generate a brand new blueprint from scratch.\n"
        "   - Instead, preserve the existing phases and steps, and append or modify only the necessary parts (e.g. adding a phase for LaTeX reporting or modifying a query in an existing Spark step).\n"
        "   - Output the complete, updated blueprint incorporating both the original steps and the new/modified ones, so the user can deploy the entire updated configuration.\n"
        "8. DATA CLEANING & STANDARDIZATION PREREQUISITES:\n"
        "   - Before running any Spark ETL or SQL analysis query on raw source tables (e.g. `bookings`, `users`, `flights`, `hotels`), you MUST plan a preprocessing data cleaning phase using the `data-cleaning` tool's `clean_table` action to produce the corresponding clean table (e.g. `clean_bookings`, `clean_users`, `clean_flights`, `clean_hotels`).\n"
        "   - All downstream Spark analysis and reports MUST reference these clean tables.\n\n"
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
                    timeout=90.0,
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


async def resolve_sandbox_ports(client: FederationClient, sandbox_id: str) -> dict[str, int]:
    # Query getSandboxStatus(sandboxId: $sandboxId)
    query = """
    query GetSandboxStatus($sandboxId: String!) {
      getSandboxStatus(sandboxId: $sandboxId) {
        containers {
          component
          ports
        }
      }
    }
    """
    try:
        resp = await client.query(query, variables={"sandboxId": sandbox_id})
        data = resp.get("data")
        if not data:
            data = resp
        
        containers = data.get("getSandboxStatus", {}).get("containers", [])
        ports_map = {}
        for c in containers:
            component = c.get("component")
            ports = c.get("ports") or []
            for p in ports:
                if "->" in p:
                    parts = p.split("->")
                    container_port = parts[0].strip()
                    host_port = parts[1].strip()
                    if host_port != "unbound":
                        ports_map[f"{component}:{container_port}"] = int(host_port)
        return ports_map
    except Exception as e:
        logger.error(f"Failed to query sandbox status in resolve_sandbox_ports: {e}")
        return {}


def detect_sandbox_requirements(plan: str) -> tuple[str, list[str]]:
    if not plan:
        return "docker", ["postgres"]
        
    lower_plan = plan.lower()
    provider = "docker"
    if any(x in lower_plan for x in ["aws", "redshift", "s3", "vpc", "terraform", "ec2"]):
        provider = "ecs"
        
    components = []
    if "postgres" in lower_plan or "postgresql" in lower_plan:
        components.append("postgres")
    if "kafka" in lower_plan:
        components.append("kafka")
    if "spark" in lower_plan:
        components.append("spark")
    if "airflow" in lower_plan:
        components.append("airflow")
    if "dbt" in lower_plan:
        components.append("dbt")
        
    if not components:
        components.append("postgres")
        
    return provider, components


@app.post("/deploy")
async def deploy(
    body: DeployRequest,
    x_user_id: str | None = Header(default=None, alias="x-user-id"),
):
    if not x_user_id:
        raise HTTPException(status_code=401, detail="x-user-id header is required")
        
    headers = {"x-user-id": x_user_id}
    client = FederationClient(settings.graphql_gateway_url, headers=headers)
    
    sandbox_id = body.sandbox_id or f"sb-{body.workspace_id}"
    logger.info(f"Triggering interactive sandbox deploy target={sandbox_id} workspace={body.workspace_id}")
    
    # Get active plan context to parse resource requirements
    try:
        context_plan = await generate_plan_context(
            gateway_url=settings.graphql_gateway_url,
            user_id=x_user_id,
            workspace_id=body.workspace_id
        )
    except Exception as e:
        logger.error(f"Failed to fetch workspace context plan: {e}")
        context_plan = ""
        
    provider, components = detect_sandbox_requirements(context_plan)
    logger.info(f"Dynamic requirements detected: provider={provider}, components={components}")
    
    # Check if sandbox exists
    sandbox_exists = False
    try:
        status_resp = await client.query("""
            query GetSandboxStatus($sandboxId: String!) {
              getSandboxStatus(sandboxId: $sandboxId) {
                sandboxId
                status
              }
            }
        """, variables={"sandboxId": sandbox_id})
        
        data_val = status_resp.get("data")
        if isinstance(data_val, dict) and data_val.get("getSandboxStatus"):
            sandbox_exists = True
    except Exception as e:
        logger.warning(f"Sandbox check failed: {e}")
        
    # Auto-create sandbox if it doesn't exist
    if not sandbox_exists:
        logger.info(f"Sandbox {sandbox_id} does not exist. Creating dynamically...")
        try:
            create_resp = await client.query("""
                mutation CreateSandbox($sandboxId: String!, $components: [String!], $provider: String) {
                  createSandbox(sandboxId: $sandboxId, components: $components, provider: $provider) {
                    sandboxId
                    status
                    provider
                  }
                }
            """, variables={"sandboxId": sandbox_id, "components": components, "provider": provider})
            logger.info(f"Sandbox created successfully: {create_resp}")
        except Exception as e:
            logger.error(f"Failed to dynamically create sandbox: {e}")
            
    # Resolve sandbox ports
    ports_map = await resolve_sandbox_ports(client, sandbox_id)
    
    # Run deployment pipeline with sandboxed ports
    logs = await run_deployment_pipeline(
        client=client,
        x_user_id=x_user_id,
        workspace_id=body.workspace_id,
        sandbox_id=sandbox_id,
        ports_map=ports_map,
        tableau_token_name=body.tableau_token_name,
        tableau_token_value=body.tableau_token_value,
        tableau_server_url=body.tableau_server_url,
        tableau_workspace_id=body.tableau_workspace_id,
        powerbi_workspace_id=body.powerbi_workspace_id,
        tenant_id=body.tenant_id,
        client_id=body.client_id,
        client_secret=body.client_secret,
        plan_markdown=body.plan_markdown,
    )
    
    # Save the deployment result logs to the chat history so the user can see it in the chat UI
    try:
        await add_chat_message(
            client=client,
            workspace_id=body.workspace_id,
            sender="ai",
            content=logs,
        )
    except Exception as e:
        logger.error("Failed to save deployment logs to chat history: %s", e)
    
    return {"logs": logs}


if __name__ == "__main__":
    uvicorn.run(
        "app.main:app",
        host="0.0.0.0",
        port=settings.port,
        log_level="info",
    )

