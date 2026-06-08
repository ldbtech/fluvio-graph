import logging
from src.tools.terraform.contracts import TerraformApplyConfig, TerraformDestroyConfig

logger = logging.getLogger("terraform-runtime")

# Real Terraform provisioning (HCL generation, AWS credentials, remote backend
# state) is not wired yet. Rather than fabricate resource IDs and endpoints — which
# would tell a client their infrastructure is live when it is not — every action
# fails honestly and asks the user to report it. Do NOT return simulated success.
_NOT_WIRED = (
    "Real Terraform provisioning is not available yet, so no AWS infrastructure "
    "was created or changed. Please report this to the Fluviome team so we can "
    "enable cloud provisioning for your workspace."
)


class TerraformRuntime:
    async def apply_infrastructure(self, config: TerraformApplyConfig) -> dict:
        logger.error("terraform apply requested but not wired (config: %s)", config.config_name)
        return {"status": "failed", "config_name": config.config_name, "error": _NOT_WIRED}

    async def destroy_infrastructure(self, config: TerraformDestroyConfig) -> dict:
        logger.error("terraform destroy requested but not wired (config: %s)", config.config_name)
        return {"status": "failed", "config_name": config.config_name, "error": _NOT_WIRED}

    async def get_infrastructure_status(self, config_name: str) -> dict:
        logger.error("terraform status requested but not wired (config: %s)", config_name)
        return {"status": "failed", "config_name": config_name, "error": _NOT_WIRED}
