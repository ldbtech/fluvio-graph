"use client";

import { useMemo } from "react";
import type { BrainTab, ConnectorId, ConnectorStatus, WorkspaceKind } from "@/lib/types";
import { DESIGN_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "@/lib/workspaceKinds";

const PERSONAL_TABS: { id: BrainTab; short: string }[] = [
  { id: "documents", short: "PDF" },
  ...PERSONAL_CONNECTOR_IDS.map((id) => ({
    id: id as BrainTab,
    short:
      id === "gmail"
        ? "Gmail"
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
                    : String(id),
  })),
  { id: "unified", short: "Unified" },
  { id: "meta", short: "Meta" },
];

const DESIGN_TABS: { id: BrainTab; short: string }[] = [
  ...DESIGN_CONNECTOR_IDS.map((id) => ({
    id: id as BrainTab,
    short: "Architecture",
  })),
  { id: "unified", short: "Unified" },
  { id: "meta", short: "Meta" },
];

type Props = {
  workspaceKind: WorkspaceKind;
  active: BrainTab;
  onChange: (id: BrainTab) => void;
  documentGraphReady: boolean;
  gmailLiveReady: boolean;
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
  const tabs = useMemo(() => {
    if (workspaceKind === "design") return DESIGN_TABS;
    return PERSONAL_TABS;
  }, [workspaceKind]);

  const ready = (id: BrainTab) => {
    if (id === "unified" || id === "meta") return true;
    if (id === "documents") return documentGraphReady;
    if (id === "gmail") return gmailLiveReady || gmailOAuthConnected;
    return connectorStatus[id as ConnectorId] === "mock_on";
  };

  return (
    <div className="flex shrink-0 items-center gap-1.5 overflow-x-auto border-b border-white/[0.06] bg-zinc-950/60 px-3 py-2.5 backdrop-blur-md [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
      <span className="hidden shrink-0 pr-1 text-[11px] font-medium uppercase tracking-wide text-zinc-600 sm:inline">
        {workspaceKind === "design" ? "Design" : "Graphs"}
      </span>
      {tabs.map(({ id, short }) => {
        const isReady = ready(id);
        const isActive = active === id;
        return (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            className={`flex shrink-0 items-center gap-2 rounded-full px-3.5 py-1.5 text-[13px] font-medium tracking-tight transition-all duration-200 ease-out ${
              isActive
                ? "bg-zinc-100 text-zinc-900 shadow-sm"
                : "text-zinc-500 hover:bg-zinc-800/60 hover:text-zinc-200"
            }`}
          >
            {short}
            <span
              className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                isReady ? "bg-emerald-400" : "bg-zinc-600"
              }`}
              title={
                id === "unified"
                  ? "Fused view"
                  : id === "meta"
                    ? "Control plane"
                    : isReady
                      ? "Data available"
                      : "Connect in Sources"
              }
            />
          </button>
        );
      })}
    </div>
  );
}
