"use client";

import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useRef, useState } from "react";

const LS_REPLY_ASSISTANT_EXPANDED = "fluvio_reply_assistant_expanded";

export type GmailAgentSendModeUi = "always_review" | "auto_when_confident";

export type GmailAgentContextSourcesUi = {
  account_profile: boolean;
  uploads: boolean;
  github_codebase: boolean;
  ingested_email: boolean;
  twin_notes: boolean;
  network_connections: boolean;
};

const DEFAULT_SOURCES: GmailAgentContextSourcesUi = {
  account_profile: true,
  uploads: true,
  github_codebase: true,
  ingested_email: true,
  twin_notes: true,
  network_connections: true,
};

function mergeSources(s: Partial<GmailAgentContextSourcesUi> | undefined): GmailAgentContextSourcesUi {
  return { ...DEFAULT_SOURCES, ...(s ?? {}) };
}

type AgentSettingsWire = {
  send_mode?: string;
  context_sources?: Partial<GmailAgentContextSourcesUi>;
};

type ReviewDraftWire = {
  gmail_message_id: string;
  thread_id: string | null;
  subject_hint: string | null;
  reply_proposal: string | null;
  detail: string | null;
  processed_at: string;
};

function parseSendMode(send_mode?: string): GmailAgentSendModeUi {
  return send_mode === "auto_when_confident" ? "auto_when_confident" : "always_review";
}

function applySettingsWire(
  setters: {
    setSendMode: (v: GmailAgentSendModeUi) => void;
    setCtx: (v: GmailAgentContextSourcesUi) => void;
  },
  j: AgentSettingsWire,
) {
  setters.setSendMode(parseSendMode(j.send_mode));
  setters.setCtx(mergeSources(j.context_sources));
}

export type GmailReplyAgentPanelProps = {
  disabled?: boolean;
  kgEngineBaseUrl: string;
  bearerHeaders: () => HeadersInit;
  jsonHeaders: () => HeadersInit;
  onBanner?: (msg: string | null) => void;
};

/** Shared panel chrome — aligns with Workspace email surface */
const SECTION = "rounded-[22px] border border-white/[0.045] bg-zinc-950/40 px-5 py-5 ring-1 ring-white/[0.03]";
const LABEL = "text-[11px] font-medium uppercase tracking-[0.13em] text-zinc-500";

