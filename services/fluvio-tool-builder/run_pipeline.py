import asyncio
import json
import os
import sys

# Load env variables from root .env if it exists
from dotenv import load_dotenv
load_dotenv(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../.env"))

# Ensure PYTHONPATH includes root directory of tool builder
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from src.tools.registry import registry

async def run_step(tool_id: str, action: str, arguments: dict):
    print(f"\n=========================================")
    print(f"🚀 Executing Tool: {tool_id} -> {action}")
    print(f"Arguments: {json.dumps(arguments, indent=2)}")
    print(f"=========================================")
    
    logs = []
    # DynamicRegistry expects inputs_json to contain { "action": ..., "arguments": ... }
    inputs = {
        "action": action,
        "arguments": json.dumps(arguments)
    }
    inputs_json = json.dumps(inputs)
    
    try:
        res = await registry.execute_tool_action(tool_id, inputs_json, logs)
        print("\n--- Execution Logs ---")
        for log in logs:
            print(log)
        print("----------------------")
        
        print("\n--- Result Status ---")
        print(json.dumps(res, indent=2))
        print("----------------------")
        return res
    except Exception as e:
        print(f"❌ Execution exception: {e}")
        return {"status": "failed", "error": str(e)}

async def main():
    print("=================================================================")
    print("🎯 STARTING AUTOMATED VOWAYAGE DATABASE ANALYTICS PIPELINE")
    print("=================================================================")
    
    # We target the actual company database: vowayage
    db_context = {
        "database_url": "postgres://localhost/vowayage",
        "environment": "local"
    }
    
    # ----------------------------------------------------
    # Phase 1: Database Cleaning & Preparation
    # ----------------------------------------------------
    print("\n--- Phase 1: Data Cleansing & Preparation ---")
    
    # Step 1: Clean Users Table
    await run_step(
        tool_id="data-cleaning",
        action="clean_table",
        arguments={
            "context": db_context,
            "table_name": "users",
            "operations": ["normalize_headers", "drop_nulls", "deduplicate"]
        }
    )
    
    # Step 2: Clean Bookings Table
    await run_step(
        tool_id="data-cleaning",
        action="clean_table",
        arguments={
            "context": db_context,
            "table_name": "bookings",
            "operations": ["normalize_headers", "drop_nulls", "deduplicate", "standardize_currency"]
        }
    )
    
    # ----------------------------------------------------
    # Phase 2: Spark Analytics Execution
    # ----------------------------------------------------
    print("\n--- Phase 2: Spark Analytics Execution ---")
    
    spark_context = {
        "master_url": "local[*]",
        "app_name": "VowayageAnalyticsJob",
        "environment": "local"
    }
    
    # Step 3: Signup Trends Analysis
    # Analyzes user signup growth monthly
    signup_query = """
    SELECT 
        DATE_TRUNC('month', created_at) as month, 
        COUNT(*) as new_users,
        SUM(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)) as cumulative_users
    FROM clean_users 
    GROUP BY DATE_TRUNC('month', created_at) 
    ORDER BY month
    """
    await run_step(
        tool_id="spark",
        action="execute_sql",
        arguments={
            "context": spark_context,
            "query": signup_query.strip(),
            "output_table": "signup_trends_analytics"
        }
    )
    
    # Step 4: Booking Revenue & Count by Country
    revenue_query = """
    SELECT 
        destination_country, 
        COUNT(*) as total_bookings, 
        SUM(amount_paid) as total_revenue
    FROM clean_bookings 
    GROUP BY destination_country 
    ORDER BY total_revenue DESC
    """
    await run_step(
        tool_id="spark",
        action="execute_sql",
        arguments={
            "context": spark_context,
            "query": revenue_query.strip(),
            "output_table": "revenue_by_country_analytics"
        }
    )
    
    # Step 5: User Membership Tier Metrics
    membership_query = """
    SELECT 
        membership_tier, 
        COUNT(*) as user_count, 
        AVG(monthly_membership_fee) as avg_fee
    FROM clean_users 
    GROUP BY membership_tier 
    ORDER BY user_count DESC
    """
    await run_step(
        tool_id="spark",
        action="execute_sql",
        arguments={
            "context": spark_context,
            "query": membership_query.strip(),
            "output_table": "membership_metrics_analytics"
        }
    )
    
    # ----------------------------------------------------
    # Phase 3: BI Dashboard Synchronization
    # ----------------------------------------------------
    print("\n--- Phase 3: BI Dashboard Synchronization ---")
    
    bi_platform = os.environ.get("BI_PLATFORM", "powerbi").lower().strip()
    if bi_platform == "tableau":
        dashboard_context = {
            "platform": "tableau",
            "workspace_id": os.environ.get("TABLEAU_WORKSPACE_ID", os.environ.get("TABLEAU_SITE_ID", "vowayage-executive-site")),
            "tableau_token_name": os.environ.get("TABLEAU_TOKEN_NAME"),
            "tableau_token_value": os.environ.get("TABLEAU_TOKEN_VALUE"),
            "tableau_server_url": os.environ.get("TABLEAU_SERVER_URL", "10ax.online.tableau.com"),
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
    
    # Step 6: Deploy/Publish BI Report
    publish_res = await run_step(
        tool_id="dashboard-syncer",
        action="publish_report",
        arguments={
            "context": dashboard_context,
            "report_name": "Vowayage Executive Performance Dashboard",
            "datasource_name": "vowayage_postgres_clean"
        }
    )
    
    # Step 7: Trigger Live Dataset Refresh
    dataset_id = "ds_vowayage_growth_123"
    if publish_res.get("status") == "success":
        res_data = publish_res.get("result", {})
        if "dataset_id" in res_data:
            dataset_id = res_data["dataset_id"]
        elif "report_id" in res_data:
            dataset_id = res_data["report_id"]
            
    await run_step(
        tool_id="dashboard-syncer",
        action="trigger_refresh",
        arguments={
            "context": dashboard_context,
            "dataset_id": dataset_id
        }
    )
    
    # Step 8: Retrieve Secure Embed/Share URL
    report_id = "rep_vowayage_growth_456"
    if publish_res.get("status") == "success":
        res_data = publish_res.get("result", {})
        if "report_id" in res_data:
            report_id = res_data["report_id"]
            
    await run_step(
        tool_id="dashboard-syncer",
        action="get_share_link",
        arguments={
            "context": dashboard_context,
            "report_id": report_id
        }
    )
    
    print("\n=================================================================")
    print("🎉 AUTOMATED PIPELINE EXECUTION COMPLETED SUCCESSFULLY!")
    print("=================================================================")

if __name__ == "__main__":
    asyncio.run(main())
