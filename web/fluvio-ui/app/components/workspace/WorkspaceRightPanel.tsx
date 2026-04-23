"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { KG_URL } from "@/lib/constants";
import { mockAssistantReply } from "@/lib/mockWorkspace";
import type { BrainTab, ChatMessage, MockAgent, WorkspaceKind } from "@/lib/types";

const DEPLOYABLE: MockAgent[] = [
  {
    id: "summarizer",
    name: "Rolling digest",
    description: "Daily summary of new graph nodes across sources.",
    icon: "◈",
  },
  {
    id: "task-runner",
    name: "Task scout",
    description: "Watches chat for actionable items and drafts checklists.",
    icon: "⬡",
  },
  {
    id: "code-linker",
    name: "Repo linker",
    description: "Maps symbols mentioned in chat to GitHub paths (mock).",
    icon: "⌁",
  },
];

const FUSION_AGENTS: MockAgent[] = [
  {
    id: "fusion-orchestrator",
    name: "Fusion orchestrator",
    description: "Schedules parallel reads across subgraphs; merges ranked context for chat (mock).",
    icon: "◉",
  },
  {
    id: "subgraph-weaver",
    name: "Subgraph weaver",
    description: "Maintains cross-domain edges + invalidation when any source syncs (mock).",
    icon: "⎔",
  },
  {
    id: "policy-sentinel",
    name: "Policy sentinel",
    description: "Enforces consent + retention per domain before agents act (mock).",
    icon: "⌬",
  },
];

const DESIGN_AGENTS: MockAgent[] = [
  {
    id: "load-weaver",
    name: "Load combination weaver",
    description:
      "Maps code clauses to tributary areas and member demand sets; flags mismatches between arch intent and adopted ASCE/IBC edition (mock).",
    icon: "⏉",
  },
  {
    id: "physics-sentinel",
    name: "Physics sentinel",
    description:
      "Runs drift, acceleration, and wind comfort gates; opens tickets when solver outputs diverge from BIM assumptions (mock).",
    icon: "⌭",
  },
  {
    id: "clash-arbiter",
    name: "Clash arbiter",
    description:
      "Prioritizes MEP vs structural clashes by constructability and code minimums; proposes reroutes before field RFIs (mock).",
    icon: "⎍",
  },
];

const MARKETS_AGENTS: MockAgent[] = [
  {
    id: "tape-librarian",
    name: "Tape librarian",
    description:
      "Normalizes vendor symbology, corporate actions, and session calendars into canonical ticker nodes (mock).",
    icon: "▤",
  },
  {
    id: "roll-scheduler",
    name: "Roll scheduler",
    description:
      "Tracks front-month liquidity and proposes roll windows across futures + hedged crypto perps (mock).",
    icon: "↻",
  },
  {
    id: "risk-governor",
    name: "Risk governor",
    description:
      "Enforces exposure caps vs benchmark, margin buffers, and kill-switches before desk agents trade (mock).",
    icon: "⚖",
  },
];

const RESEARCH_AGENTS: MockAgent[] = [
  {
    id: "net-scout",
    name: "Web scout",
    description:
      "Bounded internet search with citations; feeds the unified graph for CVE advisories, vendor bulletins, and best-practice deltas (mock).",
    icon: "◎",
  },
  {
    id: "error-radar",
    name: "Error & anomaly radar",
    description:
      "Scans web crawl + PDF nodes for contradictions, risky patterns, and missing controls; proposes concrete edits / tickets (mock).",
    icon: "⚑",
  },
  {
    id: "patch-runner",
    name: "Remediation runner",
    description:
      "Turns findings into draft patches or PRs; human-in-the-loop + kill-switch before any auto-merge (mock).",
    icon: "⛭",
  },
];

type RunningAgent = {
  def: MockAgent;
  status: "spawning" | "running" | "paused";
  started: number;
};

type Tab = "chat" | "agents";

type Props = {
  workspaceKind: WorkspaceKind;
  /** When true, panel is a fixed right column in the brain layout (not floating). */
  dock?: boolean;
  /** Changes reset chat history (brain tab, or GitHub `owner/repo` when docked on GitHub). */
  domainKey: string;
  chatSource: "live" | "mock";
  brainTab: BrainTab;
  graphEmpty: boolean;
  nodeCount: number;
  chatPrefill: string | null;
  onConsumeChatPrefill: () => void;
};

