"use client";

import { type ReactNode } from "react";
import type { ConnectorDef, ConnectorId, ConnectorStatus, WorkspaceSurface } from "@/shared/lib/types";

const PERSONAL_CONNECTORS: ConnectorDef[] = [
  {
    id: "gmail",
    name: "Gmail",
    blurb: "Threads → entities",
    accent: "#ea4335",
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
];

function statusLabel(st: ConnectorStatus): string {
  if (st === "ready") return "Ready";
  if (st === "connecting") return "Connecting";
  return "Off";
}

type Props = {
  pdfInputId: string;
  activeSurface: WorkspaceSurface | null;
  statusById: Partial<Record<ConnectorId, ConnectorStatus>>;
  onSelectSurface: (surface: WorkspaceSurface) => void;
  activity: string | null;
};

/** Grouped list shell — reads like an inset iOS / macOS settings table. */
function GroupedSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="px-3 pt-5 first:pt-4">
      <div className="px-1">
        <h2 className="text-[13px] font-semibold leading-tight text-zinc-400">{title}</h2>
        {description ? (
          <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">{description}</p>
        ) : null}
      </div>
      <div className="mt-2 overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
        {children}
      </div>
    </section>
  );
}

export function ConnectorSidebar({
  pdfInputId,
  activeSurface,
  statusById,
  onSelectSurface,
  activity,
}: Props) {
  const docActive = activeSurface === "documents";

  return (
    <aside
      className="pointer-events-auto flex h-full min-h-0 w-[min(100%,288px)] shrink-0 flex-col border-r border-white/[0.08] bg-[rgba(24,24,27,0.78)] backdrop-blur-2xl supports-[backdrop-filter]:bg-[rgba(24,24,27,0.65)]"
      onClick={(e) => e.stopPropagation()}
    >
      <div className="shrink-0 border-b border-white/[0.06] px-4 pb-3 pt-4">
        <h1 className="text-[17px] font-semibold leading-snug tracking-tight text-zinc-100">Sources</h1>
        <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
          Add documents, then connect integrations. Open Workspace to explore the graph.
        </p>
      </div>

      <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto [scrollbar-gutter:stable]">
        <GroupedSection title="Documents" description="Live PDF ingest via kg-engine.">
          <div className="flex items-stretch divide-x divide-white/[0.06]">
            <button
              type="button"
              onClick={() => onSelectSurface("documents")}
              className={`flex min-w-0 flex-1 items-center gap-3 px-3 py-3 text-left transition-colors ${
                docActive ? "bg-sky-500/[0.12]" : "hover:bg-white/[0.04] active:bg-white/[0.06]"
              }`}
            >
              <span
                className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-[12px] font-semibold ${
                  docActive ? "bg-sky-500/20 text-sky-100" : "bg-white/[0.06] text-zinc-400"
                }`}
                aria-hidden
              >
                PDF
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      docActive ? "bg-sky-400 shadow-[0_0_6px_rgba(56,189,248,0.5)]" : "bg-emerald-400/90"
                    }`}
                  />
                  <span className="truncate text-[15px] font-medium text-zinc-100">Documents</span>
                </div>
                <p className="mt-0.5 truncate text-[12px] text-zinc-500">
                  <span className="font-mono text-[11px] text-zinc-500">/ingest/pdf</span>
                </p>
              </div>
            </button>
            <label
              htmlFor={pdfInputId}
              onClick={(e) => e.stopPropagation()}
              className="flex w-[52px] shrink-0 cursor-pointer flex-col items-center justify-center text-zinc-400 transition-colors hover:bg-white/[0.05] hover:text-zinc-200 active:bg-white/[0.07]"
              title="Upload PDF"
            >
              <span className="text-xl font-light leading-none">+</span>
              <span className="mt-0.5 text-[10px] font-medium text-zinc-600">Add</span>
            </label>
          </div>
        </GroupedSection>

        <GroupedSection title="Integrations" description="Choose a source to configure in the panel.">
          <ul className="divide-y divide-white/[0.06]">
            {PERSONAL_CONNECTORS.map((c) => {
              const st = statusById[c.id] ?? "off";
              const rowActive = activeSurface === c.id;
              return (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => onSelectSurface(c.id)}
                    className={`flex w-full items-center gap-3 px-3 py-3 text-left transition-colors ${
                      rowActive ? "bg-white/[0.08]" : "hover:bg-white/[0.04] active:bg-white/[0.06]"
                    }`}
                  >
                    <span
                      className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-white/[0.06] text-[11px] font-semibold text-zinc-200 ring-1 ring-inset ring-white/[0.06]"
                      style={{ boxShadow: `inset 0 0 0 1px ${c.accent}33` }}
                    >
                      {c.name.slice(0, 2).toUpperCase()}
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="truncate text-[15px] font-medium text-zinc-100">{c.name}</span>
                        <span
                          className={`shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium ${
                            st === "ready"
                              ? "bg-emerald-500/15 text-emerald-300/95"
                              : st === "connecting"
                                ? "bg-amber-500/15 text-amber-200/95"
                                : "bg-zinc-800/90 text-zinc-500"
                          }`}
                        >
                          {statusLabel(st)}
                        </span>
                      </div>
                      <p className="mt-0.5 truncate text-[12px] text-zinc-500">{c.blurb}</p>
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        </GroupedSection>

        {activity ? (
          <div className="border-t border-white/[0.06] px-4 py-3">
            <p className="text-[12px] leading-relaxed text-zinc-500">{activity}</p>
          </div>
        ) : null}
      </div>
    </aside>
  );
}
