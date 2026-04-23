"use client";

import Link from "next/link";

export function QaTopChrome() {
  return (
    <header className="fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b border-white/[0.06] bg-zinc-950/80 px-4 backdrop-blur-2xl supports-[backdrop-filter]:bg-zinc-950/70">
      <div className="flex min-w-0 flex-wrap items-center gap-3 sm:gap-4">
        <Link
          href="/"
          className="shrink-0 rounded-full border border-white/[0.08] bg-zinc-900/60 px-3 py-1.5 text-[12px] font-medium text-zinc-400 transition hover:border-white/[0.12] hover:bg-zinc-800/80 hover:text-zinc-200"
        >
          ← Workspace
        </Link>
        <div className="h-4 w-px shrink-0 bg-white/[0.08]" aria-hidden />
        <div className="min-w-0">
          <h1 className="truncate text-sm font-semibold tracking-tight text-zinc-100">QA review</h1>
          <p className="truncate text-[11px] text-zinc-500">Human-in-the-loop · graphs · agents</p>
        </div>
      </div>
      <p className="hidden max-w-sm text-right text-[11px] leading-relaxed text-zinc-600 sm:block">
        Mock data for UI wiring — connect approvals to your Rust API when ready.
      </p>
    </header>
  );
}
