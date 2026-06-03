"""Credential vault abstraction.

Steps carry credential_ref tokens instead of raw secrets.
The executor resolves refs to actual values at execution time so credentials
never appear in Claude's context window or in logs.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class CredentialRef:
    ref: str  # "cred:<workspace_id>:<connector_kind>"

    @classmethod
    def build(cls, workspace_id: str, connector_kind: str) -> "CredentialRef":
        return cls(ref=f"cred:{workspace_id}:{connector_kind}")

    @property
    def connector_kind(self) -> str:
        return self.ref.split(":")[2] if self.ref.count(":") == 2 else ""


def resolve_credentials(ref: CredentialRef, connector_configs: dict[str, dict]) -> dict:
    """Return the actual credential dict for ref from already-fetched connector configs."""
    return connector_configs.get(ref.connector_kind, {})


def scrub_credentials(environment_context: dict) -> dict:
    """Return a copy of environment_context with secrets replaced by credential_ref tokens.

    This is the object Claude sees — no passwords, tokens, or connection strings.
    """
    workspace_id = environment_context.get("sandbox_id", "workspace")
    scrubbed = {
        "sandbox_id": environment_context.get("sandbox_id"),
        "environment": environment_context.get("environment"),
        "ports_map": environment_context.get("ports_map", {}),
        "database": {
            "host": environment_context.get("database", {}).get("host"),
            "port": environment_context.get("database", {}).get("port"),
            "database": environment_context.get("database", {}).get("database"),
            "credential_ref": CredentialRef.build(workspace_id, "postgres").ref,
        },
        "connectors": {
            kind: {"credential_ref": CredentialRef.build(workspace_id, kind).ref}
            for kind in environment_context.get("connectors", {})
        },
    }
    return scrubbed
