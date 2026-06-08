from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel


# ============================
# Execution Context
# ============================
class EmailExecutionContext(BaseModel):
    """Routing + identity context for outbound email.

    SMTP credentials are read from the environment by the runtime (generic SMTP
    today; AWS SES API can be swapped in later behind the same send_report
    contract). ``user_email`` / ``company_email`` are injected by the agent-planner
    worker at execution time so the planner can address mail to "me" without ever
    seeing the address. ``environment`` mirrors other tools.
    """
    user_email: Optional[str] = None      # authenticated user's address (for "me")
    company_email: Optional[str] = None   # the user's company email, if any
    environment: str = "local"


# ============================
# Capability Contract
# ============================
class EmailSenderTool(ABC):
    """Send a composed report email (plan summary, PDF attachment, BI links)."""
    name: str = "email-sender"

    @abstractmethod
    async def send_report(
        self,
        context: EmailExecutionContext,
        to: List[str],
        subject: str,
        body: str,
        cc: Optional[List[str]] = None,
        attachments: Optional[List[str]] = None,
        links: Optional[List[str]] = None,
        body_is_html: bool = False,
    ) -> Dict[str, Any]:
        """Send an email.

        Args:
          to / cc:      recipient addresses. The literal tokens "me", "myself" or
                        "self" resolve to context.user_email.
          subject:      email subject line.
          body:         message body (plain text or HTML — see body_is_html). The
                        planner authors this from the plan details / report summary.
          attachments:  absolute file paths, OR bare filenames which are resolved
                        against the reports directory (e.g. the generated PDF).
          links:        URLs (e.g. a Tableau share link or the report web URL) that
                        are appended to the body as a "Links" section.
          body_is_html: render the body as HTML instead of plain text.
        """
        raise NotImplementedError
