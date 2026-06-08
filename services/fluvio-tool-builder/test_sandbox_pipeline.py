import asyncio
import os
import sys
import json
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("test-sandbox-pipeline")

# Add the tool builder directory to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from src.sandbox.orchestrator import orchestrator
from src.tools.registry import registry
from src.tools.database.contracts import DatabaseExecutionContext
from src.tools.data_cleaning.contracts import DataCleaningExecutionContext
from src.tools.spark.contracts import SparkExecutionContext
from src.tools.airflow.contracts import AirflowExecutionContext
from src.tools.dbt.contracts import DbtExecutionContext
from src.tools.kafka.contracts import KafkaExecutionContext, KafkaTopicConfig

async def run_tool_action(tool_id: str, action: str, arguments: dict):
    logger.info(f"🚀 Running Tool: {tool_id} -> {action}")
    inputs = {
        "action": action,
        "arguments": json.dumps(arguments)
    }
    inputs_json = json.dumps(inputs)
    logs = []
    res = await registry.execute_tool_action(tool_id, inputs_json, logs)
    logger.info(f"Logs:\n" + "\n".join(f"  {l}" for l in logs))
    logger.info(f"Result: {json.dumps(res, indent=2)}")
    return res

async def main():
    sandbox_id = "test-sandbox-env"
    
    logger.info("--------------------------------------------------")
    logger.info("🧹 1. Cleaning up any leftover sandbox containers...")
    logger.info("--------------------------------------------------")
    await orchestrator.clean_sandbox(sandbox_id)
    
    logger.info("--------------------------------------------------")
    logger.info("🏗️ 2. Creating Sandbox and seeding Database...")
    logger.info("--------------------------------------------------")
    status = await orchestrator.create_sandbox(sandbox_id)
    logger.info(f"Sandbox created. Status: {status['status']}")
    for c in status['containers']:
        logger.info(f"  Container: {c['name']} | Component: {c['component']} | Status: {c['status']} | Ports: {c['ports']}")
        
    logger.info("--------------------------------------------------")
    logger.info("📊 3. Verifying Database tables inside sandbox...")
    logger.info("--------------------------------------------------")
    db_ctx = DatabaseExecutionContext(
        database_url="postgres://postgres:postgres@localhost:5432/vowayage",
        environment="local",
        sandbox_id=sandbox_id
    )
    res_tables = await run_tool_action("database", "list_tables", {"context": db_ctx.model_dump()})
    assert res_tables["status"] == "success", "Failed to list tables in Postgres sandbox."
    tables = res_tables["result"]
    logger.info(f"Tables in sandbox: {tables}")
    assert "users" in tables and "bookings" in tables, "Sandbox Postgres was not properly seeded with vowayage tables."

    logger.info("--------------------------------------------------")
    logger.info("🧼 4. Running Data Cleansing inside sandbox...")
    logger.info("--------------------------------------------------")
    clean_ctx = DataCleaningExecutionContext(
        database_url="postgres://postgres:postgres@localhost:5432/vowayage",
        environment="local",
        sandbox_id=sandbox_id
    )
    res_clean = await run_tool_action("data-cleaning", "run_cleaning", {
        "context": clean_ctx.model_dump(),
        "table_name": "users",
        "statements": [
            "DELETE FROM {table} WHERE email IS NULL OR email = '';",
        ],
    })
    assert res_clean["status"] == "success", "Failed to clean users table in sandbox."
    
    # Verify clean_users table now exists
    res_tables = await run_tool_action("database", "list_tables", {"context": db_ctx.model_dump()})
    assert "clean_users" in res_tables["result"], "clean_users table not found in sandbox after cleansing."

    logger.info("--------------------------------------------------")
    logger.info("⚡ 5. Running Spark SQL inside sandbox container...")
    logger.info("--------------------------------------------------")
    spark_ctx = SparkExecutionContext(
        master_url="local[*]",
        app_name="TestSandboxSparkJob",
        environment="local",
        sandbox_id=sandbox_id
    )
    query = "SELECT membership_tier, COUNT(*) as user_count FROM clean_users GROUP BY membership_tier"
    res_spark = await run_tool_action("spark", "execute_sql", {
        "context": spark_ctx.model_dump(),
        "query": query,
        "output_table": "spark_tier_metrics"
    })
    assert res_spark["status"] == "success", "Spark query failed in sandbox."
    
    # Verify fallback spark query created table in Postgres sandbox
    res_tables = await run_tool_action("database", "list_tables", {"context": db_ctx.model_dump()})
    assert "spark_tier_metrics" in res_tables["result"], "Spark output table not found in Postgres sandbox."

    logger.info("--------------------------------------------------")
    logger.info("🌬️ 6. Triggering Airflow DAG inside sandbox...")
    logger.info("--------------------------------------------------")
    airflow_ctx = AirflowExecutionContext(
        host_url="http://localhost:8080",
        environment="local",
        sandbox_id=sandbox_id
    )
    res_airflow = await run_tool_action("airflow", "trigger_dag", {
        "context": airflow_ctx.model_dump(),
        "dag_id": "vowayage_etl_dag",
        "conf": {"run_date": "2026-05-30"}
    })
    assert res_airflow["status"] == "success", "Airflow trigger failed."

    logger.info("--------------------------------------------------")
    logger.info("📈 7. Running dbt models inside sandbox...")
    logger.info("--------------------------------------------------")
    dbt_ctx = DbtExecutionContext(
        project_dir="/workspace/dbt_project",
        profile_name="vowayage",
        target_name="dev",
        sandbox_id=sandbox_id
    )
    res_dbt = await run_tool_action("dbt", "run_models", {
        "context": dbt_ctx.model_dump()
    })
    assert res_dbt["status"] == "success", "dbt run failed."

    logger.info("--------------------------------------------------")
    logger.info("📬 8. Running Kafka operations inside sandbox...")
    logger.info("--------------------------------------------------")
    kafka_ctx = KafkaExecutionContext(
        cluster_id="sandbox-cluster",
        environment="local",
        bootstrap_servers=["localhost:9092"],
        sandbox_id=sandbox_id
    )
    # Create topic
    topic_config = KafkaTopicConfig(name="sandbox-events", partitions=1, replication_factor=1)
    res_kafka_topic = await run_tool_action("kafka", "create_topic", {
        "context": kafka_ctx.model_dump(),
        "config": topic_config.model_dump()
    })
    logger.info(f"Kafka topic create result: {res_kafka_topic}")

    # List topics
    res_kafka_list = await run_tool_action("kafka", "list_topics", {
        "context": kafka_ctx.model_dump()
    })
    logger.info(f"Kafka topic list result: {res_kafka_list}")

    logger.info("--------------------------------------------------")
    logger.info("🧹 9. Cleaning up sandbox...")
    logger.info("--------------------------------------------------")
    clean_res = await orchestrator.clean_sandbox(sandbox_id)
    assert clean_res is True, "Failed to clean up sandbox."
    
    logger.info("==============================================")
    logger.info("🎉 ALL SANDBOX INTEGRATION TESTS PASSED SUCCESSFULLY!")
    logger.info("==============================================")

if __name__ == "__main__":
    asyncio.run(main())