export function GmailReplyAgentPanel({
  disabled = false,
  kgEngineBaseUrl,
  bearerHeaders,
  jsonHeaders,
  onBanner,
}: GmailReplyAgentPanelProps) {
  const [busy, setBusy] = useState(false);
  const [loadErr, setLoadErr] = useState<string | null>(null);

  const [sendMode, setSendMode] = useState<GmailAgentSendModeUi>("always_review");
  const [ctx, setCtx] = useState<GmailAgentContextSourcesUi>(DEFAULT_SOURCES);

  const [lastPreview, setLastPreview] = useState<string | null>(null);

  const [reviewsOpen, setReviewsOpen] = useState(false);
  const [reviewsBusy, setReviewsBusy] = useState(false);
  const [reviewsErr, setReviewsErr] = useState<string | null>(null);
  const [reviews, setReviews] = useState<ReviewDraftWire[]>([]);
  const [pickedId, setPickedId] = useState<string | null>(null);

  /** Collapsed by default; remember last choice in localStorage (`1` expanded, missing/`0` collapsed). */
  const [expanded, setExpanded] = useState(false);
  const replyPrefsHydrated = useRef(false);

  useEffect(() => {
    if (!replyPrefsHydrated.current) {
      replyPrefsHydrated.current = true;
      try {
        if (window.localStorage.getItem(LS_REPLY_ASSISTANT_EXPANDED) === "1") setExpanded(true);
      } catch {
        /* ignore */
      }
      return;
    }
    try {
      window.localStorage.setItem(LS_REPLY_ASSISTANT_EXPANDED, expanded ? "1" : "0");
    } catch {
      /* private mode etc. */
    }
  }, [expanded]);

  const bearerRef = useRef(bearerHeaders);
  const jsonRef = useRef(jsonHeaders);
  const onBannerRef = useRef(onBanner);
  bearerRef.current = bearerHeaders;
  jsonRef.current = jsonHeaders;
  onBannerRef.current = onBanner;

  const baseUrl = kgEngineBaseUrl.replace(/\/$/, "");

  const picked = pickedId ? reviews.find((r) => r.gmail_message_id === pickedId) : undefined;

  useEffect(() => {
    if (disabled || !baseUrl.trim()) return;
    let cancelled = false;
    setLoadErr(null);
    setBusy(true);
    void (async () => {
      try {
        const res = await fetch(`${baseUrl}/gmail/agent/settings`, {
          headers: bearerRef.current(),
        });
        const text = await res.text().catch(() => "");
        if (!res.ok) throw new Error(text ? text.slice(0, 280) : `HTTP ${res.status}`);
        const j = JSON.parse(text) as AgentSettingsWire;
        if (!cancelled) applySettingsWire({ setSendMode, setCtx }, j);
      } catch (e: unknown) {
        if (!cancelled) {
          const msg = e instanceof Error ? e.message : String(e);
          setLoadErr(msg);
          onBannerRef.current?.(msg);
        }
      } finally {
        if (!cancelled) setBusy(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [disabled, baseUrl]);

  const persistAgentSettings = async (next: {
    send_mode: GmailAgentSendModeUi;
    context_sources: GmailAgentContextSourcesUi;
  }) => {
    const body = JSON.stringify(next);
    const res = await fetch(`${baseUrl}/gmail/agent/settings`, {
      method:  "PUT",
      headers: jsonRef.current(),
      body,
    });
    const text = await res.text().catch(() => "");
    if (!res.ok) throw new Error(text ? text.slice(0, 280) : `Save HTTP ${res.status}`);
    const j = JSON.parse(text) as AgentSettingsWire;
    applySettingsWire({ setSendMode, setCtx }, j);
  };

  const save = async () => {
    setLoadErr(null);
    setBusy(true);
    try {
      await persistAgentSettings({ send_mode: sendMode, context_sources: ctx });
      onBannerRef.current?.(null);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setLoadErr(msg);
      onBannerRef.current?.(msg);
    } finally {
      setBusy(false);
    }
  };

  const preview = async () => {
    setLoadErr(null);
    setBusy(true);
    setLastPreview(null);
    try {
      await persistAgentSettings({ send_mode: sendMode, context_sources: ctx });
      onBannerRef.current?.(null);
      const res = await fetch(`${baseUrl}/gmail/agent/run`, {
        method:  "POST",
        headers: jsonRef.current(),
        body:    JSON.stringify({ dry_run: true, max_candidates: 5 }),
      });
      const text = await res.text().catch(() => "");
      if (!res.ok) throw new Error(text ? text.slice(0, 280) : `Preview HTTP ${res.status}`);
      const j = JSON.parse(text) as {
        items?: Array<{
          outcome: string;
          reply_proposal?: string | null;
          detail?: string | null;
        }>;
      };
      const summary = (j.items ?? [])
        .slice(0, 12)
        .map((row) =>
          `${row.outcome}${row.reply_proposal ? ` — preview: ${row.reply_proposal.slice(0, 120)}…` : ""}${
            row.detail ? ` (${row.detail.slice(0, 80)})` : ""
          }`,
        )
        .join("\n");
      setLastPreview(summary || "(no inbox candidates in this preview pass)");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setLoadErr(msg);
      onBannerRef.current?.(msg);
    } finally {
      setBusy(false);
    }
  };

  const openReviewsModal = async () => {
    setReviewsOpen(true);
    setReviewsErr(null);
    setReviews([]);
    setPickedId(null);
    setReviewsBusy(true);
    try {
      const res = await fetch(`${baseUrl}/gmail/agent/reviews?limit=80`, {
        headers: bearerRef.current(),
      });
      const text = await res.text().catch(() => "");
      if (!res.ok) throw new Error(text ? text.slice(0, 280) : `Reviews HTTP ${res.status}`);
      const j = JSON.parse(text) as { items?: ReviewDraftWire[] };
      const items = Array.isArray(j.items) ? j.items : [];
      setReviews(items);
      if (items[0]?.gmail_message_id) setPickedId(items[0].gmail_message_id);
    } catch (e: unknown) {
      setReviewsErr(e instanceof Error ? e.message : String(e));
    } finally {
      setReviewsBusy(false);
    }
  };

  const row = (
    label: string,
    hint: string,
    key: keyof GmailAgentContextSourcesUi,
  ) => (
    <label
      className="flex cursor-pointer gap-4 rounded-2xl border border-transparent px-1 py-2.5 transition hover:border-white/[0.05] hover:bg-white/[0.02]"
      style={{ WebkitTapHighlightColor: "transparent" }}
    >
      <input
        type="checkbox"
        className="mt-0.5 size-[18px] shrink-0 rounded border-white/25 bg-transparent accent-zinc-100"
        checked={ctx[key]}
        disabled={disabled || busy}
        onChange={(e) => setCtx((c) => ({ ...c, [key]: e.target.checked }))}
      />
      <span className="min-w-0">
        <span className="block text-[15px] font-medium tracking-[-0.02em] text-zinc-100">{label}</span>
        <span className="mt-1 block text-[13px] leading-snug tracking-[-0.01em] text-zinc-500">{hint}</span>
      </span>
    </label>
  );

  function gmailThreadUrl(threadId: string): string {
    return `https://mail.google.com/mail/u/0/#inbox/${encodeURIComponent(threadId)}`;
  }

  const sourceCountOn = (
    Object.keys(DEFAULT_SOURCES) as (keyof GmailAgentContextSourcesUi)[]
  ).filter((k) => ctx[k]).length;
  const modeShort =
    sendMode === "auto_when_confident" ? "Auto-send when confident" : "Always review";
  const collapsedSummary = `${modeShort} · ${sourceCountOn}/${Object.keys(DEFAULT_SOURCES).length} knowledge sources`;

  const panelBodyId = "reply-assistant-panel-body";

  return (
    <section className={`${SECTION} overflow-hidden`} aria-label="Gmail reply agent">
      <div className="-mx-[1px] border-b border-white/[0.05] px-[1px] pb-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <button
            type="button"
            id={`${panelBodyId}-toggle`}
            aria-expanded={expanded}
            aria-controls={panelBodyId}
            onClick={() => setExpanded((x) => !x)}
            className="flex min-w-0 flex-1 items-start gap-3 rounded-2xl p-2 text-left -m-2 transition-colors hover:bg-white/[0.035] [-webkit-tap-highlight-color:transparent]"
          >
            <span
              className={`mt-1 inline-flex shrink-0 text-zinc-400 transition-transform duration-200 ease-out ${expanded ? "rotate-180" : ""}`}
              aria-hidden
            >
              <svg width={20} height={20} viewBox="0 0 24 24" fill="none" aria-hidden className="shrink-0">
                <path
                  stroke="currentColor"
                  strokeWidth={1.6}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="m7 10 5 5 5-5"
                />
              </svg>
            </span>
            <span className="min-w-0 pt-px">
              <span className="block text-[22px] font-semibold tracking-[-0.035em] text-zinc-50">Reply assistant</span>
              <span className="mt-2 block max-w-xl text-[15px] leading-[1.52] tracking-[-0.015em] text-zinc-500">
                {!expanded ? (
                  collapsedSummary
                ) : (
                  <>
                    Runs on a server timer ({""}
                    <span className="font-mono text-[13px] text-zinc-400">GMAIL_AGENT_AUTO_POLL_INTERVAL_SECS</span>
                    {""}). Uses only sources you enable. Gmail needs send scope once — reconnect if you linked before send shipped.
                  </>
                )}
              </span>
              <span className="mt-2 block text-[12px] text-zinc-600">
                {expanded ? "Tap header to shrink" : "Tap header to expand send mode, sources, and preview"}
              </span>
            </span>
          </button>
          <button
            type="button"
            disabled={disabled || busy}
            onClick={() => void openReviewsModal()}
            className="h-11 shrink-0 rounded-full border border-white/[0.1] bg-white/[0.07] px-5 text-[14px] font-semibold tracking-[-0.02em] text-zinc-100 transition hover:bg-white/[0.1] disabled:pointer-events-none disabled:opacity-35"
          >
            Review drafts
          </button>
        </div>
      </div>

      {loadErr ? (
        <p className="mt-6 rounded-2xl border border-red-500/18 bg-red-950/28 px-4 py-3 text-[14px] text-red-50/95">{loadErr}</p>
      ) : null}

      <AnimatePresence initial={false}>
        {expanded ? (
          <motion.div
            key="reply-assistant-body"
            id={panelBodyId}
            role="region"
            aria-labelledby={`${panelBodyId}-toggle`}
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.28, ease: [0.33, 1, 0.68, 1] }}
            className="overflow-hidden"
          >
            <div className="pt-1">
              <fieldset disabled={disabled || busy} className="mt-7 space-y-3">
                <legend className={LABEL}>Send behaviour</legend>
                <div className="space-y-2 pt-2">
                  <label className="flex cursor-pointer items-start gap-3.5 rounded-2xl border border-white/[0.045] bg-white/[0.02] px-4 py-4 transition hover:bg-white/[0.035]">
                    <input
                      type="radio"
                      className="mt-1.5 accent-zinc-100"
                      name="gmail-send-mode"
                      checked={sendMode === "always_review"}
                      onChange={() => setSendMode("always_review")}
                    />
                    <span>
                      <span className="block text-[15px] font-semibold tracking-[-0.02em] text-zinc-100">Always review</span>
                      <span className="mt-1 block text-[13px] leading-relaxed tracking-[-0.01em] text-zinc-500">
                        Proposals queue for you — nothing sends from Fluvio until you switch modes or reply in Gmail.
                      </span>
                    </span>
                  </label>
                  <label className="flex cursor-pointer items-start gap-3.5 rounded-2xl border border-white/[0.045] bg-white/[0.02] px-4 py-4 transition hover:bg-white/[0.035]">
                    <input
                      type="radio"
                      className="mt-1.5 accent-zinc-100"
                      name="gmail-send-mode"
                      checked={sendMode === "auto_when_confident"}
                      onChange={() => setSendMode("auto_when_confident")}
                    />
                    <span>
                      <span className="block text-[15px] font-semibold tracking-[-0.02em] text-zinc-100">
                        Auto-send when confident
                      </span>
                      <span className="mt-1 block text-[13px] leading-relaxed tracking-[-0.01em] text-zinc-500">
                        Sends when estimated confidence ≥ ~0.78. Lower scores stay as drafts you can review.
                      </span>
                    </span>
                  </label>
                </div>
              </fieldset>

              <fieldset disabled={disabled || busy} className="mt-10 space-y-2">
                <legend className={LABEL}>Knowledge for replies</legend>
                <div className="mt-5 space-y-1">
                  {row(
                    "Account profile",
                    "Name, email, phone from your Fluvio account.",
                    "account_profile",
                  )}
                  {row("Uploads", "Indexed PDF and video chunks in Surreal.", "uploads")}
                  {row("GitHub · codebase", "Current codebase ingest slices.", "github_codebase")}
                  {row(
                    "Ingested mail",
                    "Earlier email-derived graph notes (not the live thread body).",
                    "ingested_email",
                  )}
                  {row("Twin notes", "Other Surreal zones outside core buckets.", "twin_notes")}
                  {row("Network connections", "NFC links and matched shared-zone excerpts.", "network_connections")}
                </div>
              </fieldset>

              <div className="mt-10 flex flex-wrap gap-3">
                <button
                  type="button"
                  disabled={disabled || busy}
                  onClick={() => void save()}
                  className="h-12 rounded-full bg-white px-7 text-[15px] font-semibold tracking-[-0.022em] text-zinc-950 transition hover:bg-zinc-100 disabled:pointer-events-none disabled:opacity-35"
                >
                  {busy ? "Saving…" : "Save"}
                </button>
                <button
                  type="button"
                  disabled={disabled || busy}
                  onClick={() => void preview()}
                  className="h-12 rounded-full border border-white/[0.14] px-7 text-[15px] font-medium tracking-[-0.018em] text-zinc-200 transition hover:bg-white/[0.05] disabled:pointer-events-none disabled:opacity-35"
                >
                  Preview (dry run)
                </button>
              </div>
              <p className="mt-5 text-[13px] leading-relaxed tracking-[-0.01em] text-zinc-600">
                Dry run calls Claude without recording message ids or changing Gmail state. Saving updates what the next timed pass
                uses.
              </p>

              {lastPreview ? (
                <pre className="mt-6 max-h-52 overflow-auto whitespace-pre-wrap rounded-2xl border border-white/[0.05] bg-black/35 px-4 py-4 font-mono text-[11px] leading-relaxed tracking-tight text-zinc-400">
                  {lastPreview}
                </pre>
              ) : null}
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>

      {reviewsOpen ? (
        <div className="fixed inset-0 z-[90] flex items-center justify-center p-6">
          <button
            type="button"
            className="absolute inset-0 bg-black/76 backdrop-blur-md"
            aria-label="Dismiss"
            onClick={() => setReviewsOpen(false)}
          />
          <div
            role="dialog"
            aria-modal="true"
            aria-label="AI drafts"
            className="relative z-[91] flex max-h-[92vh] w-full max-w-[880px] flex-col overflow-hidden rounded-[26px] border border-white/[0.065] bg-zinc-950 shadow-[0_40px_100px_-32px_rgba(0,0,0,0.9)] ring-1 ring-white/[0.04]"
          >
            <div className="flex shrink-0 items-center justify-between border-b border-white/[0.05] px-6 py-[18px]">
              <p className="text-[17px] font-semibold tracking-[-0.03em] text-zinc-50">Review drafts</p>
              <button
                type="button"
                className="rounded-full px-4 py-2 text-[14px] font-medium text-zinc-400 transition hover:bg-white/[0.06] hover:text-zinc-200"
                onClick={() => setReviewsOpen(false)}
              >
                Done
              </button>
            </div>
            <div className="min-h-[200px] flex-1 overflow-hidden">
              {reviewsBusy ? (
                <p className="px-8 py-10 text-[15px] text-zinc-500">Loading drafts…</p>
              ) : reviewsErr ? (
                <p className="px-8 py-10 text-[15px] text-red-200/95">{reviewsErr}</p>
              ) : reviews.length === 0 ? (
                <p className="px-8 py-10 text-[15px] leading-relaxed text-zinc-500">
                  Nothing in the queue yet. New mail handled in review mode (or below auto-send confidence) shows up here.
                </p>
              ) : (
                <div className="grid max-h-[min(76vh,780px)] grid-cols-1 divide-y divide-white/[0.05] sm:grid-cols-[minmax(0,268px)_1fr] sm:divide-x sm:divide-y-0">
                  <div className="max-h-[min(76vh,780px)] overflow-y-auto pt-3 sm:border-r sm:border-white/[0.045]">
                    {reviews.map((r) => (
                      <button
                        key={r.gmail_message_id}
                        type="button"
                        onClick={() => setPickedId(r.gmail_message_id)}
                        className={`flex w-full flex-col gap-1 border-b border-white/[0.04] px-5 py-[14px] text-left transition last:border-b-0 hover:bg-white/[0.03] sm:border-b-white/[0.04] ${
                          pickedId === r.gmail_message_id ? "bg-white/[0.06]" : ""
                        }`}
                      >
                        <span className="line-clamp-2 text-[14px] font-semibold tracking-[-0.02em] text-zinc-100">
                          {r.subject_hint?.trim() || "(no subject)"}
                        </span>
                        <span className="text-[12px] text-zinc-500">{new Date(r.processed_at).toLocaleString()}</span>
                      </button>
                    ))}
                  </div>
                  <div className="max-h-[min(76vh,780px)] overflow-y-auto px-6 py-8">
                    {picked ? (
                      <div className="space-y-8">
                        <div>
                          <p className={LABEL}>Subject</p>
                          <p className="mt-3 text-[16px] font-medium tracking-[-0.024em] text-zinc-100">
                            {picked.subject_hint?.trim() || "(no subject)"}
                          </p>
                        </div>
                        {picked.detail ? (
                          <div>
                            <p className={LABEL}>Model note</p>
                            <p className="mt-3 text-[15px] leading-relaxed text-zinc-400">{picked.detail}</p>
                          </div>
                        ) : null}
                        {picked.thread_id ? (
                          <a
                            href={gmailThreadUrl(picked.thread_id)}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex h-11 items-center justify-center rounded-full border border-white/[0.14] px-6 text-[14px] font-semibold tracking-[-0.015em] text-zinc-100 transition hover:bg-white/[0.07]"
                          >
                            Open in Gmail
                          </a>
                        ) : null}
                        <div>
                          <p className={LABEL}>Draft</p>
                          <pre className="mt-3 max-h-[40vh] overflow-auto whitespace-pre-wrap rounded-[18px] border border-white/[0.05] bg-black/35 px-4 py-4 text-[13px] leading-relaxed tracking-[-0.01em] text-zinc-200">
                            {picked.reply_proposal?.trim() || "(empty)"}
                          </pre>
                        </div>
                      </div>
                    ) : (
                      <p className="text-[15px] text-zinc-500">Select an item.</p>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
