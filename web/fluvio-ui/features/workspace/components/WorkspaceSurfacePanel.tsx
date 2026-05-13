"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import type { CodebaseCloneResult, ConnectorId, WorkspaceSurface } from "@/shared/lib/types";
import { postCodebaseClone } from "@/shared/lib/fetchCodebaseClone";
import { kgBearerHeaders } from "@/shared/lib/kgBearer";
import { GmailReplyAgentPanel } from "@/shared/components/GmailReplyAgentPanel";

type OAuthPhase = "form" | "busy" | "done";

type Props = {
  surface: WorkspaceSurface;
  onClose: () => void;
  pdfInputId: string;
  kgUrl: string;
  graphNodes: number;
  graphEdges: number;
  onOAuthPreviewComplete: (id: ConnectorId) => void;
  /** After Gmail sync succeeds — parent can refresh workspace UI (counts, lists). */
  onGmailGraphRefresh?: () => void | Promise<void>;
  /** After GitHub repo is accepted — parent enables GitHub brain context. */
  onGithubPublicCloneSuccess?: (result: CodebaseCloneResult) => void;
  /** After GitHub ingest starts — parent can refresh workspace state. */
  onGithubCloneSessionStart?: () => void | Promise<void>;
};

function parseGithubRepoInput(input: string): { owner: string; repo: string } | null {
  const trimmed = input.trim().replace(/\.git$/i, "");
  if (!trimmed) return null;
  if (/^https?:\/\//i.test(trimmed)) {
    try {
      const u = new URL(trimmed);
      const parts = u.pathname.split("/").filter(Boolean);
      if (parts.length >= 2) {
        return { owner: parts[0], repo: parts[1] };
      }
      return null;
    } catch {
      return null;
    }
  }
  const parts = trimmed.split("/").filter(Boolean);
  if (parts.length >= 2) {
    return { owner: parts[0], repo: parts[1] };
  }
  return null;
}

