"use client";

import { useEffect, useRef, useState } from "react";
import {
  getSecurityResult,
  getSecurityStatus,
  postSecurityDeploy,
  postRulesLink,
  type AgentProgress,
  type SecurityAgentResult,
} from "@/shared/lib/fetchSecurityWorkflow";

type Props = {
  kgUrl: string;
  /** True when the workspace graph already has PDF chunks (ingest a rules PDF in Sources first). */
  pdfReady: boolean;
  /** Repo-relative path from file-tree / chat focus — optional default for scope filter. */
  focusPathPrefix: string | null;
  onGraphRefresh: () => void | Promise<void>;
  /** Open Sources on the Documents surface so the user can upload a security PDF. */
  onOpenSourcesDocuments: () => void;
};

export function GithubBrainSecurityPanel({
  kgUrl,
  pdfReady,
  focusPathPrefix,
  onGraphRefresh,
  onOpenSourcesDocuments,
}: Props) {
  const [scopePrefix, setScopePrefix] = useState("");
  const [rulesLinkBusy, setRulesLinkBusy] = useState(false);
  const [rulesLinkErr, setRulesLinkErr] = useState<string | null>(null);
  const [rulesLinkOk, setRulesLinkOk] = useState<string | null>(null);
  const [securityBusy, setSecurityBusy] = useState(false);
  const [securityErr, setSecurityErr] = useState<string | null>(null);
  const [securityProgress, setSecurityProgress] = useState<AgentProgress | null>(null);
  const [securityResult, setSecurityResult] = useState<SecurityAgentResult | null>(null);
  const [securityAgentId, setSecurityAgentId] = useState<string | null>(null);
  const securityAbortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => {
      securityAbortRef.current?.abort();
    };
  }, []);

  const normalizedScope = scopePrefix.replace(/\\/g, "/").trim();

  return (
    <div className="shrink-0 border-b border-white/[0.06] bg-zinc-950/80 px-3 py-3 backdrop-blur-sm sm:px-4">
      <div className="mx-auto max-w-4xl">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <p className="text-[13px] font-semibold text-sky-100">Security rules + agent</p>
          <span className="font-mono text-[10px] text-zinc-600">/rules/link · /agents/security/*</span>
        </div>
        <p className="mt-1 text-[12px] leading-relaxed text-zinc-500">
          Uses PDF rule nodes and codebase nodes already in this workspace graph. Clone/ingest the repo in Sources first;
          ingest a security PDF there too so rules exist in the graph.
        </p>

        {!pdfReady ? (
          <div className="mt-3 rounded-xl border border-amber-500/25 bg-amber-950/20 px-3 py-2.5 text-[12px] leading-relaxed text-amber-100/90">
            No PDF chunks in the graph yet. Upload a security rules PDF from{" "}
            <span className="font-medium text-amber-50">Sources → Documents</span>, then return here to link rules and
            run the agent.
            <button
              type="button"
              onClick={onOpenSourcesDocuments}
              className="ml-2 inline-flex rounded-lg border border-amber-400/40 bg-amber-500/15 px-2.5 py-1 text-[11px] font-semibold text-amber-50 transition hover:bg-amber-500/25"
            >
              Open Documents
            </button>
          </div>
        ) : null}

        <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-end">
          <label className="min-w-[min(100%,220px)] flex-1 text-[11px] font-medium text-zinc-500">
            Code path filter / agent scope (optional)
            <input
              type="text"
              value={scopePrefix}
              onChange={(e) => setScopePrefix(e.target.value)}
              disabled={!pdfReady || rulesLinkBusy || securityBusy}
              placeholder="e.g. src — empty = whole repo"
              className="mt-1 w-full rounded-lg border border-white/[0.08] bg-zinc-900/60 px-2.5 py-2 font-mono text-[12px] text-zinc-200 outline-none placeholder:text-zinc-600 focus:border-sky-500/35"
            />
          </label>
          {focusPathPrefix?.trim() ? (
            <button
              type="button"
              disabled={!pdfReady || rulesLinkBusy || securityBusy}
              onClick={() => setScopePrefix(focusPathPrefix.replace(/\\/g, "/").trim())}
              className="shrink-0 rounded-lg border border-white/[0.1] px-2.5 py-2 text-[11px] font-medium text-zinc-400 transition hover:bg-white/[0.06] disabled:opacity-40"
              title={focusPathPrefix}
            >
              Use tree focus path
            </button>
          ) : null}
        </div>

        <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:flex-wrap">
          <button
            type="button"
            disabled={!pdfReady || rulesLinkBusy || securityBusy}
            onClick={async () => {
              setRulesLinkErr(null);
              setRulesLinkOk(null);
              setRulesLinkBusy(true);
              try {
                const data = await postRulesLink(kgUrl, {
                  code_path_filter: normalizedScope.length ? normalizedScope : null,
                  use_llm: true,
                });
                setRulesLinkOk(
                  `Linked ${data.matches.length} rule↔code pairs (${data.violates_count} violates / ${data.implements_count} implements / ${data.related_count} related).`,
                );
                await onGraphRefresh();
              } catch (e: unknown) {
                setRulesLinkErr(e instanceof Error ? e.message : String(e));
              } finally {
                setRulesLinkBusy(false);
              }
            }}
            className="rounded-xl border border-sky-500/35 bg-sky-500/10 px-4 py-2.5 text-[13px] font-semibold text-sky-100 transition hover:bg-sky-500/20 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {rulesLinkBusy ? "Linking…" : "Link rules"}
          </button>
          <button
            type="button"
            disabled={!pdfReady || securityBusy || rulesLinkBusy}
            onClick={async () => {
              securityAbortRef.current?.abort();
              const ac = new AbortController();
              securityAbortRef.current = ac;
              setSecurityErr(null);
              setSecurityResult(null);
              setSecurityProgress(null);
              setSecurityAgentId(null);
              setSecurityBusy(true);
              try {
                const deploy = await postSecurityDeploy(kgUrl, {
                  scope: normalizedScope.length ? normalizedScope : undefined,
                });
                setSecurityAgentId(deploy.agent_id);

                const pollMs = 1500;
                const maxStatusPolls = 2000;
                for (let i = 0; i < maxStatusPolls; i++) {
                  if (ac.signal.aborted) throw new DOMException("Aborted", "AbortError");
                  await new Promise<void>((resolve, reject) => {
                    const t = window.setTimeout(resolve, pollMs);
                    const onAbort = () => {
                      window.clearTimeout(t);
                      reject(new DOMException("Aborted", "AbortError"));
                    };
                    ac.signal.addEventListener("abort", onAbort, { once: true });
                  });

                  const status = await getSecurityStatus(kgUrl, deploy.agent_id);
                  setSecurityProgress(status);
                  if (status.phase === "failed") {
                    throw new Error(status.error || "security agent failed");
                  }
                  if (status.phase === "done") break;
                  if (i === maxStatusPolls - 1) {
                    throw new Error("timed out waiting for security agent status");
                  }
                }

                let result: SecurityAgentResult | null = null;
                const maxResultPolls = 120;
                for (let i = 0; i < maxResultPolls; i++) {
                  if (ac.signal.aborted) throw new DOMException("Aborted", "AbortError");
                  result = await getSecurityResult(kgUrl, deploy.agent_id);
                  if (result) break;
                  await new Promise<void>((resolve, reject) => {
                    const t = window.setTimeout(resolve, 400);
                    const onAbort = () => {
                      window.clearTimeout(t);
                      reject(new DOMException("Aborted", "AbortError"));
                    };
                    ac.signal.addEventListener("abort", onAbort, { once: true });
                  });
                }
                if (!result) {
                  throw new Error("timed out waiting for security agent result");
                }
                setSecurityResult(result);
                await onGraphRefresh();
              } catch (e: unknown) {
                if (e instanceof DOMException && e.name === "AbortError") return;
                setSecurityErr(e instanceof Error ? e.message : String(e));
              } finally {
                setSecurityBusy(false);
                securityAbortRef.current = null;
              }
            }}
            className="rounded-xl bg-zinc-100 px-4 py-2.5 text-[13px] font-semibold text-zinc-900 transition hover:bg-white disabled:cursor-not-allowed disabled:opacity-40"
          >
            {securityBusy ? "Agent running…" : "Deploy security agent"}
          </button>
          {securityBusy ? (
            <button
              type="button"
              onClick={() => {
                securityAbortRef.current?.abort();
                setSecurityBusy(false);
              }}
              className="rounded-xl border border-white/[0.12] px-4 py-2.5 text-[13px] font-medium text-zinc-300 transition hover:bg-white/[0.06]"
            >
              Stop polling
            </button>
          ) : null}
        </div>

        {rulesLinkErr && (
          <p className="mt-2 rounded-lg border border-red-500/25 bg-red-950/40 px-2.5 py-2 text-[11px] text-red-200/95">
            {rulesLinkErr}
          </p>
        )}
        {rulesLinkOk && (
          <p className="mt-2 rounded-lg border border-emerald-500/20 bg-emerald-950/30 px-2.5 py-2 text-[11px] text-emerald-200/95">
            {rulesLinkOk}
          </p>
        )}
        {securityProgress && (
          <div className="mt-2 rounded-lg border border-white/[0.08] bg-zinc-900/50 px-2.5 py-2 font-mono text-[10px] text-zinc-400">
            {securityAgentId ? (
              <span className="block truncate text-zinc-500">agent_id {securityAgentId}</span>
            ) : null}
            <span className="text-zinc-300">{securityProgress.phase}</span>
            {securityProgress.current_file ? (
              <span className="mt-0.5 block truncate text-zinc-500" title={securityProgress.current_file}>
                {securityProgress.current_file}
              </span>
            ) : null}
            <span className="mt-0.5 block tabular-nums text-zinc-500">
              files {securityProgress.files_done}/{securityProgress.files_total} · violations {securityProgress.violations}
            </span>
          </div>
        )}
        {securityErr && (
          <p className="mt-2 rounded-lg border border-red-500/25 bg-red-950/40 px-2.5 py-2 text-[11px] text-red-200/95">
            {securityErr}
          </p>
        )}
        {securityResult && (
          <div className="mt-2 max-h-56 overflow-y-auto rounded-lg border border-white/[0.08] bg-black/25 p-2.5 text-[11px]">
            <p className="font-medium text-zinc-200">
              Done · {securityResult.files_analyzed} files · {securityResult.violations.length} findings ·{" "}
              {securityResult.violates_count} violates / {securityResult.implements_count} implements
            </p>
            {securityResult.violations.length > 0 ? (
              <ul className="mt-1.5 space-y-1.5 text-zinc-500">
                {securityResult.violations.slice(0, 12).map((v, i) => (
                  <li key={`${v.code_uri}-${i}`} className="border-l-2 border-sky-500/40 pl-2">
                    <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
                      <span className="font-mono text-[10px] text-sky-300/90">{v.edge_kind}</span>
                      <span className="font-mono text-[10px] text-zinc-400">{v.file_path}</span>
                    </div>
                    <p className="mt-0.5 text-[10px] text-zinc-500">{v.rule_text}</p>
                    <p className="mt-0.5 text-[10px] text-zinc-400">{v.explanation}</p>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="mt-1 text-zinc-600">No violations reported.</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
