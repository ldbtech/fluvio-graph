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
import { AuthedProfileHeader } from "@/app/components/AuthedProfileHeader";
import {
  fetchPeerGraphStatus,
  fetchFluvioSocialGraph,
  getTwinUserId,
  type PeerGraphStatus,
  type FluvioGraphPayload,
} from "@/shared/lib/fluvioDashboardApi";
import { resetTwinChatBootstrap, tryBeginTwinChatBootstrap } from "@/shared/lib/twinChatSession";
import { streamTwinAssistant } from "@/shared/lib/twinChatStream";
import {
  buildTwinGraphContext,
  inferPeerOwnerIdFromMessage,
  type TwinGraphPayload,
} from "@/shared/lib/twinGraphStore";

type Msg = { id: string; role: "user" | "assistant"; content: string };

function rid() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

const EMPTY_TWIN_GRAPH: TwinGraphPayload = { nodes: [], edges: [] };

/** When a connection node is selected, kg-engine loads that user's Surreal graph (`graph_owner_id`). */
function peerGraphOwnerId(
  selected: { id: string; label: string } | null,
  selfId: string | null | undefined,
): string | undefined {
  const self = selfId?.trim().toLowerCase();
  if (!selected?.id?.trim() || !self) return undefined;
  const id = selected.id.trim();
  if (id.toLowerCase() === self) return undefined;
  const uuid =
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  if (!uuid.test(id)) return undefined;
  return id;
}

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
  const [peerStatus, setPeerStatus] = useState<PeerGraphStatus | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const graphRef = useRef(graph);
  const selectedRef = useRef(selected);
  const messagesRef = useRef<Msg[]>(messages);
  const chatScrollRef = useRef<HTMLDivElement>(null);
  /** Set after mount so SSR / first paint match (no `localStorage` on server). */
  const [sessionUserId, setSessionUserId] = useState<string | null>(null);
  graphRef.current = graph;
  selectedRef.current = selected;
  messagesRef.current = messages;

  useEffect(() => {
    setSessionUserId(getTwinUserId());
  }, []);

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

  useEffect(() => {
    const self = getTwinUserId()?.trim().toLowerCase();
    const id = selected?.id?.trim();
    if (!id || !self || id.toLowerCase() === self) {
      setPeerStatus(null);
      return;
    }
    const ac = new AbortController();
    void (async () => {
      try {
        const s = await fetchPeerGraphStatus(id, ac.signal);
        setPeerStatus(s);
      } catch {
        setPeerStatus(null);
      }
    })();
    return () => ac.abort();
  }, [selected?.id]);

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

    const selfId = getTwinUserId();
    const fromTap = peerGraphOwnerId(selectedRef.current, selfId);
    const fromName =
      fromTap ? undefined : inferPeerOwnerIdFromMessage(userMsg.content, graphRef.current, selfId);
    const graphOwnerId = fromTap ?? fromName;

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
        {
          graphContext,
          ...(graphOwnerId ? { graphOwnerId } : {}),
        },
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

  /** Optional deep link: `?topic=…` sends one auto turn, then strips the query (canonical path `/graph`). */
  useEffect(() => {
    const topic = topicAtMount.current;
    if (!topic) return;

    resetTwinChatBootstrap();
    if (!tryBeginTwinChatBootstrap()) return;
    let cancelled = false;

    const go = async () => {
      const decoded = decodeURIComponent(topic);
      router.replace("/graph", { scroll: false });
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

  /** Split remaining viewport: emphasize chat on `/chat`, graph on `/graph`. */
  const chatPanelGrow = chatFirstMobile ? "flex-[1.35]" : "flex-1";
  const graphPanelGrow = chatFirstMobile ? "flex-1" : "flex-[1.35]";
  const knowledgeAboutPeer = peerGraphOwnerId(selected, sessionUserId);

  return (
    <div className="relative flex h-[100dvh] max-h-[100dvh] flex-col overflow-hidden overscroll-none bg-[#080712] text-white">
      <GraphBackground />

      <header className="relative z-20 flex w-full shrink-0 items-center gap-1.5 border-b border-white/[0.07] bg-[#080712]/90 px-2.5 py-2 pb-2.5 pt-[max(0.5rem,env(safe-area-inset-top))] shadow-[0_1px_0_rgba(255,255,255,0.04)] backdrop-blur-md sm:gap-2 sm:px-4">
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
            className={`whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] transition hover:bg-white/[0.05] active:bg-white/[0.08] sm:px-2 sm:py-1.5 sm:text-[12px] ${pathname === "/graph" || pathname === "/chat" ? "bg-white/[0.06] text-white" : "text-[#888780] hover:text-white"}`}
          >
            My Network
          </Link>
        </nav>
      </header>

      <div className="relative z-20 shrink-0 border-b border-white/[0.07] bg-[#080712]/95">
        <AuthedProfileHeader className="mx-auto w-full max-w-5xl px-2.5 py-2 sm:px-4" />
      </div>

      <div className="relative z-10 flex min-h-0 flex-1 flex-col overflow-hidden lg:flex-row">
        {/* Graph panel (order swaps on `/chat` for phones) */}
        <section
          className={`relative flex min-h-0 ${graphPanelGrow} flex-col overflow-hidden lg:border-r lg:border-white/[0.08] ${
            chatFirstMobile ? "order-2 lg:order-none" : "order-1"
          }`}
        >
          <div className="pointer-events-none absolute left-2 top-2 z-10 flex max-w-[min(calc(100%-1rem),18rem)] flex-col gap-0.5 rounded-xl border border-white/[0.08] bg-[#0A0A0F]/90 px-2.5 py-2 shadow-lg backdrop-blur-md sm:left-3 sm:top-3 sm:max-w-[calc(100%-1.5rem)] sm:px-3">
            <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#534AB7]">Your network</p>
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
              <p className="truncate text-[10px] text-[#5F5E5A]">
                Focus: {selected.label}
                {knowledgeAboutPeer ? (
                  <span className="ml-1 text-[#AFA9EC]">· chat uses their ingests</span>
                ) : null}
              </p>
            ) : (
              <p className="text-[10px] text-[#5F5E5A]">
                Tap a connection — chat loads their Surreal-backed uploads (zone you share on NFC).
              </p>
            )}
          </div>

          <div className="min-h-0 flex-1 lg:min-h-[200px]">
            <TwinD3Graph graph={graph} selectedId={selected?.id ?? null} onSelectNode={onSelectNode} />
          </div>

          <div className="shrink-0 border-t border-white/[0.06] bg-[#07060c]/95 px-2 py-3 backdrop-blur-sm sm:px-3">
            <div className="flex flex-wrap items-center justify-between gap-2 gap-y-2">
              <p className="text-[11px] leading-snug text-[#5F5E5A]">
                <span className="text-[#888780]">GET /twin/network</span> here. Select someone (not you) so{" "}
                <span className="font-mono text-[#5F5E5A]">/twin/chat</span> sends{" "}
                <span className="font-mono text-[#5F5E5A]">graph_owner_id</span> and loads their resume/notes from
                SurrealDB.
              </p>
              <button
                type="button"
                disabled={graphHydrateState === "loading"}
                onClick={() => void reloadTwinNetwork()}
                className="min-h-9 shrink-0 rounded-lg border border-white/[0.08] px-3 py-1.5 text-[11px] text-[#AFA9EC] transition hover:bg-white/[0.06] hover:text-white active:bg-white/[0.08] disabled:opacity-40"
              >
                Reload network
              </button>
            </div>
          </div>
        </section>

        {/* Chat column — fills share of viewport; only the thread below scrolls */}
        <section
          className={`relative flex min-h-0 w-full ${chatPanelGrow} flex-col overflow-hidden bg-[#0c0b14]/95 lg:w-[min(100%,420px)] lg:max-w-[44vw] lg:flex-none lg:shadow-[-12px_0_40px_-32px_rgba(0,0,0,0.9)] xl:w-[440px] ${
            chatFirstMobile
              ? "order-1 border-b border-white/[0.08] lg:order-none lg:border-b-0"
              : "order-2 border-t border-white/[0.08] lg:border-t-0"
          }`}
        >
          <div className="shrink-0 border-b border-white/[0.07] bg-[#0c0b14]/90 px-3 py-3 sm:px-4">
            <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[#AFA9EC]">Ask the twin</p>
            {knowledgeAboutPeer && selected ? (
              <p className="mt-2 rounded-lg border border-violet-500/25 bg-violet-500/[0.08] px-3 py-2 text-[12px] leading-snug text-violet-100/95">
                Grounding answers in <span className="font-medium text-white">{selected.label}</span>&apos;s shared
                graph (not only your uploads).
              </p>
            ) : (
              <p className="mt-1.5 text-[12px] leading-relaxed text-zinc-500 sm:text-[13px]">
                {sessionUserId
                  ? "Select a connection on the graph to ask about them — we load their Surreal ingests. Otherwise answers use your account only."
                  : "Sign in on Dashboard first. Then select someone on the graph to load their shared knowledge."}
              </p>
            )}
            {knowledgeAboutPeer && peerStatus ? (
              <p className="mt-2 rounded-lg border border-white/[0.1] bg-black/30 px-3 py-2 text-[11px] leading-snug text-zinc-400">
                {peerStatus.peer_name} · zone {peerStatus.zone ?? "?"} · Surreal rows visible:{" "}
                <span className="font-mono text-zinc-300">{peerStatus.surreal_rows_in_zone}</span>
                {" "}({peerStatus.surreal_workspace_rows} workspace) · uploads:{" "}
                <span className="font-mono text-zinc-300">{peerStatus.pg_user_upload_rows}</span>
              </p>
            ) : null}
          </div>
          <div
            ref={chatScrollRef}
            className="flex min-h-0 flex-1 flex-col overflow-y-auto overflow-x-hidden overscroll-y-contain px-3 py-3 [-webkit-overflow-scrolling:touch] sm:px-4"
          >
            <div className="mt-auto flex w-full min-w-0 flex-col gap-3 pb-1">
              {messages.length === 0 && !streaming ? (
                <p className="rounded-xl border border-dashed border-white/[0.1] bg-white/[0.02] px-4 py-8 text-center text-[13px] leading-relaxed text-zinc-500">
                  {knowledgeAboutPeer && selected ? (
                    <>
                      Asking about <span className="text-zinc-300">{selected.label}</span>. Their resume and notes show up
                      here once they&apos;ve uploaded on the Dashboard and Surreal has synced.
                    </>
                  ) : (
                    <>
                      Tap <span className="text-zinc-400">them</span> on the graph above (not your own node) so each
                      message sends their user id to the engine and loads their materials.
                    </>
                  )}
                </p>
              ) : null}
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
