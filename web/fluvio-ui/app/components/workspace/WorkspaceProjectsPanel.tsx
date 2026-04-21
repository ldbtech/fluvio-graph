"use client";

import { useCallback, useEffect, useState } from "react";
import {
  archiveWorkspace,
  deleteWorkspaceProject,
  listWorkspaceProjects,
  loadWorkspaceProject,
  resetWorkspace,
} from "@/lib/workspaceProjects";

type Props = {
  kgUrl: string;
  onAfterMutation: () => void | Promise<void>;
};

export function WorkspaceProjectsPanel({ kgUrl, onAfterMutation }: Props) {
  const [projectId, setProjectId] = useState("");
  const [projects, setProjects] = useState<{ id: string }[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  const refreshList = useCallback(async () => {
    try {
      const list = await listWorkspaceProjects(kgUrl);
      setProjects(list);
    } catch {
      setProjects([]);
    }
  }, [kgUrl]);

  useEffect(() => {
    void refreshList();
  }, [refreshList]);

  const run = async (label: string, fn: () => Promise<void>) => {
    setMsg(null);
    setBusy(true);
    try {
      await fn();
      setMsg(`${label} — ok`);
      await refreshList();
      await onAfterMutation();
    } catch (e) {
      setMsg(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="border-t border-white/5 px-3 py-3">
      <p className="mb-2 px-1 font-mono text-[10px] uppercase tracking-wider text-violet-300/80">
        graph projects
      </p>
      <p className="mb-2 px-1 text-[11px] leading-relaxed text-slate-500">
        Snapshots save <span className="font-mono text-slate-400">fluvio_graphs/workspace/*.json</span> into{" "}
        <span className="font-mono text-slate-400">fluvio_graphs/projects/&lt;id&gt;/</span> via kg-engine (same host as
        PDF ingest).
      </p>

      <label className="mb-1 block px-1 font-mono text-[10px] text-slate-500" htmlFor="ws-project-id">
        new snapshot id (letters, digits, -, _)
      </label>
      <input
        id="ws-project-id"
        value={projectId}
        onChange={(e) => setProjectId(e.target.value)}
        placeholder="e.g. client-alpha"
        disabled={busy}
        className="mb-2 w-full rounded-lg border border-white/10 bg-[#0a1020] px-2 py-1.5 font-mono text-xs text-slate-200 outline-none ring-cyan-400/30 focus:ring-1"
      />

      <div className="mb-2 flex flex-col gap-1.5">
        <button
          type="button"
          disabled={busy || !projectId.trim()}
          onClick={() =>
            void run("Snapshot saved", async () => {
              await archiveWorkspace(kgUrl, projectId.trim());
            })
          }
          className="rounded-lg border border-violet-400/30 bg-violet-500/10 py-1.5 font-mono text-[11px] text-violet-100 transition hover:bg-violet-500/20 disabled:opacity-40"
        >
          Save snapshot
        </button>
        <button
          type="button"
          disabled={busy || !projectId.trim()}
          onClick={() =>
            void run("Snapshot + cleared workspace", async () => {
              await archiveWorkspace(kgUrl, projectId.trim());
              await resetWorkspace(kgUrl);
              setProjectId("");
            })
          }
          className="rounded-lg border border-amber-400/35 bg-amber-500/10 py-1.5 font-mono text-[11px] text-amber-100 transition hover:bg-amber-500/20 disabled:opacity-40"
        >
          Save snapshot &amp; start empty
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            if (!window.confirm("Clear the live workspace graph (all nodes/edges)? This cannot be undone.")) return;
            void run("Workspace reset", async () => {
              await resetWorkspace(kgUrl);
            });
          }}
          className="rounded-lg border border-red-400/30 bg-red-500/10 py-1.5 font-mono text-[11px] text-red-100 transition hover:bg-red-500/20 disabled:opacity-40"
        >
          Reset workspace (empty)
        </button>
      </div>

      <label className="mb-1 block px-1 font-mono text-[10px] text-slate-500" htmlFor="ws-project-pick">
        load or delete saved project
      </label>
      <select
        id="ws-project-pick"
        value={selectedId}
        onChange={(e) => setSelectedId(e.target.value)}
        disabled={busy}
        className="mb-2 w-full rounded-lg border border-white/10 bg-[#0a1020] px-2 py-1.5 font-mono text-xs text-slate-200 outline-none"
      >
        <option value="">— select —</option>
        {projects.map((p) => (
          <option key={p.id} value={p.id}>
            {p.id}
          </option>
        ))}
      </select>

      <div className="flex gap-1.5">
        <button
          type="button"
          disabled={busy || !selectedId}
          onClick={() =>
            void run(`Loaded “${selectedId}”`, async () => {
              await loadWorkspaceProject(kgUrl, selectedId);
            })
          }
          className="flex-1 rounded-lg border border-cyan-400/30 bg-cyan-500/10 py-1.5 font-mono text-[11px] text-cyan-100 transition hover:bg-cyan-500/20 disabled:opacity-40"
        >
          Load into workspace
        </button>
        <button
          type="button"
          disabled={busy || !selectedId}
          onClick={() => {
            if (!window.confirm(`Delete saved project “${selectedId}” from disk?`)) return;
            void run(`Deleted “${selectedId}”`, async () => {
              await deleteWorkspaceProject(kgUrl, selectedId);
              setSelectedId("");
            });
          }}
          className="flex-1 rounded-lg border border-red-400/25 bg-red-500/5 py-1.5 font-mono text-[11px] text-red-200/90 transition hover:bg-red-500/15 disabled:opacity-40"
        >
          Delete saved
        </button>
      </div>

      {msg && (
        <p className="mt-2 break-words px-1 font-mono text-[10px] leading-relaxed text-slate-400">{msg}</p>
      )}
    </div>
  );
}
