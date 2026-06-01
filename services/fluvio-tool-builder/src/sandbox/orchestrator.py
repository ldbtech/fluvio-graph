import logging
import time
import asyncio
import uuid
import docker
from typing import List, Dict, Any, Optional

logger = logging.getLogger("sandbox-orchestrator")

class AwsEcsOrchestrator:
    def __init__(self):
        # Stores sandbox_id -> sandbox_details
        self.cloud_sandboxes = {}

    def get_sandbox_status(self, sandbox_id: str) -> Optional[Dict[str, Any]]:
        return self.cloud_sandboxes.get(sandbox_id)

    def create_sandbox(self, sandbox_id: str, components: List[str]) -> Dict[str, Any]:
        # Generate simulated AWS ECS Fargate tasks
        containers_status = []
        total_cost = 0.0
        
        # Mapping component -> cost
        cost_rates = {
            "postgres": 0.021,
            "kafka": 0.042,
            "spark": 0.084,
            "airflow": 0.042,
            "dbt": 0.021
        }
        
        images = {
            "postgres": "postgres:16-alpine",
            "kafka": "apache/kafka:3.7.0",
            "spark": "apache/spark:3.5.1",
            "airflow": "apache/airflow:2.9.1",
            "dbt": "ghcr.io/dbt-labs/dbt-postgres:1.7.3"
        }
        
        default_ports = {
            "postgres": ["5432/tcp -> ec2-54-210-99-12.compute-1.amazonaws.com:5432"],
            "kafka": ["9092/tcp -> ec2-54-210-99-12.compute-1.amazonaws.com:9092"],
            "spark": ["8080/tcp -> ec2-54-210-99-12.compute-1.amazonaws.com:8080"],
            "airflow": ["8080/tcp -> ec2-54-210-99-12.compute-1.amazonaws.com:8081"],
            "dbt": ["80/tcp -> unbound"]
        }

        for comp in components:
            cost = cost_rates.get(comp, 0.015)
            total_cost += cost
            task_id = f"task-{comp}-{uuid.uuid4().hex[:8]}"
            arn = f"arn:aws:ecs:us-east-1:123456789012:task/fluvio-cluster/{task_id}"
            
            containers_status.append({
                "name": f"fluvio-ecs-{sandbox_id}-{comp}",
                "component": comp,
                "status": "running",
                "image": images.get(comp, "generic:latest"),
                "ports": default_ports.get(comp, ["80/tcp -> unbound"]),
                "arn": arn,
                "cost_hourly": cost,
                "efficiency_score": 0.95
            })

        self.cloud_sandboxes[sandbox_id] = {
            "sandbox_id": sandbox_id,
            "status": "active",
            "provider": "ecs",
            "containers": containers_status,
            "cost_hourly": total_cost,
            "efficiency_score": 0.96,
            "agent_twin_monitored": True
        }
        return self.cloud_sandboxes[sandbox_id]

    def stop_sandbox(self, sandbox_id: str) -> Dict[str, Any]:
        if sandbox_id in self.cloud_sandboxes:
            sb = self.cloud_sandboxes[sandbox_id]
            sb["status"] = "stopped"
            sb["cost_hourly"] = 0.0
            sb["efficiency_score"] = 0.0
            for c in sb["containers"]:
                c["status"] = "stopped"
                c["cost_hourly"] = 0.0
                c["efficiency_score"] = 0.0
            return sb
        return {"sandbox_id": sandbox_id, "status": "stopped", "provider": "ecs", "containers": []}

    def clean_sandbox(self, sandbox_id: str) -> bool:
        if sandbox_id in self.cloud_sandboxes:
            del self.cloud_sandboxes[sandbox_id]
            return True
        return False

    def list_sandboxes(self) -> List[Dict[str, Any]]:
        return list(self.cloud_sandboxes.values())


