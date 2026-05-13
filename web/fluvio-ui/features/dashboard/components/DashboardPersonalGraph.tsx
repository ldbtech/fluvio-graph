"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  authBearerHeaders,
  authHeaders,
  deleteUserUpload,
  fetchGmailConnected,
  fetchGmailFocus,
  fetchGmailRecentInbox,
  fetchUserUploads,
  putGmailFocus,
  postGmailConnectStart,
  ingestWorkspaceCodebasePrefix,
  postWorkspaceIngestPdfStream,
  postWorkspaceIngestVideo,
  type GmailRecentMail,
  type UserUploadRow,
} from "@/shared/lib/fluvioDashboardApi";
import { getKgEngineUrl } from "@/shared/lib/constants";
import { GmailReplyAgentPanel } from "@/shared/components/GmailReplyAgentPanel";

type Props = {
  /** Disable actions while profile is unavailable */
  locked: boolean;
  onDone: () => void;
  /** Pass null to clear a previous dashboard error banner. */
  onError: (msg: string | null) => void;
};

function trimFileLabel(name: string, max = 36) {
  const t = name.trim();
  if (t.length <= max) return t;
  return `${t.slice(0, max - 1)}…`;
}

function formatGmailRecentWhen(m: GmailRecentMail): string {
  if (m.internal_date_ms != null && Number.isFinite(m.internal_date_ms)) {
    const d = new Date(Number(m.internal_date_ms));
    if (!Number.isNaN(d.getTime())) {
      return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
    }
  }
  const raw = m.date_header?.trim();
  return raw || "—";
}

/** Focus rings + press feedback — calibrated for dark UIs (Apple Settings–style readability). */
const focusRing =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/35 focus-visible:ring-offset-0";

function SourcesTab(props: {
  selected: boolean;
  onClick: () => void;
  id: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={props.selected}
      id={props.id}
      onClick={props.onClick}
      className={`relative flex min-h-[44px] flex-1 select-none items-center justify-center rounded-xl px-3 py-2.5 outline-none transition-[color] duration-200 sm:rounded-[13px] sm:py-3 ${focusRing}`}
    >
      {props.selected ? (
        <motion.span
          layoutId="sources-dashboard-tab-pill"
          className="absolute inset-[3px] rounded-[10px] bg-zinc-100 shadow-[0_2px_8px_-2px_rgba(0,0,0,.45),inset_0_1px_0_rgba(255,255,255,.85)] sm:inset-[4px]"
          transition={{ type: "spring", stiffness: 460, damping: 34 }}
        />
      ) : null}
      <span
        className={`relative z-10 whitespace-nowrap text-[15px] font-semibold tracking-[-0.015em] sm:text-[16px] ${
          props.selected ? "text-zinc-950" : "text-zinc-500 hover:text-zinc-300"
        }`}
      >
        {props.children}
      </span>
    </button>
  );
}

function SourceRow(props: {
  title: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-4 py-6 first:pt-2 last:pb-2 sm:flex-row sm:items-start sm:justify-between sm:gap-10 sm:py-7">
      <div className="min-w-0 shrink-0 sm:max-w-[13rem] sm:pt-0.5">
        <p className="text-[17px] font-semibold leading-snug tracking-[-0.022em] text-white">{props.title}</p>
        {props.hint ? (
          <p className="mt-1.5 text-[13px] leading-[1.45] text-zinc-500">{props.hint}</p>
        ) : null}
      </div>
      <div className="min-w-0 flex-1">{props.children}</div>
    </div>
  );
}

function SourcesPanel(props: {
  kicker: string;
  title: string;
  description: ReactNode;
  /** Slightly richer header treatment for file workflows. */
  variant?: "default" | "uploads";
  children: ReactNode;
}) {
  const v = props.variant ?? "default";
  return (
    <div className="overflow-hidden rounded-[22px] border border-white/[0.06] bg-zinc-950/40 shadow-[0_24px_48px_-32px_rgba(0,0,0,.85)] ring-1 ring-white/[0.03] backdrop-blur-xl sm:rounded-3xl">
      <header
        className={`border-b border-white/[0.05] px-5 pb-5 pt-5 sm:px-7 sm:pb-6 sm:pt-6 ${
          v === "uploads"
            ? "bg-[linear-gradient(110deg,rgba(244,63,94,0.07)_0%,rgba(249,115,22,0.04)_38%,transparent_62%),linear-gradient(-18deg,rgba(99,102,241,0.06)_0%,transparent_52%)]"
            : ""
        }`}
      >
        <p className="text-[10px] font-semibold uppercase tracking-[0.2em] text-zinc-500">{props.kicker}</p>
        <h3 className="mt-2 text-xl font-semibold tracking-[-0.035em] text-white sm:text-[1.375rem]">
          {props.title}
        </h3>
        <div className="mt-2 max-w-2xl text-pretty text-[15px] font-normal leading-[1.52] tracking-[-0.01em] text-zinc-400">
          {props.description}
        </div>
      </header>
      <div className="divide-y divide-white/[0.05] px-5 sm:px-7">{props.children}</div>
    </div>
  );
}

type ButtonProps = ComponentPropsWithoutRef<"button"> & { children: ReactNode };

function BtnPrimary({ className = "", ...p }: ButtonProps) {
  return (
    <button
      {...p}
      className={`inline-flex min-h-[44px] items-center justify-center rounded-full bg-zinc-100 px-5 py-2.5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-950 shadow-[inset_0_1px_0_rgba(255,255,255,.85)] transition-[transform,opacity,background-color] duration-150 enabled:hover:bg-white enabled:active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 ${focusRing} ${className}`.trim()}
    />
  );
}

