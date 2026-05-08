"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import {
  fetchGmailConnected,
  fetchGmailSyncProgress,
  gmailOAuthStartUrl,
  ingestWorkspaceCodebasePrefix,
  postGmailSync,
  postWorkspaceIngestPdf,
  postWorkspaceIngestVideo,
  type GmailSyncProgressSnapshot,
} from "@/lib/fluvioDashboardApi";
import { getKgEngineUrl } from "@/lib/constants";

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

function SourceRow(props: {
  title: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 border-t border-white/[0.06] py-5 first:border-t-0 first:pt-0 sm:flex-row sm:items-start sm:justify-between sm:gap-8">
      <div className="min-w-0 shrink-0 sm:w-[8.5rem]">
        <p className="text-[15px] font-medium tracking-[-0.01em] text-white">{props.title}</p>
        {props.hint ? <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">{props.hint}</p> : null}
      </div>
      <div className="min-w-0 flex-1">{props.children}</div>
    </div>
  );
}

export function DashboardPersonalGraph({ locked, onDone, onError }: Props) {
  const pdfRef = useRef<HTMLInputElement>(null);
  const videoRef = useRef<HTMLInputElement>(null);

  const [pdfBusy, setPdfBusy] = useState(false);
  const [videoBusy, setVideoBusy] = useState(false);
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
  const [codeReceipt, setCodeReceipt] = useState<{
    chunks: number;
    graphNodes: number;
    graphEdges: number;
  } | null>(null);
  const [repoUrl, setRepoUrl] = useState("");
  const [repoPath, setRepoPath] = useState("");
  const [codeBusy, setCodeBusy] = useState(false);

  const [gmailConnected, setGmailConnected] = useState<boolean | null>(null);
  const [gmailBusy, setGmailBusy] = useState(false);
  const [gmailPhase, setGmailPhase] = useState<GmailSyncProgressSnapshot | null>(null);

  const refreshGmail = useCallback(async () => {
    try {
      const c = await fetchGmailConnected();
      setGmailConnected(c);
    } catch {
      setGmailConnected(null);
    }
  }, []);

  useEffect(() => {
    void refreshGmail();
  }, [refreshGmail]);

  useEffect(() => {
    const onVis = () => {
      if (document.visibilityState === "visible") void refreshGmail();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [refreshGmail]);

  const ingestPdf = (file: File) => {
    setPdfBusy(true);
    onError(null);
    void (async () => {
      const ac = new AbortController();
      const t = window.setTimeout(() => ac.abort(), 120_000);
      try {
        const r = await postWorkspaceIngestPdf(file, ac.signal);
        setPdfReceipt({
          fileName: file.name,
          graphNodes: r.nodes,
          graphEdges: r.edges,
        });
        void Promise.resolve(onDone()).catch(() => {});
      } catch (e) {
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
        void Promise.resolve(onDone()).catch(() => {});
      } catch (e) {
        onError(e instanceof Error ? e.message : "Video ingest failed");
      } finally {
        window.clearTimeout(t);
        setVideoBusy(false);
      }
    })();
  };

  const gmailPollStop = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      gmailPollStop.current?.();
    };
  }, []);

  const pollGmailUntilIdle = () => {
    gmailPollStop.current?.();
    const tick = window.setInterval(async () => {
      try {
        const snap = await fetchGmailSyncProgress();
        setGmailPhase(snap);
        if (!snap.running) {
          window.clearInterval(tick);
          gmailPollStop.current = null;
          setGmailBusy(false);
          if (snap.error) onError(`Gmail: ${snap.error}`);
          else {
            onError(null);
            onDone();
          }
        }
      } catch {
        window.clearInterval(tick);
        gmailPollStop.current = null;
        setGmailBusy(false);
      }
    }, 850);
    const cleanup = () => window.clearInterval(tick);
    gmailPollStop.current = cleanup;
    return cleanup;
  };

  const onSyncGmail = () => {
    setGmailBusy(true);
    onError(null);
    void (async () => {
      let stopPoll: () => void = () => {};
      try {
        await postGmailSync();
        stopPoll = pollGmailUntilIdle();
      } catch (e) {
        stopPoll();
        setGmailBusy(false);
        onError(e instanceof Error ? e.message : "Could not start Gmail sync");
      }
    })();
  };

  const onIngestRepo = () => {
    const url = repoUrl.trim();
    if (!url) {
      onError("Paste a Git HTTPS URL.");
      return;
    }
    setCodeBusy(true);
    onError(null);
    void (async () => {
      const ac = new AbortController();
      try {
        const r = await ingestWorkspaceCodebasePrefix(url, repoPath.trim(), ac.signal);
        setCodeReceipt({
          chunks: r.chunks,
          graphNodes: r.nodes,
          graphEdges: r.edges,
        });
        setRepoUrl("");
        setRepoPath("");
        void Promise.resolve(onDone()).catch(() => {});
      } catch (e) {
        onError(e instanceof Error ? e.message : "Code ingest failed");
      } finally {
        setCodeBusy(false);
      }
    })();
  };

  const blocked = locked;

  return (
    <motion.section
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      className="rounded-[20px] border border-white/[0.06] bg-white/[0.02] px-6 py-8 sm:px-9 sm:py-9"
    >
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h2 className="text-[1.35rem] font-semibold tracking-[-0.03em] text-white">Personal graph</h2>
          <p className="mt-2 max-w-xl text-pretty text-[15px] leading-relaxed text-zinc-500">
            Add sources here for the kg-engine workspace graph—the same dataset as{" "}
            <Link href="/graph" className="text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline">
              Map
            </Link>
            . API base{" "}
            <span className="font-mono text-[12px] text-zinc-400">{getKgEngineUrl()}</span>
            . Tap chat pulls profile + notes for now—not this workspace graph unless we connect it later.
          </p>
        </div>
      </div>

      <div className="mt-8">
        <SourceRow
          title="PDF"
          hint="Extracts text into the graph. Private to this engine instance."
        >
          <input
            ref={pdfRef}
            type="file"
            accept="application/pdf,.pdf"
            className="sr-only"
            onChange={(ev) => {
              const f = ev.target.files?.[0];
              ev.target.value = "";
              if (f) ingestPdf(f);
            }}
          />
          <div className="flex flex-col gap-2">
            <button
              type="button"
              disabled={blocked || pdfBusy}
              onClick={() => pdfRef.current?.click()}
              className="self-start rounded-full border border-white/[0.1] bg-white/[0.04] px-5 py-2.5 text-[14px] font-medium text-white transition hover:bg-white/[0.08] disabled:opacity-35"
            >
              {pdfBusy ? "Ingesting…" : "Choose PDF"}
            </button>
            {pdfReceipt ? (
              <p className="max-w-lg text-[13px] leading-relaxed text-emerald-400/90">
                <span className="font-medium text-emerald-300/95">Added to graph.</span>{" "}
                <span className="text-emerald-200/85">{trimFileLabel(pdfReceipt.fileName)}</span> · workspace totals{" "}
                {pdfReceipt.graphNodes} nodes · {pdfReceipt.graphEdges} edges.{" "}
                <Link href="/graph" className="text-violet-300 underline-offset-4 hover:text-violet-200 hover:underline">
                  Open Map
                </Link>
              </p>
            ) : null}
          </div>
        </SourceRow>

        <SourceRow title="Video" hint="Scenes and transcript flow into the graph (heavier job).">
          <input
            ref={videoRef}
            type="file"
            accept="video/*"
            className="sr-only"
            onChange={(ev) => {
              const f = ev.target.files?.[0];
              ev.target.value = "";
              if (f) ingestVideo(f);
            }}
          />
          <div className="flex flex-col gap-2">
            <button
              type="button"
              disabled={blocked || videoBusy}
              onClick={() => videoRef.current?.click()}
              className="self-start rounded-full border border-white/[0.1] bg-white/[0.04] px-5 py-2.5 text-[14px] font-medium text-white transition hover:bg-white/[0.08] disabled:opacity-35"
            >
              {videoBusy ? "Uploading…" : "Choose video"}
            </button>
            {videoReceipt ? (
              <div className="max-w-lg space-y-1.5 text-[13px] leading-relaxed text-emerald-400/90">
                <p>
                  <span className="font-medium text-emerald-300/95">Ingest complete.</span>{" "}
                  <span className="text-emerald-200/85">{trimFileLabel(videoReceipt.fileName)}</span> ·{" "}
                  {videoReceipt.scenes} scene
                  {videoReceipt.scenes === 1 ? "" : "s"} · {videoReceipt.chunkNodes} graph nodes from this clip.
                </p>
                <p className="text-zinc-500">
                  Clip id{" "}
                  <span className="font-mono text-[11px] text-zinc-400">{videoReceipt.videoId}</span>
                  {videoReceipt.status ? (
                    <>
                      {" "}
                      · <span className="text-zinc-500">{videoReceipt.status}</span>
                    </>
                  ) : null}
                </p>
                <p>
                  <Link href="/graph" className="text-violet-300 underline-offset-4 hover:text-violet-200 hover:underline">
                    Open Map
                  </Link>
                </p>
              </div>
            ) : null}
          </div>
        </SourceRow>

        <SourceRow
          title="Code"
          hint="Shallow clone on the server, then ingest paths. Use a public HTTPS repo URL."
        >
          <div className="flex flex-col gap-3 sm:max-w-md">
            <input
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
              disabled={blocked || codeBusy}
              placeholder="https://github.com/org/repo.git"
              className="w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[15px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/25 disabled:opacity-35"
            />
            <input
              value={repoPath}
              onChange={(e) => setRepoPath(e.target.value)}
              disabled={blocked || codeBusy}
              placeholder="Path prefix (optional), e.g. src"
              className="w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[15px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/25 disabled:opacity-35"
            />
            <div className="flex flex-col gap-2">
              <button
                type="button"
                disabled={blocked || codeBusy}
                onClick={onIngestRepo}
                className="self-start rounded-full bg-white px-5 py-2.5 text-[14px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-35"
              >
                {codeBusy ? "Working…" : "Clone & ingest"}
              </button>
              {codeReceipt ? (
                <p className="max-w-lg text-[13px] leading-relaxed text-emerald-400/90">
                  <span className="font-medium text-emerald-300/95">Ingested.</span> {codeReceipt.chunks} chunks · graph
                  totals {codeReceipt.graphNodes} nodes · {codeReceipt.graphEdges} edges.{" "}
                  <Link href="/graph" className="text-violet-300 underline-offset-4 hover:text-violet-200 hover:underline">
                    Open Map
                  </Link>
                </p>
              ) : null}
            </div>
          </div>
        </SourceRow>

        <SourceRow title="Gmail" hint="OAuth lives on kg-engine; sync pulls mail into chunks on the graph.">
          <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
            <p className="text-[14px] text-zinc-500">
              {gmailConnected === null ? (
                <span className="text-zinc-600">Checking…</span>
              ) : gmailConnected ? (
                <span className="text-emerald-400/90">Connected</span>
              ) : (
                <span>Not connected</span>
              )}
            </p>
            <div className="flex flex-wrap gap-2">
              <a
                href={gmailOAuthStartUrl()}
                target="_blank"
                rel="noreferrer"
                className={`inline-flex items-center justify-center rounded-full border border-white/[0.12] px-4 py-2 text-[14px] font-medium transition hover:bg-white/[0.06] ${
                  blocked ? "pointer-events-none opacity-35" : "text-white"
                }`}
              >
                Connect
              </a>
              <button
                type="button"
                disabled={blocked || gmailBusy || !gmailConnected}
                onClick={onSyncGmail}
                className="rounded-full border border-violet-500/30 bg-violet-500/[0.12] px-4 py-2 text-[14px] font-medium text-violet-200 transition hover:bg-violet-500/20 disabled:opacity-35"
              >
                {gmailBusy ? "Syncing…" : "Sync now"}
              </button>
              <button
                type="button"
                disabled={blocked}
                onClick={() => void refreshGmail()}
                className="rounded-full px-3 py-2 text-[14px] font-medium text-zinc-500 underline-offset-4 hover:text-zinc-300 hover:underline disabled:opacity-35"
              >
                Refresh status
              </button>
            </div>
            {gmailPhase?.running || (gmailPhase && !gmailPhase.running && gmailPhase.phase !== "idle") ? (
              <p className="w-full text-[13px] text-zinc-500">
                {gmailPhase.running
                  ? `${gmailPhase.phase.replace(/_/g, " ")}${gmailPhase.percent != null ? ` · ~${Math.round(gmailPhase.percent)}%` : ""}`
                  : gmailPhase.result
                    ? `Done · ${gmailPhase.result.nodes_added} nodes`
                    : null}
              </p>
            ) : null}
          </div>
        </SourceRow>
      </div>

      <p className="mt-8 border-t border-white/[0.06] pt-6 text-[14px] text-zinc-500">
        Short thoughts and uploads tied to your profile card stay in{" "}
        <a href="#dashboard-note" className="text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline">
          notes
        </a>{" "}
        below.
      </p>
    </motion.section>
  );
}
