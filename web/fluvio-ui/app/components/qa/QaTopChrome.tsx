"use client";

import Link from "next/link";

export function QaTopChrome() {
  return (
    <header className="fixed left-0 right-0 top-0 z-50 flex h-12 items-center justify-between border-b border-emerald-400/20 bg-[#04040f]/95 px-4 backdrop-blur-md">
      <div className="flex flex-wrap items-center gap-4">
        <Link
          href="/"
          className="font-mono text-[10px] text-slate-500 transition hover:text-emerald-300/90"
        >
          ← workspace
        </Link>
        <span className="h-4 w-px bg-white/10" aria-hidden />
        <div className="flex flex-col gap-0">
          <span className="font-mono text-xs font-semibold tracking-tight text-emerald-200/95">
            QA infrastructure
          </span>
          <span className="font-mono text-[10px] text-slate-500">
            Human-in-the-loop · graphs · agents
          </span>
        </div>
      </div>
      <p className="hidden max-w-md text-right font-mono text-[10px] leading-relaxed text-slate-500 sm:block">
        Mock data for UI wiring — connect approvals to your Rust API when ready.
      </p>
    </header>
  );
}
