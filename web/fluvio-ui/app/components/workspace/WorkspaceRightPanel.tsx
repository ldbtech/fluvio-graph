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
  /** Changes reset chat history (switching graph tabs). */
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
      className={`flex min-h-0 flex-1 flex-col overflow-hidden bg-[#060616]/95 backdrop-blur-xl ${
        dock
          ? "h-full"
          : "max-h-[min(560px,calc(100vh-5rem))] w-[min(100vw-1.5rem,380px)] rounded-2xl border border-cyan-400/20 shadow-[0_0_48px_rgba(0,0,0,0.45)]"
      }`}
      onClick={(e) => e.stopPropagation()}
    >
          <div className="flex border-b border-white/5">
            {(["chat", "agents"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTab(t)}
                className={`flex-1 py-2.5 font-mono text-xs uppercase tracking-wider transition ${
                  tab === t
                    ? "border-b-2 border-cyan-400 text-cyan-200"
                    : "text-slate-500 hover:text-slate-300"
                }`}
              >
                {t}
              </button>
            ))}
          </div>

          {tab === "chat" && (
            <>
              <div className="border-b border-cyan-400/10 bg-cyan-400/5 px-3 py-2">
                <p className="font-mono text-[10px] text-cyan-200/80">
                  {brainTab === "unified" && (
                    <>
                      Unified brain · mock fusion chat · <span className="text-violet-300">{nodeCount}</span> nodes
                    </>
                  )}
                  {brainTab === "meta" && (
                    <>
                      Meta-graph · control-plane mock · <span className="text-violet-300">{nodeCount}</span> nodes
                    </>
                  )}
                  {brainTab !== "unified" && brainTab !== "meta" && chatSource === "live" && !graphEmpty && (
                    <>
                      Live graph chat · <span className="text-violet-300">{nodeCount}</span> nodes ·{" "}
                      <span className="text-slate-500">PDF brain</span>
                    </>
                  )}
                  {brainTab !== "unified" && brainTab !== "meta" && !(chatSource === "live" && !graphEmpty) && graphEmpty && (
                    <>
                      No nodes — mock assistant · <span className="text-violet-300">{brainTab}</span>
                    </>
                  )}
                  {brainTab !== "unified" && brainTab !== "meta" && !(chatSource === "live" && !graphEmpty) && !graphEmpty && (
                    <>
                      Preview chat · <span className="text-violet-300">{nodeCount}</span> nodes ·{" "}
                      <span className="text-slate-500">{brainTab}</span>
                    </>
                  )}
                </p>
              </div>
              <div ref={messagesRef} className="min-h-[200px] flex-1 space-y-3 overflow-y-auto p-3">
                {messages.length === 0 && (
                  <div className="flex flex-col items-center justify-center gap-2 py-10 text-center opacity-50">
                    <span className="text-3xl">⬢</span>
                    <p className="px-4 font-mono text-xs text-slate-400">
                      Ask the graph to explain a claim, compare sections, or delegate a follow-up.
                    </p>
                  </div>
                )}
                {messages.map((m, i) => (
                  <div
                    key={i}
                    className={`rounded-xl border px-3 py-2.5 font-mono text-xs leading-relaxed break-words ${
                      m.role === "user"
                        ? "ml-6 border-cyan-400/15 bg-cyan-400/10 text-cyan-100"
                        : "mr-4 border-white/10 bg-white/[0.04] text-slate-200"
                    }`}
                  >
                    {m.content}
                  </div>
                ))}
                {loading && (
                  <div className="mr-4 rounded-xl border border-white/10 bg-white/[0.04] px-3 py-2.5 font-mono text-xs text-slate-400">
                    <span className="animate-pulse">thinking</span>
                    <span className="animate-bounce">...</span>
                  </div>
                )}
              </div>
              <div className="flex gap-2 border-t border-white/5 p-3">
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
                      ? "Ask across all subgraphs (mock)…"
                      : brainTab === "meta"
                        ? "Ask the control plane (mock)…"
                        : chatSource === "live"
                          ? "Ask your PDF graph…"
                          : `Task or question for ${brainTab}…`
                  }
                  className="flex-1 rounded-xl border border-white/10 bg-white/[0.06] px-3 py-2 font-mono text-xs text-slate-200 outline-none placeholder:text-slate-600 focus:border-cyan-400/35"
                />
                <button
                  type="button"
                  onClick={() => void sendMessage()}
                  disabled={loading || !input.trim()}
                  className="rounded-xl border border-cyan-400/30 px-3 py-2 font-mono text-xs font-bold text-cyan-950 disabled:opacity-25"
                  style={{
                    background:
                      loading || !input.trim()
                        ? "transparent"
                        : "linear-gradient(135deg,#5dfff8,#9b85ff)",
                    color: loading || !input.trim() ? "#64748b" : "#0a0a12",
                  }}
                >
                  →
                </button>
              </div>
            </>
          )}

          {tab === "agents" && (
            <div className="flex min-h-[320px] flex-1 flex-col gap-3 overflow-y-auto p-3">
              <p className="font-mono text-[10px] leading-relaxed text-slate-500">
                Autonomous workers (mock): they spin up, log fake heartbeats, and sit ready for real
                orchestration later.
              </p>
              <div className="space-y-2">
                <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">source agents</p>
                {DEPLOYABLE.map((def) => (
                  <div
                    key={def.id}
                    className="flex items-center gap-3 rounded-xl border border-white/8 bg-white/[0.03] p-2.5"
                  >
                    <span className="text-lg text-cyan-200/80">{def.icon}</span>
                    <div className="min-w-0 flex-1">
                      <p className="text-sm text-slate-200">{def.name}</p>
                      <p className="text-[11px] text-slate-500">{def.description}</p>
                    </div>
                    <button
                      type="button"
                      onClick={() => deploy(def)}
                      className="shrink-0 rounded-lg border border-violet-400/30 bg-violet-500/15 px-2.5 py-1 font-mono text-[10px] text-violet-100 hover:bg-violet-500/25"
                    >
                      deploy
                    </button>
                  </div>
                ))}
              </div>
              {dock && (
                <div className="space-y-2 border-t border-violet-500/15 pt-3">
                  <p className="font-mono text-[10px] uppercase tracking-wider text-violet-400/70">
                    fusion mesh · infra (mock)
                  </p>
                  <p className="font-mono text-[10px] leading-relaxed text-slate-600">
                    Spin these when you want the UI to stand in for axum workers that own cross-domain sync, rebuilds,
                    and policy.
                  </p>
                  {FUSION_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-xl border border-violet-500/20 bg-violet-500/[0.06] p-2.5"
                    >
                      <span className="text-lg text-violet-200/90">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm text-violet-100">{def.name}</p>
                        <p className="text-[11px] text-slate-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-fuchsia-400/35 bg-fuchsia-500/15 px-2.5 py-1 font-mono text-[10px] text-fuchsia-100 hover:bg-fuchsia-500/25"
                      >
                        spin up
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {dock && workspaceKind === "personal" && (
                <div className="space-y-2 border-t border-amber-500/20 pt-3">
                  <p className="font-mono text-[10px] uppercase tracking-wider text-amber-400/80">
                    research & remediation (mock)
                  </p>
                  <p className="font-mono text-[10px] leading-relaxed text-slate-600">
                    Pair with <span className="text-amber-600/90">Website</span> + PDF learnings: scout the open web,
                    diff your graph for issues, then queue fixes — all to be backed by axum workers + tool policies.
                  </p>
                  {RESEARCH_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-xl border border-amber-500/25 bg-amber-500/[0.07] p-2.5"
                    >
                      <span className="text-lg text-amber-200/90">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm text-amber-50">{def.name}</p>
                        <p className="text-[11px] text-slate-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-amber-400/40 bg-amber-500/20 px-2.5 py-1 font-mono text-[10px] text-amber-50 hover:bg-amber-500/30"
                      >
                        spin up
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {dock && workspaceKind === "invest" && (
                <div className="space-y-2 border-t border-amber-500/20 pt-3">
                  <p className="font-mono text-[10px] uppercase tracking-wider text-amber-400/80">
                    markets desk (mock)
                  </p>
                  <p className="font-mono text-[10px] leading-relaxed text-slate-600">
                    Spin workers that would own symbology, roll windows, and exposure caps before any vendor pull hits
                    the fusion graph.
                  </p>
                  {MARKETS_AGENTS.map((def) => (
                    <div
                      key={def.id}
                      className="flex items-center gap-3 rounded-xl border border-amber-500/25 bg-amber-500/[0.07] p-2.5"
                    >
                      <span className="text-lg text-amber-200/90">{def.icon}</span>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm text-amber-50">{def.name}</p>
                        <p className="text-[11px] text-slate-500">{def.description}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => deploy(def)}
                        className="shrink-0 rounded-lg border border-amber-400/40 bg-amber-500/20 px-2.5 py-1 font-mono text-[10px] text-amber-50 hover:bg-amber-500/30"
                      >
                        spin up
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {running.length > 0 && (
                <div className="space-y-2 border-t border-white/5 pt-3">
                  <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">running</p>
                  {running.map((a) => (
                    <div
                      key={`${a.def.id}-${a.started}`}
                      className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-2.5"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-sm text-emerald-100">{a.def.name}</span>
                        <span className="font-mono text-[10px] text-emerald-300/80">
                          {a.status === "spawning" && "spawning…"}
                          {a.status === "running" && "● autonomous"}
                          {a.status === "paused" && "○ paused"}
                        </span>
                      </div>
                      <div className="mt-2 flex gap-2">
                        {a.status === "running" && (
                          <button
                            type="button"
                            onClick={() => pause(a.started)}
                            className="rounded-md border border-white/15 px-2 py-1 font-mono text-[10px] text-slate-300 hover:bg-white/5"
                          >
                            pause
                          </button>
                        )}
                        {a.status === "paused" && (
                          <button
                            type="button"
                            onClick={() => resume(a.started)}
                            className="rounded-md border border-emerald-400/30 px-2 py-1 font-mono text-[10px] text-emerald-200 hover:bg-emerald-500/10"
                          >
                            resume
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
      <div className="flex h-full min-h-0 w-[min(100%,380px)] shrink-0 flex-col border-l border-cyan-400/15 bg-[#050510]/80">
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
          className="rounded-full border border-cyan-400/25 bg-[#0a0a1f]/90 px-4 py-2 font-mono text-xs text-cyan-100 shadow-[0_0_24px_rgba(0,255,242,0.12)] backdrop-blur-md transition hover:border-cyan-300/50"
        >
          {open ? "hide panel" : "workspace"}
        </button>
      </div>

      {open && <div className="pointer-events-auto flex max-h-[min(560px,calc(100vh-5rem))]">{panelInner}</div>}
    </div>
  );
}
