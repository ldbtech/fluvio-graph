# Email Report Sender (`email-sender`)

Delivers a finished deliverable by email. Use this as the **final step** of a plan
when the user asks to "send", "email", or "share" the results.

## Action: `send_report`

Arguments (JSON string under `arguments`):

| field | type | notes |
|-------|------|-------|
| `to` | list[str] | **required.** Recipient emails. Use the literal token `"me"` to send to the requesting user — their address is resolved automatically; do not guess it. |
| `cc` | list[str] | optional carbon-copy recipients. |
| `subject` | str | author a clear subject, e.g. `"Q4 Revenue Report — Vowayage"`. |
| `body` | str | **you author this.** Summarize the plan and the report in prose. |
| `attachments` | list[str] | file paths to attach. A generated PDF lives at `<db_name>_executive_report.pdf` in the reports directory — pass just that filename and it is resolved automatically. |
| `links` | list[str] | URLs to include, e.g. a Tableau share link or the report `web_url`. |
| `body_is_html` | bool | set true if `body` is HTML. |

## How to use it in a plan

- After a `dashboard-syncer / generate_pdf_report` step, attach the report by its
  deterministic filename `<db_name>_executive_report.pdf` and include the returned
  `web_url` in `links`.
- After a `dashboard-syncer / publish_report` (Tableau/PowerBI), put the share link
  in `links` instead of an attachment.
- Always include the plan summary in `body` so recipients have context.

## Recipients

- Explicit addresses are sent as-is.
- `"me"` / `"myself"` / `"self"` → the authenticated user's email (injected by the
  orchestrator; never fabricate it).
- Team / multiple stakeholders: pass each explicit address in `to`/`cc`.

## Failure behavior

No silent success. The tool returns `status: "failed"` if SMTP is not configured,
if a recipient address is invalid, if an attachment can't be found, or if the SMTP
send errors — so a client is never told a report was delivered when it wasn't.
