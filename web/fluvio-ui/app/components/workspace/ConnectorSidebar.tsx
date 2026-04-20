"use client";

import type { ConnectorDef, ConnectorId, ConnectorStatus, WorkspaceKind, WorkspaceSurface } from "@/lib/types";

const PERSONAL_CONNECTORS: ConnectorDef[] = [
  {
    id: "gmail",
    name: "Gmail",
    blurb: "Threads → entities",
    accent: "#ea4335",
  },
  {
    id: "spotify",
    name: "Spotify",
    blurb: "Listening graph",
    accent: "#1db954",
  },
  {
    id: "github",
    name: "GitHub",
    blurb: "Repos & code",
    accent: "#a371f7",
  },
  {
    id: "calendar",
    name: "Google Calendar",
    blurb: "Time anchors",
    accent: "#4285f4",
  },
  {
    id: "whatsapp",
    name: "WhatsApp",
    blurb: "Chats (opt-in)",
    accent: "#25d366",
  },
  {
    id: "slack",
    name: "Slack",
    blurb: "Team signal",
    accent: "#4a154b",
  },
  {
    id: "notion",
    name: "Notion",
    blurb: "Docs graph",
    accent: "#ffffff",
  },
  {
    id: "web",
    name: "Website",
    blurb: "Crawl + PDF learnings",
    accent: "#f59e0b",
  },
];

const INVEST_CONNECTORS: ConnectorDef[] = [
  {
    id: "equities",
    name: "Stocks & equities",
    blurb: "Tape, fundamentals, events",
    accent: "#22c55e",
  },
  {
    id: "futures",
    name: "Futures",
    blurb: "Curves, rolls, margin",
    accent: "#38bdf8",
  },
  {
    id: "cryptocurrencies",
    name: "Crypto",
    blurb: "Pairs, flows, venue risk",
    accent: "#f472b6",
  },
  {
    id: "fin_news",
    name: "News wires",
    blurb: "Multi-vendor headlines",
    accent: "#fb923c",
  },
  {
    id: "fin_market_data",
    name: "Market data APIs",
    blurb: "Bars, depth, alt data",
    accent: "#a78bfa",
  },
  {
    id: "fin_research",
    name: "Research & books",
    blurb: "PDFs + desk notes",
    accent: "#fcd34d",
  },
];

type Props = {
  workspaceKind: WorkspaceKind;
  onWorkspaceKindChange: (kind: WorkspaceKind) => void;
  pdfInputId: string;
  activeSurface: WorkspaceSurface | null;
  statusById: Partial<Record<ConnectorId, ConnectorStatus>>;
  onSelectSurface: (surface: WorkspaceSurface) => void;
  activity: string | null;
};

