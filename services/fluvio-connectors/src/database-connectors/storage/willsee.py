# database-connectors/storage/local.py

import json
import shutil
from pathlib import Path
from datetime import datetime


class LocalStorage:
    """
    Mirrors s3://fluvio-snapshots/ on local disk.
    Swap this class for S3Storage later — zero other code changes.

    Structure:
        s3/
          {org_id}/
            {connector_id}/
              {table}/
                snapshots/
                  2026-05-20T02-00-00.csv
                  latest.csv
                metadata.json
    """

    def __init__(self, base_path: str = "./s3"):
        self.base = Path(base_path)
        self.base.mkdir(exist_ok=True)

    # ── Snapshots ─────────────────────────────────────────────────

    def save_snapshot(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
        content:      bytes,
        filename:     str,
        timestamp:    datetime | None = None,
    ) -> Path:
        """Save CSV bytes to disk. Updates latest.csv automatically."""
        ts  = (timestamp or datetime.utcnow()).strftime("%Y-%m-%dT%H-%M-%S")
        ext = Path(filename).suffix          # .csv or .xlsx
        dir = self._snapshot_dir(org_id, connector_id, table)

        # Save timestamped copy
        path = dir / f"{ts}{ext}"
        path.write_bytes(content)

        # Always update latest
        latest = dir / f"latest{ext}"
        shutil.copy2(path, latest)

        return path

    def get_latest(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
        ext:          str = ".csv",
    ) -> Path | None:
        p = self._snapshot_dir(org_id, connector_id, table) / f"latest{ext}"
        return p if p.exists() else None

    def list_snapshots(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
        ext:          str = ".csv",
    ) -> list[Path]:
        dir = self._snapshot_dir(org_id, connector_id, table)
        if not dir.exists():
            return []
        return sorted(
            [f for f in dir.glob(f"*{ext}") if not f.stem.startswith("latest")],
            reverse=True,
        )

    def prune_snapshots(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
        keep:         int = 7,
        ext:          str = ".csv",
    ) -> int:
        """Delete old snapshots, keep N most recent. Returns deleted count."""
        old = self.list_snapshots(org_id, connector_id, table, ext)[keep:]
        for f in old:
            f.unlink()
        return len(old)

    # ── Metadata ──────────────────────────────────────────────────

    def save_metadata(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
        meta:         dict,
    ) -> None:
        dir = self._snapshot_dir(org_id, connector_id, table)
        dir.mkdir(parents=True, exist_ok=True)
        (dir.parent / "metadata.json").write_text(
            json.dumps(meta, indent=2, default=str)
        )

    # ── Tasks (future agents) ─────────────────────────────────────

    def create_task(self, task_id: str, plan: str) -> Path:
        task_dir = self.base / "tasks" / task_id
        task_dir.mkdir(parents=True, exist_ok=True)
        (task_dir / "plan.md").write_text(plan)
        (task_dir / "status.json").write_text(json.dumps({
            "status":     "awaiting_approval",
            "created_at": datetime.utcnow().isoformat(),
        }, indent=2))
        (task_dir / "input").mkdir(exist_ok=True)
        (task_dir / "output").mkdir(exist_ok=True)
        return task_dir

    def update_task_status(self, task_id: str, status: str, **kwargs) -> None:
        p    = self.base / "tasks" / task_id / "status.json"
        data = json.loads(p.read_text()) if p.exists() else {}
        data.update({
            "status":     status,
            "updated_at": datetime.utcnow().isoformat(),
            **kwargs,
        })
        p.write_text(json.dumps(data, indent=2))

    # ── Internal ──────────────────────────────────────────────────

    def _snapshot_dir(
        self,
        org_id:       str,
        connector_id: str,
        table:        str,
    ) -> Path:
        p = self.base / org_id / connector_id / table / "snapshots"
        p.mkdir(parents=True, exist_ok=True)
        return p


# Singleton — one line swap for S3 later
storage = LocalStorage(base_path="./s3")


# ── Test ──────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    from datetime import datetime

    s = LocalStorage(base_path="/tmp/fluvio-test-s3")

    # Save a fake CSV
    fake_csv = b"id,name,role\nabc,Alice,owner\ndef,Bob,contributor\n"
    path = s.save_snapshot(
        org_id=       "org-001",
        connector_id= "connector-001",
        table=        "users",
        content=      fake_csv,
        filename=     "users.csv",
        timestamp=    datetime.utcnow(),
    )

    print(f"Saved:  {path}")
    print(f"Latest: {s.get_latest('org-001', 'connector-001', 'users')}")
    print(f"List:   {s.list_snapshots('org-001', 'connector-001', 'users')}")
    print(f"\nContent:\n{path.read_text()}")