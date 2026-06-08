import asyncio
import sys
import os

# Add parent directories to sys.path so we can import src
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from src.tools.dashboard_syncer.runtime import DashboardSyncerRuntime
from src.tools.dashboard_syncer.contracts import DashboardExecutionContext

async def main():
    report_name = sys.argv[1] if len(sys.argv) > 1 else "Executive Performance Report"
    db_url = sys.argv[2] if len(sys.argv) > 2 else "postgres://alidaho@localhost:5432/vowayage"
    latex_file = sys.argv[3] if len(sys.argv) > 3 else None
    
    latex_content = None
    if latex_file and os.path.exists(latex_file):
        with open(latex_file, "r") as f:
            latex_content = f.read()
        print(f"Loaded custom LaTeX file: {latex_file}")

    ctx = DashboardExecutionContext()
    runtime = DashboardSyncerRuntime()
    print(f"Generating PDF report: name='{report_name}', database='{db_url}'")
    
    res = await runtime.generate_pdf_report(ctx, report_name, database_url=db_url, latex_content=latex_content)
    print("Result:", res)

if __name__ == "__main__":
    asyncio.run(main())
