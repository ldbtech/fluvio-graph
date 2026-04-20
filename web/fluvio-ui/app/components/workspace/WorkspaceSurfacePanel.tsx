"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ConnectorId, WorkspaceSurface } from "@/lib/types";

type OAuthPhase = "form" | "busy" | "done";

type Props = {
  surface: WorkspaceSurface;
  onClose: () => void;
  pdfInputId: string;
  kgUrl: string;
  graphNodes: number;
  graphEdges: number;
  onOAuthPreviewComplete: (id: ConnectorId) => void;
  /** After Gmail sync succeeds, refresh the graph from `GET /graph`. */
  onGmailGraphRefresh?: () => void | Promise<void>;
};

function RustFootnote({ lines }: { lines: string[] }) {
  return (
    <div className="mt-8 rounded-xl border border-white/10 bg-black/30 p-4">
      <p className="mb-2 font-mono text-[10px] uppercase tracking-wider text-slate-500">
        sketch for Rust / axum
      </p>
      <ul className="space-y-1.5 font-mono text-[11px] leading-relaxed text-cyan-200/70">
        {lines.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}

function PreviewBanner() {
  return (
    <div className="mb-6 rounded-lg border border-amber-500/25 bg-amber-500/10 px-3 py-2 font-mono text-[11px] text-amber-100/90">
      Preview UI — buttons do not hit Google, Meta, etc. Wire these screens to your OAuth routes and
      ingestion workers in Rust.
    </div>
  );
}

/** Snapshot from `GET /sync/gmail/progress` (Rust `GmailSyncProgressSnapshot`). */
type GmailProgressSnapshot = {
  running: boolean;
  mode: string;
  phase: string;
  threads_done: number;
  threads_total: number;
  percent: number | null;
  chunks: number;
  error: string | null;
  result?: {
    chunks: number;
    nodes_added: number;
    structured_edges: number;
    graph_nodes: number;
    graph_edges: number;
  };
};

/** Live Gmail: `GET /connect/gmail?redirect=1`, `POST /sync/gmail` (202) + poll `/sync/gmail/progress`. */
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
  const [syncing, setSyncing] = useState(false);
  const [syncKind, setSyncKind] = useState<"incremental" | "full" | null>(null);
  const [elapsedSec, setElapsedSec] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  const [progressSnap, setProgressSnap] = useState<GmailProgressSnapshot | null>(null);
  const [maxThreads, setMaxThreads] = useState("");
  const [maxMessages, setMaxMessages] = useState("");
  const [threadQuery, setThreadQuery] = useState("");
  const [bootstrapQuery, setBootstrapQuery] = useState("");
  const pollRef = useRef<number | null>(null);

  const stopPoll = () => {
    if (pollRef.current !== null) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  };

  useEffect(() => () => stopPoll(), []);

  useEffect(() => {
    if (!syncing) {
      setElapsedSec(0);
      return;
    }
    setElapsedSec(0);
    const id = window.setInterval(() => {
      setElapsedSec((s) => s + 1);
    }, 1000);
    return () => window.clearInterval(id);
  }, [syncing]);

  const loadStatus = async () => {
    try {
      const r = await fetch(`${kgUrl}/connect/gmail/status`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const j = (await r.json()) as { connected: boolean };
      setConnected(j.connected);
      if (j.connected) onOAuthPreviewComplete("gmail");
    } catch {
      setConnected(false);
    }
  };

  useEffect(() => {
    void loadStatus();
  }, [kgUrl]);

  const startOAuth = (opts?: { forceConsent?: boolean }) => {
    const qs = new URLSearchParams({ redirect: "1" });
    if (opts?.forceConsent) qs.set("force_consent", "1");
    window.location.href = `${kgUrl}/connect/gmail?${qs.toString()}`;
  };

  const runSync = async (mode: "full" | "incremental") => {
    stopPoll();
    setSyncing(true);
    setSyncKind(mode);
    setErr(null);
    setProgressSnap(null);
    try {
      const payload: Record<string, string | number> = { mode };
      const mt = parseInt(maxThreads.trim(), 10);
      if (maxThreads.trim() !== "" && !Number.isNaN(mt) && mt > 0) payload.max_threads = mt;
      const mm = parseInt(maxMessages.trim(), 10);
      if (maxMessages.trim() !== "" && !Number.isNaN(mm) && mm > 0) payload.max_messages = mm;
      const tq = threadQuery.trim();
      if (tq) payload.thread_query = tq;
      const bq = bootstrapQuery.trim();
      if (bq) payload.bootstrap_query = bq;

      const r = await fetch(`${kgUrl}/sync/gmail`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const text = await r.text();
      if (r.status === 409) {
        throw new Error(text || "Gmail sync is already running on the server.");
      }
      if (!r.ok) throw new Error(text || `HTTP ${r.status}`);

      if (r.status === 202) {
        const poll = async () => {
          try {
            const pr = await fetch(`${kgUrl}/sync/gmail/progress`);
            if (!pr.ok) return;
            const p = (await pr.json()) as GmailProgressSnapshot;
            setProgressSnap(p);
            if (!p.running) {
              stopPoll();
              setSyncing(false);
              setSyncKind(null);
              if (p.error) {
                setErr(p.error);
              } else {
                await loadStatus();
                await onGraphRefresh?.();
              }
            }
          } catch {
            /* ignore transient poll failures */
          }
        };
        await poll();
        pollRef.current = window.setInterval(poll, 600);
        return;
      }

      await loadStatus();
      await onGraphRefresh?.();
      setSyncing(false);
      setSyncKind(null);
    } catch (e: unknown) {
      stopPoll();
      setErr(e instanceof Error ? e.message : String(e));
      setSyncing(false);
      setSyncKind(null);
      setProgressSnap(null);
    }
  };

  const syncTitle =
    syncKind === "full"
      ? "Full sync"
      : syncKind === "incremental"
        ? "Incremental sync"
        : "Sync";

  return (
    <div className="mx-auto max-w-lg">
      <div className="mb-6 rounded-lg border border-emerald-500/25 bg-emerald-500/10 px-3 py-2 font-mono text-[11px] text-emerald-100/90">
        Live · kg-engine Gmail — OAuth and sync call your Rust server at{" "}
        <code className="text-cyan-200/90">{kgUrl}</code>
      </div>
      <div className="relative min-h-[320px]">
        {syncing && (
          <div
            className="absolute inset-0 z-20 flex flex-col items-center justify-center gap-4 rounded-2xl border border-cyan-500/20 bg-[#040410]/92 px-8 py-10 text-center shadow-[inset_0_0_40px_rgba(34,211,238,0.06)] backdrop-blur-md"
            role="status"
            aria-live="polite"
            aria-busy="true"
          >
            <span className="inline-flex h-12 w-12 animate-spin rounded-full border-2 border-cyan-400/80 border-t-transparent" />
            <div className="w-full max-w-sm space-y-3">
              <p className="font-mono text-sm font-semibold tracking-tight text-cyan-100">
                Syncing Gmail — {syncTitle}
              </p>
              <div className="h-2 w-full overflow-hidden rounded-full bg-white/10">
                {progressSnap?.percent != null ? (
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-cyan-500 to-violet-500 transition-[width] duration-300 ease-out"
                    style={{ width: `${Math.min(100, Math.max(0, progressSnap.percent))}%` }}
                  />
                ) : (
                  <div className="h-full w-full animate-pulse rounded-full bg-gradient-to-r from-cyan-500/40 via-violet-500/50 to-cyan-500/40" />
                )}
              </div>
              <p className="font-mono text-[11px] text-slate-300">
                {progressSnap ? (
                  <>
                    <span className="text-cyan-200/90">{progressSnap.phase}</span>
                    {progressSnap.threads_total > 0 && (
                      <span className="text-slate-500">
                        {" "}
                        · threads {progressSnap.threads_done}/{progressSnap.threads_total}
                      </span>
                    )}
                    {progressSnap.percent != null && (
                      <span className="text-slate-500"> · {progressSnap.percent.toFixed(1)}%</span>
                    )}
                    {progressSnap.chunks > 0 && (
                      <span className="text-slate-500"> · {progressSnap.chunks} chunks</span>
                    )}
                  </>
                ) : (
                  <span className="text-slate-500">Starting…</span>
                )}
              </p>
              <p className="text-xs leading-relaxed text-slate-500">
                Server returns live progress on <code className="text-cyan-600/80">GET /sync/gmail/progress</code>.
                Large mailboxes can take many minutes — keep this tab open.
              </p>
              <p className="font-mono text-[11px] tabular-nums text-slate-500">Elapsed {elapsedSec}s</p>
            </div>
          </div>
        )}
        <div
          className={`overflow-hidden rounded-2xl border border-white/10 bg-[#0c0c18] shadow-[0_24px_80px_rgba(0,0,0,0.55)] ${syncing ? "pointer-events-none opacity-40" : ""}`}
        >
        <div className="flex items-center gap-3 border-b border-white/5 px-5 py-4">
          <span className="flex h-11 w-11 items-center justify-center rounded-full bg-white text-lg font-bold text-blue-600">
            G
          </span>
          <div>
            <p className="text-xs font-medium text-slate-400">Connect</p>
            <p className="text-lg font-semibold tracking-tight text-slate-100">Gmail</p>
          </div>
        </div>
        <div className="space-y-4 px-5 py-5 text-sm text-slate-400">
          <p>
            Read-only Gmail access: threads and messages are normalized in Rust, embedded, and written under{" "}
            <code className="text-cyan-200/80">fluvio_graphs/workspace/</code> as{" "}
            <code className="text-cyan-200/80">unified.json</code> (full graph) plus{" "}
            <code className="text-cyan-200/80">email.json</code> and{" "}
            <code className="text-cyan-200/80">pdf.json</code> slices. The file{" "}
            <code className="text-cyan-200/80">fluvio_graphs/email.json</code> at the repo root is only the CLI{" "}
            <code className="text-cyan-200/80">DomainGraph</code> placeholder — the HTTP server does not fill it.
          </p>
          <p className="font-mono text-[10px] text-slate-500">
            Status:{" "}
            {connected === null
              ? "…"
              : connected
                ? "credentials on disk"
                : "not connected"}
          </p>
          {connected && (
            <p className="text-[11px] leading-relaxed text-slate-500">
              One-time Google sign-in saves a token; use <strong className="text-slate-400">Sync incremental / Full</strong> and
              optional limits below without going through Google again. Use “Full consent again” only after revoke or to force a
              new refresh token.
            </p>
          )}
          {err && <p className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 font-mono text-[11px] text-red-200/90">{err}</p>}
        </div>
        <div className="border-t border-white/5 px-5 py-4 space-y-3">
          <button
            type="button"
            onClick={() => startOAuth()}
            disabled={syncing}
            className="w-full rounded-xl bg-white py-3 text-sm font-semibold text-slate-900 transition hover:bg-slate-200 disabled:opacity-50"
          >
            {connected ? "Reconnect Google account" : "Sign in with Google"}
          </button>
          {connected && (
            <button
              type="button"
              onClick={() => startOAuth({ forceConsent: true })}
              disabled={syncing}
              className="w-full rounded-lg border border-white/15 py-2 font-mono text-[11px] text-slate-400 transition hover:border-amber-400/30 hover:text-amber-100/90 disabled:opacity-50"
            >
              Full consent again (new refresh token)
            </button>
          )}
          <details className="rounded-lg border border-white/10 bg-white/[0.02] px-3 py-2 text-xs text-slate-400 open:pb-3">
            <summary className="cursor-pointer select-none font-mono text-[11px] text-slate-300">
              Sync scope &amp; limits (optional)
            </summary>
            <p className="mt-2 leading-relaxed text-slate-500">
              Leave blank for defaults. <span className="text-slate-400">max_threads</span> caps thread listing;
              <span className="text-slate-400"> max_messages</span> caps query list and each incremental run.
              <span className="text-slate-400"> bootstrap_query</span> applies on first incremental when no history
              is stored yet (e.g. <code className="text-cyan-600/80">newer_than:90d</code>).
            </p>
            <div className="mt-2 grid grid-cols-2 gap-2">
              <label className="col-span-1 flex flex-col gap-1 font-mono text-[10px] text-slate-500">
                max_threads
                <input
                  type="number"
                  min={1}
                  value={maxThreads}
                  onChange={(e) => setMaxThreads(e.target.value)}
                  disabled={syncing}
                  placeholder="e.g. 200"
                  className="rounded border border-white/10 bg-[#0a0a14] px-2 py-1.5 text-[11px] text-slate-200 placeholder:text-slate-600"
                />
              </label>
              <label className="col-span-1 flex flex-col gap-1 font-mono text-[10px] text-slate-500">
                max_messages
                <input
                  type="number"
                  min={1}
                  value={maxMessages}
                  onChange={(e) => setMaxMessages(e.target.value)}
                  disabled={syncing}
                  placeholder="e.g. 500"
                  className="rounded border border-white/10 bg-[#0a0a14] px-2 py-1.5 text-[11px] text-slate-200 placeholder:text-slate-600"
                />
              </label>
              <label className="col-span-2 flex flex-col gap-1 font-mono text-[10px] text-slate-500">
                thread_query (Gmail <code className="text-cyan-600/70">q</code> for full thread list)
                <input
                  type="text"
                  value={threadQuery}
                  onChange={(e) => setThreadQuery(e.target.value)}
                  disabled={syncing}
                  placeholder="newer_than:30d -in:spam"
                  className="rounded border border-white/10 bg-[#0a0a14] px-2 py-1.5 text-[11px] text-slate-200 placeholder:text-slate-600"
                />
              </label>
              <label className="col-span-2 flex flex-col gap-1 font-mono text-[10px] text-slate-500">
                bootstrap_query (first incremental only, if no history yet)
                <input
                  type="text"
                  value={bootstrapQuery}
                  onChange={(e) => setBootstrapQuery(e.target.value)}
                  disabled={syncing}
                  placeholder="newer_than:180d"
                  className="rounded border border-white/10 bg-[#0a0a14] px-2 py-1.5 text-[11px] text-slate-200 placeholder:text-slate-600"
                />
              </label>
            </div>
          </details>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => void runSync("incremental")}
              disabled={syncing}
              className="flex-1 rounded-xl border border-cyan-400/35 bg-cyan-500/10 py-2.5 font-mono text-xs font-medium text-cyan-100 transition hover:bg-cyan-500/20 disabled:opacity-50"
            >
              {syncing && syncKind === "incremental" ? "Syncing…" : "Sync incremental"}
            </button>
            <button
              type="button"
              onClick={() => void runSync("full")}
              disabled={syncing}
              className="flex-1 rounded-xl border border-violet-400/35 bg-violet-500/10 py-2.5 font-mono text-xs font-medium text-violet-100 transition hover:bg-violet-500/20 disabled:opacity-50"
            >
              {syncing && syncKind === "full" ? "Syncing…" : "Sync full"}
            </button>
          </div>
          <p className="text-center font-mono text-[10px] text-slate-500">
            OAuth callback: <code className="text-cyan-600/80">GET /connect/gmail/callback</code>
          </p>
        </div>
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
      <div className="overflow-hidden rounded-2xl border border-white/10 bg-[#0c0c18] shadow-[0_24px_80px_rgba(0,0,0,0.55)]">
        <div className="flex items-center gap-3 border-b border-white/5 px-5 py-4">
          {brand}
          <div>
            <p className="text-xs font-medium text-slate-400">Connect</p>
            <p className="text-lg font-semibold tracking-tight text-slate-100">{title}</p>
          </div>
        </div>
        <div className="px-5 py-5">{body}</div>
        {oauthPhase === "form" && (
          <div className="border-t border-white/5 px-5 py-4">
            <button
              type="button"
              onClick={runOAuthPreview}
              className="w-full rounded-xl bg-white py-3 text-sm font-semibold text-slate-900 transition hover:bg-slate-200"
            >
              Continue (preview flow)
            </button>
            <p className="mt-2 text-center font-mono text-[10px] text-slate-500">
              Production: 302 to IdP → <code className="text-cyan-600/80">/oauth/{id}/callback</code>
            </p>
          </div>
        )}
        {oauthPhase === "busy" && (
          <div className="border-t border-white/5 px-5 py-8 text-center font-mono text-sm text-slate-400">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-cyan-400 border-t-transparent" />{" "}
            Simulating redirect & token exchange…
          </div>
        )}
        {oauthPhase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-500/5 px-5 py-4">
            <p className="font-mono text-xs text-emerald-200/90">
              Mock session stored. Next: background job pulls deltas → <code className="text-emerald-300">ingest_chunk(domain)</code>.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function fieldClass(disabled?: boolean) {
  return `mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-200 outline-none placeholder:text-slate-600 focus:border-cyan-400/35 ${
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
              Mock: watchlist “{watchlistName}” and order sync flags stored. Worker would call vendor REST/WebSocket with
              rotated secrets from vault.
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
              Mock: Apex session bound ({env === "apex_paper" ? "paper" : "live"}). Roots & sync flags registered — next
              step is real OAuth callback + encrypted refresh token in Rust.
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
              Mock session active. Next: route inbound messages to <code className="text-emerald-300">ingest_chunk</code> with
              consent flags.
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

function WebGraphSetupCard({ onOAuthPreviewComplete }: { onOAuthPreviewComplete: (id: ConnectorId) => void }) {
  const [phase, setPhase] = useState<OAuthPhase>("form");

  const run = () => {
    if (phase !== "form") return;
    setPhase("busy");
    window.setTimeout(() => {
      setPhase("done");
      onOAuthPreviewComplete("web");
    }, 1100);
  };

  return (
    <div className="mx-auto max-w-lg">
      <PreviewBanner />
      <div className="overflow-hidden rounded-2xl border border-amber-500/25 bg-[#0c0c18] shadow-[0_24px_80px_rgba(245,158,11,0.08)]">
        <div className="border-b border-white/5 px-5 py-4">
          <div className="flex items-center gap-3">
            <span className="flex h-11 w-11 items-center justify-center rounded-lg border border-amber-500/40 bg-amber-500/15 font-mono text-xs font-bold text-amber-200">
              WEB
            </span>
            <div>
              <p className="text-xs font-medium text-amber-200/80">Crawl graph</p>
              <p className="text-lg font-semibold tracking-tight text-slate-100">Website → graph + PDF learnings</p>
            </div>
          </div>
        </div>
        <div className="space-y-4 px-5 py-5 text-sm text-slate-400">
          <p>
            Paste a site root; workers fetch HTML, scripts, routes, and headers into one graph. Attach cybersecurity PDFs
            (or any manuals) into the <span className="font-mono text-amber-200/90">same graph_id</span> so findings
            can link literature to live surface area — e.g. hunt CSRF/XSS gaps against your real endpoints.
          </p>
          <label className="block text-xs">
            <span className="text-slate-500">Site URL</span>
            <input
              type="url"
              readOnly
              defaultValue="https://myproduct.example/login"
              className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
            />
          </label>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block text-xs opacity-80">
              <span className="text-slate-500">Max depth</span>
              <input type="range" disabled className="mt-1 w-full" defaultValue={40} />
            </label>
            <label className="flex items-center gap-2 rounded-lg border border-white/10 bg-black/20 p-3 text-xs opacity-80">
              <input type="checkbox" disabled defaultChecked />
              Same-origin only (recommended)
            </label>
          </div>
          <div>
            <p className="mb-2 font-mono text-[10px] uppercase tracking-wider text-slate-500">PDFs merged into this crawl</p>
            <div className="flex flex-wrap gap-2">
              {["owasp-csrf-sheet.pdf", "corp-baseline-v3.pdf", "threat-model-notes.pdf"].map((name) => (
                <span
                  key={name}
                  className="rounded-full border border-emerald-500/25 bg-emerald-500/10 px-3 py-1 font-mono text-[10px] text-emerald-200/90"
                >
                  {name}
                </span>
              ))}
            </div>
            <p className="mt-2 font-mono text-[10px] text-slate-600">
              Production: POST PDFs with <code className="text-cyan-700/80">graph_id=crawl:&lt;site&gt;</code> so chunks
              sit beside DOM/route nodes for joint retrieval.
            </p>
          </div>
          <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-100/85">
            <span className="font-mono text-[10px] uppercase tracking-wider text-amber-400/80">agents (later)</span>
            <p className="mt-1">
              Spin a <strong className="text-amber-100">web scout</strong> for internet context, an{" "}
              <strong className="text-amber-100">error radar</strong> for contradictions / risky patterns, and a{" "}
              <strong className="text-amber-100">remediation runner</strong> for draft fixes — all behind approvals in
              Rust.
            </p>
          </div>
        </div>
        {phase === "form" && (
          <div className="border-t border-white/5 px-5 py-4">
            <button
              type="button"
              onClick={run}
              className="w-full rounded-xl bg-gradient-to-r from-amber-500 to-orange-600 py-3 text-sm font-semibold text-slate-950 transition hover:opacity-95"
            >
              Start crawl (preview)
            </button>
          </div>
        )}
        {phase === "busy" && (
          <div className="border-t border-white/5 px-5 py-8 text-center font-mono text-sm text-slate-400">
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-amber-400 border-t-transparent" />{" "}
            Fetching sitemap, parsing bundles, staging PDF merge slots…
          </div>
        )}
        {phase === "done" && (
          <div className="border-t border-emerald-500/20 bg-emerald-500/5 px-5 py-4">
            <p className="font-mono text-xs text-emerald-200/90">
              Mock crawl registered. Open <strong className="text-emerald-100">Workspace brain → Web</strong> to see
              route/finding nodes; attach real PDFs once <code className="text-emerald-300">/ingest/web</code> exists.
            </p>
          </div>
        )}
      </div>
      <RustFootnote
        lines={[
          "POST /ingest/web/crawl { url, depth, same_origin } → job queue; store graph_id keyed by normalized origin.",
          "POST /ingest/web/attach-pdf { graph_id, file } reuses PDF pipeline with domain=\"web\" + parent_crawl_id.",
          "GET /graph?domain=web&graph_id=… for fused site + literature view; agents call tool APIs with audit logs.",
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
}: Props) {
  return (
    <div
      className="absolute inset-0 z-40 flex flex-col overflow-y-auto bg-[#050510]/97 p-6 backdrop-blur-md"
      role="dialog"
      aria-modal="true"
      aria-labelledby="surface-title"
    >
      <header className="mx-auto mb-6 flex w-full max-w-3xl shrink-0 items-start justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] uppercase tracking-[0.25em] text-slate-500">workspace</p>
          <h2 id="surface-title" className="mt-1 text-xl font-semibold text-slate-100">
            {surface === "documents" && "Documents · PDF ingestion"}
            {surface === "gmail" && "Gmail"}
            {surface === "spotify" && "Spotify"}
            {surface === "github" && "GitHub"}
            {surface === "calendar" && "Google Calendar"}
            {surface === "whatsapp" && "WhatsApp"}
            {surface === "slack" && "Slack"}
            {surface === "notion" && "Notion"}
            {surface === "web" && "Website crawl graph"}
            {surface === "equities" && "Stocks & equities"}
            {surface === "futures" && "Futures"}
            {surface === "cryptocurrencies" && "Cryptocurrencies"}
            {surface === "fin_news" && "News wires"}
            {surface === "fin_market_data" && "Market data APIs"}
            {surface === "fin_research" && "Research & books"}
          </h2>
          <p className="mt-1 max-w-xl text-sm text-slate-500">
            This is the connect experience you ship later; layout and fields mirror what you will persist and sync from
            the server.
          </p>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="shrink-0 rounded-full border border-white/15 px-4 py-2 font-mono text-xs text-slate-300 transition hover:border-cyan-400/40 hover:text-white"
        >
          back to graph
        </button>
      </header>

      <div className="mx-auto w-full max-w-3xl flex-1 pb-12">
        {surface === "documents" && (
          <div>
            <PreviewBanner />
            <div className="grid gap-6 lg:grid-cols-[1fr_280px]">
              <div className="rounded-2xl border border-emerald-500/20 bg-emerald-500/[0.04] p-6">
                <h3 className="text-sm font-medium text-emerald-100">Ingest pipeline (live today)</h3>
                <p className="mt-2 text-sm leading-relaxed text-slate-400">
                  Multipart upload, mmap chunking, embeddings, and graph persistence. This block is what operators see
                  before upload.
                </p>
                <dl className="mt-5 space-y-3 font-mono text-[11px] text-slate-400">
                  <div className="flex justify-between gap-4 border-b border-white/5 pb-2">
                    <dt>Endpoint</dt>
                    <dd className="text-right text-cyan-200/80">POST {kgUrl}/ingest/pdf</dd>
                  </div>
                  <div className="flex justify-between gap-4 border-b border-white/5 pb-2">
                    <dt>Body</dt>
                    <dd className="text-right">multipart field `file` (.pdf)</dd>
                  </div>
                  <div className="flex justify-between gap-4 border-b border-white/5 pb-2">
                    <dt>Graph</dt>
                    <dd className="text-right text-cyan-200/80">GET {kgUrl}/graph</dd>
                  </div>
                  <div className="flex justify-between gap-4">
                    <dt>Current graph</dt>
                    <dd className="text-right text-slate-200">
                      {graphNodes} nodes · {graphEdges} edges
                    </dd>
                  </div>
                </dl>
                <div className="mt-6 flex flex-wrap gap-3">
                  <label
                    htmlFor={pdfInputId}
                    className="inline-flex cursor-pointer items-center justify-center rounded-xl bg-gradient-to-r from-emerald-400 to-cyan-400 px-6 py-3 font-mono text-sm font-semibold text-slate-950 shadow-lg shadow-emerald-500/20"
                  >
                    Choose PDF…
                  </label>
                  <p className="flex min-w-[12rem] flex-1 items-center text-xs text-slate-500">
                    Or use the <span className="mx-1 font-mono text-slate-400">+</span> shortcut in the sidebar for a
                    quick upload without leaving the graph view.
                  </p>
                </div>
              </div>
              <aside className="space-y-4 rounded-2xl border border-white/10 bg-[#0a0a14] p-5">
                <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">future knobs</p>
                <div className="space-y-3 text-xs text-slate-500">
                  <label className="block">
                    <span className="text-slate-400">Chunk target tokens</span>
                    <input
                      type="range"
                      disabled
                      className="mt-1 w-full opacity-40"
                      defaultValue={50}
                    />
                  </label>
                  <label className="flex items-center gap-2 opacity-50">
                    <input type="checkbox" disabled defaultChecked />
                    Deduplicate pages on re-upload
                  </label>
                  <label className="flex items-center gap-2 opacity-50">
                    <input type="checkbox" disabled />
                    OCR for scanned PDFs (later)
                  </label>
                </div>
              </aside>
            </div>
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
                "GET /connect/gmail → JSON { url, state }; ?redirect=1 → 302 to Google consent.",
                "POST /sync/gmail → 202 Accepted; poll GET /sync/gmail/progress for phase, threads_done/total, percent, chunks.",
              ]}
            />
          </>
        )}

        {surface === "spotify" && (
          <>
            <OauthChromeCard
              id="spotify"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full bg-[#1db954] text-sm font-bold text-black">
                  ♪
                </span>
              }
              title="Spotify"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <p>Map recent plays, playlists, and audio features into artist/track nodes for a listening layer on your graph.</p>
                  <ul className="space-y-2 rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[11px] text-green-200/80">
                    <li>user-read-recently-played</li>
                    <li>playlist-read-private</li>
                  </ul>
                  <p className="font-mono text-[10px] text-slate-500">
                    Production: poll or webhooks where available; rate-limit per user in Rust.
                  </p>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "OAuth PKCE in axum; store tokens in token_store (see ingestion_registry/email pattern).",
                "Cron: spotify::recently_played → ingest_chunk(..., \"music\", …) with ISRC / track URI edges.",
              ]}
            />
          </>
        )}

        {surface === "github" && (
          <>
            <OauthChromeCard
              id="github"
              brand={
                <span className="flex h-11 w-11 items-center justify-center rounded-full border border-white/15 bg-[#24292f] text-xs font-bold text-white">
                  GH
                </span>
              }
              title="GitHub"
              body={
                <div className="space-y-4 text-sm text-slate-400">
                  <div className="flex gap-4 text-xs">
                    <label className="flex flex-1 cursor-pointer items-center gap-2 rounded-lg border border-violet-400/30 bg-violet-500/10 p-3">
                      <input type="radio" name="gh" defaultChecked readOnly className="accent-violet-400" />
                      GitHub App (recommended)
                    </label>
                    <label className="flex flex-1 cursor-pointer items-center gap-2 rounded-lg border border-white/10 p-3 opacity-60">
                      <input type="radio" name="gh" readOnly className="accent-slate-500" />
                      Fine-grained PAT
                    </label>
                  </div>
                  <label className="block text-xs">
                    <span className="text-slate-500">Org / repos</span>
                    <input
                      type="text"
                      readOnly
                      defaultValue="acme-corp/api, acme-corp/kg-engine"
                      className="mt-1 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-xs text-slate-300"
                    />
                  </label>
                  <p className="rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[10px] text-slate-500">
                    Webhook URL (server prints on boot):{" "}
                    <code className="text-violet-300/90">https://&lt;host&gt;/hooks/github</code>
                  </p>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "Verify X-Hub-Signature-256 in axum middleware; map payload → PR / push / issue nodes.",
                "POST /ingest/github/sync?repo=… optional full scan using git2 or GitHub API tree.",
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

        {surface === "web" && <WebGraphSetupCard onOAuthPreviewComplete={onOAuthPreviewComplete} />}

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
                  <p>Per-venue pairs, funding, and optional on-chain tags — fused with equities/futures in Unified (mock).</p>
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
                  <p>Stack multiple headline APIs; dedupe by hash and entity-link into tickers (mock form).</p>
                  <label className="block text-xs">
                    <span className="text-slate-500">Wire + sentiment vendor</span>
                    <input
                      type="text"
                      readOnly
                      defaultValue="Reuters (mock), RavenPack (mock)"
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
                    Desk PDFs and books use the same PDF pipeline as Personal; optional <span className="font-mono">graph_id</span>{" "}
                    binds literature to this markets workspace.
                  </p>
                  <p className="font-mono text-[10px] text-slate-500">
                    Upload still runs from Personal workspace today; here you only mock-enable the research slice.
                  </p>
                </div>
              }
              onOAuthPreviewComplete={onOAuthPreviewComplete}
            />
            <RustFootnote
              lines={[
                "POST /ingest/pdf with header X-Workspace-Kind: invest binds document to markets meta ledger slot.",
                "Citations extracted as nodes; cross-edge to equities tickers when ISIN/CUSIP match.",
              ]}
            />
          </>
        )}
      </div>
    </div>
  );
}
