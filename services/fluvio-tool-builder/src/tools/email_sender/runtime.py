import os
import re
import ssl
import smtplib
import logging
from email.message import EmailMessage
from typing import Dict, Any, List, Optional

from src.config import REPORTS_DIR
from src.tools.email_sender.contracts import EmailSenderTool, EmailExecutionContext

logger = logging.getLogger("email-sender-runtime")

_ME_TOKENS = {"me", "myself", "self", "@me"}
_EMAIL_RE = re.compile(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")


class EmailSenderRuntime(EmailSenderTool):
    """Generic SMTP sender. SMTP_* env vars configure the transport so the same
    contract can later be backed by AWS SES (SMTP interface or API) without
    changing callers."""

    def _resolve_recipients(self, addrs: Optional[List[str]], ctx: EmailExecutionContext) -> List[str]:
        out: List[str] = []
        for a in addrs or []:
            if not a:
                continue
            a = a.strip()
            if a.lower() in _ME_TOKENS:
                if ctx.user_email:
                    out.append(ctx.user_email)
                # if "me" requested but no user_email known, skip silently here;
                # validated later so the caller gets a clear error if nothing resolves
                continue
            out.append(a)
        # de-dupe, preserve order
        seen = set()
        return [x for x in out if not (x in seen or seen.add(x))]

    def _resolve_attachment(self, path: str) -> Optional[str]:
        if not path:
            return None
        if os.path.isabs(path) and os.path.exists(path):
            return path
        # bare filename (or relative) -> resolve against the reports dir
        candidate = os.path.join(REPORTS_DIR, os.path.basename(path))
        if os.path.exists(candidate):
            return candidate
        if os.path.exists(path):
            return os.path.abspath(path)
        return None

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
        # ── SMTP config from environment (no hardcoded creds) ──────────────────
        host = os.getenv("SMTP_HOST")
        port = int(os.getenv("SMTP_PORT", "587"))
        user = os.getenv("SMTP_USER")
        password = os.getenv("SMTP_PASS")
        sender = os.getenv("SMTP_FROM") or user
        use_tls = os.getenv("SMTP_STARTTLS", "true").lower() != "false"

        if not host or not sender:
            return {
                "status": "failed",
                "error": (
                    "SMTP is not configured. Set SMTP_HOST, SMTP_PORT, SMTP_USER, "
                    "SMTP_PASS and SMTP_FROM in the environment (.env) so email can "
                    "be sent. No mail was sent."
                ),
            }

        # ── Recipients ────────────────────────────────────────────────────────
        to_list = self._resolve_recipients(to, context)
        cc_list = self._resolve_recipients(cc, context)
        if not to_list:
            wanted_me = any((a or "").strip().lower() in _ME_TOKENS for a in (to or []))
            if wanted_me and not context.user_email:
                return {"status": "failed", "error": "Recipient 'me' was requested but the authenticated user's email is unknown."}
            return {"status": "failed", "error": "No recipients provided."}

        bad = [a for a in (to_list + cc_list) if not _EMAIL_RE.match(a)]
        if bad:
            return {"status": "failed", "error": f"Invalid recipient address(es): {', '.join(bad)}"}

        # ── Compose ───────────────────────────────────────────────────────────
        full_body = body or ""
        if links:
            link_lines = "\n".join(f"- {u}" for u in links if u)
            if link_lines:
                if body_is_html:
                    items = "".join(f'<li><a href="{u}">{u}</a></li>' for u in links if u)
                    full_body += f"<h3>Links</h3><ul>{items}</ul>"
                else:
                    full_body += f"\n\nLinks:\n{link_lines}\n"

        msg = EmailMessage()
        msg["From"] = sender
        msg["To"] = ", ".join(to_list)
        if cc_list:
            msg["Cc"] = ", ".join(cc_list)
        msg["Subject"] = subject or "(no subject)"
        if body_is_html:
            msg.set_content("This message requires an HTML-capable email client.")
            msg.add_alternative(full_body, subtype="html")
        else:
            msg.set_content(full_body)

        # ── Attachments (PDF report, etc.) ────────────────────────────────────
        attached: List[str] = []
        missing: List[str] = []
        for path in attachments or []:
            resolved = self._resolve_attachment(path)
            if not resolved:
                missing.append(path)
                continue
            try:
                with open(resolved, "rb") as f:
                    data = f.read()
                ctype = "application/pdf" if resolved.lower().endswith(".pdf") else "application/octet-stream"
                maintype, subtype = ctype.split("/", 1)
                msg.add_attachment(data, maintype=maintype, subtype=subtype, filename=os.path.basename(resolved))
                attached.append(os.path.basename(resolved))
            except Exception as e:
                logger.error("Failed to attach %s: %s", resolved, e)
                missing.append(path)

        if missing:
            # Fail honestly: a report email that silently drops its attachment
            # would misrepresent the deliverable to a client.
            return {
                "status": "failed",
                "error": f"Could not attach the following file(s): {', '.join(missing)}. No mail was sent.",
            }

        # ── Send ──────────────────────────────────────────────────────────────
        recipients = to_list + cc_list
        try:
            if port == 465:
                ctx_ssl = ssl.create_default_context()
                with smtplib.SMTP_SSL(host, port, context=ctx_ssl, timeout=30) as s:
                    if user and password:
                        s.login(user, password)
                    s.send_message(msg, from_addr=sender, to_addrs=recipients)
            else:
                with smtplib.SMTP(host, port, timeout=30) as s:
                    s.ehlo()
                    if use_tls:
                        s.starttls(context=ssl.create_default_context())
                        s.ehlo()
                    if user and password:
                        s.login(user, password)
                    s.send_message(msg, from_addr=sender, to_addrs=recipients)
        except Exception as e:
            logger.error("SMTP send failed: %s", e, exc_info=True)
            return {"status": "failed", "error": f"SMTP send failed: {e}"}

        logger.info("Email sent to %s (cc %s) with %d attachment(s).", to_list, cc_list, len(attached))
        return {
            "status": "success",
            "recipients": to_list,
            "cc": cc_list,
            "attachments": attached,
            "subject": msg["Subject"],
        }
