from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Generic, TypeVar, Optional, Dict, Any, List
from pydantic import BaseModel 

# ============================
# Generic Payload Type
# ============================
T = TypeVar("T")

# ============================
# Core Kafka Message Contract
# ============================
class KafkaMessage(BaseModel, Generic[T]):
    """
        Generic Kafka Message.
        The Payload is completely schema-agnostic 
    """

    key: Optional[str] = None
    value: T
    headers: Dict[str, str] = {}
    timestamp: Optional[int] = None

# ============================
# Topic Contract
# ============================
class KafkaTopicConfig(BaseModel):
    name: str
    partitions: int = 1
    replication_factor: int = 1
    config: Dict[str, Any] = {}

# ============================
# Execution Context
# ============================
class KafkaExecutionContext(BaseModel):
    """
        Runtime context passed to every tool execution.
        This makes kafka tool environment-aware.
    """
    cluster_id: str 
    environment: str # local | dev | prod
    bootstrap_servers: List[str]

# ============================
# Capability Contract
# ============================
class KafkaTool(ABC, Generic[T]):
    """
        This defines What KAFKA can do in your system

        Not implementation. 
        Only capabilities exposed to the agent.
    """

    name: str = "kafka"

    # ============================
    # Topic Contract
    # ============================
    @abstractmethod
    async def create_topic(self, context: KafkaExecutionContext, config: KafkaTopicConfig) -> bool:
        pass

    @abstractmethod
    async def list_topics(
        self, 
        context: KafkaExecutionContext,
    ) -> List[str]: 
        pass

    # ============================
    # Producer / Consumer Ops
    # ============================
    @abstractmethod
    async def produce(
        self,
        context: KafkaExecutionContext,
        topic: str,
        message: KafkaMessage[T],
    ) -> bool:
        pass

    @abstractmethod
    async def consume(
        self,
        context: KafkaExecutionContext,
        topic: str,
        limit: int = 100,
    ) -> List[KafkaMessage[T]]:
        pass

    # ============================
    # Observability Ops
    # ============================
    @abstractmethod
    async def get_consumer_lag(
        self,
        context: KafkaExecutionContext,
        consumer_group: str,
    ) -> Dict[str, Any]:
        pass