class SandboxOrchestrator:
    def __init__(self):
        self.ecs_orchestrator = AwsEcsOrchestrator()
        try:
            self.client = docker.from_env(timeout=2.0)
        except Exception as e:
            logger.error(f"Failed to initialize Docker client: {e}")
            self.client = None

    def _is_docker_available(self) -> bool:
        if self.client is None:
            try:
                self.client = docker.from_env(timeout=2.0)
            except Exception as e:
                logger.error(f"Failed to initialize Docker client: {e}")
                return False
        try:
            self.client.ping()
            return True
        except Exception as e:
            logger.error(f"Docker ping failed: {e}")
            self.client = None
            return False

    def get_network(self, sandbox_id: str) -> Any:
        if not self._is_docker_available():
            raise Exception("Docker daemon is not available.")
        net_name = f"fluvio-sandbox-{sandbox_id}-net"
        try:
            return self.client.networks.get(net_name)
        except docker.errors.NotFound:
            return self.client.networks.create(net_name, driver="bridge")

    async def create_sandbox(self, sandbox_id: str, components: Optional[List[str]] = None, provider: Optional[str] = "docker") -> Dict[str, Any]:
        if provider == "ecs":
            return self.ecs_orchestrator.create_sandbox(sandbox_id, components or ["postgres"])

        if not self._is_docker_available():
            raise Exception("Docker daemon is not available.")

        if components is None or len(components) == 0:
            components = ["postgres"]

        # Ensure network exists
        self.get_network(sandbox_id)
        net_name = f"fluvio-sandbox-{sandbox_id}-net"

        # 1. Postgres
        if "postgres" in components:
            postgres_name = f"fluvio-sandbox-{sandbox_id}-postgres"
            try:
                pg_container = self.client.containers.get(postgres_name)
                if pg_container.status != "running":
                    pg_container.start()
            except docker.errors.NotFound:
                logger.info(f"Creating Postgres container: {postgres_name}")
                pg_container = self.client.containers.run(
                    image="postgres:16-alpine",
                    name=postgres_name,
                    network=net_name,
                    environment={
                        "POSTGRES_DB": "vowayage",
                        "POSTGRES_USER": "postgres",
                        "POSTGRES_PASSWORD": "postgres"
                    },
                    ports={"5432/tcp": None}, # Bind to random host port
                    labels={
                        "fluvio-sandbox-id": sandbox_id,
                        "fluvio-sandbox-component": "postgres"
                    },
                    detach=True
                )

            # Wait for Postgres to be ready
            logger.info(f"Waiting for Postgres container {postgres_name} to be ready...")
            ready = False
            for _ in range(30):
                pg_container.reload()
                if pg_container.status == "running":
                    exit_code, output = pg_container.exec_run("pg_isready -U postgres")
                    if exit_code == 0:
                        ready = True
                        break
                await asyncio.sleep(1)
            
            if not ready:
                raise Exception("Postgres sandbox container failed to initialize or become ready.")

            # Seed Postgres from host
            try:
                await self.seed_postgres(sandbox_id)
            except Exception as e:
                logger.error(f"Failed to seed Postgres sandbox: {e}")
                raise Exception(f"Seeding failed: {e}")

        # 2. Spark
        if "spark" in components:
            spark_name = f"fluvio-sandbox-{sandbox_id}-spark"
            try:
                spark_container = self.client.containers.get(spark_name)
                if spark_container.status != "running":
                    spark_container.start()
            except docker.errors.NotFound:
                logger.info(f"Creating Spark container: {spark_name}")
                spark_container = self.client.containers.run(
                    image="apache/spark:3.5.1",
                    name=spark_name,
                    network=net_name,
                    command="tail -f /dev/null",
                    ports={"8080/tcp": None}, # Spark UI
                    labels={
                        "fluvio-sandbox-id": sandbox_id,
                        "fluvio-sandbox-component": "spark"
                    },
                    detach=True
                )

        # 3. Airflow
        if "airflow" in components:
            airflow_name = f"fluvio-sandbox-{sandbox_id}-airflow"
            try:
                airflow_container = self.client.containers.get(airflow_name)
                if airflow_container.status != "running":
                    airflow_container.start()
            except docker.errors.NotFound:
                logger.info(f"Creating Airflow container: {airflow_name}")
                # Runs Airflow standalone which includes webserver, scheduler, triggerer
                airflow_container = self.client.containers.run(
                    image="apache/airflow:2.9.1",
                    name=airflow_name,
                    network=net_name,
                    command="standalone",
                    environment={
                        "AIRFLOW__CORE__LOAD_EXAMPLES": "False"
                    },
                    ports={"8080/tcp": None}, # Airflow UI
                    labels={
                        "fluvio-sandbox-id": sandbox_id,
                        "fluvio-sandbox-component": "airflow"
                    },
                    detach=True
                )

        # 4. dbt
        if "dbt" in components:
            dbt_name = f"fluvio-sandbox-{sandbox_id}-dbt"
            try:
                dbt_container = self.client.containers.get(dbt_name)
                if dbt_container.status != "running":
                    dbt_container.start()
            except docker.errors.NotFound:
                logger.info(f"Creating dbt container: {dbt_name}")
                dbt_container = self.client.containers.run(
                    image="ghcr.io/dbt-labs/dbt-postgres:1.7.3",
                    name=dbt_name,
                    network=net_name,
                    entrypoint=["sleep", "infinity"],
                    labels={
                        "fluvio-sandbox-id": sandbox_id,
                        "fluvio-sandbox-component": "dbt"
                    },
                    detach=True
                )

        # 5. Kafka
        if "kafka" in components:
            kafka_name = f"fluvio-sandbox-{sandbox_id}-kafka"
            try:
                kafka_container = self.client.containers.get(kafka_name)
                if kafka_container.status != "running":
                    kafka_container.start()
            except docker.errors.NotFound:
                logger.info(f"Creating Kafka container: {kafka_name}")
                # Standard Kraft configuration
                kafka_container = self.client.containers.run(
                    image="apache/kafka:3.7.0",
                    name=kafka_name,
                    network=net_name,
                    environment={
                        "KAFKA_NODE_ID": "1",
                        "KAFKA_PROCESS_ROLES": "broker,controller",
                        "KAFKA_LISTENERS": "PLAINTEXT://0.0.0.0:9092,CONTROLLER://0.0.0.0:9093",
                        "KAFKA_ADVERTISED_LISTENERS": f"PLAINTEXT://{kafka_name}:9092",
                        "KAFKA_CONTROLLER_LISTENER_NAMES": "CONTROLLER",
                        "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP": "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT",
                        "KAFKA_CONTROLLER_QUORUM_VOTERS": "1@localhost:9093",
                        "KAFKA_LOG_DIRS": "/tmp/kraft-combined-logs"
                    },
                    ports={"9092/tcp": None}, # Broker port
                    labels={
                        "fluvio-sandbox-id": sandbox_id,
                        "fluvio-sandbox-component": "kafka"
                    },
                    detach=True
                )

        return await self.get_sandbox_status(sandbox_id)

    async def seed_postgres(self, sandbox_id: str):
        postgres_name = f"fluvio-sandbox-{sandbox_id}-postgres"
        logger.info(f"Seeding Postgres database for sandbox: {sandbox_id}")
        
        # Dump host vowayage database and load it in the container
        cmd = f"pg_dump -d vowayage | docker exec -i {postgres_name} psql -U postgres -d vowayage"
        proc = await asyncio.create_subprocess_shell(
            cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        stdout, stderr = await proc.communicate()
        if proc.returncode != 0:
            raise Exception(f"Failed to seed Postgres database: {stderr.decode().strip()}")
        logger.info(f"Successfully seeded database for {sandbox_id}")

    async def stop_sandbox(self, sandbox_id: str) -> Dict[str, Any]:
        # Try stop in cloud first
        cloud_sb = self.ecs_orchestrator.get_sandbox_status(sandbox_id)
        if cloud_sb:
            return self.ecs_orchestrator.stop_sandbox(sandbox_id)

        if not self._is_docker_available():
            raise Exception("Docker daemon is not available.")
        
        containers = self.client.containers.list(all=True, filters={"label": f"fluvio-sandbox-id={sandbox_id}"})
        for c in containers:
            if c.status == "running":
                logger.info(f"Stopping container: {c.name}")
                c.stop()
        
        return await self.get_sandbox_status(sandbox_id)

    async def clean_sandbox(self, sandbox_id: str) -> bool:
        # Try clean in cloud first
        cloud_sb = self.ecs_orchestrator.get_sandbox_status(sandbox_id)
        if cloud_sb:
            return self.ecs_orchestrator.clean_sandbox(sandbox_id)

        if not self._is_docker_available():
            raise Exception("Docker daemon is not available.")

        containers = self.client.containers.list(all=True, filters={"label": f"fluvio-sandbox-id={sandbox_id}"})
        for c in containers:
            logger.info(f"Removing container: {c.name}")
            try:
                c.remove(force=True)
            except Exception as e:
                logger.error(f"Error removing container {c.name}: {e}")

        # Remove network
        net_name = f"fluvio-sandbox-{sandbox_id}-net"
        try:
            net = self.client.networks.get(net_name)
            logger.info(f"Removing network: {net_name}")
            net.remove()
        except docker.errors.NotFound:
            pass
        except Exception as e:
            logger.error(f"Error removing network {net_name}: {e}")
        
        return True

    async def get_sandbox_status(self, sandbox_id: str) -> Dict[str, Any]:
        # Check cloud first
        cloud_sb = self.ecs_orchestrator.get_sandbox_status(sandbox_id)
        if cloud_sb:
            return cloud_sb

        if not self._is_docker_available():
            return {"sandbox_id": sandbox_id, "status": "stopped", "provider": "docker", "containers": [], "cost_hourly": 0.0, "efficiency_score": 1.0, "agent_twin_monitored": False}

        containers = self.client.containers.list(all=True, filters={"label": f"fluvio-sandbox-id={sandbox_id}"})
        if not containers:
            return {"sandbox_id": sandbox_id, "status": "not_found", "provider": "docker", "containers": [], "cost_hourly": 0.0, "efficiency_score": 1.0, "agent_twin_monitored": False}

        all_running = True
        any_running = False
        containers_status = []

        for c in containers:
            c.reload()
            component = c.labels.get("fluvio-sandbox-component", "unknown")
            status = c.status
            if status == "running":
                any_running = True
            else:
                all_running = False

            # Format ports
            ports_config = c.attrs.get('NetworkSettings', {}).get('Ports', {}) or {}
            formatted_ports = []
            for container_port, host_bindings in ports_config.items():
                if host_bindings:
                    for binding in host_bindings:
                        formatted_ports.append(f"{container_port} -> {binding.get('HostPort')}")
                else:
                    formatted_ports.append(f"{container_port} -> unbound")

            containers_status.append({
                "name": c.name,
                "component": component,
                "status": status,
                "image": c.image.tags[0] if c.image.tags else c.image.id,
                "ports": formatted_ports,
                "arn": None,
                "cost_hourly": 0.0,
                "efficiency_score": 0.98 if status == "running" else 0.0
            })

        sandbox_status = "active" if all_running else ("stopped" if not any_running else "degraded")
        return {
            "sandbox_id": sandbox_id,
            "status": sandbox_status,
            "provider": "docker",
            "containers": containers_status,
            "cost_hourly": 0.0,
            "efficiency_score": 0.98 if all_running else 0.0,
            "agent_twin_monitored": True
        }

    async def list_sandboxes(self) -> List[Dict[str, Any]]:
        # Get cloud sandboxes
        cloud_sbs = self.ecs_orchestrator.list_sandboxes()
        
        # Get local sandboxes
        local_sbs = []
        if self._is_docker_available():
            try:
                # Fetch all containers in a single Docker API call to avoid N+1 query overhead
                containers = self.client.containers.list(all=True)
                
                # Group containers by sandbox ID
                sandbox_groups = {}
                for c in containers:
                    s_id = c.labels.get("fluvio-sandbox-id")
                    if s_id:
                        if s_id not in sandbox_groups:
                            sandbox_groups[s_id] = []
                        sandbox_groups[s_id].append(c)
                
                for s_id, group in sorted(sandbox_groups.items()):
                    all_running = True
                    any_running = False
                    containers_status = []
                    
                    for c in group:
                        component = c.labels.get("fluvio-sandbox-component", "unknown")
                        status = c.status
                        if status == "running":
                            any_running = True
                        else:
                            all_running = False
                            
                        # Format ports without calling the daemon (using cached container attrs)
                        ports_config = c.attrs.get('NetworkSettings', {}).get('Ports', {}) or {}
                        formatted_ports = []
                        for container_port, host_bindings in ports_config.items():
                            if host_bindings:
                                for binding in host_bindings:
                                    formatted_ports.append(f"{container_port} -> {binding.get('HostPort')}")
                            else:
                                formatted_ports.append(f"{container_port} -> unbound")
                                
                        # Extract image name safely from cached attrs to avoid registry/daemon calls
                        image_name = "generic:latest"
                        if c.attrs.get('Config', {}).get('Image'):
                            image_name = c.attrs['Config']['Image']
                        elif c.image.tags:
                            image_name = c.image.tags[0]
                        else:
                            image_name = c.image.id[:12]
                            
                        containers_status.append({
                            "name": c.name,
                            "component": component,
                            "status": status,
                            "image": image_name,
                            "ports": formatted_ports,
                            "arn": None,
                            "cost_hourly": 0.0,
                            "efficiency_score": 0.98 if status == "running" else 0.0
                        })
                        
                    sandbox_status = "active" if all_running else ("stopped" if not any_running else "degraded")
                    local_sbs.append({
                        "sandbox_id": s_id,
                        "status": sandbox_status,
                        "provider": "docker",
                        "containers": containers_status,
                        "cost_hourly": 0.0,
                        "efficiency_score": 0.98 if all_running else 0.0,
                        "agent_twin_monitored": True
                    })
            except Exception as e:
                logger.error(f"Error listing local sandboxes: {e}")
                    
        return cloud_sbs + local_sbs

    def get_container_port(self, sandbox_id: str, component: str, container_port_proto: str = "5432/tcp") -> Optional[int]:
        """Helper to get the mapped host port for a sandbox service container."""
        if not self._is_docker_available():
            return None
        container_name = f"fluvio-sandbox-{sandbox_id}-{component}"
        try:
            container = self.client.containers.get(container_name)
            container.reload()
            ports = container.attrs.get('NetworkSettings', {}).get('Ports', {}) or {}
            host_bindings = ports.get(container_port_proto)
            if host_bindings:
                return int(host_bindings[0]['HostPort'])
        except Exception as e:
            logger.error(f"Error resolving port for {container_name}: {e}")
        return None

orchestrator = SandboxOrchestrator()
