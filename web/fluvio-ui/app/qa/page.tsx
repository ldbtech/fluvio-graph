"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { GraphCanvas } from "../components/workspace/GraphCanvas";
import { QaTopChrome } from "../components/qa/QaTopChrome";
import { emptyApprovalsMap, QA_AGENTS, QA_GRAPHS } from "@/lib/mockQa";
import type { QaGraphApprovals, QaItemStatus } from "@/lib/qaTypes";
import { qaEdgeKey } from "@/lib/qaTypes";
import type { GraphNode, SelectedNode } from "@/lib/types";

type RightTab = "node" | "edges" | "agents";

type QaDetail = {
  node: GraphNode;
  outgoing: { node: GraphNode; token: number; probability: number }[];
  incoming: { node: GraphNode; token: number; probability: number }[];
};

function statusLabel(s: QaItemStatus) {
  if (s === "approved") return "Approved";
  if (s === "rejected") return "Rejected";
  return "Pending";
}

function statusPill(s: QaItemStatus) {
  const base = "rounded-full px-2 py-0.5 text-[11px] font-medium";
  if (s === "approved") return `${base} bg-emerald-500/15 text-emerald-200/95`;
  if (s === "rejected") return `${base} bg-red-500/15 text-red-200/95`;
  return `${base} bg-zinc-800/90 text-zinc-500`;
}

const tabBase =
  "rounded-full px-3 py-1.5 text-[13px] font-medium tracking-tight transition-colors duration-200 ease-out";
const tabIdle = "text-zinc-500 hover:text-zinc-300";
const tabActive = "bg-zinc-100 text-zinc-900 shadow-sm";

