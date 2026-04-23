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
  /** Compact layout inside sidebar disclosure (no large section title). */
  embedded?: boolean;
};

export function WorkspaceProjectsPanel({ kgUrl, onAfterMutation, embedded = false }: Props) {
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
    <div className={embedded ? "px-2 pt-2" : "border-t border-white/[0.06] px-3 py-4"}>
      {!embedded ? (
        <>
          <p className="mb-2 px-1 text-[11px] font-medium uppercase tracking-wide text-zinc-500">Graph projects</p>
          <p className="mb-3 px-1 text-[12px] leading-relaxed text-zinc-600">
            Snapshots save <span className="font-mono text-[11px] text-zinc-500">fluvio_graphs/workspace/*.json</span>{" "}
            into <span className="font-mono text-[11px] text-zinc-500">fluvio_graphs/projects/&lt;id&gt;/</span> via
            kg-engine.
          </p>
        </>
      ) : (
        <p className="mb-3 px-1 text-[11px] leading-relaxed text-zinc-600">
          Snapshots write under <span className="font-mono text-zinc-500">fluvio_graphs/projects/&lt;id&gt;/</span>.
        </p>
      )}

      <label className="mb-1.5 block px-1 text-[11px] font-medium text-zinc-500" htmlFor="ws-project-id">
        Snapshot ID
      </label>
      <input
        id="ws-project-id"
        value={projectId}
        onChange={(e) => setProjectId(e.target.value)}
        placeholder="e.g. client-alpha"
        disabled={busy}
        className="mb-2 w-full rounded-xl border border-white/[0.08] bg-zinc-900/80 px-3 py-2 font-mono text-[13px] text-zinc-100 outline-none focus:border-sky-500/40 focus:ring-1 focus:ring-sky-500/20"
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
          className="rounded-xl bg-zinc-100 py-2 text-[13px] font-semibold text-zinc-900 transition hover:bg-white disabled:opacity-40"
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
          className="rounded-xl border border-white/[0.1] bg-zinc-800 py-2 text-[13px] font-semibold text-zinc-100 transition hover:bg-zinc-700 disabled:opacity-40"
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
          className="rounded-xl border border-red-500/25 bg-red-950/40 py-2 text-[13px] font-semibold text-red-200/95 transition hover:bg-red-950/70 disabled:opacity-40"
        >
          Reset workspace (empty)
        </button>
      </div>

      <label className="mb-1.5 block px-1 text-[11px] font-medium text-zinc-500" htmlFor="ws-project-pick">
        Saved project
      </label>
      <select
        id="ws-project-pick"
        value={selectedId}
        onChange={(e) => setSelectedId(e.target.value)}
        disabled={busy}
        className="mb-2 w-full rounded-xl border border-white/[0.08] bg-zinc-900/80 px-3 py-2 font-mono text-[13px] text-zinc-100 outline-none"
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
          className="flex-1 rounded-xl bg-zinc-100 py-2 text-[12px] font-semibold text-zinc-900 transition hover:bg-white disabled:opacity-40"
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
          className="flex-1 rounded-xl border border-red-500/20 bg-red-950/35 py-2 text-[12px] font-semibold text-red-200/95 transition hover:bg-red-950/55 disabled:opacity-40"
        >
          Delete saved
        </button>
      </div>

      {msg && (
        <p className="mt-2 break-words px-1 text-[12px] leading-relaxed text-zinc-500">{msg}</p>
      )}
    </div>
  );
}
