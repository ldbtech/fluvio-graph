"use client";

import Link from "next/link";
import type { WorkspaceKind } from "@/lib/types";

type WorkspaceMode = "sources" | "brain";

type Props = {
  mode: WorkspaceMode;
  onModeChange: (mode: WorkspaceMode) => void;
  workspaceKind: WorkspaceKind;
  onWorkspaceKindChange: (kind: WorkspaceKind) => void;
  documentGraphReady: boolean;
  personalPreviewCount: number;
  investPreviewCount: number;
};

export function WorkspaceTopChrome({
  mode,
  onModeChange,
  workspaceKind,
  onWorkspaceKindChange,
  documentGraphReady,
  personalPreviewCount,
  investPreviewCount,
}: Props) {
  return (
    <header className="fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b border-cyan-400/15 bg-[#04040f]/95 px-4 backdrop-blur-md">
      <div className="flex flex-wrap items-center gap-4 sm:gap-6">
        <span className="font-mono text-xs font-semibold tracking-tight text-cyan-200/90">kg workspace</span>
        <nav className="flex rounded-full border border-amber-400/15 bg-amber-500/[0.04] p-0.5 font-mono text-[10px]">
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("personal")}
            className={`rounded-full px-2.5 py-1.5 transition sm:px-3 ${
              workspaceKind === "personal"
                ? "bg-cyan-500/20 text-cyan-100 shadow-[0_0_10px_rgba(34,211,238,0.12)]"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Personal
          </button>
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("invest")}
            className={`rounded-full px-2.5 py-1.5 transition sm:px-3 ${
              workspaceKind === "invest"
                ? "bg-amber-500/25 text-amber-100 shadow-[0_0_10px_rgba(245,158,11,0.12)]"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Investment
          </button>
        </nav>
        <Link
          href="/qa"
          className="rounded-full border border-emerald-400/20 bg-emerald-500/[0.06] px-3 py-1.5 font-mono text-[10px] text-emerald-200/90 transition hover:border-emerald-400/35 hover:bg-emerald-500/12"
        >
          QA infrastructure
        </Link>
        <nav className="flex rounded-full border border-white/10 bg-white/[0.03] p-0.5 font-mono text-[11px]">
          <button
            type="button"
            onClick={() => onModeChange("sources")}
            className={`rounded-full px-3 py-1.5 transition ${
              mode === "sources"
                ? "bg-cyan-500/20 text-cyan-100 shadow-[0_0_12px_rgba(34,211,238,0.15)]"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Sources
          </button>
          <button
            type="button"
            onClick={() => onModeChange("brain")}
            className={`rounded-full px-3 py-1.5 transition ${
              mode === "brain"
                ? "bg-violet-500/20 text-violet-100 shadow-[0_0_12px_rgba(139,92,246,0.12)]"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Workspace brain
          </button>
        </nav>
      </div>
      <div className="hidden flex-col items-end gap-0.5 font-mono text-[10px] text-slate-500 lg:flex">
        <span>
          PDF graph:{" "}
          <span className={documentGraphReady ? "text-emerald-400/90" : "text-slate-600"}>
            {documentGraphReady ? "ready" : "empty"}
          </span>
        </span>
        <span className="text-violet-300/60">
          previews · personal{" "}
          <span className="text-violet-200">{personalPreviewCount}</span>
          <span className="mx-1 text-slate-600">·</span>
          invest{" "}
          <span className="text-amber-200/90">{investPreviewCount}</span>
        </span>
      </div>
    </header>
  );
}
