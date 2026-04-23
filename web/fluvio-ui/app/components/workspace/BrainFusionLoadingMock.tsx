"use client";

import { useEffect, useState } from "react";
import type { WorkspaceKind } from "@/lib/types";

type Variant = "unified" | "meta";

export function BrainFusionLoadingMock({
  variant,
  workspaceKind = "personal",
}: {
  variant: Variant;
  workspaceKind?: WorkspaceKind;
}) {
  const [pct, setPct] = useState(12);

  useEffect(() => {
    const t = window.setInterval(() => {
      setPct((p) => (p >= 94 ? 12 : p + Math.random() * 11 + 4));
    }, 220);
    return () => window.clearInterval(t);
  }, []);

  const markets = workspaceKind === "invest";
  const design = workspaceKind === "design";

  const title =
    variant === "unified"
      ? markets
        ? "Fusing vendor feeds into markets workspace…"
        : design
          ? "Fusing BIM, loads, and solver outputs into design workspace…"
          : "Fusing subgraphs into workspace view…"
      : markets
        ? "Recomputing markets meta-graph…"
        : design
          ? "Recomputing design meta-graph (codes + contracts)…"
          : "Recomputing meta-graph layout…";

  const steps =
    variant === "unified"
      ? markets
        ? [
            "Normalize tickers across venues",
            "Reconcile vendor clock skew",
            "Join news → price events (mock)",
            "Publish fusion snapshot for desk chat",
          ]
        : design
          ? [
              "Pin IFC federation revision IDs",
              "Join code clauses to load combinations",
              "Reconcile structural vs physics envelopes",
              "Publish fusion snapshot for design chat",
            ]
          : [
              "Resolve entity IDs across domains",
              "Align embedding spaces (mock L2)",
              "Materialize cross-edges in fusion layer",
              "Hand off to query router",
            ]
      : markets
        ? [
            "Refresh API entitlement capsules",
            "Redraw data-vendor fan-out",
            "Reconcile agent mesh slots",
          ]
        : design
          ? [
              "Refresh solver + code capsule health",
              "Redraw BIM → structural fan-out",
              "Reconcile agent mesh slots",
            ]
          : [
              "Refresh domain capsule health",
              "Redraw orchestrator fan-out",
              "Reconcile agent mesh slots",
            ];

  return (
    <div className="absolute inset-0 z-30 flex flex-col items-center justify-center bg-[#04040f]/92 px-6 backdrop-blur-md">
      <div className="w-full max-w-md rounded-2xl border border-violet-500/25 bg-[#0a0618]/95 p-6 shadow-[0_0_48px_rgba(139,92,246,0.15)]">
        <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-violet-300/70">
          infrastructure · mock
        </p>
        <h3 className="mt-2 text-sm font-semibold text-violet-100">{title}</h3>
        <div className="mt-4 h-2 w-full overflow-hidden rounded-full bg-white/5">
          <div
            className="h-full rounded-full bg-gradient-to-r from-cyan-400 via-violet-400 to-fuchsia-500 transition-[width] duration-300 ease-out"
            style={{ width: `${Math.round(pct)}%` }}
          />
        </div>
        <p className="mt-2 text-right font-mono text-xs tabular-nums text-violet-200/80">{Math.round(pct)}%</p>
        <ul className="mt-5 space-y-2 border-t border-white/5 pt-4">
          {steps.map((s, i) => (
            <li key={s} className="flex items-center gap-3 font-mono text-[11px] text-slate-400">
              <span
                className={`flex h-5 w-5 shrink-0 items-center justify-center rounded border text-[9px] ${
                  pct > 22 + i * 18
                    ? "border-emerald-500/40 bg-emerald-500/15 text-emerald-300"
                    : "border-white/10 text-slate-600"
                }`}
              >
                {pct > 22 + i * 18 ? "✓" : "·"}
              </span>
              <span className={pct > 22 + i * 18 ? "text-slate-300" : ""}>{s}</span>
            </li>
          ))}
        </ul>
        <p className="mt-4 font-mono text-[10px] leading-relaxed text-slate-600">
          {markets ? (
            <>
              Rust job sketch: keyed <code className="text-amber-700/80">MarketGraphId</code> per workspace, ingest
              bars/news as typed vertices, then materialize a fusion layer for cross-vendor queries (mock UI only).
            </>
          ) : design ? (
            <>
              Rust job sketch: keyed <code className="text-violet-200/90">DesignGraphId</code> per project, ingest IFC
              + solver artifacts as typed vertices, then materialize a fusion layer so agents can prove loads and physics
              before construction (mock UI only).
            </>
          ) : (
            <>
              Rust will run this as a job: load each <code className="text-cyan-700/80">GraphId</code>, union adjacency,
              then persist a materialized fusion projection for low-latency chat.
            </>
          )}
        </p>
      </div>
    </div>
  );
}
