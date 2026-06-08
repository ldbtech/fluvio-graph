import os
from dotenv import load_dotenv

load_dotenv()

PORT = int(os.getenv("PORT", "3008"))

# Directory where generated PDF/LaTeX reports are written so the web app can serve
# them from /reports. Override with REPORTS_DIR; falls back to the local web public dir.
REPORTS_DIR = os.getenv(
    "REPORTS_DIR",
    os.path.expanduser("~/Developer/AWS/rust/fluviome-web/public/reports"),
)

# Public base URL the web app serves reports from.
REPORTS_BASE_URL = os.getenv("REPORTS_BASE_URL", "http://localhost:3000/reports")
