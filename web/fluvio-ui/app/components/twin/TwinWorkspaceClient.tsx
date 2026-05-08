"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence } from "framer-motion";
import { ChatMessage } from "./ChatMessage";
import { FluvioTwinMark } from "./FluvioTwinMark";
import { GraphBackground } from "./GraphBackground";
import { InputBar } from "./InputBar";
import { LoadingDots } from "./LoadingDots";
import { TwinD3Graph } from "./TwinD3Graph";
import { fetchFluvioSocialGraph, type FluvioGraphPayload } from "@/lib/fluvioDashboardApi";
import { resetTwinChatBootstrap, tryBeginTwinChatBootstrap } from "@/lib/twinChatSession";
import { streamTwinAssistant } from "@/lib/twinChatStream";
import { buildTwinGraphContext, type TwinGraphPayload } from "@/lib/twinGraphStore";

type Msg = { id: string; role: "user" | "assistant"; content: string };

function rid() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

const EMPTY_TWIN_GRAPH: TwinGraphPayload = { nodes: [], edges: [] };

function twinPayloadFromNetwork(p: FluvioGraphPayload): TwinGraphPayload {
  return {
    nodes: p.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      page: n.page,
      source: n.source,
    })),
    edges: p.edges.map((e) => ({
      from: e.from,
      to: e.to,
      token: e.token,
      probability: e.probability,
      label: e.label,
    })),
  };
}