function RustFootnote({ lines }: { lines: string[] }) {
  return (
    <div className="mt-6 overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.02] p-4">
      <p className="mb-2 text-[11px] font-semibold text-zinc-500">Implementation notes</p>
      <ul className="space-y-1.5 font-mono text-[11px] leading-relaxed text-zinc-500">
        {lines.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}

function PreviewBanner() {
  return (
    <div className="mb-4 rounded-xl border border-amber-500/20 bg-amber-500/[0.06] px-3 py-2.5 text-[12px] leading-relaxed text-amber-100/90">
      <span className="font-medium text-amber-50/95">Preview</span> — controls do not call real providers yet. Wire to
      your OAuth routes and ingestion workers in Rust.
    </div>
  );
}

type WorkspaceGmailRecentRow = {
  id: string;
  thread_id: string;
  snippet?: string | null;
  subject?: string | null;
  from?: string | null;
  date_header?: string | null;
  internal_date_ms?: number | null;
  is_new?: boolean | null;
};

function gmailSenderInitial(fromRaw: string | null | undefined): string {
  const from = fromRaw?.trim() ?? "";
  const angle = from.indexOf("<");
  let label = angle >= 0 ? from.slice(0, angle).replace(/"/g, "").trim() : from;
  if (!label && angle >= 0) {
    const rest = from.slice(angle + 1).replace(">", "").trim();
    label = rest.split("@")[0] ?? rest;
  }
  const ch = /^[\w]/u.exec(label)?.[0] ?? /^[^\s@]/u.exec(from)?.[0];
  return (ch ?? "?").toUpperCase().slice(0, 1);
}

function gmailInboxTime(row: WorkspaceGmailRecentRow): string {
  if (row.date_header?.trim()) return row.date_header.trim();
  if (typeof row.internal_date_ms === "number") {
    return new Intl.DateTimeFormat(undefined, {
      month:       "short",
      day:         "numeric",
      hour:        "numeric",
      minute:      "2-digit",
      hourCycle: "h12",
    }).format(new Date(row.internal_date_ms));
  }
  return "";
}

/** Gmail OAuth + inbox preview + optional sender focus list (~25s poll; server uses History for “new”). */
function GmailKgEngineConnect({
  kgUrl,
  onOAuthPreviewComplete,
  onGraphRefresh,
}: {
  kgUrl: string;
  onOAuthPreviewComplete: (id: ConnectorId) => void;
  onGraphRefresh?: () => void | Promise<void>;
}) {
  const [connected, setConnected] = useState<boolean | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [recent, setRecent] = useState<WorkspaceGmailRecentRow[] | null>(null);
  const [recentBusy, setRecentBusy] = useState(false);
  const [focusDraft, setFocusDraft] = useState("");
  const [focusBusy, setFocusBusy] = useState(false);

  const loadStatus = async () => {
    try {
      const r = await fetch(`${kgUrl}/connect/gmail/status`, { headers: kgBearerHeaders() });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const j = (await r.json()) as { connected: boolean };
      setConnected(j.connected);
      if (j.connected) onOAuthPreviewComplete("gmail");
    } catch {
      setConnected(false);
    }
  };

  const loadRecentSilent = async (signal?: AbortSignal) => {
    if (!connected) return;
    try {
      const r = await fetch(`${kgUrl}/gmail/recent?limit=10`, {
        headers: kgBearerHeaders(),
        signal,
      });
      const text = await r.text();
      if (!r.ok) return;
      const rows = JSON.parse(text) as WorkspaceGmailRecentRow[];
      setRecent(Array.isArray(rows) ? rows : []);
      await onGraphRefresh?.();
    } catch {
      /* aborted or transient — keep snapshot */
    }
  };

  useEffect(() => {
    void loadStatus();
  }, [kgUrl]);

  useEffect(() => {
    if (!connected) {
      setRecent(null);
      return;
    }
    const ac = new AbortController();
    void loadRecentSilent(ac.signal);
    const id = window.setInterval(() => void loadRecentSilent(), 25_000);
    return () => {
      ac.abort();
      window.clearInterval(id);
    };
  }, [connected, kgUrl]);

  useEffect(() => {
    if (!connected) {
      setFocusDraft("");
      return;
    }
    void (async () => {
      try {
        const r = await fetch(`${kgUrl}/gmail/focus`, { headers: kgBearerHeaders() });
        const text = await r.text();
        if (!r.ok) return;
        const j = JSON.parse(text) as { senders?: string[] };
        if (Array.isArray(j.senders)) setFocusDraft(j.senders.join("\n"));
      } catch {
        /* ignore */
      }
    })();
  }, [connected, kgUrl]);

  const saveFocus = () => {
    setErr(null);
    setFocusBusy(true);
    void (async () => {
      try {
        const senders = focusDraft
          .split(/[\n,]+/)
          .map((s) => s.trim())
          .filter(Boolean);
        const r = await fetch(`${kgUrl}/gmail/focus`, {
          method:  "PUT",
          headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
          body:    JSON.stringify({ senders }),
        });
        const text = await r.text();
        if (!r.ok) throw new Error(text ? text.slice(0, 260) : `HTTP ${r.status}`);
        const j = JSON.parse(text) as { senders?: string[] };
        if (Array.isArray(j.senders)) setFocusDraft(j.senders.join("\n"));
        await loadRecentSilent();
        await onGraphRefresh?.();
      } catch (e: unknown) {
        setErr(e instanceof Error ? e.message : String(e));
      } finally {
        setFocusBusy(false);
      }
    })();
  };

  const startOAuth = (opts?: { forceConsent?: boolean }) => {
    setErr(null);
    void (async () => {
      try {
        const r = await fetch(`${kgUrl}/connect/gmail/start`, {
          method:  "POST",
          headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
          body:    JSON.stringify({ force_consent: !!opts?.forceConsent }),
        });
        const text = await r.text();
        if (!r.ok) {
          throw new Error(text ? text.slice(0, 280) : `OAuth start HTTP ${r.status}`);
        }
        let j: { url?: string };
        try {
          j = JSON.parse(text) as { url?: string };
        } catch {
          throw new Error("Invalid OAuth start response");
        }
        if (!j.url) throw new Error("Missing OAuth url");
        window.location.href = j.url;
      } catch (e: unknown) {
        setErr(e instanceof Error ? e.message : String(e));
      }
    })();
  };

  const refreshInboxManual = () => {
    setErr(null);
    setRecentBusy(true);
    void (async () => {
      try {
        const r = await fetch(`${kgUrl}/gmail/recent?limit=10`, { headers: kgBearerHeaders() });
        const text = await r.text();
        if (!r.ok) throw new Error(text ? text.slice(0, 260) : `HTTP ${r.status}`);
        const rows = JSON.parse(text) as WorkspaceGmailRecentRow[];
        setRecent(Array.isArray(rows) ? rows : []);
        await onGraphRefresh?.();
      } catch (e: unknown) {
        setErr(e instanceof Error ? e.message : String(e));
      } finally {
        setRecentBusy(false);
      }
    })();
  };

  const connectedPulse =
    connected === null ? (
      <span className="inline-flex h-2 w-2 rounded-full bg-zinc-500" aria-hidden />
    ) : connected ? (
      <span className="inline-flex h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_12px_rgba(52,211,153,0.45)]" aria-hidden />
    ) : (
      <span className="inline-flex h-2 w-2 rounded-full bg-zinc-600" aria-hidden />
    );

  return (
    <div className="mx-auto max-w-xl">
      {/* Footnote strip — restrained, avoids “banner” clutter */}
      <p className="mb-6 text-center text-[12px] leading-relaxed tracking-wide text-zinc-500">
        Inbox previews from{" "}
        <code className="rounded-md bg-white/[0.04] px-1.5 py-0.5 font-mono text-[11px] text-zinc-400" suppressHydrationWarning>
          {kgUrl}
        </code>
        <span className="mx-2 text-zinc-700">·</span>
        refreshes ~25s
      </p>

      <div className="overflow-hidden rounded-[28px] border border-white/[0.055] bg-gradient-to-b from-zinc-900/92 to-zinc-950 backdrop-blur-2xl shadow-[0_28px_80px_-34px_rgba(0,0,0,0.85)] ring-1 ring-white/[0.04]">
        {/* Header */}
        <div className="border-b border-white/[0.05] px-6 pb-6 pt-7">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div className="flex min-w-0 flex-1 items-start gap-4">
              <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-white/[0.12] to-white/[0.04] ring-1 ring-white/[0.08]">
                <svg aria-hidden className="h-7 w-7 text-zinc-100" fill="none" viewBox="0 0 24 24">
                  <path
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={1.35}
                    d="M4 7.5 12 13l8-5.5M6.5 6h11A2 2 0 0 1 19.5 8v8a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z"
                  />
                </svg>
              </div>
              <div className="min-w-0">
                <p className="text-[28px] font-semibold tracking-[-0.04em] text-zinc-50">Email</p>
                <p className="mt-2 max-w-md text-[15px] font-normal leading-[1.45] tracking-[-0.01em] text-zinc-500">
                  Polls Gmail for new messages only in your inbox. Narrow with optional focus addresses; Claude drafts replies from your workspace graph — review or send automatically.
                </p>
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2 rounded-full border border-white/[0.06] bg-white/[0.03] px-3.5 py-2 ring-1 ring-white/[0.03]">
              {connectedPulse}
              <span className="text-[13px] font-medium tracking-[-0.01em] text-zinc-300">
                {connected === null ? "Checking…" : connected ? "Connected" : "Not connected"}
              </span>
            </div>
          </div>

          {connected && recent === null ? (
            <p className="mt-5 text-[14px] text-zinc-500">Synchronizing inbox…</p>
          ) : null}
          {err ? (
            <p className="mt-5 rounded-2xl border border-red-500/20 bg-red-950/25 px-4 py-3 text-[13px] leading-snug text-red-100/90">
              {err}
            </p>
          ) : null}
        </div>

        {/* Actions */}
        <div className="space-y-3 px-6 py-6">
          <button
            type="button"
            onClick={() => startOAuth()}
            className="h-14 w-full rounded-full bg-white text-[16px] font-semibold tracking-[-0.02em] text-zinc-950 shadow-[inset_0_-1px_0_rgba(0,0,0,0.08)] transition hover:bg-zinc-100 active:scale-[0.992]"
          >
            {connected ? "Reconnect Gmail" : "Connect Gmail"}
          </button>

          {connected ? (
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-center sm:gap-3">
              <button
                type="button"
                onClick={() => startOAuth({ forceConsent: true })}
                className="h-11 flex-1 rounded-full border border-white/[0.1] bg-transparent text-[13px] font-medium text-zinc-400 transition hover:border-white/[0.14] hover:bg-white/[0.04] hover:text-zinc-200"
              >
                Renew OAuth consent
              </button>
              <button
                type="button"
                onClick={refreshInboxManual}
                disabled={recentBusy}
                className="h-11 flex-1 rounded-full border border-white/[0.1] bg-white/[0.04] text-[13px] font-semibold tracking-[-0.01em] text-zinc-100 transition hover:bg-white/[0.07] disabled:opacity-45"
              >
                {recentBusy ? "Refreshing…" : "Refresh inbox"}
              </button>
            </div>
          ) : null}
        </div>

        {connected ? (
          <>
            {/* Inbox — feed-first (X-like density + Apple typography) */}
            <div className="border-y border-white/[0.045] bg-black/[0.12] px-6 py-8">
              <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
                <h2 className="text-[22px] font-semibold tracking-[-0.03em] text-zinc-50">Latest in inbox</h2>
                <p className="text-[13px] text-zinc-500">Poll + history deltas · “New” = unseen since sync</p>
              </div>
              {recent && recent.length > 0 ? (
                <ul className="-mx-1 max-h-[min(420px,calc(100vh-12rem))] divide-y divide-white/[0.05] overflow-y-auto rounded-2xl border border-white/[0.045] bg-zinc-950/40 shadow-inner">
                  {recent.map((m) => (
                    <li key={m.id} className="group flex gap-4 px-4 py-[18px] transition hover:bg-white/[0.02]">
                      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-zinc-800/90 text-[14px] font-semibold uppercase tracking-[0.02em] text-zinc-300 ring-1 ring-white/[0.06]">
                        {gmailSenderInitial(m.from)}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
                          <p className="truncate text-[16px] font-semibold tracking-[-0.025em] text-zinc-100">
                            {(m.subject && m.subject.trim()) || "No subject"}
                          </p>
                          {m.is_new ? (
                            <span className="shrink-0 rounded-full bg-sky-500/15 px-2 py-px text-[11px] font-semibold lowercase tracking-normal text-[#38bdf8] ring-1 ring-sky-500/25">
                              new
                            </span>
                          ) : null}
                        </div>
                        {m.from?.trim() ? (
                          <p className="mt-1 truncate text-[14px] text-zinc-500">{m.from.trim()}</p>
                        ) : null}
                        {m.snippet?.trim() ? (
                          <p className="mt-2 line-clamp-2 text-[14px] leading-relaxed tracking-[-0.01em] text-zinc-500">
                            {m.snippet.trim()}
                          </p>
                        ) : null}
                      </div>
                      {gmailInboxTime(m) ? (
                        <div className="hidden shrink-0 text-right text-[13px] text-zinc-600 sm:block">
                          {gmailInboxTime(m)}
                        </div>
                      ) : null}
                    </li>
                  ))}
                </ul>
              ) : recent && recent.length === 0 ? (
                <p className="rounded-2xl border border-white/[0.05] bg-zinc-950/35 px-5 py-10 text-center text-[15px] text-zinc-500">
                  No messages in view. Relax focus filters or refresh.
                </p>
              ) : null}
            </div>

            {/* Focus — grouped field (Apple settings style) */}
            <div className="px-6 py-8">
              <p className="text-[22px] font-semibold tracking-[-0.03em] text-zinc-50">Focus senders</p>
              <p className="mt-2 max-w-lg text-[15px] leading-[1.5] tracking-[-0.01em] text-zinc-500">
                One address or domain per line. Empty means the full inbox. Examples:{" "}
                <span className="rounded-md bg-white/[0.05] px-1 py-px font-mono text-[13px] text-zinc-400">you@acme.co</span>{" "}
                · <span className="rounded-md bg-white/[0.05] px-1 py-px font-mono text-[13px] text-zinc-400">@vendor.com</span>
              </p>
              <div className="mt-6 overflow-hidden rounded-[20px] border border-white/[0.055] bg-zinc-950/50 ring-1 ring-white/[0.03]">
                <textarea
                  value={focusDraft}
                  onChange={(e) => setFocusDraft(e.target.value)}
                  disabled={focusBusy}
                  rows={5}
                  className="w-full resize-y bg-transparent px-4 py-4 font-mono text-[13px] leading-relaxed text-zinc-200 outline-none placeholder:text-zinc-600"
                  spellCheck={false}
                  placeholder={`you@client.com\n@company.com`}
                />
                <div className="border-t border-white/[0.045] px-4 py-3">
                  <button
                    type="button"
                    onClick={saveFocus}
                    disabled={focusBusy}
                    className="h-11 min-w-[132px] rounded-full bg-white/[0.08] px-5 text-[14px] font-semibold tracking-[-0.015em] text-zinc-100 transition hover:bg-white/[0.12] disabled:opacity-45"
                  >
                    {focusBusy ? "Saving…" : "Save"}
                  </button>
                </div>
              </div>
            </div>

            <div className="border-t border-white/[0.045] px-6 py-8">
              <GmailReplyAgentPanel
                kgEngineBaseUrl={kgUrl}
                bearerHeaders={() => kgBearerHeaders()}
                jsonHeaders={() => ({ ...kgBearerHeaders(), "Content-Type": "application/json" })}
                disabled={recentBusy || focusBusy}
                onBanner={(msg) => (msg ? setErr(msg) : setErr(null))}
              />
            </div>
          </>
        ) : null}

        <div className="border-t border-white/[0.04] px-6 py-5 text-center text-[11px] tracking-[0.02em] text-zinc-600">
          <code className="rounded-md bg-white/[0.04] px-1.5 py-px font-mono text-zinc-500">GET /connect/gmail/callback</code>
        </div>
      </div>
    </div>
  );
}

function OauthChromeCard({
  id,
  brand,
  title,
  body,
  onOAuthPreviewComplete,
}: {
  id: ConnectorId;
  brand: ReactNode;
  title: string;
  body: ReactNode;
  onOAuthPreviewComplete: (id: ConnectorId) => void;
}) {
  const [oauthPhase, setOauthPhase] = useState<OAuthPhase>("form");

  const runOAuthPreview = () => {
    if (oauthPhase !== "form") return;
    setOauthPhase("busy");
    window.setTimeout(() => {
      setOauthPhase("done");
      onOAuthPreviewComplete(id);
    }, 900);
  };

  return (
    <div className="mx-auto max-w-lg">
      <PreviewBanner />
      <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
        <div className="flex items-center gap-3 border-b border-white/[0.06] px-4 py-3">
          {brand}
          <div className="min-w-0">
            <p className="truncate text-[17px] font-semibold tracking-tight text-zinc-100">{title}</p>
            <p className="text-[11px] text-zinc-600">Connect this source to your workspace.</p>
          </div>
        </div>
        <div className="px-4 py-4 text-[13px] leading-relaxed text-zinc-500">{body}</div>
        {oauthPhase === "form" && (
          <div className="border-t border-white/[0.06] px-4 py-4">
            <button
              type="button"
              onClick={runOAuthPreview}
              className="w-full rounded-xl bg-zinc-100 py-3 text-[14px] font-semibold text-zinc-900 transition hover:bg-white active:scale-[0.99]"
            >
              Continue (preview)
            </button>
            <p className="mt-2 text-center text-[11px] text-zinc-600">
              Production: OAuth redirect → <code className="rounded bg-black/25 px-1 font-mono text-zinc-500">/oauth/{id}/callback</code>
            </p>
          </div>
        )}
        {oauthPhase === "busy" && (
          <div className="border-t border-white/[0.06] px-4 py-8 text-center text-[13px] text-zinc-500">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-sky-400 border-t-transparent align-middle" />{" "}
            Simulating redirect…
          </div>
        )}
        {oauthPhase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-950/25 px-4 py-3">
            <p className="text-[12px] leading-relaxed text-emerald-200/90">
              Preview session on. Next: wire background sync to{" "}
              <code className="rounded bg-black/20 px-1 font-mono text-[11px] text-emerald-300/90">ingest_chunk</code>.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function fieldClass(disabled?: boolean) {
  return `mt-1 w-full rounded-xl border border-white/[0.08] bg-zinc-950/50 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-sky-500/35 focus:ring-1 focus:ring-sky-500/20 ${
    disabled ? "cursor-not-allowed opacity-50" : ""
  }`;
}

/** Equities: vendor + watchlist + order book sync — credentials via UI (OAuth or vault), not hard-coded keys. */
function EquitiesApiConnectCard({ onOAuthPreviewComplete }: { onOAuthPreviewComplete: (id: ConnectorId) => void }) {
  const [phase, setPhase] = useState<OAuthPhase>("form");
  const [authMode, setAuthMode] = useState<"oauth_broker" | "vendor_api">("oauth_broker");
  const [vendor, setVendor] = useState("polygon");
  const [watchlist, setWatchlist] = useState("ORCL, NVDA, SPY, QQQ");
  const [watchlistName, setWatchlistName] = useState("main desk");
  const [syncOpenOrders, setSyncOpenOrders] = useState(true);
  const [syncFills, setSyncFills] = useState(true);
  const [orderHistoryDays, setOrderHistoryDays] = useState("90");
  const [syncPaper, setSyncPaper] = useState(false);
  const [pollQuotes, setPollQuotes] = useState(true);
  const [vaultKeyLabel, setVaultKeyLabel] = useState("equities:polygon:primary");

  const runPreview = () => {
    if (phase !== "form") return;
    setPhase("busy");
    window.setTimeout(() => {
      setPhase("done");
      onOAuthPreviewComplete("equities");
    }, 1100);
  };

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <PreviewBanner />
      <div className="overflow-hidden rounded-2xl border border-emerald-500/20 bg-[#0c0c18] shadow-[0_24px_80px_rgba(0,0,0,0.55)]">
        <div className="flex items-center gap-3 border-b border-white/5 px-5 py-4">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-emerald-600 text-sm font-bold text-white">
            EQ
          </span>
          <div>
            <p className="text-xs font-medium text-slate-400">Connect · stocks & equities</p>
            <p className="text-lg font-semibold tracking-tight text-slate-100">Tape, watchlist & order book</p>
          </div>
        </div>

        {phase === "form" && (
          <div className="space-y-6 px-5 py-5">
            <p className="text-sm leading-relaxed text-slate-400">
              Configure what to pull into the graph. Keys and tokens are stored server-side after you authorize — nothing
              is pasted into app source or checked into git.
            </p>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">authentication</p>
              <div className="mt-3 flex flex-col gap-3 sm:flex-row">
                <label className="flex flex-1 cursor-pointer items-start gap-2 rounded-lg border border-emerald-400/25 bg-emerald-500/5 p-3">
                  <input
                    type="radio"
                    name="eq-auth"
                    checked={authMode === "oauth_broker"}
                    onChange={() => setAuthMode("oauth_broker")}
                    className="mt-0.5 accent-emerald-400"
                  />
                  <span>
                    <span className="text-sm font-medium text-emerald-100">Broker / vendor OAuth</span>
                    <span className="mt-1 block text-[11px] text-slate-500">
                      Redirect in browser; refresh token in encrypted store.
                    </span>
                  </span>
                </label>
                <label className="flex flex-1 cursor-pointer items-start gap-2 rounded-lg border border-white/10 p-3">
                  <input
                    type="radio"
                    name="eq-auth"
                    checked={authMode === "vendor_api"}
                    onChange={() => setAuthMode("vendor_api")}
                    className="mt-0.5 accent-slate-400"
                  />
                  <span>
                    <span className="text-sm font-medium text-slate-200">Vendor API via secrets vault</span>
                    <span className="mt-1 block text-[11px] text-slate-500">
                      UI posts once to <span className="font-mono text-cyan-600/90">POST /vault/secrets</span> — never
                      embedded in clients.
                    </span>
                  </span>
                </label>
              </div>
              {authMode === "vendor_api" && (
                <div className="mt-4 space-y-3 border-t border-white/5 pt-4">
                  <label className="block text-xs">
                    <span className="text-slate-500">Secret label (for rotation)</span>
                    <input
                      type="text"
                      value={vaultKeyLabel}
                      onChange={(e) => setVaultKeyLabel(e.target.value)}
                      className={fieldClass()}
                    />
                  </label>
                  <label className="block text-xs">
                    <span className="text-slate-500">API key (sent over TLS to server only)</span>
                    <input
                      type="password"
                      placeholder="••••••••••••••••"
                      className={fieldClass()}
                      autoComplete="off"
                    />
                  </label>
                  <p className="font-mono text-[10px] text-amber-200/70">
                    Preview: value is not transmitted. Production: one-shot submit from memory, never log raw secret.
                  </p>
                </div>
              )}
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">market data vendor</p>
              <label className="mt-3 block text-xs">
                <span className="text-slate-500">Primary feed</span>
                <select
                  value={vendor}
                  onChange={(e) => setVendor(e.target.value)}
                  className={fieldClass()}
                >
                  <option value="polygon">Polygon.io — US equities</option>
                  <option value="alpaca">Alpaca — market + optional brokerage</option>
                  <option value="schwab">Schwab developer (OAuth)</option>
                  <option value="ibkr">Interactive Brokers — TWS / Client Portal</option>
                </select>
              </label>
              <label className="mt-3 flex items-center gap-2 text-xs text-slate-400">
                <input type="checkbox" checked={pollQuotes} onChange={(e) => setPollQuotes(e.target.checked)} className="accent-emerald-400" />
                Stream or poll quotes for watchlist symbols
              </label>
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">watchlists</p>
              <label className="mt-3 block text-xs">
                <span className="text-slate-500">List name</span>
                <input
                  type="text"
                  value={watchlistName}
                  onChange={(e) => setWatchlistName(e.target.value)}
                  className={fieldClass()}
                />
              </label>
              <label className="mt-3 block text-xs">
                <span className="text-slate-500">Symbols (comma or newline separated)</span>
                <textarea
                  value={watchlist}
                  onChange={(e) => setWatchlist(e.target.value)}
                  rows={4}
                  className={`${fieldClass()} min-h-[5.5rem] resize-y`}
                  placeholder="AAPL, MSFT, …"
                />
              </label>
              <p className="mt-2 font-mono text-[10px] text-slate-600">
                Rust: normalize → <span className="text-cyan-700/90">WatchlistId</span> + vertices per ticker; diff on
                each sync.
              </p>
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">orders & blotter</p>
              <div className="mt-3 space-y-2 text-xs text-slate-400">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={syncOpenOrders}
                    onChange={(e) => setSyncOpenOrders(e.target.checked)}
                    className="accent-emerald-400"
                  />
                  Keep open orders in sync (working / pending)
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={syncFills}
                    onChange={(e) => setSyncFills(e.target.checked)}
                    className="accent-emerald-400"
                  />
                  Import fills & canceled history for graph edges to executions
                </label>
                <label className="flex items-center gap-2">
                  <input type="checkbox" checked={syncPaper} onChange={(e) => setSyncPaper(e.target.checked)} className="accent-emerald-400" />
                  Include paper-trading accounts
                </label>
              </div>
              <label className="mt-4 block text-xs">
                <span className="text-slate-500">Order history lookback (days)</span>
                <input
                  type="number"
                  min={1}
                  max={365}
                  value={orderHistoryDays}
                  onChange={(e) => setOrderHistoryDays(e.target.value)}
                  className={fieldClass()}
                />
              </label>
            </div>
          </div>
        )}

        {phase === "busy" && (
          <div className="border-t border-white/5 px-5 py-10 text-center font-mono text-sm text-slate-400">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-emerald-400 border-t-transparent" />{" "}
            Validating credentials & registering sync jobs…
          </div>
        )}

        {phase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-500/5 px-5 py-4">
            <p className="font-mono text-xs text-emerald-200/90">
              Preview: watchlist “{watchlistName}” and order sync flags stored locally. Production workers call vendor REST or
              WebSocket with rotated secrets from vault.
            </p>
          </div>
        )}

        {phase === "form" && (
          <div className="border-t border-white/5 px-5 py-4">
            <button
              type="button"
              onClick={runPreview}
              className="w-full rounded-xl bg-gradient-to-r from-emerald-500 to-cyan-500 py-3 text-sm font-semibold text-slate-950 transition hover:opacity-95"
            >
              {authMode === "oauth_broker" ? "Continue with OAuth (preview)" : "Save & connect (preview)"}
            </button>
            <p className="mt-2 text-center font-mono text-[10px] text-slate-500">
              Production: <code className="text-cyan-600/80">POST /markets/equities/connect</code> then background ETL
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

/** Futures: Apex-first connect via browser OAuth; optional advanced key path discouraged in copy. */
function FuturesApexConnectCard({ onOAuthPreviewComplete }: { onOAuthPreviewComplete: (id: ConnectorId) => void }) {
  const [phase, setPhase] = useState<OAuthPhase>("form");
  const [env, setEnv] = useState<"apex_paper" | "apex_live">("apex_paper");
  const [roots, setRoots] = useState("ES, NQ, MES, MNQ");
  const [syncPositions, setSyncPositions] = useState(true);
  const [syncOpenOrders, setSyncOpenOrders] = useState(true);
  const [syncFills, setSyncFills] = useState(true);
  const [rollAlerts, setRollAlerts] = useState(true);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const runPreview = () => {
    if (phase !== "form") return;
    setPhase("busy");
    window.setTimeout(() => {
      setPhase("done");
      onOAuthPreviewComplete("futures");
    }, 1200);
  };

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <PreviewBanner />
      <div className="overflow-hidden rounded-2xl border border-sky-500/25 bg-[#0c0c18] shadow-[0_24px_80px_rgba(0,0,0,0.55)]">
        <div className="flex items-center gap-3 border-b border-white/5 px-5 py-4">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-sky-600 text-xs font-bold text-white">
            AX
          </span>
          <div>
            <p className="text-xs font-medium text-slate-400">Connect · futures</p>
            <p className="text-lg font-semibold tracking-tight text-slate-100">Apex Trading API</p>
            <p className="mt-0.5 text-[11px] text-slate-500">
              Use the broker login flow in your browser — no API keys in env vars or CLI flags.
            </p>
          </div>
        </div>

        {phase === "form" && (
          <div className="space-y-6 px-5 py-5">
            <div className="rounded-xl border border-sky-500/20 bg-sky-500/[0.06] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-sky-300/80">environment</p>
              <div className="mt-3 flex flex-wrap gap-3">
                <label className="flex cursor-pointer items-center gap-2 rounded-lg border border-white/10 bg-black/20 px-4 py-2 text-sm text-slate-200">
                  <input
                    type="radio"
                    name="apex-env"
                    checked={env === "apex_paper"}
                    onChange={() => setEnv("apex_paper")}
                    className="accent-sky-400"
                  />
                  Apex paper
                </label>
                <label className="flex cursor-pointer items-center gap-2 rounded-lg border border-amber-500/25 bg-amber-500/10 px-4 py-2 text-sm text-amber-100">
                  <input
                    type="radio"
                    name="apex-env"
                    checked={env === "apex_live"}
                    onChange={() => setEnv("apex_live")}
                    className="accent-amber-400"
                  />
                  Apex live
                </label>
              </div>
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">contracts to graph</p>
              <label className="mt-3 block text-xs">
                <span className="text-slate-500">Roots (continuous symbol roots)</span>
                <textarea
                  value={roots}
                  onChange={(e) => setRoots(e.target.value)}
                  rows={3}
                  className={`${fieldClass()} min-h-[4.5rem] resize-y`}
                />
              </label>
              <label className="mt-3 flex items-center gap-2 text-xs text-slate-400">
                <input type="checkbox" checked={rollAlerts} onChange={(e) => setRollAlerts(e.target.checked)} className="accent-sky-400" />
                Emit roll-window nodes when front month liquidity shifts
              </label>
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">sync into knowledge graph</p>
              <div className="mt-3 space-y-2 text-xs text-slate-400">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={syncPositions}
                    onChange={(e) => setSyncPositions(e.target.checked)}
                    className="accent-sky-400"
                  />
                  Positions & net exposure by product
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={syncOpenOrders}
                    onChange={(e) => setSyncOpenOrders(e.target.checked)}
                    className="accent-sky-400"
                  />
                  Open orders (limits, stops, brackets)
                </label>
                <label className="flex items-center gap-2">
                  <input type="checkbox" checked={syncFills} onChange={(e) => setSyncFills(e.target.checked)} className="accent-sky-400" />
                  Fills & partials for audit trail nodes
                </label>
              </div>
            </div>

            <div className="rounded-xl border border-white/8 bg-white/[0.02] p-4">
              <button
                type="button"
                onClick={() => setAdvancedOpen((o) => !o)}
                className="flex w-full items-center justify-between text-left font-mono text-[11px] uppercase tracking-wider text-slate-500"
              >
                <span>Advanced · machine credentials (discouraged)</span>
                <span className="text-slate-600">{advancedOpen ? "−" : "+"}</span>
              </button>
              {advancedOpen && (
                <div className="mt-4 space-y-3 border-t border-white/5 pt-4">
                  <p className="text-[11px] leading-relaxed text-slate-500">
                    Prefer Apex OAuth above. If your org requires a service account, paste a key once; the server stores it
                    in the vault and the UI never reads it back in full.
                  </p>
                  <label className="block text-xs">
                    <span className="text-slate-500">Apex API key ID</span>
                    <input type="text" placeholder="key_id_…" className={fieldClass()} autoComplete="off" />
                  </label>
                  <label className="block text-xs">
                    <span className="text-slate-500">Apex API secret</span>
                    <input type="password" placeholder="••••••••" className={fieldClass()} autoComplete="off" />
                  </label>
                </div>
              )}
            </div>
          </div>
        )}

        {phase === "busy" && (
          <div className="border-t border-white/5 px-5 py-10 text-center font-mono text-sm text-slate-400">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-sky-400 border-t-transparent" />{" "}
            Opening Apex authorization in a secure browser context…
          </div>
        )}

        {phase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-500/5 px-5 py-4">
            <p className="font-mono text-xs text-emerald-200/90">
              Preview: Apex session ({env === "apex_paper" ? "paper" : "live"}) — roots and sync flags registered here. Production
              uses OAuth callback and encrypted refresh token storage in Rust.
            </p>
          </div>
        )}

        {phase === "form" && (
          <div className="space-y-3 border-t border-white/5 px-5 py-4">
            <button
              type="button"
              onClick={runPreview}
              className="w-full rounded-xl bg-sky-500 py-3 text-sm font-semibold text-slate-950 transition hover:bg-sky-400"
            >
              Open Apex sign-in (preview)
            </button>
            <p className="text-center font-mono text-[10px] text-slate-500">
              Production: <code className="text-cyan-600/80">GET /oauth/apex/start?env=paper|live</code> → Apex consent →{" "}
              <code className="text-cyan-600/80">/oauth/apex/callback</code>
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function WhatsappPreviewCard({ onOAuthPreviewComplete }: { onOAuthPreviewComplete: (id: ConnectorId) => void }) {
  const [oauthPhase, setOauthPhase] = useState<OAuthPhase>("form");

  const run = () => {
    if (oauthPhase !== "form") return;
    setOauthPhase("busy");
    window.setTimeout(() => {
      setOauthPhase("done");
      onOAuthPreviewComplete("whatsapp");
    }, 900);
  };

  return (
    <div className="mx-auto max-w-lg">
      <PreviewBanner />
      <div className="overflow-hidden rounded-2xl border border-emerald-500/20 bg-[#0c0c18]">
        <div className="p-6">
          <h3 className="text-sm font-medium text-emerald-100">Link WhatsApp (Baileys / Cloud API)</h3>
          <p className="mt-2 text-sm text-slate-400">
            Production shows a QR or Meta embedded signup. Here is the pairing frame operators expect.
          </p>
          <div className="mx-auto mt-6 grid h-44 w-44 place-items-center rounded-xl border-2 border-dashed border-emerald-500/30 bg-black/40 text-center font-mono text-[10px] text-slate-500">
            QR placeholder
            <br />
            <span className="mt-1 text-emerald-600/60">scan in WhatsApp</span>
          </div>
          <label className="mt-6 block text-xs text-slate-500">
            Phone (E.164)
            <input
              type="text"
              readOnly
              defaultValue="+1 · · · · · · · · ·"
              className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-400"
            />
          </label>
        </div>
        {oauthPhase === "form" && (
          <div className="border-t border-white/5 px-5 py-4">
            <button
              type="button"
              onClick={run}
              className="w-full rounded-xl border border-emerald-500/40 bg-emerald-600/20 py-3 text-sm font-semibold text-emerald-100 transition hover:bg-emerald-600/30"
            >
              Simulate paired session
            </button>
          </div>
        )}
        {oauthPhase === "busy" && (
          <div className="border-t border-white/5 px-5 py-8 text-center font-mono text-sm text-slate-400">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-emerald-400 border-t-transparent" />{" "}
            Waiting for scan / handshake…
          </div>
        )}
        {oauthPhase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-500/5 px-5 py-4">
            <p className="font-mono text-xs text-emerald-200/90">
              Preview session active. Next production step: route inbound messages to{" "}
              <code className="text-emerald-300">ingest_chunk</code> with consent flags.
            </p>
          </div>
        )}
      </div>
      <RustFootnote
        lines={[
          "Session store in Rust; message stream → normalize → ingest_chunk(..., \"whatsapp\", …).",
          "Strict consent + retention flags before any cloud backup.",
        ]}
      />
    </div>
  );
}

export function WorkspaceSurfacePanel({
  surface,
  onClose,
  pdfInputId,
  kgUrl,
  graphNodes,
  graphEdges,
  onOAuthPreviewComplete,
  onGmailGraphRefresh,
  onGithubPublicCloneSuccess,
  onGithubCloneSessionStart,
}: Props) {
  /** Public GitHub URL for shallow clone (`ingestion_registry::codebase::clone`). */
  const [githubPublicRepoUrl, setGithubPublicRepoUrl] = useState("");
  /** Repo-relative path prefix for `POST /ingest { path }`, rules link filter, and security deploy `scope`. */
  const [githubIngestPath, setGithubIngestPath] = useState("");
  /** Git network: shallow clone or pull into ~/.fluvio/repos/… */
  const [githubPullBusy, setGithubPullBusy] = useState(false);
  const [githubPullErr, setGithubPullErr] = useState<string | null>(null);
  const [githubPullOk, setGithubPullOk] = useState<string | null>(null);

  /** Parse + embed chunks from an already-cloned repo */
  const [githubCloneBusy, setGithubCloneBusy] = useState(false);
  const [githubCloneErr, setGithubCloneErr] = useState<string | null>(null);
  const [githubCloneOk, setGithubCloneOk] = useState<string | null>(null);

  return (
    <div
      className="absolute inset-0 z-40 flex flex-col overflow-y-auto bg-zinc-950/96 p-5 backdrop-blur-xl supports-[backdrop-filter]:bg-zinc-950/88 sm:p-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="surface-title"
    >
      <header className="mx-auto mb-5 flex w-full max-w-3xl shrink-0 items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 id="surface-title" className="text-[17px] font-semibold tracking-tight text-zinc-100">
            {surface === "documents" && "Documents"}
            {surface === "gmail" && "Gmail"}
            {surface === "github" && "GitHub"}
            {surface === "calendar" && "Google Calendar"}
            {surface === "whatsapp" && "WhatsApp"}
            {surface === "slack" && "Slack"}
            {surface === "notion" && "Notion"}
            {surface === "equities" && "Stocks & equities"}
            {surface === "futures" && "Futures"}
            {surface === "cryptocurrencies" && "Cryptocurrencies"}
            {surface === "fin_news" && "News wires"}
            {surface === "fin_market_data" && "Market data APIs"}
            {surface === "fin_research" && "Research & books"}
            {surface === "des_bim" && "BIM / IFC"}
            {surface === "des_arch_plans" && "Architectural plans"}
            {surface === "des_structural" && "Structural analysis"}
            {surface === "des_civil_site" && "Civil & site"}
            {surface === "des_building_codes" && "Codes & loads"}
            {surface === "des_physics_sim" && "Physics & simulation"}
          </h2>
          <p className="mt-1 max-w-xl text-[13px] leading-relaxed text-zinc-500">
            Configure how this source connects. Fields mirror what you will persist and sync from the server.
          </p>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="shrink-0 rounded-full border border-white/[0.1] bg-white/[0.04] px-4 py-2 text-[13px] font-medium text-zinc-200 transition hover:bg-white/[0.08] active:scale-[0.98]"
        >
          Done
        </button>
      </header>

      <div className="mx-auto w-full max-w-3xl flex-1 pb-12">
        {surface === "documents" && (
          <div>
            <PreviewBanner />
            <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
              <div className="border-b border-white/[0.06] px-4 py-3">
                <h3 className="text-[15px] font-semibold text-zinc-100">PDF ingestion</h3>
                <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
                  Multipart upload, chunking, embeddings, and graph persistence — live on kg-engine today.
                </p>
              </div>
              <dl className="divide-y divide-white/[0.06] px-4">
                <div className="flex flex-wrap items-baseline justify-between gap-3 py-3 text-[13px]">
                  <dt className="font-medium text-zinc-500">Endpoint</dt>
                  <dd
                    className="min-w-0 break-all text-right font-mono text-[12px] text-zinc-300"
                    suppressHydrationWarning
                  >
                    POST {kgUrl}/ingest/pdf
                  </dd>
                </div>
                <div className="flex flex-wrap items-baseline justify-between gap-3 py-3 text-[13px]">
                  <dt className="font-medium text-zinc-500">Body</dt>
                  <dd className="text-right text-[12px] text-zinc-400">multipart field “file” (.pdf)</dd>
                </div>
                <div className="flex flex-wrap items-baseline justify-between gap-3 py-3 text-[13px]">
                  <dt className="font-medium text-zinc-500">Current graph</dt>
                  <dd className="text-right tabular-nums text-zinc-200">
                    {graphNodes} nodes · {graphEdges} edges
                  </dd>
                </div>
              </dl>
              <div className="flex flex-col gap-3 border-t border-white/[0.06] px-4 py-4 sm:flex-row sm:items-center">
                <label
                  htmlFor={pdfInputId}
                  className="inline-flex cursor-pointer items-center justify-center rounded-xl bg-zinc-100 px-6 py-3 text-center text-[14px] font-semibold text-zinc-900 transition hover:bg-white active:scale-[0.99]"
                >
                  Choose PDF…
                </label>
                <p className="text-[12px] leading-relaxed text-zinc-600 sm:flex-1">
                  Or use <span className="font-medium text-zinc-400">Add</span> in the sidebar for a quick upload.
                </p>
              </div>
            </div>
            <details className="mt-4 overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.02] open:pb-3">
              <summary className="cursor-pointer list-none px-4 py-3 text-[13px] font-medium text-zinc-400 transition-colors hover:bg-white/[0.04] [&::-webkit-details-marker]:hidden">
                Advanced options (coming soon)
              </summary>
              <div className="space-y-3 border-t border-white/[0.06] px-4 py-3 text-[12px] text-zinc-600">
                <label className="block opacity-60">
                  <span className="text-zinc-500">Chunk target tokens</span>
                  <input type="range" disabled className="mt-1.5 w-full" defaultValue={50} />
                </label>
                <label className="flex items-center gap-2 opacity-60">
                  <input type="checkbox" disabled defaultChecked />
                  Deduplicate pages on re-upload
                </label>
                <label className="flex items-center gap-2 opacity-60">
                  <input type="checkbox" disabled />
                  OCR for scanned PDFs
                </label>
              </div>
            </details>
            <RustFootnote
              lines={[
                "Axum: Multipart extract → temp path → PDFChunkIterator → IngestionPipeline::ingest_chunk(..., \"pdf\", seq).",
                "Expose optional PUT /settings/pdf { chunk_tokens, dedupe } and reload pipeline config from disk.",
              ]}
            />
          </div>
        )}

        {surface === "gmail" && (
          <>
            <GmailKgEngineConnect
              kgUrl={kgUrl}
              onOAuthPreviewComplete={onOAuthPreviewComplete}
              onGraphRefresh={onGmailGraphRefresh}
            />
            <RustFootnote
              lines={[
                "POST /connect/gmail/start → JSON { url } (Bearer session); open url for Google consent; callback persists tokens per user.",
                "GET/PUT /gmail/focus — optional sender allow-list; GET /gmail/recent merges History deltas (is_new) + polled inbox previews.",
                "GET/PUT /gmail/agent/settings — send_mode + context_sources; Gmail-linked users get automatic inbox passes (server env GMAIL_AGENT_AUTO_POLL_INTERVAL_SECS).",
                "GET /gmail/agent/reviews?limit — saved AI drafts (draft_only). POST /gmail/agent/run { dry_run } — preview without persisting inbox state.",
              ]}
            />
          </>
        )}

        {surface === "github" && (
          <>
            <OauthChromeCard
              id="github"
              brand={
                <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/[0.1] bg-[#24292f] text-[11px] font-bold text-white shadow-sm">
                  GH
                </span>
              }
              title="GitHub"
              body={
                <div className="space-y-4">
                  <div className="overflow-hidden rounded-xl border border-white/[0.08] bg-black/20">
                    <div className="border-b border-white/[0.06] px-3 py-2.5">
                      <p className="text-[13px] font-semibold text-zinc-200">Public repository</p>
                      <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">
                        Paste a URL or <span className="font-mono text-zinc-400">owner/repo</span>.{" "}
                        <span className="font-mono text-zinc-500">POST /codebase/clone</span> (or{" "}
                        <span className="font-mono text-zinc-500">/sync/codebase/clone</span>) pulls the repo to disk;{" "}
                        <span className="font-mono text-zinc-500">POST /ingest</span> parses the local mirror into the graph.
                      </p>
                    </div>
                    <div className="space-y-3 p-3">
                      <label className="block text-[12px] font-medium text-zinc-600">
                        Repository URL
                        <input
                          type="text"
                          inputMode="url"
                          autoComplete="off"
                          spellCheck={false}
                          placeholder="https://github.com/org/repo"
                          value={githubPublicRepoUrl}
                          onChange={(e) => {
                            setGithubCloneErr(null);
                            setGithubCloneOk(null);
                            setGithubPullErr(null);
                            setGithubPullOk(null);
                            setGithubPublicRepoUrl(e.target.value);
                          }}
                          className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950/50 px-3 py-2.5 font-mono text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-sky-500/35 focus:ring-1 focus:ring-sky-500/20"
                        />
                      </label>
                      <label className="block text-[12px] font-medium text-zinc-600">
                        Path prefix (optional)
                        <input
                          type="text"
                          autoComplete="off"
                          spellCheck={false}
                          placeholder="src — same as curl -d path"
                          value={githubIngestPath}
                          onChange={(e) => {
                            setGithubCloneErr(null);
                            setGithubCloneOk(null);
                            setGithubPullErr(null);
                            setGithubPullOk(null);
                            setGithubIngestPath(e.target.value);
                          }}
                          className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950/50 px-3 py-2.5 font-mono text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-sky-500/35 focus:ring-1 focus:ring-sky-500/20"
                        />
                      </label>
                      <p className="text-[11px] leading-relaxed text-zinc-600">
                        Sent as <span className="font-mono text-zinc-400">path</span> on{" "}
                        <span className="font-mono text-zinc-500">POST /ingest</span>; also used for rules link filter and
                        security agent <span className="font-mono text-zinc-500">scope</span> below.
                      </p>
                      <div className="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:gap-3">
                        <button
                          type="button"
                          disabled={githubPullBusy || githubCloneBusy || !githubPublicRepoUrl.trim()}
                          onClick={async () => {
                            const url = githubPublicRepoUrl.trim();
                            if (!url) return;
                            const repoRef = parseGithubRepoInput(url);
                            if (!repoRef) {
                              setGithubPullErr("Enter a valid GitHub URL or owner/repo.");
                              return;
                            }
                            setGithubPullBusy(true);
                            setGithubPullErr(null);
                            setGithubPullOk(null);
                            try {
                              const data = await postCodebaseClone(kgUrl, url);
                              setGithubPullOk(
                                data.was_cloned
                                  ? `Cloned ${data.owner}/${data.repo} → ${data.local_path}`
                                  : `Repo already present; pulled latest for ${data.owner}/${data.repo}`,
                              );
                              onGithubPublicCloneSuccess?.({
                                owner: data.owner,
                                repo: data.repo,
                                local_path: data.local_path,
                                was_cloned: data.was_cloned,
                              });
                            } catch (e: unknown) {
                              setGithubPullErr(e instanceof Error ? e.message : String(e));
                            } finally {
                              setGithubPullBusy(false);
                            }
                          }}
                          className="rounded-xl border border-sky-500/35 bg-sky-500/10 px-4 py-2.5 text-[13px] font-semibold text-sky-100 transition hover:bg-sky-500/20 disabled:cursor-not-allowed disabled:opacity-40"
                        >
                          {githubPullBusy ? "Cloning…" : "Clone or pull repo"}
                        </button>
                        <span className="font-mono text-[11px] text-zinc-600">POST …/codebase/clone</span>
                        <button
                          type="button"
                          disabled={githubCloneBusy || githubPullBusy || !githubPublicRepoUrl.trim()}
                          onClick={async () => {
                            const url = githubPublicRepoUrl.trim();
                            if (!url) return;
                            const repoRef = parseGithubRepoInput(url);
                            if (!repoRef) {
                              setGithubCloneErr("Enter a valid GitHub URL or owner/repo.");
                              return;
                            }
                            setGithubCloneBusy(true);
                            setGithubCloneErr(null);
                            setGithubCloneOk(null);
                            try {
                              const pathNorm = githubIngestPath.replace(/\\/g, "/").trim();
                              const res = await fetch(`${kgUrl}/ingest`, {
                                method: "POST",
                                headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
                                body: JSON.stringify({ url, path: pathNorm }),
                              });
                              const text = await res.text();
                              if (!res.ok) {
                                throw new Error(text || `HTTP ${res.status}`);
                              }
                              let data: { chunks: number; nodes: number; edges: number };
                              try {
                                data = JSON.parse(text) as { chunks: number; nodes: number; edges: number };
                              } catch {
                                throw new Error(text || "invalid JSON from server");
                              }
                              onGithubPublicCloneSuccess?.({
                                owner: repoRef.owner,
                                repo: repoRef.repo,
                                local_path: `${repoRef.owner}/${repoRef.repo}`,
                                was_cloned: false,
                              });
                              setGithubCloneOk(
                                `${repoRef.owner}/${repoRef.repo} ingested (${data.chunks} chunks, ${data.nodes} nodes, ${data.edges} edges).`,
                              );
                              await onGithubCloneSessionStart?.();
                            } catch (e: unknown) {
                              const msg = e instanceof Error ? e.message : String(e);
                              setGithubCloneErr(msg);
                            } finally {
                              setGithubCloneBusy(false);
                            }
                          }}
                          className="rounded-xl bg-zinc-100 px-4 py-2.5 text-[13px] font-semibold text-zinc-900 transition hover:bg-white disabled:cursor-not-allowed disabled:opacity-40 active:scale-[0.99]"
                        >
                          {githubCloneBusy ? "Ingesting…" : "Ingest into graph"}
                        </button>
                        <span className="font-mono text-[11px] text-zinc-600">POST /ingest</span>
                      </div>
                      <p className="text-[11px] leading-relaxed text-zinc-600">
                        New machine or repo: run <span className="font-medium text-zinc-400">Clone or pull</span> first; ingest
                        only reads files under the path prefix from the local clone.
                      </p>
                      {githubPullErr && (
                        <p className="rounded-xl border border-red-500/25 bg-red-950/40 px-3 py-2 text-[12px] text-red-200/95">
                          {githubPullErr}
                        </p>
                      )}
                      {githubPullOk && (
                        <p className="rounded-xl border border-sky-500/20 bg-sky-950/25 px-3 py-2 text-[12px] text-sky-100/95">
                          {githubPullOk}
                        </p>
                      )}
                      {githubCloneErr && (
                        <p className="rounded-xl border border-red-500/25 bg-red-950/40 px-3 py-2 text-[12px] text-red-200/95">
                          {githubCloneErr}
                        </p>
                      )}
                      {githubCloneOk && (
                        <p className="rounded-xl border border-emerald-500/20 bg-emerald-950/30 px-3 py-2 text-[12px] text-emerald-200/95">
                          {githubCloneOk}
                        </p>
                      )}
                    </div>
                  </div>

                  <div className="grid gap-2 sm:grid-cols-2">
                    <label className="flex cursor-pointer items-center gap-2 rounded-xl border border-white/[0.08] bg-white/[0.02] p-3 text-[12px] text-zinc-300">
                      <input type="radio" name="gh" defaultChecked readOnly className="accent-sky-500" />
                      GitHub App (recommended)
                    </label>
                    <label className="flex cursor-pointer items-center gap-2 rounded-xl border border-white/[0.06] bg-white/[0.02] p-3 text-[12px] text-zinc-500 opacity-80">
                      <input type="radio" name="gh" readOnly className="accent-zinc-500" />
                      Fine-grained PAT
                    </label>
                  </div>
                  <label className="block text-[12px] font-medium text-zinc-600">
                    Org / repos
                    <input
                      type="text"
                      readOnly
                      defaultValue="acme-corp/api, acme-corp/kg-engine"
                      className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950/40 px-3 py-2.5 font-mono text-[12px] text-zinc-400"
                    />
                  </label>
                  <p className="rounded-xl border border-white/[0.06] bg-black/20 px-3 py-2.5 text-[11px] leading-relaxed text-zinc-600">
                    Webhook URL (example):{" "}
                    <code className="rounded bg-black/30 px-1 font-mono text-zinc-400">https://&lt;host&gt;/hooks/github</code>
                  </p>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "Codebase: POST /codebase/clone or /sync/codebase/clone { url } (git shallow clone or pull) then POST /ingest { url, path } from the local mirror.",
                "Security rules + agent: use Workspace brain → GitHub (after PDF + repo are in the graph).",
                "Verify X-Hub-Signature-256 for apps; map payload → PR / push / issue nodes; PAT for private API tree later.",
              ]}
            />
          </>
        )}

        {surface === "calendar" && (
          <>
            <OauthChromeCard
              id="calendar"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-blue-500 text-sm font-bold text-white">
                  31
                </span>
              }
              title="Google Calendar"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>Events become time-bounded nodes linked to people and projects already in your graph.</p>
                  <ul className="rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[11px] text-blue-200/80">
                    <li>https://www.googleapis.com/auth/calendar.readonly</li>
                  </ul>
                  <div className="text-xs">
                    <p className="mb-2 text-slate-500">Calendars to include</p>
                    <div className="flex flex-wrap gap-2">
                      {["Work", "Personal", "Focus blocks"].map((c) => (
                        <span key={c} className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-slate-300">
                          {c}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "Share OAuth client with Gmail or separate; store calendar.list + syncToken per calendar.",
                "Worker: incremental sync → Event vertex + attendee edges → ingest_chunk(..., \"calendar\", …).",
              ]}
            />
          </>
        )}

        {surface === "whatsapp" && <WhatsappPreviewCard onOAuthPreviewComplete={onOAuthPreviewComplete} />}

        {surface === "slack" && (
          <>
            <OauthChromeCard
              id="slack"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-[#4a154b] text-lg text-white">
                  #
                </span>
              }
              title="Slack workspace"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <label className="block text-xs">
                    <span className="text-slate-500">Workspace domain</span>
                    <input
                      type="text"
                      readOnly
                      defaultValue="your-team.slack.com"
                      className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
                    />
                  </label>
                  <ul className="rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[11px] text-pink-200/80">
                    <li>channels:history</li>
                    <li>users:read</li>
                    <li>reactions:read</li>
                  </ul>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "OAuth v2 install URL; store bot token per workspace; Events API subscription in axum.",
                "Map channel + thread_ts → conversation tree in the graph.",
              ]}
            />
          </>
        )}

        {surface === "notion" && (
          <>
            <OauthChromeCard
              id="notion"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full border border-white/20 bg-white text-sm font-bold text-slate-900">
                  N
                </span>
              }
              title="Notion"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>Internal integration: pages and databases become typed nodes with parent/child and relation edges.</p>
                  <label className="block text-xs">
                    <span className="text-slate-500">Integration secret</span>
                    <input
                      type="password"
                      readOnly
                      defaultValue="secret_preview_only"
                      className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
                    />
                  </label>
                  <label className="block text-xs">
                    <span className="text-slate-500">Root page URL</span>
                    <input
                      type="text"
                      readOnly
                      defaultValue="https://notion.so/Workspace-…"
                      className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
                    />
                  </label>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "POST /integrations/notion with secret → validate with Notion API; store encrypted.",
                "Recursive children crawl with cursor; rate-limit 3 rps; ingest_chunk(..., \"notion\", …).",
              ]}
            />
          </>
        )}

        {surface === "equities" && (
          <>
            <EquitiesApiConnectCard onOAuthPreviewComplete={onOAuthPreviewComplete} />
            <RustFootnote
              lines={[
                "POST /markets/equities/connect { auth, vendor, watchlists[], order_sync } → persist EncryptedCredential + job spec.",
                "GET /markets/equities/orders?since=… paginates broker/vendor; map OrderId → graph nodes; webhook optional for deltas.",
                "Watchlists: PUT /markets/equities/watchlists/{id}/symbols — diff engine updates ticker vertices without full re-ingest.",
              ]}
            />
          </>
        )}

        {surface === "futures" && (
          <>
            <FuturesApexConnectCard onOAuthPreviewComplete={onOAuthPreviewComplete} />
            <RustFootnote
              lines={[
                "GET /oauth/apex/start → redirect_uri to Apex; GET /oauth/apex/callback → exchange code; store refresh in vault (per env paper|live).",
                "POST /markets/futures/apex/sync { roots[], positions, orders, fills } — worker pulls REST + subscribes WS for bracket updates.",
                "Contract roll: nightly job resolves front month → ingest_chunk(..., \"futures\", …) with explicit roll_from → roll_to edges.",
              ]}
            />
          </>
        )}

        {surface === "cryptocurrencies" && (
          <>
            <OauthChromeCard
              id="cryptocurrencies"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-fuchsia-600 text-xs font-bold text-white">
                  CR
                </span>
              }
              title="Cryptocurrencies"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>Per-venue pairs, funding, and optional on-chain tags — fused with equities and futures in Unified when backends are wired.</p>
                  <ul className="rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[11px] text-fuchsia-200/80">
                    <li>Venue A — REST + WS keys</li>
                    <li>Venue B — read-only subaccount</li>
                  </ul>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "POST /markets/crypto/venues[] with API keys; normalize symbol + venue id.",
                "Stream funding + trades → ingest_chunk(..., \"cryptocurrencies\", …); risk edges to futures hub.",
              ]}
            />
          </>
        )}

        {surface === "fin_news" && (
          <>
            <OauthChromeCard
              id="fin_news"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-orange-600 text-xs font-bold text-white">
                  NW
                </span>
              }
              title="News wires"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>Stack multiple headline APIs; dedupe by hash and entity-link into tickers (preview fields below).</p>
                  <label className="block text-xs">
                    <span className="text-slate-500">Wire + sentiment vendor</span>
                    <input
                      type="text"
                      readOnly
                      defaultValue="Reuters-style wire, RavenPack-style sentiment"
                      className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
                    />
                  </label>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "POST /markets/news/sources[] { vendor, api_key } → fan-in worker with per-vendor rate limits.",
                "Headline → entity resolution → edges to equities:futures nodes; store raw payload in object store.",
              ]}
            />
          </>
        )}

        {surface === "fin_market_data" && (
          <>
            <OauthChromeCard
              id="fin_market_data"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-violet-600 text-xs font-bold text-white">
                  MD
                </span>
              }
              title="Market data APIs"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>OHLCV, L2 depth, and alt-data indices from more than one vendor — conflict policy in Rust later.</p>
                  <label className="block text-xs">
                    <span className="text-slate-500">Bar resolution</span>
                    <select className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300" disabled defaultValue="1m">
                      <option>1m fused</option>
                      <option>tick (premium)</option>
                    </select>
                  </label>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "POST /markets/data/vendors[] → register pull or S3 drop targets; clock_skew_ms per vendor.",
                "Fusion job merges bars into canonical series; ingest_chunk(..., \"fin_market_data\", …) with source tag.",
              ]}
            />
          </>
        )}

        {surface === "fin_research" && (
          <>
            <OauthChromeCard
              id="fin_research"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-amber-500 text-xs font-bold text-slate-900">
                  RS
                </span>
              }
              title="Research & books"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>
                    Desk PDFs and books use the same PDF pipeline as Personal; optional <span className="font-mono">physical_id</span>{" "}
                    binds literature to this markets workspace.
                  </p>
                  <p className="font-mono text-[10px] text-slate-500">
                    PDF upload still runs from the Personal workspace; this card is a preview of how research binds to markets.
                  </p>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "Citations extracted as nodes; cross-edge to equities tickers when ISIN/CUSIP match.",
              ]}
            />
          </>
        )}
      </div>
    </div>
  );
}
