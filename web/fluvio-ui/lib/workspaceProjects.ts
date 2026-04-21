export type WorkspaceProject = { id: string };

export async function listWorkspaceProjects(kgUrl: string): Promise<WorkspaceProject[]> {
  const r = await fetch(`${kgUrl}/workspace/projects`);
  if (!r.ok) throw new Error(`workspace/projects HTTP ${r.status}`);
  const j = (await r.json()) as { projects?: WorkspaceProject[] };
  return j.projects ?? [];
}

export async function archiveWorkspace(kgUrl: string, id: string): Promise<void> {
  const r = await fetch(`${kgUrl}/workspace/archive`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  });
  if (!r.ok) {
    const t = await r.text();
    throw new Error(t || `archive HTTP ${r.status}`);
  }
}

export async function resetWorkspace(kgUrl: string): Promise<void> {
  const r = await fetch(`${kgUrl}/workspace/reset`, { method: "POST" });
  if (!r.ok) {
    const t = await r.text();
    throw new Error(t || `reset HTTP ${r.status}`);
  }
}

export async function loadWorkspaceProject(kgUrl: string, id: string): Promise<void> {
  const r = await fetch(`${kgUrl}/workspace/load`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  });
  if (!r.ok) {
    const t = await r.text();
    throw new Error(t || `load HTTP ${r.status}`);
  }
}

export async function deleteWorkspaceProject(kgUrl: string, id: string): Promise<void> {
  const r = await fetch(`${kgUrl}/workspace/delete`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  });
  if (!r.ok) {
    const t = await r.text();
    throw new Error(t || `delete HTTP ${r.status}`);
  }
}