function BtnSecondary({ className = "", ...p }: ButtonProps) {
  return (
    <button
      {...p}
      className={`inline-flex min-h-[44px] items-center justify-center rounded-full border border-white/[0.12] bg-white/[0.04] px-5 py-2.5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-100 transition-[transform,opacity,background-color,border-color] duration-150 enabled:hover:border-white/[0.18] enabled:hover:bg-white/[0.07] enabled:active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 ${focusRing} ${className}`.trim()}
    />
  );
}

function BtnGhost({ className = "", ...p }: ButtonProps) {
  return (
    <button
      {...p}
      className={`inline-flex min-h-[44px] items-center justify-center rounded-full px-4 py-2.5 text-[14px] font-medium tracking-[-0.01em] text-zinc-400 transition-colors duration-150 enabled:hover:bg-white/[0.05] enabled:hover:text-zinc-200 disabled:pointer-events-none disabled:opacity-40 ${focusRing} ${className}`.trim()}
    />
  );
}

function TextField(props: Omit<ComponentPropsWithoutRef<"input">, "className"> & { className?: string }) {
  const { className = "", ...rest } = props;
  return (
    <input
      {...rest}
      className={`w-full min-h-[46px] rounded-2xl border border-white/[0.08] bg-black/35 px-4 text-[15px] font-normal tracking-[-0.01em] text-white shadow-[inset_0_1px_2px_rgba(0,0,0,.2)] outline-none placeholder:text-zinc-600 transition-[border-color,background-color] duration-150 focus:border-white/[0.16] focus:bg-black/45 focus:ring-0 disabled:opacity-38 ${focusRing} ${className}`.trim()}
    />
  );
}

function PdfGlyph({ className = "size-6" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.4} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
      />
    </svg>
  );
}

function VideoGlyph({ className = "size-6" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.4} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="m15.75 10.5 4.72-4.72a.75.75 0 0 1 1.28.53v11.38a.75.75 0 0 1-1.28.53l-4.72-4.72M4.5 18.75h9a2.25 2.25 0 0 0 2.25-2.25v-9a2.25 2.25 0 0 0-2.25-2.25h-9A2.25 2.25 0 0 0 2.25 7.5v9a2.25 2.25 0 0 0 2.25 2.25Z"
      />
    </svg>
  );
}

function RepoGlyph({ className = "size-6" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.4} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M8 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0Zm12 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM8 21a2 2 0 1 1-4 0 2 2 0 0 1 4 0Zm0-14v8m4-4h4a2 2 0 0 1 2 2v2"
      />
    </svg>
  );
}

function ArrowPathGlyph({ className = "size-4" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
      />
    </svg>
  );
}

function TrashGlyph({ className = "size-4" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="m14.74 9-.346 9m-4.788 0L9.261 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
      />
    </svg>
  );
}

function CogGlyph({ className = "size-[18px]" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.43.992.001.07.001.141 0 .212-.009.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.372.48l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.075-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 0 1 0-.212c.008-.378-.136-.75-.43-.99l-1.004-.827a1.125 1.125 0 0 1-.261-1.432l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.213-1.281Z"
      />
      <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
    </svg>
  );
}

