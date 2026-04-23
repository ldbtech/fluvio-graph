"use client";

import { useId, type ReactNode } from "react";
import type { ConnectorDef, ConnectorId, ConnectorStatus, WorkspaceKind, WorkspaceSurface } from "@/lib/types";
import { WorkspaceProjectsPanel } from "./WorkspaceProjectsPanel";

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

const DESIGN_CONNECTORS: ConnectorDef[] = [
  {
    id: "des_bim",
    name: "BIM / IFC",
    blurb: "Model federation & clash context",
    accent: "#38bdf8",
  },
  {
    id: "des_arch_plans",
    name: "Architectural plans",
    blurb: "Sheets, rooms, envelopes",
    accent: "#c084fc",
  },
  {
    id: "des_structural",
    name: "Structural analysis",
    blurb: "FEM, members, load paths",
    accent: "#f472b6",
  },
  {
    id: "des_civil_site",
    name: "Civil & site",
    blurb: "Grading, utilities, geotech",
    accent: "#34d399",
  },
  {
    id: "des_building_codes",
    name: "Codes & loads",
    blurb: "IBC, ASCE, local amendments",
    accent: "#fbbf24",
  },
  {
    id: "des_physics_sim",
    name: "Physics & simulation",
    blurb: "Checks that geometry survives reality",
    accent: "#fb7185",
  },
];

function statusLabel(st: ConnectorStatus): string {
  if (st === "mock_on") return "On";
  if (st === "connecting") return "Connecting";
  return "Off";
}

