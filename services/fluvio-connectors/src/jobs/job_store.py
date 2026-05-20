"""In-memory sync job tracking."""
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional


@dataclass
class SyncJob:
    id:           str
    connector_id: str
    status:       str        # queued | running | complete | failed
    nodes_added:  int = 0
    error:        Optional[str] = None
    started_at:   datetime = field(default_factory=datetime.utcnow)
    finished_at:  Optional[datetime] = None
    # Store token at job creation time so sync doesn't need to re-fetch
    access_token: str = ""
    owner_id:     str = ""
    kind:         str = ""

    def finish(self, nodes_added: int):
        self.status      = "complete"
        self.nodes_added = nodes_added
        self.finished_at = datetime.utcnow()

    def fail(self, error: str):
        self.status      = "failed"
        self.error       = error
        self.finished_at = datetime.utcnow()


class JobStore:
    def __init__(self):
        self._jobs: dict[str, SyncJob] = {}

    def create(self, connector_id: str, access_token: str = "", owner_id: str = "", kind: str = "") -> SyncJob:
        job = SyncJob(
            id=           str(uuid.uuid4()),
            connector_id= connector_id,
            status=       "queued",
            access_token= access_token,
            owner_id=     owner_id,
            kind=         kind,
        )
        self._jobs[job.id] = job
        return job

    def get(self, job_id: str) -> Optional[SyncJob]:
        return self._jobs.get(job_id)

    def update_status(self, job_id: str, status: str):
        if job := self._jobs.get(job_id):
            job.status = status

    def finish(self, job_id: str, nodes_added: int):
        if job := self._jobs.get(job_id):
            job.finish(nodes_added)

    def fail(self, job_id: str, error: str):
        if job := self._jobs.get(job_id):
            job.fail(error)

    def evict_old(self, max_age_seconds: int = 3600):
        """Remove completed jobs older than max_age_seconds."""
        now = datetime.utcnow()
        to_delete = [
            jid for jid, job in self._jobs.items()
            if job.finished_at and
               (now - job.finished_at).total_seconds() > max_age_seconds
        ]
        for jid in to_delete:
            del self._jobs[jid]


# Singleton
job_store = JobStore()