export function DashboardPersonalGraph({ locked, onDone, onError }: Props) {
  const pdfRef = useRef<HTMLInputElement>(null);
  const videoRef = useRef<HTMLInputElement>(null);

  const [pdfBusy, setPdfBusy] = useState(false);
  const [pdfPct, setPdfPct] = useState<number | null>(null);
  const [videoBusy, setVideoBusy] = useState(false);
  const [uploads, setUploads] = useState<UserUploadRow[]>([]);
  const [uploadDeletingId, setUploadDeletingId] = useState<string | null>(null);
  const [pdfReceipt, setPdfReceipt] = useState<{
    fileName: string;
    graphNodes: number;
    graphEdges: number;
  } | null>(null);
  const [videoReceipt, setVideoReceipt] = useState<{
    fileName: string;
    videoId: string;
    scenes: number;
    chunkNodes: number;
    chunkEdges: number;
    status?: string;
  } | null>(null);
  const [repoUrl, setRepoUrl] = useState("");
  const [repoPath, setRepoPath] = useState("");
  const [codeBusy, setCodeBusy] = useState(false);
  const [codeIngestPct, setCodeIngestPct] = useState(0);
  const [codeIngestPhase, setCodeIngestPhase] = useState<"clone" | "ingest" | null>(null);

  const [gmailConnected, setGmailConnected] = useState<boolean | null>(null);
  const [gmailRecent, setGmailRecent] = useState<GmailRecentMail[] | null>(null);
  const [gmailRecentBusy, setGmailRecentBusy] = useState(false);
  const [gmailOauthBusy, setGmailOauthBusy] = useState(false);
  const [gmailFocusDraft, setGmailFocusDraft] = useState("");
  const [gmailFocusBusy, setGmailFocusBusy] = useState(false);

  /** Toggle Connect vs Uploads so the page doesn’t stack both tall sections. */
  const [sourcesSection, setSourcesSection] = useState<"connect" | "uploads">("connect");
  const [gmailSettingsModalOpen, setGmailSettingsModalOpen] = useState(false);

  const codebaseLibUpload = useMemo(
    () => uploads.find((u) => u.kind.toLowerCase() === "codebase"),
    [uploads],
  );

  const refreshGmail = useCallback(async () => {
    try {
      const c = await fetchGmailConnected();
      setGmailConnected(c);
      if (c) {
        try {
          const focus = await fetchGmailFocus();
          setGmailFocusDraft(focus.join("\n"));
        } catch {
          /* leave draft unchanged */
        }
        try {
          setGmailRecent(await fetchGmailRecentInbox({ limit: 10 }));
        } catch {
          setGmailRecent([]);
        }
      } else {
        setGmailRecent(null);
        setGmailFocusDraft("");
      }
    } catch {
      setGmailConnected(null);
      setGmailRecent(null);
      setGmailFocusDraft("");
    }
  }, []);

  useEffect(() => {
    void refreshGmail();
  }, [refreshGmail]);

  const refreshUploads = useCallback(async () => {
    try {
      const rows = await fetchUserUploads();
      setUploads(rows);
    } catch {
      setUploads([]);
    }
  }, []);

  useEffect(() => {
    const onVis = () => {
      if (document.visibilityState === "visible") void refreshGmail();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [refreshGmail]);

  useEffect(() => {
    if (!gmailSettingsModalOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setGmailSettingsModalOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [gmailSettingsModalOpen]);

  useEffect(() => {
    if (!gmailSettingsModalOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, [gmailSettingsModalOpen]);

  const ingestPdf = (file: File) => {
    setPdfBusy(true);
    setPdfPct(0);
    onError(null);
    void (async () => {
      const ac = new AbortController();
      const t = window.setTimeout(() => ac.abort(), 120_000);
      try {
        const r = await postWorkspaceIngestPdfStream(
          file,
          (p) => setPdfPct(Math.round(Math.min(100, Math.max(0, p.percent)))),
          ac.signal,
        );
        setPdfPct(null);
        setPdfReceipt({
          fileName: file.name,
          graphNodes: r.nodes,
          graphEdges: r.edges,
        });
        await refreshUploads();
        void Promise.resolve(onDone()).catch(() => {});
      } catch (e) {
        setPdfPct(null);
        onError(e instanceof Error ? e.message : "PDF ingest failed");
      } finally {
        window.clearTimeout(t);
        setPdfBusy(false);
      }
    })();
  };

  const ingestVideo = (file: File) => {
    setVideoBusy(true);
    onError(null);
    void (async () => {
      const ac = new AbortController();
      const t = window.setTimeout(() => ac.abort(), 600_000);
      try {
        const r = await postWorkspaceIngestVideo(file, ac.signal);
        setVideoReceipt({
          fileName: file.name,
          videoId: r.video_id,
          scenes: r.scenes ?? 0,
          chunkNodes: r.nodes ?? 0,
          chunkEdges: r.edges ?? 0,
          status: r.status,
        });
        await refreshUploads();
        void Promise.resolve(onDone()).catch(() => {});
      } catch (e) {
        onError(e instanceof Error ? e.message : "Video ingest failed");
      } finally {
        window.clearTimeout(t);
        setVideoBusy(false);
      }
    })();
  };

  /** Background poll + Gmail History merge on server (~25s). */
  useEffect(() => {
    if (!gmailConnected) return;
    const id = window.setInterval(() => {
      void (async () => {
        try {
          setGmailRecent(await fetchGmailRecentInbox({ limit: 10 }));
        } catch {
          /* keep prior list on transient errors */
        }
      })();
    }, 25_000);
    return () => window.clearInterval(id);
  }, [gmailConnected]);

  const refreshGmailInbox = useCallback(() => {
    if (!gmailConnected) return;
    setGmailRecentBusy(true);
    onError(null);
    void (async () => {
      try {
        setGmailRecent(await fetchGmailRecentInbox({ limit: 10 }));
      } catch (e) {
        onError(e instanceof Error ? e.message : "Could not load inbox");
      } finally {
        setGmailRecentBusy(false);
      }
    })();
  }, [gmailConnected, onError]);

  const saveGmailFocus = useCallback(() => {
    if (!gmailConnected) return;
    setGmailFocusBusy(true);
    onError(null);
    void (async () => {
      try {
        const lines = gmailFocusDraft
          .split(/[\n,]+/)
          .map((s) => s.trim())
          .filter(Boolean);
        const normalized = await putGmailFocus(lines);
        setGmailFocusDraft(normalized.join("\n"));
        try {
          setGmailRecent(await fetchGmailRecentInbox({ limit: 10 }));
        } catch {
          /* ignore */
        }
      } catch (e) {
        onError(e instanceof Error ? e.message : "Could not save sender list");
      } finally {
        setGmailFocusBusy(false);
      }
    })();
  }, [gmailConnected, gmailFocusDraft, onError]);

  const onIngestRepo = () => {
    const url = repoUrl.trim();
    if (!url) {
      onError("Paste a Git HTTPS URL.");
      return;
    }
    const pathTrim = repoPath.trim();
    setCodeBusy(true);
    setCodeIngestPhase("clone");
    setCodeIngestPct(5);
    onError(null);
    void (async () => {
      const ac = new AbortController();
      try {
        await ingestWorkspaceCodebasePrefix(url, pathTrim, ac.signal, (p) => {
          setCodeIngestPhase(p.phase);
          setCodeIngestPct(p.pct);
        });
        setRepoUrl("");
        setRepoPath("");
        await refreshUploads();
        /** Do not call `onDone` here: parent `reload()` sets auth to loading, unmounts this tree, and can break scroll. */
      } catch (e) {
        onError(e instanceof Error ? e.message : "Code ingest failed");
      } finally {
        setCodeBusy(false);
        setCodeIngestPhase(null);
        setCodeIngestPct(0);
      }
    })();
  };

  const blocked = locked;

  const connectGmailOAuth = useCallback(() => {
    if (blocked) return;
    setGmailOauthBusy(true);
    onError(null);
    void (async () => {
      try {
        const url = await postGmailConnectStart();
        window.open(url, "_blank", "noopener,noreferrer");
      } catch (e) {
        onError(e instanceof Error ? e.message : "Could not start Gmail OAuth");
      } finally {
        setGmailOauthBusy(false);
      }
    })();
  }, [blocked, onError]);

  useEffect(() => {
    if (gmailConnected !== true) setGmailSettingsModalOpen(false);
  }, [gmailConnected]);

  /** If this panel unmounts mid-flow (e.g. parent refresh), avoid leaving `body` scroll-locked from the email modal. */
  useEffect(() => {
    return () => {
      document.body.style.removeProperty("overflow");
    };
  }, []);

  useEffect(() => {
    if (!blocked) void refreshUploads();
  }, [blocked, refreshUploads]);

  const gmailSettingsModal =
    typeof document !== "undefined" && gmailSettingsModalOpen && gmailConnected === true
      ? createPortal(
          <div
            className="fixed inset-0 z-[100] flex items-end justify-center sm:items-center sm:p-4"
            role="presentation"
          >
            <button
              type="button"
              aria-label="Close email settings"
              className="absolute inset-0 bg-black/70 backdrop-blur-[2px] transition-opacity"
              onClick={() => setGmailSettingsModalOpen(false)}
            />
            <div
              role="dialog"
              aria-modal="true"
              aria-labelledby="dashboard-gmail-settings-title"
              className="relative z-10 flex max-h-[min(92dvh,880px)] w-full max-w-lg flex-col overflow-hidden rounded-t-[22px] border border-white/[0.08] border-b-0 bg-zinc-950 shadow-[0_-24px_80px_rgba(0,0,0,0.65)] ring-1 ring-black/50 sm:max-h-[min(88dvh,820px)] sm:rounded-2xl sm:border-b sm:shadow-2xl"
            >
              <div className="flex shrink-0 items-center justify-between gap-3 border-b border-white/[0.06] px-5 py-4">
                <div className="min-w-0">
                  <h2
                    id="dashboard-gmail-settings-title"
                    className="text-[1.05rem] font-semibold tracking-[-0.03em] text-white"
                  >
                    Email settings
                  </h2>
                  <p className="mt-0.5 truncate text-[12px] text-zinc-500">Inbox, focus senders, and reply assistant</p>
                </div>
                <button
                  type="button"
                  onClick={() => setGmailSettingsModalOpen(false)}
                  className={`shrink-0 rounded-full p-2.5 text-zinc-400 transition hover:bg-white/[0.08] hover:text-white ${focusRing}`}
                  aria-label="Close"
                >
                  <svg className="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} aria-hidden>
                    <path strokeLinecap="round" d="M6 6l12 12M18 6L6 18" />
                  </svg>
                </button>
              </div>
              <div className="min-h-0 flex-1 space-y-5 overflow-y-auto overscroll-contain px-5 py-5">
                <div className="flex flex-wrap gap-2.5">
                  <BtnGhost
                    type="button"
                    disabled={blocked || gmailRecentBusy}
                    onClick={refreshGmailInbox}
                    className="shrink-0"
                  >
                    {gmailRecentBusy ? "Refreshing…" : "Refresh inbox"}
                  </BtnGhost>
                  <BtnGhost type="button" disabled={blocked} onClick={() => void refreshGmail()} className="shrink-0">
                    Connection status
                  </BtnGhost>
                  <BtnGhost
                    type="button"
                    disabled={blocked || gmailOauthBusy}
                    onClick={connectGmailOAuth}
                    className="shrink-0"
                  >
                    {gmailOauthBusy ? "Opening…" : "Reconnect Gmail"}
                  </BtnGhost>
                </div>
                <div className="space-y-2">
                  <label className="block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
                    Focus senders (optional)
                  </label>
                  <p className="text-[12px] leading-relaxed text-zinc-500">
                    One per line: full address <span className="text-zinc-400">you@company.com</span> or domain{" "}
                    <span className="text-zinc-400">@company.com</span>. Leave empty to show the whole inbox.
                  </p>
                  <textarea
                    value={gmailFocusDraft}
                    onChange={(e) => setGmailFocusDraft(e.target.value)}
                    disabled={blocked || gmailFocusBusy}
                    rows={4}
                    placeholder={"you@client.com\n@important-vendor.com"}
                    className={`w-full resize-y rounded-2xl border border-white/[0.08] bg-black/30 px-3.5 py-2.5 font-mono text-[13px] leading-relaxed text-zinc-100 outline-none ring-1 ring-black/30 placeholder:text-zinc-600 focus:border-sky-500/35 focus:ring-sky-500/20 ${focusRing}`}
                  />
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      disabled={blocked || gmailFocusBusy}
                      onClick={saveGmailFocus}
                      className={`inline-flex min-h-[40px] items-center justify-center rounded-full border border-amber-400/30 bg-amber-500/12 px-4 py-2 text-[13px] font-semibold text-amber-100/95 transition enabled:hover:bg-amber-500/18 disabled:pointer-events-none disabled:opacity-40 ${focusRing}`}
                    >
                      {gmailFocusBusy ? "Saving…" : "Save focus list"}
                    </button>
                  </div>
                </div>
                <GmailReplyAgentPanel
                  disabled={blocked}
                  kgEngineBaseUrl={getKgEngineUrl()}
                  bearerHeaders={() => authBearerHeaders()}
                  jsonHeaders={() => authHeaders()}
                  onBanner={onError}
                />
                <div className="space-y-1.5">
                  <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
                    Latest in inbox (~25s · History for new items)
                  </p>
                  {gmailRecent === null ? (
                    <p className="rounded-2xl bg-white/[0.02] px-3 py-3 text-[13px] text-zinc-500 ring-1 ring-white/[0.05]">
                      Loading messages…
                    </p>
                  ) : gmailRecent.length === 0 ? (
                    <p className="rounded-2xl bg-white/[0.02] px-3 py-3 text-[13px] text-zinc-500 ring-1 ring-white/[0.05]">
                      No inbox messages returned (empty mailbox or Gmail API error — try Refresh inbox).
                    </p>
                  ) : (
                    <ul className="max-h-[min(40vh,22rem)] space-y-2 overflow-y-auto pr-1 sm:max-h-[28rem]">
                      {gmailRecent.map((m) => (
                        <li
                          key={m.id}
                          className="rounded-xl border border-white/[0.06] bg-white/[0.02] px-3 py-2.5 ring-1 ring-black/20"
                        >
                          <p className="flex flex-wrap items-center gap-2 text-[13px] font-semibold tracking-[-0.02em] text-zinc-100">
                            <span>{(m.subject && m.subject.trim()) || "(No subject)"}</span>
                            {m.is_new ? (
                              <span className="rounded-full bg-sky-500/20 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-sky-200/95 ring-1 ring-sky-400/25">
                                New
                              </span>
                            ) : null}
                          </p>
                          <p className="mt-0.5 text-[12px] text-zinc-500">
                            <span className="text-zinc-400">{formatGmailRecentWhen(m)}</span>
                            {m.from && m.from.trim() ? (
                              <span className="block truncate text-[11px] text-zinc-600 sm:inline sm:ml-2">
                                · {m.from.trim()}
                              </span>
                            ) : null}
                          </p>
                          {m.snippet && m.snippet.trim() ? (
                            <p className="mt-1 line-clamp-2 text-[12px] leading-snug tracking-[-0.01em] text-zinc-500">
                              {m.snippet.trim()}
                            </p>
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            </div>
          </div>,
          document.body,
        )
      : null;

  return (
    <>
    <motion.section
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="rounded-[22px] border border-white/[0.05] bg-gradient-to-b from-zinc-900/55 to-zinc-950/85 px-5 py-8 shadow-[0_0_0_1px_rgba(255,255,255,.02)_inset] ring-1 ring-black/50 backdrop-blur-2xl sm:rounded-[28px] sm:px-9 sm:py-10"
    >
      <div className="flex flex-col gap-1">
        <h2 className="text-[1.65rem] font-semibold tracking-[-0.042em] text-white sm:text-[1.85rem]">Sources</h2>
        <p className="max-w-xl text-pretty text-[15px] leading-relaxed tracking-[-0.015em] text-zinc-400">
          Sync connectors or upload files to FluvioMe.
        </p>
      </div>

      <div className="mt-7 rounded-2xl bg-black/55 p-[3px] ring-1 ring-white/[0.07] backdrop-blur-xl sm:mt-9 sm:rounded-[18px] sm:p-[4px]">
        <div
          className="flex gap-0.5 sm:gap-1"
          role="tablist"
          aria-label="Sources sections"
        >
          <SourcesTab
            id="sources-tab-connect"
            selected={sourcesSection === "connect"}
            onClick={() => setSourcesSection("connect")}
          >
            Connect sources
          </SourcesTab>
          <SourcesTab
            id="sources-tab-uploads"
            selected={sourcesSection === "uploads"}
            onClick={() => setSourcesSection("uploads")}
          >
            Uploads
          </SourcesTab>
        </div>
      </div>

      <div
        className="mt-4 sm:mt-5"
        role="tabpanel"
        aria-labelledby={sourcesSection === "connect" ? "sources-tab-connect" : "sources-tab-uploads"}
      >
        {sourcesSection === "connect" ? (
        <SourcesPanel
          kicker="Connect"
          title="Connect sources"
          description={
            <>
              Link email and ingest a <span className="text-zinc-400">public</span> Git repository (e.g. this project on
              GitHub). Clone and indexing run on the server.
            </>
          }
        >
          <SourceRow
            title="Email"
            hint="Gmail inbox preview, optional sender focus, and reply assistant. Open Email settings to configure."
          >
            <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-between sm:gap-x-4">
              <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
                {gmailConnected === null ? (
                  <span className="inline-flex items-center rounded-full bg-zinc-800/70 px-3 py-1 text-[12px] font-medium text-zinc-500 ring-1 ring-white/[0.06]">
                    Checking connection…
                  </span>
                ) : gmailConnected ? (
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/12 px-3 py-1 text-[12px] font-semibold tracking-wide text-emerald-400/95 ring-1 ring-emerald-400/22">
                    <span className="size-1.5 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,.55)]" />
                    Connected · live inbox
                  </span>
                ) : (
                  <span className="inline-flex items-center rounded-full bg-zinc-800/65 px-3 py-1 text-[12px] font-semibold tabular-nums text-zinc-400 ring-1 ring-white/[0.07]">
                    Not connected
                  </span>
                )}
              </div>
              <div className="flex flex-wrap gap-2.5">
                <button
                  type="button"
                  onClick={connectGmailOAuth}
                  disabled={blocked || gmailOauthBusy}
                  className={`inline-flex min-h-[44px] shrink-0 items-center justify-center rounded-full border border-white/[0.11] bg-white/[0.04] px-5 py-2.5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-100 shadow-sm transition-colors duration-150 enabled:hover:border-white/[0.16] enabled:hover:bg-white/[0.07] disabled:pointer-events-none disabled:opacity-40 ${focusRing}`}
                >
                  {gmailOauthBusy ? "Opening…" : "Connect Gmail"}
                </button>
                {gmailConnected ? (
                  <BtnSecondary
                    type="button"
                    disabled={blocked}
                    onClick={() => {
                      setGmailSettingsModalOpen(true);
                      void refreshGmailInbox();
                    }}
                    className="inline-flex shrink-0 items-center gap-2"
                  >
                    <CogGlyph className="size-[17px] opacity-90" />
                    Email settings
                  </BtnSecondary>
                ) : null}
              </div>
            </div>
          </SourceRow>

          <SourceRow
            title="GitHub · codebase"
            hint="Linked repo is listed in Library (Uploads tab). Ingesting another public repo replaces the previous codebase on your graph; you can also remove it there."
          >
            <div className="flex max-w-xl flex-col gap-3">
              <TextField
                value={repoUrl}
                onChange={(e) => setRepoUrl(e.target.value)}
                disabled={blocked || codeBusy}
                placeholder="Repository URL · https://github.com/org/repo.git"
              />
              <TextField
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
                disabled={blocked || codeBusy}
                placeholder="Path inside repo · optional · e.g. apps/web"
              />
              <div className="flex flex-col gap-3 pt-0.5">
                <BtnPrimary
                  type="button"
                  disabled={blocked || codeBusy}
                  className="w-full sm:w-auto"
                  onClick={onIngestRepo}
                >
                  {codeBusy ? "Working…" : "Clone & ingest"}
                </BtnPrimary>
                {codeBusy ? (
                  <div className="max-w-lg space-y-2 rounded-2xl border border-white/[0.08] bg-black/25 px-3.5 py-3 ring-1 ring-black/30">
                    <div className="flex items-center justify-between gap-3 text-[12px] text-zinc-400">
                      <span className="font-medium text-zinc-300">
                        {codeIngestPhase === "ingest" ? "Indexing codebase…" : "Cloning repository…"}
                      </span>
                      <span className="tabular-nums text-zinc-500">{Math.round(codeIngestPct)}%</span>
                    </div>
                    <div className="h-2 overflow-hidden rounded-full bg-zinc-800/90 ring-1 ring-white/[0.05]">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-violet-500/95 via-indigo-500/90 to-sky-400/95 transition-[width] duration-500 ease-out"
                        style={{ width: `${Math.min(100, Math.max(3, codeIngestPct))}%` }}
                      />
                    </div>
                    <p className="text-[11px] leading-relaxed text-zinc-600">
                      Step {codeIngestPhase === "ingest" ? "2" : "1"} of 2 · keep this tab open until finished.
                    </p>
                  </div>
                ) : null}
                {codebaseLibUpload && !codeBusy ? (
                  <div className="max-w-xl space-y-2 rounded-2xl border border-white/[0.08] bg-black/30 px-3.5 py-3 text-[13px] leading-snug tracking-[-0.01em] text-zinc-200 ring-1 ring-white/[0.05]">
                    <p className="font-semibold text-white/95">Current linked repository</p>
                    <p className="break-all font-mono text-[12px] leading-relaxed text-zinc-400">{codebaseLibUpload.file_name}</p>
                    {codebaseLibUpload.document_id ? (
                      <p className="text-[12px] text-zinc-500">
                        Scope <span className="font-mono text-zinc-400">{codebaseLibUpload.document_id}</span>
                      </p>
                    ) : null}
                    <p className="pt-0.5 text-[12px] text-zinc-500">
                      {codebaseLibUpload.graph_nodes != null && codebaseLibUpload.graph_edges != null
                        ? `${codebaseLibUpload.graph_nodes} subgraph nodes · ${codebaseLibUpload.graph_edges} subgraph edges on record`
                        : "Indexed into your personal graph"}
                    </p>
                    <p className="text-[11px] leading-relaxed text-zinc-600">
                      Open the <span className="text-zinc-500">Uploads</span> tab → Library to remove it, or run{" "}
                      <span className="font-medium text-zinc-500">Clone &amp; ingest</span> on a new URL to replace it.
                    </p>
                  </div>
                ) : null}
              </div>
            </div>
          </SourceRow>
        </SourcesPanel>
        ) : (
        <SourcesPanel
          variant="uploads"
          kicker="Files"
          title="Uploads"
          description={
            <>
              Add documents or clips from your device. <span className="text-zinc-300">Library</span>.
            </>
          }
        >
          <input
            ref={pdfRef}
            type="file"
            accept="application/pdf,.pdf"
            className="sr-only"
            aria-hidden
            tabIndex={-1}
            onChange={(ev) => {
              const f = ev.target.files?.[0];
              ev.target.value = "";
              if (f) ingestPdf(f);
            }}
          />
          <input
            ref={videoRef}
            type="file"
            accept="video/*"
            className="sr-only"
            aria-hidden
            tabIndex={-1}
            onChange={(ev) => {
              const f = ev.target.files?.[0];
              ev.target.value = "";
              if (f) ingestVideo(f);
            }}
          />

          <SourceRow title="Import" hint="Tap a tile to pick one file at a time. Large videos may take several minutes after upload.">
            <div className="grid gap-4 sm:grid-cols-2 lg:gap-5">
              <article
                className={`relative flex min-h-[220px] flex-col overflow-hidden rounded-[22px] border border-white/[0.06] bg-gradient-to-br from-rose-500/[0.09] via-zinc-950/60 to-black/78 p-[1px] shadow-[inset_0_1px_0_rgba(255,255,255,.06)] transition-[border-color,box-shadow] duration-200 hover:border-white/[0.11] hover:shadow-[0_28px_64px_-40px_rgba(244,63,94,.45)] ${blocked ? "opacity-45" : ""}`}
              >
                <div className="flex h-full flex-col rounded-[21px] bg-zinc-950/75 p-5 backdrop-blur-sm sm:p-[1.35rem]">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex size-14 shrink-0 items-center justify-center rounded-2xl bg-rose-500/15 ring-1 ring-rose-400/22">
                      <PdfGlyph className="size-[1.4rem] text-rose-200/95" />
                    </div>
                    <span className="rounded-lg bg-black/40 px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.12em] text-zinc-500 ring-1 ring-white/[0.06]">
                      PDF
                    </span>
                  </div>
                  <h4 className="mt-4 text-[1.0625rem] font-semibold tracking-[-0.028em] text-white">Documents</h4>
                  <p className="mt-1 text-[13px] leading-snug tracking-[-0.01em] text-zinc-500">
                    Paginated ingest with live progress · ideal for manuals, resumes, decks.
                  </p>
                  <div className="mt-auto pt-6">
                    <BtnSecondary
                      type="button"
                      disabled={blocked || pdfBusy}
                      className="w-full rounded-2xl border-white/[0.14] bg-white/[0.06] py-3 hover:bg-white/[0.095]"
                      onClick={() => pdfRef.current?.click()}
                    >
                      {pdfBusy
                        ? pdfPct != null
                          ? `${pdfPct}%`
                          : "Importing…"
                        : "Choose PDF"}
                    </BtnSecondary>
                    {pdfPct != null && pdfBusy ? (
                      <div className="mt-3 rounded-full bg-zinc-800/95 p-[2px] ring-1 ring-white/[0.05]">
                        <div className="h-1 overflow-hidden rounded-full bg-black/55">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-rose-400/95 to-orange-300/95 transition-[width] duration-300 ease-out"
                            style={{ width: `${pdfPct}%` }}
                          />
                        </div>
                      </div>
                    ) : null}
                    {pdfReceipt ? (
                      <p className="mt-3 rounded-xl border border-emerald-400/14 bg-emerald-500/[0.08] px-3 py-2 text-[12px] font-medium leading-snug tracking-[-0.012em] text-emerald-200/93">
                        <span className="text-emerald-100">Done —</span> {trimFileLabel(pdfReceipt.fileName)}{" "}
                        <span className="tabular-nums text-emerald-200/85">
                          · {pdfReceipt.graphNodes}n / {pdfReceipt.graphEdges}e
                        </span>
                      </p>
                    ) : null}
                  </div>
                </div>
              </article>

              <article
                className={`relative flex min-h-[220px] flex-col overflow-hidden rounded-[22px] border border-white/[0.06] bg-gradient-to-br from-indigo-500/[0.1] via-violet-500/[0.05] to-black/78 p-[1px] shadow-[inset_0_1px_0_rgba(255,255,255,.06)] transition-[border-color,box-shadow] duration-200 hover:border-white/[0.11] hover:shadow-[0_28px_64px_-40px_rgba(99,102,241,.4)] ${blocked ? "opacity-45" : ""}`}
              >
                <div className="flex h-full flex-col rounded-[21px] bg-zinc-950/75 p-5 backdrop-blur-sm sm:p-[1.35rem]">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex size-14 shrink-0 items-center justify-center rounded-2xl bg-indigo-500/18 ring-1 ring-indigo-400/25">
                      <VideoGlyph className="size-[1.4rem] text-indigo-200/96" />
                    </div>
                    <span className="rounded-lg bg-black/40 px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.12em] text-zinc-500 ring-1 ring-white/[0.06]">
                      Video
                    </span>
                  </div>
                  <h4 className="mt-4 text-[1.0625rem] font-semibold tracking-[-0.028em] text-white">Clips</h4>
                  <p className="mt-1 text-[13px] leading-snug tracking-[-0.01em] text-zinc-500">
                    Scene split + transcripts · heavier jobs; confirmation shows when encoding finishes.
                  </p>
                  <div className="mt-auto pt-6">
                    <BtnSecondary
                      type="button"
                      disabled={blocked || videoBusy}
                      className="w-full rounded-2xl border-white/[0.14] bg-white/[0.06] py-3 hover:bg-white/[0.095]"
                      onClick={() => videoRef.current?.click()}
                    >
                      {videoBusy ? "Uploading…" : "Choose video"}
                    </BtnSecondary>
                    {videoReceipt ? (
                      <div className="mt-3 space-y-1.5 rounded-xl border border-emerald-400/14 bg-emerald-500/[0.08] px-3 py-2.5 text-[12px] font-medium tracking-[-0.012em] text-emerald-200/93">
                        <p>
                          <span className="text-emerald-100">Processed —</span> {trimFileLabel(videoReceipt.fileName)}
                        </p>
                        <p className="text-[11px] font-normal text-emerald-200/72">
                          {videoReceipt.scenes} scene
                          {videoReceipt.scenes === 1 ? "" : "s"}
                          {" · "}
                          {videoReceipt.chunkNodes} nodes
                        </p>
                        <p className="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-[11px] font-normal text-zinc-400">
                          <span className="text-zinc-500">ID</span>
                          <span className="rounded-md bg-black/45 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300">
                            {videoReceipt.videoId}
                          </span>
                          {videoReceipt.status ? (
                            <>
                              <span className="text-zinc-600">·</span>
                              <span>{videoReceipt.status}</span>
                            </>
                          ) : null}
                        </p>
                      </div>
                    ) : null}
                  </div>
                </div>
              </article>
            </div>
          </SourceRow>

          <SourceRow
            title="Library"
            hint="Synced catalog for PDF, video, and linked GitHub codebases. Removing an item clears matching graph nodes and Surreal records (and on-disk video when applicable)."
          >
            <div className="flex max-w-xl flex-col gap-4 lg:max-w-none">
              <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-white/[0.06] bg-black/30 px-4 py-3 ring-1 ring-white/[0.03] backdrop-blur-sm">
                <div className="min-w-0">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500">On record</p>
                  <p className="mt-0.5 text-[20px] font-semibold tracking-[-0.04em] text-white tabular-nums">
                    {uploads.length}
                    <span className="text-[14px] font-medium tracking-[-0.02em] text-zinc-500">
                      {" "}
                      file{uploads.length === 1 ? "" : "s"}
                    </span>
                  </p>
                </div>
                <button
                  type="button"
                  disabled={blocked}
                  onClick={() => void refreshUploads()}
                  className={`inline-flex min-h-[44px] shrink-0 items-center justify-center gap-2 rounded-full border border-white/[0.1] bg-white/[0.05] px-4 py-2 pr-5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-100 transition-[transform,border-color,background-color] duration-150 enabled:hover:border-white/[0.15] enabled:hover:bg-white/[0.085] enabled:active:scale-[0.97] disabled:pointer-events-none disabled:opacity-40 ${focusRing}`}
                >
                  <ArrowPathGlyph className="size-[1.05rem] text-zinc-400" />
                  Reload library
                </button>
              </div>

              {uploads.length === 0 ? (
                <div className="rounded-[22px] border border-dashed border-white/[0.1] bg-gradient-to-b from-white/[0.03] to-transparent px-6 py-10 text-center sm:py-12">
                  <p className="text-[15px] font-semibold tracking-[-0.02em] text-zinc-300">No imports yet</p>
                  <p className="mx-auto mt-2 max-w-xs text-[14px] leading-relaxed text-zinc-500">
                    Bring in a PDF, video, or GitHub repo from Connect — items appear here automatically.
                  </p>
                </div>
              ) : (
                <ul className="flex flex-col gap-2 sm:gap-2.5" aria-label="Upload library">
                  {uploads.map((u) => {
                    const k = u.kind.toLowerCase();
                    const isPdf = k === "pdf";
                    const isCodebase = k === "codebase";
                    return (
                      <li key={u.id}>
                        <div className="group flex flex-wrap items-start gap-4 rounded-[20px] border border-white/[0.06] bg-black/35 px-4 py-[0.9rem] shadow-sm ring-1 ring-white/[0.03] transition-[border-color,background-color] duration-150 hover:border-white/[0.1] hover:bg-black/45 sm:flex-nowrap sm:items-center sm:justify-between">
                          <div className="flex min-w-0 flex-1 items-start gap-3.5">
                            <div
                              className={`mt-0.5 flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl ring-1 ${
                                isPdf
                                  ? "bg-rose-500/[0.13] ring-rose-400/22 text-rose-200/93"
                                  : isCodebase
                                    ? "bg-emerald-500/[0.13] ring-emerald-400/22 text-emerald-200/92"
                                    : "bg-indigo-500/[0.15] ring-indigo-400/24 text-indigo-200/92"
                              }`}
                            >
                              {isPdf ? (
                                <PdfGlyph className="size-[1.15rem]" />
                              ) : isCodebase ? (
                                <RepoGlyph className="size-[1.15rem]" />
                              ) : (
                                <VideoGlyph className="size-[1.15rem]" />
                              )}
                            </div>
                            <div className="min-w-0 flex-1 pt-0.5">
                              <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                                <span className="truncate text-[15px] font-semibold tracking-[-0.022em] text-zinc-100">
                                  {trimFileLabel(u.file_name, 52)}
                                </span>
                                <span className="shrink-0 rounded-md bg-white/[0.06] px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.1em] text-zinc-500">
                                  {u.kind}
                                </span>
                              </div>
                              <p className="mt-1 truncate font-mono text-[11px] leading-relaxed tracking-tight text-zinc-600">
                                {u.document_id ?? "—"}
                              </p>
                            </div>
                          </div>
                          <button
                            type="button"
                            disabled={blocked || uploadDeletingId === u.id}
                            aria-label={`Remove ${u.file_name}`}
                            onClick={() => {
                              void (async () => {
                                setUploadDeletingId(u.id);
                                onError(null);
                                try {
                                  await deleteUserUpload(u.id);
                                  await refreshUploads();
                                } catch (e) {
                                  onError(e instanceof Error ? e.message : "Delete failed");
                                } finally {
                                  setUploadDeletingId(null);
                                }
                              })();
                            }}
                            className={`inline-flex min-h-[40px] items-center gap-2 rounded-full px-4 py-2 text-[13px] font-semibold text-red-400/94 transition-colors duration-150 hover:bg-red-500/14 disabled:pointer-events-none disabled:opacity-40 ${focusRing}`}
                          >
                            <TrashGlyph className="size-3.5 opacity-85" />
                            {uploadDeletingId === u.id ? "…" : "Remove"}
                          </button>
                        </div>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          </SourceRow>
        </SourcesPanel>
        )}
      </div>

      <p className="mt-8 border-t border-white/[0.05] pt-7 text-[14px] leading-relaxed tracking-[-0.01em] text-zinc-500">
        Short notes tied to your card live in{" "}
        <a
          href="#dashboard-note"
          className="font-medium text-sky-400/90 underline-offset-[5px] transition hover:text-sky-300 hover:underline"
        >
          Notes
        </a>{" "}
        below.
      </p>
    </motion.section>
    {gmailSettingsModal}
    </>
  );
}
