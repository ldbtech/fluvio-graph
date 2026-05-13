"use client";

import Link from "next/link";

type WorkspaceMode = "sources" | "brain";

type Props = {
  mode: WorkspaceMode;
  onModeChange: (mode: WorkspaceMode) => void;
  documentGraphReady: boolean;
  personalPreviewCount: number;
};

const segmentBase =
  "rounded-full px-3.5 py-1.5 text-[13px] font-medium tracking-tight transition-colors duration-200 ease-out";
const segmentIdle = "text-zinc-500 hover:text-zinc-300";
const segmentActive = "bg-zinc-100 text-zinc-900 shadow-sm";

export function WorkspaceTopChrome({
  mode,
  onModeChange,
  documentGraphReady,
  personalPreviewCount,
}: Props) {
  return (
    <header className="ui-chrome fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b px-4 backdrop-blur-2xl">
      <div className="flex min-w-0 flex-wrap items-center gap-3 sm:gap-5">
        <Link
          href="/"
          className="truncate text-sm font-semibold tracking-tight text-[var(--ui-text)] transition hover:text-sky-300/90"
          title="FluvioMe home"
        >
          Fluvio
        </Link>

        <nav
          className="ui-pill flex rounded-full border p-0.5 shadow-inner"
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

      <div className="ui-text-muted hidden flex-col items-end gap-0.5 text-[11px] font-medium lg:flex">
        <span>
          PDF{" "}
          <span className={documentGraphReady ? "text-emerald-500" : "text-[var(--ui-text-tertiary)]"}>
            {documentGraphReady ? "ready" : "empty"}
          </span>
        </span>
        <span className="tabular-nums text-[var(--ui-text-tertiary)]">
          Previews · {personalPreviewCount} connected
        </span>
      </div>
    </header>
  );
}
