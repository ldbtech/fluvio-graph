"use client";

import { useMemo } from "react";
import type { BrainTab, ConnectorId, ConnectorStatus, WorkspaceKind } from "@/lib/types";
import { INVEST_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "@/lib/workspaceKinds";

const PERSONAL_TABS: { id: BrainTab; short: string }[] = [
  { id: "documents", short: "PDF" },
  ...PERSONAL_CONNECTOR_IDS.map((id) => ({
    id: id as BrainTab,
    short:
      id === "gmail"
        ? "Gmail"
        : id === "spotify"
          ? "Spotify"
          : id === "github"
            ? "GitHub"
            : id === "calendar"
              ? "Cal"
              : id === "whatsapp"
                ? "WA"
                : id === "slack"
                  ? "Slack"
                  : id === "notion"
                    ? "Notion"
                    : "Web",
  })),
  { id: "unified", short: "Unified" },
  { id: "meta", short: "Meta" },
];

const INVEST_TABS: { id: BrainTab; short: string }[] = [
  ...INVEST_CONNECTOR_IDS.map((id) => ({
    id: id as BrainTab,
    short:
      id === "equities"
        ? "Stocks"
        : id === "futures"
          ? "Fut."
          : id === "cryptocurrencies"
            ? "Crypto"
            : id === "fin_news"
              ? "News"
              : id === "fin_market_data"
                ? "Data"
                : "Books",
  })),
  { id: "unified", short: "Unified" },
  { id: "meta", short: "Meta" },
];

type Props = {
  workspaceKind: WorkspaceKind;
  active: BrainTab;
  onChange: (id: BrainTab) => void;
  /** Live PDF chunks from kg-engine (`source === "pdf"`). */
  documentGraphReady: boolean;
  /** Live Gmail chunks ingested (`source === "email"`). */
  gmailLiveReady: boolean;
  /** Gmail OAuth token present (sync may not have run yet). */
  gmailOAuthConnected: boolean;
  connectorStatus: Partial<Record<ConnectorId, ConnectorStatus>>;
};

export function BrainDomainTabs({
  workspaceKind,
  active,
  onChange,
  documentGraphReady,
  gmailLiveReady,
  gmailOAuthConnected,
  connectorStatus,
}: Props) {
  const tabs = useMemo(
    () => (workspaceKind === "invest" ? INVEST_TABS : PERSONAL_TABS),
    [workspaceKind],
  );

  const ready = (id: BrainTab) => {
    if (id === "unified" || id === "meta") return true;
    if (id === "documents") return documentGraphReady;
    if (id === "gmail") return gmailLiveReady || gmailOAuthConnected;
    return connectorStatus[id as ConnectorId] === "mock_on";
  };

  return (
    <div className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-white/5 bg-[#06060f]/90 px-2 py-2 backdrop-blur-sm">
      <span className="hidden shrink-0 px-2 font-mono text-[9px] uppercase tracking-wider text-slate-600 sm:inline">
        {workspaceKind === "invest" ? "markets" : "graphs"}
      </span>
      {tabs.map(({ id, short }) => {
        const isReady = ready(id);
        const isActive = active === id;
        return (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            className={`shrink-0 rounded-lg px-3 py-1.5 font-mono text-[11px] transition ${
              isActive
                ? id === "unified" || id === "meta"
                  ? "bg-violet-500/15 text-violet-100 ring-1 ring-violet-400/40"
                  : "bg-cyan-500/15 text-cyan-100 ring-1 ring-cyan-400/35"
                : "text-slate-500 hover:bg-white/5 hover:text-slate-300"
            }`}
          >
            {short}
            <span
              className={`ml-1.5 inline-block h-1.5 w-1.5 rounded-full ${
                isReady ? "bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.6)]" : "bg-slate-700"
              }`}
              title={
                id === "unified"
                  ? "Fused view (mock materialization)"
                  : id === "meta"
                    ? "Control-plane graph (mock)"
                    : isReady
                      ? "Graph data available"
                      : "Empty — connect or ingest in Sources"
              }
            />
          </button>
        );
      })}
    </div>
  );
}