export function TwinWorkspaceClient() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const topicAtMount = useRef(searchParams.get("topic"));
  /** Phone: put chat above graph on `/chat`; graph-first on `/graph`. */
  const chatFirstMobile = pathname === "/chat";

  const [graph, setGraph] = useState<TwinGraphPayload>(() => ({ ...EMPTY_TWIN_GRAPH }));
  const [selected, setSelected] = useState<{ id: string; label: string } | null>(null);
  const [graphHydrateState, setGraphHydrateState] = useState<"loading" | "ok" | "error">(
    "loading",
  );
  const [graphHydrateMsg, setGraphHydrateMsg] = useState<string | null>(null);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const graphRef = useRef(graph);
  const selectedRef = useRef(selected);
  const messagesRef = useRef<Msg[]>(messages);
  const chatScrollRef = useRef<HTMLDivElement>(null);
  graphRef.current = graph;
  selectedRef.current = selected;
  messagesRef.current = messages;

  useEffect(() => {
    const el = chatScrollRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [messages]);

  const reloadTwinNetwork = useCallback(async (signal?: AbortSignal) => {
    setGraphHydrateState("loading");
    setGraphHydrateMsg(null);
    try {
      const raw = await fetchFluvioSocialGraph(signal);
      const next = twinPayloadFromNetwork(raw);
      graphRef.current = next;
      setGraph(next);
      setSelected((sel) => {
        if (!sel) return null;
        return next.nodes.some((n) => n.id === sel.id) ? sel : null;
      });
      setGraphHydrateState("ok");
    } catch (e) {
      if ((e as Error).name === "AbortError") return;
      const msg = e instanceof Error ? e.message : "Could not load graph.";
      setGraphHydrateState("error");
      setGraphHydrateMsg(msg);
      const cleared: TwinGraphPayload = { nodes: [], edges: [] };
      graphRef.current = cleared;
      setGraph(cleared);
      setSelected(null);
    }
  }, []);

  useEffect(() => {
    const ac = new AbortController();
    void reloadTwinNetwork(ac.signal);
    return () => ac.abort();
  }, [reloadTwinNetwork]);

  const onSelectNode = useCallback((id: string, label: string) => {
    setSelected((prev) => (prev?.id === id ? null : { id, label }));
  }, []);

  const runUserTurn = useCallback(async (userText: string) => {
    const graphContext = buildTwinGraphContext(graphRef.current, selectedRef.current);
    const userMsg: Msg = { id: rid(), role: "user", content: userText.trim() };
    const assistantId = rid();

    // Do not read history from inside setState — async callers can run the updater later,
    // leaving an empty array. Sync from ref + exclude empty assistant placeholders (Next
    // `/api/chat` rejects blank content).
    const priorForApi = messagesRef.current.filter(
      (m) => m.role === "user" || (m.role === "assistant" && m.content.trim().length > 0),
    );
    const apiMessages = [...priorForApi, userMsg];

    setMessages((prev) => [...prev, userMsg, { id: assistantId, role: "assistant", content: "" }]);

    setStreaming(true);
    setError(null);
    abortRef.current?.abort();
    abortRef.current = new AbortController();
    const signal = abortRef.current.signal;

    let acc = "";
    try {
      await streamTwinAssistant(
        apiMessages.map((m) => ({ role: m.role, content: m.content })),
        (delta) => {
          acc += delta;
          setMessages((prev) =>
            prev.map((m) => (m.id === assistantId ? { ...m, content: acc } : m)),
          );
        },
        signal,
        { graphContext },
      );
    } catch (e) {
      if ((e as Error).name === "AbortError") return;
      const msg = e instanceof Error ? e.message : "Something went wrong.";
      setError(msg);
      setMessages((prev) => prev.filter((m) => m.id !== assistantId && m.id !== userMsg.id));
    } finally {
      setStreaming(false);
    }
  }, []);

  /** Optional deep link: `/chat?topic=…` sends one auto turn, then strips the query. No default prompt on plain `/chat`. */
  useEffect(() => {
    const topic = topicAtMount.current;
    if (!topic) return;

    resetTwinChatBootstrap();
    if (!tryBeginTwinChatBootstrap()) return;
    let cancelled = false;

    const go = async () => {
      const decoded = decodeURIComponent(topic);
      router.replace("/chat", { scroll: false });
      if (cancelled) return;
      await runUserTurn(`Tell me about “${decoded}”.`);
    };

    void go();

    return () => {
      cancelled = true;
      abortRef.current?.abort();
      resetTwinChatBootstrap();
    };
  }, [router, runUserTurn]);

  const onSubmit = useCallback(() => {
    const t = input.trim();
    if (!t || streaming) return;
    setInput("");
    void runUserTurn(t);
  }, [input, streaming, runUserTurn]);

  return (
    <div className="relative flex min-h-dvh flex-col overscroll-none bg-[#0A0A0F] text-[#FFFFFF]">
      <GraphBackground />

      <header className="relative z-10 flex w-full shrink-0 items-center gap-1.5 border-b border-white/[0.06] px-2.5 py-2 pb-2.5 pt-[max(0.5rem,env(safe-area-inset-top))] sm:gap-2 sm:px-4">
        <Link
          href="/"
          className="flex min-h-11 min-w-11 shrink-0 items-center justify-center rounded-lg text-[13px] font-medium text-[#888780] transition hover:bg-white/[0.05] hover:text-white active:bg-white/[0.08] sm:min-h-10 sm:min-w-10 sm:text-[12px]"
        >
          Tap
        </Link>
        <div className="flex min-w-0 flex-[0.95] items-center justify-center gap-1.5 px-1 sm:flex-1 sm:gap-2">
          <FluvioTwinMark size={26} className="size-6 shrink-0 sm:size-[26px]" />
          <span className="truncate text-[10px] uppercase tracking-[0.14em] text-[#5F5E5A] sm:text-[11px]">Twin</span>
        </div>
        <nav
          aria-label="Twin navigation"
          className="scrollbar-none ml-auto flex max-w-[calc(100vw-9.5rem)] items-center gap-0.5 overflow-x-auto py-0.5 [scrollbar-width:none] sm:max-w-none sm:gap-1.5 [&::-webkit-scrollbar]:hidden"
        >
          <Link
            href="/dashboard"
            className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#888780] transition hover:bg-white/[0.05] hover:text-white active:bg-white/[0.08] sm:px-2 sm:py-1.5 sm:text-[12px]"
          >
            Dashboard
          </Link>
          <Link
            href="/graph"
            className={`whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] transition hover:bg-white/[0.05] active:bg-white/[0.08] sm:px-2 sm:py-1.5 sm:text-[12px] ${pathname === "/graph" ? "bg-white/[0.06] text-white" : "text-[#888780] hover:text-white"}`}
          >
            Graph
          </Link>
          <Link
            href="/chat"
            className={`whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] transition hover:bg-white/[0.05] active:bg-white/[0.08] sm:px-2 sm:py-1.5 sm:text-[12px] ${pathname === "/chat" ? "bg-white/[0.06] text-white" : "text-[#888780] hover:text-white"}`}
          >
            Chat
          </Link>
          <Link
            href="/product"
            className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#5F5E5A] transition hover:bg-white/[0.04] hover:text-[#888780] active:bg-white/[0.06] sm:px-2 sm:py-1.5 sm:text-[12px]"
          >
            Product
          </Link>
        </nav>
      </header>

      <div className="relative z-10 flex min-h-0 flex-1 flex-col divide-y divide-white/[0.06] lg:flex-row lg:divide-x lg:divide-y-0 lg:overflow-hidden">
        {/* Graph panel (order swaps on `/chat` for phones) */}
        <section
          className={`relative flex min-h-[min(260px,36vh)] flex-1 flex-col lg:min-h-0 lg:flex-1 ${
            chatFirstMobile ? "order-2 lg:order-none" : "order-1"
          }`}
        >
          <div className="pointer-events-none absolute left-2 top-2 z-10 flex max-w-[min(calc(100%-1rem),18rem)] flex-col gap-0.5 rounded-xl border border-white/[0.08] bg-[#0A0A0F]/90 px-2.5 py-2 shadow-lg backdrop-blur-md sm:left-3 sm:top-3 sm:max-w-[calc(100%-1.5rem)] sm:px-3">
            <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#534AB7]">Your graph</p>
            <p className="text-[11px] text-[#888780]">
              {graphHydrateState === "loading" ? (
                <span className="text-[#5F5E5A]">Loading from Twin…</span>
              ) : (
                <>
                  <span className="tabular-nums text-[#AFA9EC]">{graph.nodes.length}</span> nodes ·{" "}
                  <span className="tabular-nums text-[#AFA9EC]">{graph.edges.length}</span> edges
                </>
              )}
            </p>
            {graphHydrateState === "error" && graphHydrateMsg ? (
              <p className="break-words text-[10px] text-amber-400/95">{graphHydrateMsg}</p>
            ) : selected ? (
              <p className="truncate text-[10px] text-[#5F5E5A]">Focus: {selected.label}</p>
            ) : (
              <p className="text-[10px] text-[#5F5E5A]">
                Loaded from kg-engine — tap a node; chat sends this graph as context
              </p>
            )}
          </div>

          <div className="min-h-0 flex-1 lg:min-h-[200px]">
            <TwinD3Graph graph={graph} selectedId={selected?.id ?? null} onSelectNode={onSelectNode} />
          </div>

          <div className="shrink-0 border-t border-white/[0.06] bg-[#07060c]/95 px-2 py-3 backdrop-blur-sm sm:px-3">
            <div className="flex flex-wrap items-center justify-between gap-2 gap-y-2">
              <p className="text-[11px] leading-snug text-[#5F5E5A]">
                <span className="text-[#888780]">GET /twin/network</span> drives this viz (NFC taps). Chat pulls that
                view plus Documents from Dashboard ingest into kg-engine—not only what you tap here.
              </p>
              <button
                type="button"
                disabled={graphHydrateState === "loading"}
                onClick={() => void reloadTwinNetwork()}
                className="min-h-9 shrink-0 rounded-lg border border-white/[0.08] px-3 py-1.5 text-[11px] text-[#AFA9EC] transition hover:bg-white/[0.06] hover:text-white active:bg-white/[0.08] disabled:opacity-40"
              >
                Reload graph
              </button>
            </div>
          </div>
        </section>

        {/* Chat column */}
        <section
          className={`relative flex min-h-[min(300px,min(52dvh,55vh))] w-full flex-1 flex-col bg-[#0A0A0F]/80 lg:min-h-0 lg:w-[min(100%,440px)] lg:max-w-[46vw] lg:flex-none lg:flex-shrink-0 xl:w-[460px] ${
            chatFirstMobile ? "order-1 lg:order-none" : "order-2"
          }`}
        >
          <div className="border-b border-white/[0.06] px-3 py-2.5 sm:px-4">
            <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#534AB7]/90">Ask the twin</p>
              <p className="mt-0.5 max-w-[min(100%,40ch)] text-pretty text-[13px] leading-snug text-[#888780] sm:max-w-none sm:text-[12px]">
              Profile plus this NFC graph snapshot, merged with Dashboard · Personal graph ingests (PDF, video,
              code, Gmail) — same kg-engine workspace as Map.
            </p>
          </div>
          <div
            ref={chatScrollRef}
            className="flex min-h-0 flex-1 flex-col overflow-y-auto overflow-x-hidden px-3 py-3 sm:px-4"
          >
            <div className="mt-auto flex w-full min-w-0 flex-col gap-3">
              <AnimatePresence initial={false}>
                {messages.map((m) =>
                  m.role === "assistant" && m.content === "" && streaming ? (
                    <div key={m.id} className="flex justify-start">
                      <LoadingDots />
                    </div>
                  ) : (
                    <ChatMessage key={m.id} role={m.role} content={m.content} />
                  ),
                )}
              </AnimatePresence>
              {error ? (
                <p className="text-center text-[13px] text-red-400/90" role="alert">
                  {error}
                </p>
              ) : null}
            </div>
          </div>
          <InputBar
            value={input}
            onChange={setInput}
            onSubmit={onSubmit}
            disabled={streaming}
            placeholder="Ask about your graph, a person, or who to reach…"
          />
        </section>
      </div>
    </div>
  );
}
