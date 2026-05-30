import asyncio
import json
import logging
from typing import Dict, List, Any
from src.tools.kafka.contracts import (
    KafkaTool,
    KafkaExecutionContext,
    KafkaTopicConfig,
    KafkaMessage
)

logger = logging.getLogger("kafka-runtime")

class KafkaRuntime(KafkaTool[Any]):
    """
    Kafka Tool implementation that executes Kafka CLI commands inside the docker container.
    This fulfills the local mode docker runtime assumptions.
    """

    async def _ensure_container_running(self) -> bool:
        """Verify if the fluvio-kafka container is running, and try starting it if not."""
        try:
            # Check if container is running
            proc = await asyncio.create_subprocess_exec(
                "docker", "inspect", "-f", "{{.State.Running}}", "fluvio-kafka",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, _ = await proc.communicate()
            if proc.returncode == 0 and stdout.strip() == b"true":
                return True

            logger.info("fluvio-kafka container is not running. Attempting to start it...")
            # Try running docker-compose up
            import os
            docker_dir = os.path.dirname(os.path.abspath(__file__))
            compose_path = os.path.join(docker_dir, "docker", "docker-compose.yaml")
            
            start_proc = await asyncio.create_subprocess_exec(
                "docker", "compose", "-f", compose_path, "up", "-d",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            await start_proc.communicate()
            if start_proc.returncode == 0:
                logger.info("Successfully started fluvio-kafka container.")
                # Give it a moment to initialize
                await asyncio.sleep(5)
                return True
        except Exception as e:
            logger.error(f"Error checking/starting docker container: {e}")
        return False

    async def create_topic(self, context: KafkaExecutionContext, config: KafkaTopicConfig) -> bool:
        await self._ensure_container_running()
        
        bootstrap = ",".join(context.bootstrap_servers)
        # Match bootstrap_servers host if running inside/outside docker network.
        # Since Kafka docker advertises localhost:9092 to host, we map it to localhost:9092 inside the container command.
        bootstrap_in_container = "localhost:9092"
        
        cmd = [
            "docker", "exec", "fluvio-kafka",
            "kafka-topics.sh", "--create",
            "--bootstrap-server", bootstrap_in_container,
            "--topic", config.name,
            "--partitions", str(config.partitions),
            "--replication-factor", str(config.replication_factor)
        ]
        
        for k, v in config.config.items():
            cmd.extend(["--config", f"{k}={v}"])
            
        try:
            logger.info(f"Creating Kafka topic: {config.name}")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                logger.info(f"Topic '{config.name}' created successfully.")
                return True
            else:
                logger.error(f"Failed to create topic: {stderr.decode()}")
                return False
        except Exception as e:
            logger.error(f"Exception during topic creation: {e}")
            return False

    async def list_topics(self, context: KafkaExecutionContext) -> List[str]:
        await self._ensure_container_running()
        
        cmd = [
            "docker", "exec", "fluvio-kafka",
            "kafka-topics.sh", "--list",
            "--bootstrap-server", "localhost:9092"
        ]
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode == 0:
                topics = stdout.decode().strip().split("\n")
                # Filter out empty or system topics
                return [t.strip() for t in topics if t.strip() and not t.startswith("__")]
            else:
                logger.error(f"Failed to list topics: {stderr.decode()}")
                return []
        except Exception as e:
            logger.error(f"Exception during listing topics: {e}")
            return []

    async def produce(
        self,
        context: KafkaExecutionContext,
        topic: str,
        message: KafkaMessage[Any],
    ) -> bool:
        await self._ensure_container_running()
        
        cmd = [
            "docker", "exec", "-i", "fluvio-kafka",
            "kafka-console-producer.sh",
            "--bootstrap-server", "localhost:9092",
            "--topic", topic,
            "--property", "parse.key=true",
            "--property", "key.separator=:"
        ]
        
        # Serialize the value payload to JSON
        value_str = json.dumps(message.value) if not isinstance(message.value, str) else message.value
        key_str = message.key or ""
        
        # Construct line: key:value
        payload_line = f"{key_str}:{value_str}\n".encode()
        
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate(input=payload_line)
            if proc.returncode == 0:
                logger.info(f"Successfully produced message to topic: {topic}")
                return True
            else:
                logger.error(f"Failed to produce message: {stderr.decode()}")
                return False
        except Exception as e:
            logger.error(f"Exception during produce: {e}")
            return False

    async def consume(
        self,
        context: KafkaExecutionContext,
        topic: str,
        limit: int = 100,
    ) -> List[KafkaMessage[Any]]:
        await self._ensure_container_running()
        
        cmd = [
            "docker", "exec", "fluvio-kafka",
            "kafka-console-consumer.sh",
            "--bootstrap-server", "localhost:9092",
            "--topic", topic,
            "--from-beginning",
            "--max-messages", str(limit),
            "--property", "print.key=true",
            "--property", "key.separator=:"
        ]
        
        try:
            # We enforce a timeout in case there are fewer than 'limit' messages
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            try:
                # Wait up to 5 seconds for consumer to read historical buffer
                stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=5.0)
            except asyncio.TimeoutError:
                # Terminate the process if it times out
                try:
                    proc.terminate()
                except Exception:
                    pass
                stdout, stderr = await proc.communicate()

            messages = []
            if stdout:
                lines = stdout.decode().strip().split("\n")
                for line in lines:
                    if not line.strip():
                        continue
                    if ":" in line:
                        key, val_raw = line.split(":", 1)
                    else:
                        key = None
                        val_raw = line
                    
                    try:
                        # Attempt to parse as JSON, fallback to raw string
                        val = json.loads(val_raw)
                    except Exception:
                        val = val_raw
                        
                    messages.append(KafkaMessage(key=key, value=val))
            return messages
        except Exception as e:
            logger.error(f"Exception during consume: {e}")
            return []

    async def get_consumer_lag(
        self,
        context: KafkaExecutionContext,
        consumer_group: str,
    ) -> Dict[str, Any]:
        await self._ensure_container_running()
        
        cmd = [
            "docker", "exec", "fluvio-kafka",
            "kafka-consumer-groups.sh",
            "--bootstrap-server", "localhost:9092",
            "--describe",
            "--group", consumer_group
        ]
        
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                return {"status": "error", "message": stderr.decode()}
                
            # Parse the tabular output of consumer groups describe command
            output = stdout.decode().strip()
            lines = output.split("\n")
            
            # Find the header row (contains GROUP, TOPIC, PARTITION, CURRENT-OFFSET, LOG-END-OFFSET, LAG...)
            partitions_lag = []
            total_lag = 0
            
            headers = []
            for line in lines:
                if "GROUP" in line and "TOPIC" in line and "LAG" in line:
                    headers = [h.strip() for h in line.split() if h.strip()]
                    continue
                if headers and line.strip():
                    parts = line.split()
                    if len(parts) >= len(headers) - 1:
                        # Extract topic, partition, offset, lag
                        row = dict(zip(headers, parts))
                        lag_val = row.get("LAG", "0")
                        try:
                            lag_int = int(lag_val)
                        except ValueError:
                            lag_int = 0
                        total_lag += lag_int
                        partitions_lag.append({
                            "topic": row.get("TOPIC"),
                            "partition": row.get("PARTITION"),
                            "current_offset": row.get("CURRENT-OFFSET"),
                            "log_end_offset": row.get("LOG-END-OFFSET"),
                            "lag": lag_int
                        })
            
            return {
                "status": "success",
                "consumer_group": consumer_group,
                "total_lag": total_lag,
                "partitions": partitions_lag
            }
        except Exception as e:
            logger.error(f"Exception during lag check: {e}")
            return {"status": "error", "message": str(e)}