export function WorkspaceRightPanel({
  workspaceKind,
  dock = false,
  domainKey,
  chatSource,
  brainTab,
  graphEmpty,
  nodeCount,
  chatPrefill,
  onConsumeChatPrefill,
}: Props) {
  const messagesRef = useRef<HTMLDivElement>(null);
  const [tab, setTab] = useState<Tab>("chat");
  const [open, setOpen] = useState(true);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [running, setRunning] = useState<RunningAgent[]>([]);

  useEffect(() => {
    if (messagesRef.current) {
      messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
    }
  }, [messages, loading, tab]);

  useEffect(() => {
    setMessages([]);
    setInput("");
  }, [domainKey]);

  useEffect(() => {
    if (!chatPrefill) return;
    setTab("chat");
    setOpen(true);
    setInput(chatPrefill);
    onConsumeChatPrefill();
  }, [chatPrefill, onConsumeChatPrefill]);

  const sendMessage = useCallback(async () => {
    if (!input.trim() || loading) return;
    const question = input.trim();
    setInput("");
    setMessages((m) => [...m, { role: "user", content: question }]);
    setLoading(true);

    try {
      if (chatSource !== "live" || graphEmpty) {
        await new Promise((r) => setTimeout(r, 420 + Math.random() * 400));
        setMessages((m) => [
          ...m,
          { role: "assistant", content: mockAssistantReply(question, brainTab, workspaceKind) },
        ]);
        setLoading(false);
        return;
      }

      const res = await fetch(`${KG_URL}/chat`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ question, history: messages }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as { answer: string };
      setMessages((m) => [...m, { role: "assistant", content: data.answer }]);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setMessages((m) => [...m, { role: "assistant", content: `error: ${message}` }]);
    } finally {
      setLoading(false);
    }
  }, [input, loading, messages, graphEmpty, chatSource, brainTab, workspaceKind]);

  const deploy = (def: MockAgent) => {
    setRunning((r) => {
      const paused = r.find((x) => x.def.id === def.id && x.status === "paused");
      if (paused) {
        return r.map((a) => (a === paused ? { ...a, status: "running" as const } : a));
      }
      if (r.some((x) => x.def.id === def.id && (x.status === "spawning" || x.status === "running"))) {
        return r;
      }
      const next: RunningAgent = { def, status: "spawning", started: Date.now() };
      const instanceKey = `${def.id}-${next.started}`;
      setTimeout(() => {
        setRunning((cur) =>
          cur.map((a) =>
            `${a.def.id}-${a.started}` === instanceKey && a.status === "spawning"
              ? { ...a, status: "running" as const }
              : a,
          ),
        );
      }, 900);
      return [...r, next];
    });
    setTab("agents");
  };

  const pause = (started: number) => {
    setRunning((r) =>
      r.map((a) => (a.started === started && a.status === "running" ? { ...a, status: "paused" } : a)),
    );
  };

  const resume = (started: number) => {
    setRunning((r) =>
      r.map((a) => (a.started === started && a.status === "paused" ? { ...a, status: "running" } : a)),
    );
  };

  const panelInner = (
    <div
      className={`flex min-h-0 flex-1 flex-col overflow-hidden border-white/[0.08] bg-zinc-950/90 backdrop-blur-2xl ${
        dock
          ? "h-full border-l"
          : "max-h-[min(560px,calc(100vh-5rem))] w-[min(100vw-1.5rem,380px)] rounded-2xl border shadow-2xl shadow-black/50"
      }`}
      onClick={(e) => e.stopPropagation()}
    >
          <div className="flex border-b border-white/[0.06] p-1">
            {(["chat", "agents"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTab(t)}
                className={`flex-1 rounded-lg py-2 text-[13px] font-semibold tracking-tight transition-colors duration-200 ${
                  tab === t
                    ? "bg-zinc-100 text-zinc-900"
                    : "text-zinc-500 hover:bg-zinc-800/50 hover:text-zinc-200"
                }`}
              >
                {t === "chat" ? "Chat" : "Agents"}
              </button>
            ))}
          </div>

          {tab === "chat" && (
            <>
              <div className="border-b border-white/[0.06] bg-zinc-900/40 px-3 py-2.5">
                <p className="text-[12px] font-medium leading-snug text-zinc-400">
                  {brainTab === "unified" && (
                    <>
                      Unified · mock fusion · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes
                    </>
                  )}
                  {brainTab === "meta" && (
                    <>
                      Meta · control plane · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes
                    </>
                  )}
                  {brainTab === "github" && (
                    <>
                      GitHub · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes ·{" "}
                      <span className="text-zinc-600">preview</span>
                    </>
                  )}
                  {brainTab !== "unified" && brainTab !== "meta" && brainTab !== "github" && chatSource === "live" && !graphEmpty && (
                    <>
                      Live graph · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes ·{" "}
                      <span className="text-zinc-600">PDF</span>
                    </>
                  )}
                  {brainTab !== "unified" &&
                    brainTab !== "meta" &&
                    brainTab !== "github" &&
                    !(chatSource === "live" && !graphEmpty) &&
                    graphEmpty && (
                    <>
                      No graph yet · <span className="text-zinc-300">{brainTab}</span>
                    </>
                  )}
                  {brainTab !== "unified" &&
                    brainTab !== "meta" &&
                    brainTab !== "github" &&
                    !(chatSource === "live" && !graphEmpty) &&
                    !graphEmpty && (
                    <>
                      Preview · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes ·{" "}
                      <span className="text-zinc-600">{brainTab}</span>
                    </>
                  )}
                </p>
              </div>
              <div ref={messagesRef} className="min-h-[200px] flex-1 space-y-3 overflow-y-auto p-3">
                {messages.length === 0 && (
                  <div className="flex flex-col items-center justify-center gap-3 py-12 text-center">
                    <span className="text-2xl text-zinc-600">◇</span>
                    <p className="max-w-[240px] px-2 text-[13px] leading-relaxed text-zinc-500">
                      Ask about the graph, compare claims, or queue a follow-up.
                    </p>
                  </div>
                )}
                {messages.map((m, i) => (
                  <div
                    key={i}
                    className={`rounded-2xl border px-3.5 py-2.5 text-[13px] leading-relaxed break-words ${
                      m.role === "user"
                        ? "ml-5 border-sky-500/20 bg-sky-500/10 text-zinc-100"
                        : "mr-3 border-white/[0.06] bg-zinc-900/80 text-zinc-300"
                    }`}
                  >
                    {m.content}
                  </div>
                ))}
                {loading && (
                  <div className="mr-3 rounded-2xl border border-white/[0.06] bg-zinc-900/60 px-3.5 py-2.5 text-[13px] text-zinc-500">
                    <span className="animate-pulse">Thinking</span>
                    <span className="animate-bounce">…</span>
                  </div>
                )}
              </div>
              <div className="flex gap-2 border-t border-white/[0.06] p-3">
                <input
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      void sendMessage();
                    }
                  }}
                  placeholder={
                    brainTab === "unified"
                      ? "Message unified graph…"
                      : brainTab === "meta"
                        ? "Message control plane…"
                        : brainTab === "github"
                          ? "Ask about the repo graph…"
                          : chatSource === "live"
                            ? "Message PDF graph…"
                            : `Message ${brainTab}…`
                  }
                  className="flex-1 rounded-xl border border-white/[0.08] bg-zinc-900/80 px-3 py-2.5 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-sky-500/40 focus:ring-1 focus:ring-sky-500/20"
                />
                <button
                  type="button"
                  onClick={() => void sendMessage()}
                  disabled={loading || !input.trim()}
                  className="shrink-0 rounded-xl bg-zinc-100 px-4 py-2.5 text-[13px] font-semibold text-zinc-900 transition enabled:hover:bg-white disabled:cursor-not-allowed disabled:opacity-30"
                >
                  Send
                </button>
              </div>
            </>
          )}

          {tab === "agents" && (
            <div className="flex min-h-[320px] flex-1 flex-col gap-4 overflow-y-auto p-3">
              <p className="text-[12px] leading-relaxed text-zinc-500">
                Mock agents for layout and flows. Real orchestration hooks in later.
              </p>
              <div className="space-y-2">
                <p className="px-0.5 text-[11px] font-medium text-zinc-500">Source</p>
                {DEPLOYABLE.map((def) => (
                  <div
                    key={def.id}
                    className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-zinc-900/50 p-3"
                  >
                    <span className="text-lg text-zinc-400">{def.icon}</span>
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-medium text-zinc-100">{def.name}</p>
                      <p className="text-[12px] text-zinc-500">{def.description}</p>
                    </div>
                    <button
                      type="button"
                      onClick={() => deploy(def)}
                      className="shrink-0 rounded-lg bg-zinc-100 px-3 py-1.5 text-[11px] font-semibold text-zinc-900 transition hover:bg-white"
                    >
                      Deploy
                    </button>
                  </div>
                ))}
              </div>
              {dock && (
                <div className="space-y-2 border-t border-white/[0.06] pt-4">
                  <p className="px-0.5 text-[11px] font-medium text-zinc-500">Fusion · mock</p>
                  <p className="text-[12px] leading-relaxed text-zinc-600">
                    Cross-domain sync and policy workers (placeholder UI).
                  </p>
                  {FUSION_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-zinc-900/50 p-3"
                    >
                      <span className="text-lg text-zinc-400">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-zinc-100">{def.name}</p>
                        <p className="text-[12px] text-zinc-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-white/[0.1] bg-zinc-800 px-3 py-1.5 text-[11px] font-semibold text-zinc-200 transition hover:bg-zinc-700"
                      >
                        Start
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {dock && workspaceKind === "personal" && (
                <div className="space-y-2 border-t border-white/[0.06] pt-4">
                  <p className="px-0.5 text-[11px] font-medium text-zinc-500">Research · mock</p>
                  <p className="text-[12px] leading-relaxed text-zinc-600">
                    Web scout and remediation flows (placeholder).
                  </p>
                  {RESEARCH_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-zinc-900/50 p-3"
                    >
                      <span className="text-lg text-zinc-400">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-zinc-100">{def.name}</p>
                        <p className="text-[12px] text-zinc-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-white/[0.1] bg-zinc-800 px-3 py-1.5 text-[11px] font-semibold text-zinc-200 transition hover:bg-zinc-700"
                      >
                        Start
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {dock && workspaceKind === "invest" && (
                <div className="space-y-2 border-t border-white/[0.06] pt-4">
                  <p className="px-0.5 text-[11px] font-medium text-zinc-500">Markets · mock</p>
                  <p className="text-[12px] leading-relaxed text-zinc-600">
                    Desk agents for symbology, rolls, and risk (placeholder).
                  </p>
                  {MARKETS_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-zinc-900/50 p-3"
                    >
                      <span className="text-lg text-zinc-400">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-zinc-100">{def.name}</p>
                        <p className="text-[12px] text-zinc-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-white/[0.1] bg-zinc-800 px-3 py-1.5 text-[11px] font-semibold text-zinc-200 transition hover:bg-zinc-700"
                      >
                        Start
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {dock && workspaceKind === "design" && (
                <div className="space-y-2 border-t border-white/[0.06] pt-4">
                  <p className="px-0.5 text-[11px] font-medium text-zinc-500">Design validation · mock</p>
                  <p className="text-[12px] leading-relaxed text-zinc-600">
                    Agents that cross-link architecture, civil, codes, and physics so automated checks stay grounded in
                    graph provenance (placeholder).
                  </p>
                  {DESIGN_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-2xl border border-white/[0.06] bg-zinc-900/50 p-3"
                    >
                      <span className="text-lg text-zinc-400">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-zinc-100">{def.name}</p>
                        <p className="text-[12px] text-zinc-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-white/[0.1] bg-zinc-800 px-3 py-1.5 text-[11px] font-semibold text-zinc-200 transition hover:bg-zinc-700"
                      >
                        Start
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {running.length > 0 && (
                <div className="space-y-2 border-t border-white/[0.06] pt-4">
                  <p className="px-0.5 text-[11px] font-medium text-zinc-500">Running</p>
                  {running.map((a) => (
                    <div
                      key={`${a.def.id}-${a.started}`}
                      className="rounded-2xl border border-emerald-500/20 bg-emerald-950/30 p-3"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-sm font-medium text-emerald-100">{a.def.name}</span>
                        <span className="text-[11px] font-medium text-emerald-400/90">
                          {a.status === "spawning" && "Starting…"}
                          {a.status === "running" && "Active"}
                          {a.status === "paused" && "Paused"}
                        </span>
                      </div>
                      <div className="mt-2 flex gap-2">
                        {a.status === "running" && (
                          <button
                            type="button"
                            onClick={() => pause(a.started)}
                            className="rounded-lg border border-white/[0.1] px-3 py-1.5 text-[11px] font-medium text-zinc-300 transition hover:bg-white/[0.05]"
                          >
                            Pause
                          </button>
                        )}
                        {a.status === "paused" && (
                          <button
                            type="button"
                            onClick={() => resume(a.started)}
                            className="rounded-lg bg-zinc-100 px-3 py-1.5 text-[11px] font-semibold text-zinc-900 transition hover:bg-white"
                          >
                            Resume
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
    </div>
  );

  if (dock) {
    return (
      <div className="flex h-full min-h-0 w-[min(100%,380px)] shrink-0 flex-col border-l border-white/[0.06] bg-zinc-950/50">
        <div className="flex min-h-0 flex-1 flex-col px-1 pt-2">
          {panelInner}
        </div>
      </div>
    );
  }

  return (
    <div className="pointer-events-none absolute right-0 top-0 z-20 flex h-full flex-col items-end p-3">
      <div className="pointer-events-auto mb-2 flex gap-2">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="rounded-full border border-white/[0.1] bg-zinc-900/95 px-4 py-2 text-[13px] font-semibold text-zinc-100 shadow-lg shadow-black/30 backdrop-blur-md transition hover:bg-zinc-800"
        >
          {open ? "Close" : "Panel"}
        </button>
      </div>

      {open && <div className="pointer-events-auto flex max-h-[min(560px,calc(100vh-5rem))]">{panelInner}</div>}
    </div>
  );
}