export default function QaInfrastructurePage() {
  const svgRef = useRef<SVGSVGElement>(null);
  const [graphId, setGraphId] = useState(QA_GRAPHS[0]!.id);
  const [approvalsByGraph, setApprovalsByGraph] = useState<Record<string, QaGraphApprovals>>(emptyApprovalsMap);
  const [rightTab, setRightTab] = useState<RightTab>("node");
  const [detail, setDetail] = useState<QaDetail | null>(null);

  const bundle = useMemo(() => QA_GRAPHS.find((g) => g.id === graphId)!, [graphId]);
  const approvals = approvalsByGraph[graphId]!;
  const nodeMap = useMemo(() => new Map(bundle.nodes.map((n) => [n.id, n])), [bundle.nodes]);

  const enrichSelection = useCallback(
    (s: SelectedNode | null) => {
      if (!s) {
        setDetail(null);
        return;
      }
      const outgoing = bundle.edges
        .filter((e) => e.from === s.node.id)
        .map((e) => ({
          node: nodeMap.get(e.to)!,
          token: e.token,
          probability: e.probability,
        }));
      const incoming = bundle.edges
        .filter((e) => e.to === s.node.id)
        .map((e) => ({
          node: nodeMap.get(e.from)!,
          token: e.token,
          probability: e.probability,
        }));
      setDetail({ node: s.node, outgoing, incoming });
    },
    [bundle.edges, nodeMap],
  );

  const setGraphApproval = useCallback((s: QaItemStatus) => {
    setApprovalsByGraph((prev) => ({
      ...prev,
      [graphId]: { ...prev[graphId]!, graph: s },
    }));
  }, [graphId]);

  const setNodeApproval = useCallback(
    (nodeId: string, s: QaItemStatus) => {
      setApprovalsByGraph((prev) => ({
        ...prev,
        [graphId]: {
          ...prev[graphId]!,
          nodes: { ...prev[graphId]!.nodes, [nodeId]: s },
        },
      }));
    },
    [graphId],
  );

  const setEdgeApproval = useCallback(
    (from: string, to: string, s: QaItemStatus) => {
      const key = qaEdgeKey(from, to);
      setApprovalsByGraph((prev) => ({
        ...prev,
        [graphId]: {
          ...prev[graphId]!,
          edges: { ...prev[graphId]!.edges, [key]: s },
        },
      }));
    },
    [graphId],
  );

  const summary = useMemo(() => {
    const nNodes = bundle.nodes.length;
    const nEdges = bundle.edges.length;
    const nodesOk = Object.values(approvals.nodes).filter((x) => x === "approved").length;
    const edgesOk = Object.values(approvals.edges).filter((x) => x === "approved").length;
    return { nNodes, nEdges, nodesOk, edgesOk };
  }, [approvals, bundle]);

  const agentsForGraph = useMemo(
    () => QA_AGENTS.filter((a) => a.graphId === graphId || a.graphId === "*"),
    [graphId],
  );

  const nodeBrief = detail ? bundle.nodeQa[detail.node.id] : undefined;

  return (
    <main className="ui-main relative min-h-screen pt-12">
      <QaTopChrome />

      <div className="flex h-[calc(100vh-3rem)] min-h-0 w-full">
        <aside className="flex w-[min(100%,288px)] shrink-0 flex-col border-r border-white/[0.08] bg-[rgba(24,24,27,0.78)] backdrop-blur-2xl supports-[backdrop-filter]:bg-[rgba(24,24,27,0.65)]">
          <div className="shrink-0 border-b border-white/[0.06] px-4 pb-3 pt-4">
            <h2 className="text-[17px] font-semibold tracking-tight text-zinc-100">Graphs</h2>
            <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">Pick a mock graph to review.</p>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-3 py-4 [scrollbar-gutter:stable]">
            <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
              <ul className="divide-y divide-white/[0.06]">
                {QA_GRAPHS.map((g) => (
                  <li key={g.id}>
                    <button
                      type="button"
                      onClick={() => {
                        setGraphId(g.id);
                        setDetail(null);
                      }}
                      className={`flex w-full flex-col gap-0.5 px-3 py-3 text-left transition-colors ${
                        g.id === graphId
                          ? "bg-sky-500/[0.1]"
                          : "hover:bg-white/[0.04] active:bg-white/[0.06]"
                      }`}
                    >
                      <span className="text-[15px] font-medium text-zinc-100">{g.title}</span>
                      <span className="text-[12px] leading-snug text-zinc-500">{g.subtitle}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </div>

            <div className="mt-4 overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] p-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
              <p className="text-[13px] font-semibold text-zinc-400">Graph verdict</p>
              <div className="mt-2 flex flex-wrap items-center gap-2">
                <span className={statusPill(approvals.graph)}>{statusLabel(approvals.graph)}</span>
              </div>
              <p className="mt-2 text-[12px] leading-relaxed text-zinc-600">
                Approve the full graph when node and edge QA is sufficient for downstream use.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={() => setGraphApproval("approved")}
                  className="rounded-xl bg-zinc-100 px-3 py-2 text-[12px] font-semibold text-zinc-900 transition hover:bg-white active:scale-[0.99]"
                >
                  Approve graph
                </button>
                <button
                  type="button"
                  onClick={() => setGraphApproval("rejected")}
                  className="rounded-xl border border-red-500/25 bg-red-950/35 px-3 py-2 text-[12px] font-semibold text-red-200/95 transition hover:bg-red-950/55"
                >
                  Reject
                </button>
                <button
                  type="button"
                  onClick={() => setGraphApproval("pending")}
                  className="rounded-xl border border-white/[0.1] px-3 py-2 text-[12px] font-medium text-zinc-400 transition hover:bg-white/[0.05]"
                >
                  Reset
                </button>
              </div>
            </div>
          </div>
        </aside>

        <section className="relative min-h-0 min-w-0 flex-1 bg-black/25">
          <GraphCanvas
            svgRef={svgRef}
            nodes={bundle.nodes}
            edges={bundle.edges}
            onSelect={enrichSelection}
            qaNodeStatus={approvals.nodes}
            qaEdgeStatus={approvals.edges}
          />
          <div className="pointer-events-none absolute left-1/2 top-3 z-10 flex -translate-x-1/2 flex-col items-center gap-0.5 rounded-2xl border border-white/[0.08] bg-white/[0.04] px-4 py-2 text-center backdrop-blur-md">
            <span className="text-[13px] font-semibold text-zinc-100">{bundle.title}</span>
            <span className="tabular-nums text-[12px] text-zinc-500">
              {summary.nodesOk}/{summary.nNodes} nodes · {summary.edgesOk}/{summary.nEdges} edges approved
            </span>
          </div>
        </section>

        <aside className="flex w-[min(100%,420px)] shrink-0 flex-col border-l border-white/[0.08] bg-[rgba(24,24,27,0.78)] backdrop-blur-2xl supports-[backdrop-filter]:bg-[rgba(24,24,27,0.65)]">
          <div className="shrink-0 border-b border-white/[0.06] px-3 py-3">
            <nav
              className="flex w-full rounded-full border border-white/[0.06] bg-zinc-900/80 p-0.5 shadow-inner"
              aria-label="Review panel"
            >
              <button
                type="button"
                onClick={() => setRightTab("node")}
                className={`flex-1 ${tabBase} ${rightTab === "node" ? tabActive : tabIdle}`}
              >
                Node
              </button>
              <button
                type="button"
                onClick={() => setRightTab("edges")}
                className={`flex-1 ${tabBase} ${rightTab === "edges" ? tabActive : tabIdle}`}
              >
                Edges
              </button>
              <button
                type="button"
                onClick={() => setRightTab("agents")}
                className={`flex-1 ${tabBase} ${rightTab === "agents" ? tabActive : tabIdle}`}
              >
                Agents
              </button>
            </nav>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto p-4 [scrollbar-gutter:stable]">
            {rightTab === "node" && (
              <div className="space-y-4">
                {!detail && (
                  <p className="text-[13px] leading-relaxed text-zinc-500">
                    Select a node on the canvas to read its explanation, see inbound and outbound neighbors, and approve
                    or reject the node for this graph.
                  </p>
                )}
                {detail && nodeBrief && (
                  <>
                    <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] p-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-[11px] font-semibold uppercase tracking-wide text-zinc-500">Node</p>
                          <p className="mt-1 text-[13px] font-medium leading-snug text-zinc-200">{detail.node.label}</p>
                        </div>
                        <span className={statusPill(approvals.nodes[detail.node.id] ?? "pending")}>
                          {statusLabel(approvals.nodes[detail.node.id] ?? "pending")}
                        </span>
                      </div>
                    </div>

                    <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.02] p-3">
                      <p className="text-[11px] font-semibold text-zinc-500">What it is</p>
                      <p className="mt-1.5 text-[13px] leading-relaxed text-zinc-500">{nodeBrief.summary}</p>
                    </div>

                    <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.02] p-3">
                      <p className="text-[11px] font-semibold text-zinc-500">Neighbors & meaning</p>
                      <p className="mt-1.5 text-[13px] leading-relaxed text-zinc-500">{nodeBrief.neighborContext}</p>
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        onClick={() => setNodeApproval(detail.node.id, "approved")}
                        className="rounded-xl bg-zinc-100 px-4 py-2.5 text-[13px] font-semibold text-zinc-900 transition hover:bg-white active:scale-[0.99]"
                      >
                        Approve node
                      </button>
                      <button
                        type="button"
                        onClick={() => setNodeApproval(detail.node.id, "rejected")}
                        className="rounded-xl border border-red-500/25 bg-red-950/35 px-4 py-2.5 text-[13px] font-semibold text-red-200/95 transition hover:bg-red-950/55"
                      >
                        Reject node
                      </button>
                    </div>

                    {detail.outgoing.length > 0 && (
                      <div>
                        <p className="mb-2 px-1 text-[12px] font-semibold text-zinc-500">Outgoing</p>
                        <ul className="space-y-2">
                          {detail.outgoing.map((n, i) => (
                            <li
                              key={`o-${i}`}
                              className="rounded-xl border border-white/[0.08] bg-white/[0.02] px-3 py-2.5 text-[12px]"
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="min-w-0 truncate text-zinc-400">→ {n.node.label.slice(0, 72)}</span>
                                <span className="shrink-0 tabular-nums font-medium text-sky-300/90">
                                  {(n.probability * 100).toFixed(0)}%
                                </span>
                              </div>
                              <p className="mt-1.5 text-[11px] leading-relaxed text-zinc-600">
                                Model used {n.token} tokens on this edge — confirm the weight matches the relationship you
                                expect.
                              </p>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}

                    {detail.incoming.length > 0 && (
                      <div>
                        <p className="mb-2 px-1 text-[12px] font-semibold text-zinc-500">Incoming</p>
                        <ul className="space-y-2">
                          {detail.incoming.map((n, i) => (
                            <li
                              key={`i-${i}`}
                              className="rounded-xl border border-white/[0.08] bg-white/[0.02] px-3 py-2.5 text-[12px]"
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="min-w-0 truncate text-zinc-400">← {n.node.label.slice(0, 72)}</span>
                                <span className="shrink-0 tabular-nums font-medium text-violet-300/90">
                                  {(n.probability * 100).toFixed(0)}%
                                </span>
                              </div>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </>
                )}
              </div>
            )}

            {rightTab === "edges" && (
              <div className="space-y-4">
                <p className="text-[13px] leading-relaxed text-zinc-500">
                  Each edge carries a learned weight (probability) and a token cost. Approve when the direction and
                  strength match your judgment.
                </p>
                <ul className="space-y-2">
                  {bundle.edges.map((e) => {
                    const key = qaEdgeKey(e.from, e.to);
                    const st = approvals.edges[key] ?? "pending";
                    const fromN = nodeMap.get(e.from);
                    const toN = nodeMap.get(e.to);
                    return (
                      <li
                        key={key}
                        className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] px-3 py-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="min-w-0 truncate text-[12px] text-zinc-400">
                            {fromN?.label.slice(0, 40)} → {toN?.label.slice(0, 40)}
                          </span>
                          <span className={statusPill(st)}>{statusLabel(st)}</span>
                        </div>
                        <div className="mt-2 flex flex-wrap items-center gap-3 text-[11px] tabular-nums text-zinc-600">
                          <span>Weight {(e.probability * 100).toFixed(1)}%</span>
                          <span>Tokens {e.token}</span>
                        </div>
                        <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-white/[0.08]">
                          <div
                            className="h-full rounded-full bg-sky-500/75"
                            style={{ width: `${e.probability * 100}%` }}
                          />
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          <button
                            type="button"
                            onClick={() => setEdgeApproval(e.from, e.to, "approved")}
                            className="rounded-xl bg-zinc-100 px-3 py-1.5 text-[12px] font-semibold text-zinc-900 transition hover:bg-white"
                          >
                            Approve weight
                          </button>
                          <button
                            type="button"
                            onClick={() => setEdgeApproval(e.from, e.to, "rejected")}
                            className="rounded-xl border border-red-500/25 bg-red-950/30 px-3 py-1.5 text-[12px] font-semibold text-red-200/95 transition hover:bg-red-950/50"
                          >
                            Reject
                          </button>
                        </div>
                      </li>
                    );
                  })}
                </ul>
              </div>
            )}

            {rightTab === "agents" && (
              <div className="space-y-4">
                <p className="text-[13px] leading-relaxed text-zinc-500">
                  Background workers for this graph (and cross-graph orchestration). Traces are illustrative until wired
                  to your task runner.
                </p>
                {agentsForGraph.map((a) => (
                  <article
                    key={a.id}
                    className="overflow-hidden rounded-2xl border border-white/[0.08] bg-white/[0.03] p-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="text-[14px] font-semibold text-zinc-100">{a.name}</p>
                        <p className="mt-0.5 text-[12px] text-zinc-500">{a.role}</p>
                      </div>
                      <span
                        className={`shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium ${
                          a.status === "running"
                            ? "bg-sky-500/15 text-sky-200/95"
                            : a.status === "blocked"
                              ? "bg-amber-500/15 text-amber-200/95"
                              : a.status === "done"
                                ? "bg-emerald-500/15 text-emerald-200/95"
                                : "bg-zinc-800/90 text-zinc-500"
                        }`}
                      >
                        {a.status}
                      </span>
                    </div>
                    <p className="mt-3 text-[11px] font-semibold text-zinc-500">Environment</p>
                    <ul className="mt-1 list-inside list-disc text-[12px] leading-relaxed text-zinc-600">
                      {a.environment.map((x) => (
                        <li key={x}>{x}</li>
                      ))}
                    </ul>
                    <p className="mt-3 text-[11px] font-semibold text-zinc-500">Current task</p>
                    <p className="mt-1 text-[13px] text-zinc-400">{a.currentTask}</p>
                    <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-white/[0.08]">
                      <div
                        className="h-full rounded-full bg-emerald-500/65"
                        style={{ width: `${Math.round(a.progress * 100)}%` }}
                      />
                    </div>
                    <p className="mt-3 text-[11px] font-semibold text-zinc-500">Trace</p>
                    <ol className="mt-1 space-y-1 text-[11px] leading-relaxed text-zinc-600">
                      {a.trace.map((line, i) => (
                        <li key={i}>
                          {i + 1}. {line}
                        </li>
                      ))}
                    </ol>
                  </article>
                ))}
              </div>
            )}
          </div>
        </aside>
      </div>
    </main>
  );
}
