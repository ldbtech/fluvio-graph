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
  designPreviewCount: number;
};

const segmentBase =
  "rounded-full px-3.5 py-1.5 text-[13px] font-medium tracking-tight transition-colors duration-200 ease-out";
const segmentIdle = "text-zinc-500 hover:text-zinc-300";
const segmentActive = "bg-zinc-100 text-zinc-900 shadow-sm";

export function WorkspaceTopChrome({
  mode,
  onModeChange,
  workspaceKind,
  onWorkspaceKindChange,
  documentGraphReady,
  personalPreviewCount,
  investPreviewCount,
  designPreviewCount,
}: Props) {
  return (
    <header className="fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b border-white/[0.06] bg-zinc-950/80 px-4 backdrop-blur-2xl supports-[backdrop-filter]:bg-zinc-950/70">
      <div className="flex min-w-0 flex-wrap items-center gap-3 sm:gap-5">
        <span className="truncate text-sm font-semibold tracking-tight text-zinc-100">Workspace</span>

        <nav
          className="flex rounded-full border border-white/[0.06] bg-zinc-900/80 p-0.5 shadow-inner"
          aria-label="Workspace type"
        >
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("personal")}
            className={`${segmentBase} ${workspaceKind === "personal" ? segmentActive : segmentIdle}`}
          >
            Personal
          </button>
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("invest")}
            className={`${segmentBase} ${workspaceKind === "invest" ? segmentActive : segmentIdle}`}
          >
            Investment
          </button>
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("design")}
            className={`${segmentBase} ${workspaceKind === "design" ? segmentActive : segmentIdle}`}
          >
            Design
          </button>
        </nav>

        <Link
          href="/qa"
          className="hidden rounded-full border border-white/[0.08] bg-zinc-900/60 px-3 py-1.5 text-[12px] font-medium text-zinc-400 transition hover:border-white/[0.12] hover:bg-zinc-800/80 hover:text-zinc-200 sm:inline-flex"
        >
          QA
        </Link>

        <nav
          className="flex rounded-full border border-white/[0.06] bg-zinc-900/80 p-0.5 shadow-inner"
          aria-label="Main mode"
        >
          <button
            type="button"
            onClick={() => onModeChange("sources")}
            className={`${segmentBase} ${mode === "sources" ? segmentActive : segmentIdle}`}
          >
            Sources
          </button>
          <button
            type="button"
            onClick={() => onModeChange("brain")}
            className={`${segmentBase} ${mode === "brain" ? segmentActive : segmentIdle}`}
          >
            Brain
          </button>
        </nav>
      </div>

      <div className="hidden flex-col items-end gap-0.5 text-[11px] font-medium text-zinc-500 lg:flex">
        <span>
          PDF{" "}
          <span className={documentGraphReady ? "text-emerald-400/90" : "text-zinc-600"}>
            {documentGraphReady ? "ready" : "empty"}
          </span>
        </span>
        <span className="tabular-nums text-zinc-600">
          Previews · {personalPreviewCount} personal · {investPreviewCount} invest · {designPreviewCount} design
        </span>
      </div>
    </header>
  );
}