export function ConnectorSidebar({
  workspaceKind,
  onWorkspaceKindChange,
  pdfInputId,
  activeSurface,
  statusById,
  onSelectSurface,
  activity,
}: Props) {
  const docActive = activeSurface === "documents";
  const connectors = workspaceKind === "invest" ? INVEST_CONNECTORS : PERSONAL_CONNECTORS;

  return (
    <aside
      className="pointer-events-auto flex h-full w-[min(100%,280px)] shrink-0 flex-col border-r border-cyan-400/15 bg-[#05051a]/92 backdrop-blur-md"
      onClick={(e) => e.stopPropagation()}
    >
      <div className="border-b border-white/5 px-4 py-4">
        <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-cyan-200/50">
          workspace
        </p>
        <h1 className="mt-1 bg-gradient-to-r from-cyan-200 to-violet-300 bg-clip-text font-sans text-lg font-semibold tracking-tight text-transparent">
          {workspaceKind === "invest" ? "Markets desk" : "Build your graph"}
        </h1>
        <p className="mt-2 text-xs leading-relaxed text-slate-400">
          {workspaceKind === "invest"
            ? "Wire mock API keys for equities, futures, crypto, news, data vendors, and research PDFs. Unified / Meta fuse every stream."
            : "Tap a source for the full connect screen (preview). PDF upload still hits the real API."}
        </p>
      </div>

      {workspaceKind === "personal" && (
        <div className="px-3 py-3">
          <p className="mb-2 px-1 font-mono text-[10px] uppercase tracking-wider text-emerald-400/70">
            live · kg-engine
          </p>
          <div
            className={`flex items-stretch gap-2 overflow-hidden rounded-xl border p-0.5 transition ${
              docActive
                ? "border-emerald-400/50 bg-emerald-500/15 shadow-[0_0_20px_rgba(52,211,153,0.12)]"
                : "border-emerald-400/25 bg-emerald-500/5"
            }`}
          >
            <button
              type="button"
              onClick={() => onSelectSurface("documents")}
              className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-2.5 text-left transition hover:bg-emerald-500/10"
            >
              <span
                className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-emerald-400/30 bg-[#0a1020] text-lg"
                aria-hidden
              >
                PDF
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="relative flex h-2 w-2">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-40" />
                    <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-400" />
                  </span>
                  <span className="text-sm font-medium text-emerald-100">Documents</span>
                </div>
                <p className="mt-0.5 truncate text-[11px] text-emerald-200/50">
                  Setup & <span className="font-mono">/ingest/pdf</span>
                </p>
              </div>
            </button>
            <label
              htmlFor={pdfInputId}
              onClick={(e) => e.stopPropagation()}
              className="flex w-11 shrink-0 cursor-pointer flex-col items-center justify-center rounded-lg border border-emerald-400/30 bg-[#0a1020] font-mono text-lg text-cyan-300/70 transition hover:border-emerald-400/60 hover:bg-emerald-500/10 hover:text-cyan-200"
              title="Quick upload (stay on graph)"
            >
              +
            </label>
          </div>
        </div>
      )}

      {workspaceKind === "invest" && (
        <div className="border-b border-amber-500/15 px-3 py-3">
          <p className="mb-2 px-1 font-mono text-[10px] uppercase tracking-wider text-amber-400/80">
            PDF ingest
          </p>
          <p className="px-1 text-[11px] leading-relaxed text-slate-500">
            Live PDF pipeline lives in the <span className="text-cyan-400/80">Personal</span> workspace. Switch there
            to upload; research books here still mock-connect for graph tabs.
          </p>
          <button
            type="button"
            onClick={() => onWorkspaceKindChange("personal")}
            className="mt-2 w-full rounded-lg border border-cyan-400/25 bg-cyan-500/10 py-2 font-mono text-[11px] text-cyan-100 transition hover:bg-cyan-500/20"
          >
            Open Personal workspace
          </button>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">
        <p className="mb-2 px-1 font-mono text-[10px] uppercase tracking-wider text-slate-500">
          {workspaceKind === "invest" ? "feeds · preview UI" : "integrations · preview UI"}
        </p>
        <ul className="space-y-1.5">
          {connectors.map((c) => {
            const st = statusById[c.id] ?? "off";
            const rowActive = activeSurface === c.id;
            return (
              <li key={c.id}>
                <button
                  type="button"
                  onClick={() => onSelectSurface(c.id)}
                  className={`flex w-full items-center gap-3 rounded-xl border p-2.5 text-left transition ${
                    rowActive
                      ? "border-cyan-400/40 bg-cyan-500/10 shadow-[0_0_16px_rgba(34,211,238,0.08)]"
                      : "border-white/5 bg-white/[0.02] hover:border-white/15 hover:bg-white/[0.04]"
                  }`}
                >
                  <span
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-white/10 font-mono text-[10px] font-bold text-white/90"
                    style={{ boxShadow: `inset 0 0 0 1px ${c.accent}33` }}
                  >
                    {c.name.slice(0, 2).toUpperCase()}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <span className="truncate text-sm text-slate-200">{c.name}</span>
                      <span
                        className={`shrink-0 rounded-full px-2 py-0.5 font-mono text-[9px] ${
                          st === "mock_on"
                            ? "bg-violet-500/20 text-violet-200"
                            : st === "connecting"
                              ? "bg-amber-500/20 text-amber-200"
                              : "bg-white/5 text-slate-500"
                        }`}
                      >
                        {st === "mock_on" ? "preview on" : st === "connecting" ? "…" : "off"}
                      </span>
                    </div>
                    <p className="truncate text-[11px] text-slate-500">{c.blurb}</p>
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      </div>

      {activity && (
        <div className="border-t border-white/5 p-3">
          <p className="font-mono text-[10px] leading-relaxed text-cyan-200/70">{activity}</p>
        </div>
      )}
    </aside>
  );
}