type Props = {
  workspaceKind: WorkspaceKind;
  onWorkspaceKindChange: (kind: WorkspaceKind) => void;
  pdfInputId: string;
  activeSurface: WorkspaceSurface | null;
  statusById: Partial<Record<ConnectorId, ConnectorStatus>>;
  onSelectSurface: (surface: WorkspaceSurface) => void;
  activity: string | null;
  /** Live kg-engine base URL; when set with `onWorkspaceGraphChanged`, shows snapshot / reset controls. */
  kgUrl?: string;
  onWorkspaceGraphChanged?: () => void | Promise<void>;
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
  workspaceKind,
  onWorkspaceKindChange,
  pdfInputId,
  activeSurface,
  statusById,
  onSelectSurface,
  activity,
  kgUrl,
  onWorkspaceGraphChanged,
}: Props) {
  const docActive = activeSurface === "documents";
  const connectors =
    workspaceKind === "invest"
      ? INVEST_CONNECTORS
      : workspaceKind === "design"
        ? DESIGN_CONNECTORS
        : PERSONAL_CONNECTORS;
  const projectsDisclosureId = useId();

  return (
    <aside
      className="pointer-events-auto flex h-full min-h-0 w-[min(100%,288px)] shrink-0 flex-col border-r border-white/[0.08] bg-[rgba(24,24,27,0.78)] backdrop-blur-2xl supports-[backdrop-filter]:bg-[rgba(24,24,27,0.65)]"
      onClick={(e) => e.stopPropagation()}
    >
      <div className="shrink-0 border-b border-white/[0.06] px-4 pb-3 pt-4">
        <h1 className="text-[17px] font-semibold leading-snug tracking-tight text-zinc-100">
          {workspaceKind === "invest"
            ? "Markets desk"
            : workspaceKind === "design"
              ? "Design studio"
              : "Sources"}
        </h1>
        <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
          {workspaceKind === "invest"
            ? "Preview feeds for equities, futures, crypto, and research."
            : workspaceKind === "design"
              ? "Architecture and civil slices for a knowledge graph that ties intent to loads, codes, and simulation."
              : "Add documents, then connect integrations. Open Workspace to explore the graph."}
        </p>
      </div>

      <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto [scrollbar-gutter:stable]">
        {workspaceKind === "personal" && (
          <GroupedSection
            title="Documents"
            description="Live PDF ingest via kg-engine."
          >
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
                    docActive
                      ? "bg-sky-500/20 text-sky-100"
                      : "bg-white/[0.06] text-zinc-400"
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
        )}

        {workspaceKind === "invest" && (
          <section className="px-3 pt-5">
            <div className="rounded-2xl border border-white/[0.08] bg-white/[0.03] px-3 py-3">
              <p className="text-[12px] leading-relaxed text-zinc-500">
                Upload PDFs from the <span className="font-medium text-zinc-300">Personal</span> workspace. Connectors
                here stay preview-only for graph tabs.
              </p>
              <button
                type="button"
                onClick={() => onWorkspaceKindChange("personal")}
                className="mt-3 w-full rounded-xl bg-zinc-100 py-2.5 text-[14px] font-semibold text-zinc-900 transition hover:bg-white active:scale-[0.99]"
              >
                Open Personal
              </button>
            </div>
          </section>
        )}

        {workspaceKind === "design" && (
          <section className="px-3 pt-5">
            <div className="rounded-2xl border border-white/[0.08] bg-white/[0.03] px-3 py-3">
              <p className="text-[12px] leading-relaxed text-zinc-500">
                Spec PDFs and calc books can still live in{" "}
                <span className="font-medium text-zinc-300">Personal</span>; here we mock BIM, loads, and solver packs
                as separate graph slices until Rust ingestion lands.
              </p>
              <button
                type="button"
                onClick={() => onWorkspaceKindChange("personal")}
                className="mt-3 w-full rounded-xl border border-white/[0.1] bg-zinc-800/80 py-2.5 text-[14px] font-semibold text-zinc-100 transition hover:bg-zinc-800 active:scale-[0.99]"
              >
                Open Personal for PDFs
              </button>
            </div>
          </section>
        )}

        <GroupedSection
          title={
            workspaceKind === "invest" ? "Data feeds" : workspaceKind === "design" ? "Design sources" : "Integrations"
          }
          description={
            workspaceKind === "personal"
              ? "Choose a source to configure in the panel."
              : workspaceKind === "design"
                ? "Enable previews, then open Brain to see each slice and unified validation context."
                : undefined
          }
        >
          <ul className="divide-y divide-white/[0.06]">
            {connectors.map((c) => {
              const st = statusById[c.id] ?? "off";
              const rowActive = activeSurface === c.id;
              return (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => onSelectSurface(c.id)}
                    className={`flex w-full items-center gap-3 px-3 py-3 text-left transition-colors ${
                      rowActive
                        ? "bg-white/[0.08]"
                        : "hover:bg-white/[0.04] active:bg-white/[0.06]"
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
                            st === "mock_on"
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

        {workspaceKind === "personal" && kgUrl && onWorkspaceGraphChanged ? (
          <section className="px-3 pt-2 pb-4">
            <details className="group overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.02] open:bg-white/[0.03]">
              <summary
                className="flex cursor-pointer list-none items-center justify-between gap-2 px-3 py-3 text-left transition-colors hover:bg-white/[0.04] [&::-webkit-details-marker]:hidden"
                aria-controls={projectsDisclosureId}
              >
                <div className="min-w-0">
                  <p className="text-[13px] font-semibold text-zinc-300">Workspace & snapshots</p>
                  <p className="mt-0.5 text-[11px] leading-snug text-zinc-600">
                    Save or load graph projects — optional; expand when you need it.
                  </p>
                </div>
                <span
                  className="shrink-0 text-zinc-500 transition-transform duration-200 group-open:rotate-180"
                  aria-hidden
                >
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" className="opacity-70">
                    <path
                      d="M3.5 5.25L7 8.75L10.5 5.25"
                      stroke="currentColor"
                      strokeWidth="1.4"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </span>
              </summary>
              <div
                id={projectsDisclosureId}
                className="border-t border-white/[0.06] bg-black/20 px-1 pb-3 pt-1"
              >
                <WorkspaceProjectsPanel kgUrl={kgUrl} onAfterMutation={onWorkspaceGraphChanged} embedded />
              </div>
            </details>
          </section>
        ) : null}

        {activity ? (
          <div className="border-t border-white/[0.06] px-4 py-3">
            <p className="text-[12px] leading-relaxed text-zinc-500">{activity}</p>
          </div>
        ) : null}
      </div>
    </aside>
  );
}
