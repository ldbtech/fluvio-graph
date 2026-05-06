"use client";

import Link from "next/link";

export function QaTopChrome() {
  return (
    <header className="ui-chrome fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b px-4 backdrop-blur-2xl">
      <div className="flex min-w-0 flex-wrap items-center gap-3 sm:gap-4">
        <Link
          href="/workspace"
          className="ui-pill ui-text-muted shrink-0 rounded-full border px-3 py-1.5 text-[12px] font-medium transition hover:brightness-95"
        >
          ← Workspace
        </Link>
        <div className="h-4 w-px shrink-0 bg-[var(--ui-border)]" aria-hidden />
        <div className="min-w-0">
          <h1 className="truncate text-sm font-semibold tracking-tight text-[var(--ui-text)]">QA review</h1>
          <p className="truncate text-[11px] text-[var(--ui-text-secondary)]">Human-in-the-loop · graphs · agents</p>
        </div>
      </div>
      <p className="hidden max-w-sm text-right text-[11px] leading-relaxed text-[var(--ui-text-tertiary)] sm:block">
        Mock data for UI wiring — connect approvals to your Rust API when ready.
      </p>
    </header>
  );
}
