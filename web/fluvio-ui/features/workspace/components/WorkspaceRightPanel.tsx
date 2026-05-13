"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { getKgEngineUrl } from "@/shared/lib/constants";
import { kgBearerHeaders } from "@/shared/lib/kgBearer";
import type { BrainTab, ChatMessage } from "@/shared/lib/types";

const OFFLINE_REPLY =
  "This tab does not have live graph context yet (or kg-engine returned no rows). Add PDFs, sync mail, clone a repo, or pick Unified when your workspace graph has nodes—then chat uses POST /chat on kg-engine.";

type Props = {
  dock?: boolean;
  domainKey: string;
  canUseLiveChat: boolean;
  brainTab: BrainTab;
  graphEmpty: boolean;
  nodeCount: number;
  chatPrefill: string | null;
  onConsumeChatPrefill: () => void;
  codebaseFocusPath?: string | null;
};

export function WorkspaceRightPanel({
  dock = false,
  domainKey,
  canUseLiveChat,
  brainTab,
  graphEmpty,
  nodeCount,
  chatPrefill,
  onConsumeChatPrefill,
  codebaseFocusPath = null,
}: Props) {
  const messagesRef = useRef<HTMLDivElement>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const copyFeedbackTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(true);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [copiedAssistantIdx, setCopiedAssistantIdx] = useState<number | null>(null);

  useEffect(() => {
    if (messagesRef.current) {
      messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
    }
  }, [messages, loading]);

  useEffect(() => {
    setMessages([]);
    setInput("");
    setCopiedAssistantIdx(null);
  }, [domainKey]);

  useEffect(() => {
    if (!chatPrefill) return;
    setOpen(true);
    setInput(chatPrefill);
    onConsumeChatPrefill();
  }, [chatPrefill, onConsumeChatPrefill]);

  useEffect(() => {
    const el = composerRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 220)}px`;
  }, [input]);

  useEffect(() => {
    return () => {
      if (copyFeedbackTimerRef.current) clearTimeout(copyFeedbackTimerRef.current);
    };
  }, []);

  const copyAssistantResponse = useCallback(async (text: string, messageIndex: number) => {
    if (!text) return;
    try {
      if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const ta = document.createElement("textarea");
        ta.value = text;
        ta.setAttribute("readonly", "");
        ta.style.position = "fixed";
        ta.style.left = "-9999px";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
      }
      if (copyFeedbackTimerRef.current) clearTimeout(copyFeedbackTimerRef.current);
      setCopiedAssistantIdx(messageIndex);
      copyFeedbackTimerRef.current = setTimeout(() => {
        setCopiedAssistantIdx(null);
        copyFeedbackTimerRef.current = null;
      }, 2000);
    } catch {
      /* ignore */
    }
  }, []);

  const sendMessage = useCallback(async () => {
    if (!input.trim() || loading) return;
    const question = input.trim();
    setInput("");
    setMessages((m) => [...m, { role: "user", content: question }]);
    setLoading(true);

    try {
      if (!canUseLiveChat || graphEmpty) {
        setMessages((m) => [...m, { role: "assistant", content: OFFLINE_REPLY }]);
        setLoading(false);
        return;
      }

      const body: Record<string, unknown> = { question, history: messages };
      if (brainTab === "github" && codebaseFocusPath?.trim()) {
        body.focus_path = codebaseFocusPath.trim();
      }
      const res = await fetch(`${getKgEngineUrl()}/chat`, {
        method: "POST",
        headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
        body: JSON.stringify(body),
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
  }, [input, loading, messages, graphEmpty, canUseLiveChat, brainTab, codebaseFocusPath]);

  const statusLine = (
    <>
      {brainTab === "unified" && (
        <>
          Unified · workspace graph · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes
        </>
      )}
      {brainTab === "meta" && (
        <>
          Meta · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes (no backend view yet)
        </>
      )}
      {brainTab === "github" && (
        <>
          GitHub · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes
        </>
      )}
      {brainTab !== "unified" && brainTab !== "meta" && brainTab !== "github" && canUseLiveChat && !graphEmpty && (
        <>
          Live graph · <span className="tabular-nums text-zinc-200">{nodeCount}</span> nodes · {brainTab}
        </>
      )}
      {brainTab !== "unified" &&
        brainTab !== "meta" &&
        brainTab !== "github" &&
        (!canUseLiveChat || graphEmpty) && (
        <>
          No graph · <span className="text-zinc-300">{brainTab}</span>
        </>
      )}
    </>
  );

  const panelInner = (
    <div
      className={`flex min-h-0 flex-1 flex-col overflow-hidden border-white/[0.08] bg-zinc-950/90 backdrop-blur-2xl ${
        dock
          ? "h-full border-l"
          : "max-h-[min(560px,calc(100vh-5rem))] w-[min(100vw-1.5rem,380px)] rounded-2xl border shadow-2xl shadow-black/50"
      }`}
      onClick={(e) => e.stopPropagation()}
    >
      <div className="border-b border-white/[0.06] bg-zinc-900/40 px-3 py-2.5">
        <p className="text-[12px] font-medium leading-snug text-zinc-400">{statusLine}</p>
      </div>

      <div ref={messagesRef} className="min-h-[200px] flex-1 space-y-3 overflow-y-auto p-3 select-text">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-3 py-12 text-center">
            <span className="text-2xl text-zinc-600">◇</span>
            <p className="max-w-[240px] px-2 text-[13px] leading-relaxed text-zinc-500">
              Ask questions about the graph when kg-engine has loaded data for this tab.
            </p>
          </div>
        )}
        {messages.map((m, i) =>
          m.role === "assistant" ? (
            <div
              key={i}
              className="mr-3 select-text rounded-2xl border border-white/[0.06] bg-zinc-900/80 px-3.5 pb-2.5 pt-2 text-[13px] leading-relaxed text-zinc-300"
            >
              <div className="mb-1.5 flex justify-end">
                <button
                  type="button"
                  aria-label="Copy assistant reply"
                  onClick={() => void copyAssistantResponse(m.content, i)}
                  className="rounded-lg px-2 py-0.5 text-[11px] font-medium text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-300"
                >
                  {copiedAssistantIdx === i ? "Copied" : "Copy"}
                </button>
              </div>
              <div className="break-words whitespace-pre-wrap">{m.content}</div>
            </div>
          ) : (
            <div
              key={i}
              className="ml-5 rounded-2xl border border-sky-500/20 bg-sky-500/10 px-3.5 py-2.5 text-[13px] leading-relaxed break-words text-zinc-100 select-text"
            >
              {m.content}
            </div>
          ),
        )}
        {loading && (
          <div className="mr-3 rounded-2xl border border-white/[0.06] bg-zinc-900/60 px-3.5 py-2.5 text-[13px] text-zinc-500">
            <span className="animate-pulse">Thinking</span>
            <span className="animate-bounce">…</span>
          </div>
        )}
      </div>
      <div className="flex gap-2 border-t border-white/[0.06] p-3">
        <textarea
          ref={composerRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void sendMessage();
            }
          }}
          rows={1}
          placeholder={
            brainTab === "unified"
              ? "Message about the full workspace graph…"
              : brainTab === "meta"
                ? "Message (uses workspace graph when loaded)…"
                : brainTab === "github"
                  ? "Ask about the repo graph…"
                  : canUseLiveChat && !graphEmpty
                    ? "Message the graph…"
                    : `No data for ${brainTab} yet`
          }
          className="max-h-[220px] min-h-[42px] flex-1 resize-none overflow-y-auto rounded-xl border border-white/[0.08] bg-zinc-900/80 px-3 py-2.5 text-[13px] leading-relaxed text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-sky-500/40 focus:ring-1 focus:ring-sky-500/20"
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
    </div>
  );

  if (dock) {
    return (
      <div className="flex h-full min-h-0 w-[min(100%,380px)] shrink-0 flex-col border-l border-white/[0.06] bg-zinc-950/50">
        <div className="flex min-h-0 flex-1 flex-col px-1 pt-2">{panelInner}</div>
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
