from pydantic import BaseModel, Field
from typing import Optional, Dict, Any

class TerraformApplyConfig(BaseModel):
    config_name: str = Field(description="Unique configuration deployment slug")
    vpc_cidr: Optional[str] = Field(default="10.0.0.0/16", description="Virtual Private Cloud CIDR block")
    redshift_node_type: Optional[str] = Field(default="dc2.large", description="Redshift cluster node hardware type")
    s3_bucket_name: Optional[str] = Field(default=None, description="Global unique S3 bucket name")
    enable_monitoring: Optional[bool] = Field(default=True, description="Enable cloudwatch metrics and Agent Twin monitoring link")

class TerraformDestroyConfig(BaseModel):
    config_name: str = Field(description="Configuration deployment slug to destroy")
