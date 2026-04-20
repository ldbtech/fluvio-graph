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
  if (s === "approved") return "approved";
  if (s === "rejected") return "rejected";
  return "pending";
}

function statusPill(s: QaItemStatus) {
  const base = "rounded-full px-2 py-0.5 font-mono text-[10px]";
  if (s === "approved") return `${base} bg-emerald-500/20 text-emerald-200`;
  if (s === "rejected") return `${base} bg-red-500/20 text-red-200`;
  return `${base} bg-slate-500/15 text-slate-400`;
}

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
    <main className="relative min-h-screen bg-[#04040f] pt-12 text-slate-200">
      <QaTopChrome />

      <div className="flex h-[calc(100vh-3rem)] min-h-0 w-full">
        <aside className="flex w-[min(100%,280px)] shrink-0 flex-col gap-4 border-r border-emerald-400/10 bg-[#05051a]/80 p-4">
          <p className="font-mono text-[10px] uppercase tracking-wider text-emerald-400/80">graphs</p>
          <div className="flex flex-col gap-2">
            {QA_GRAPHS.map((g) => (
              <button
                key={g.id}
                type="button"
                onClick={() => {
                  setGraphId(g.id);
                  setDetail(null);
                }}
                className={`rounded-xl border px-3 py-2.5 text-left transition ${
                  g.id === graphId
                    ? "border-emerald-400/40 bg-emerald-500/[0.08] shadow-[0_0_20px_rgba(16,185,129,0.08)]"
                    : "border-white/10 bg-white/[0.02] hover:border-emerald-400/25"
                }`}
              >
                <span className="block font-mono text-xs text-emerald-100/95">{g.title}</span>
                <span className="mt-0.5 block font-mono text-[10px] text-slate-500">{g.subtitle}</span>
              </button>
            ))}
          </div>

          <div className="mt-auto space-y-3 rounded-xl border border-white/10 bg-white/[0.02] p-3">
            <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">graph verdict</p>
            <div className="flex flex-wrap gap-2">
              <span className={statusPill(approvals.graph)}>{statusLabel(approvals.graph)}</span>
            </div>
            <p className="font-mono text-[10px] leading-relaxed text-slate-500">
              Approve the full graph when node and edge QA is good enough for downstream use.
            </p>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={() => setGraphApproval("approved")}
                className="rounded-lg bg-emerald-500/25 px-3 py-1.5 font-mono text-[10px] text-emerald-100 hover:bg-emerald-500/35"
              >
                Approve graph
              </button>
              <button
                type="button"
                onClick={() => setGraphApproval("rejected")}
                className="rounded-lg border border-red-400/30 px-3 py-1.5 font-mono text-[10px] text-red-200/90 hover:bg-red-500/10"
              >
                Reject
              </button>
              <button
                type="button"
                onClick={() => setGraphApproval("pending")}
                className="rounded-lg border border-white/15 px-3 py-1.5 font-mono text-[10px] text-slate-400 hover:bg-white/5"
              >
                Reset
              </button>
            </div>
          </div>
        </aside>

        <section className="relative min-h-0 min-w-0 flex-1">
          <GraphCanvas
            svgRef={svgRef}
            nodes={bundle.nodes}
            edges={bundle.edges}
            onSelect={enrichSelection}
            qaNodeStatus={approvals.nodes}
            qaEdgeStatus={approvals.edges}
          />
          <div className="pointer-events-none absolute left-1/2 top-3 z-10 flex -translate-x-1/2 flex-col items-center gap-1 rounded-xl border border-emerald-400/15 bg-[#0a0a1a]/90 px-4 py-2 font-mono text-[11px] text-slate-400 backdrop-blur-sm">
            <span className="text-emerald-200/90">{bundle.title}</span>
            <span>
              {summary.nodesOk}/{summary.nNodes} nodes · {summary.edgesOk}/{summary.nEdges} edges approved
            </span>
          </div>
        </section>

        <aside className="flex w-[min(100%,420px)] shrink-0 flex-col border-l border-emerald-400/10 bg-[#05051a]/90">
          <div className="flex border-b border-white/10 font-mono text-[11px]">
            {(["node", "edges", "agents"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setRightTab(t)}
                className={`flex-1 px-3 py-3 transition ${
                  rightTab === t
                    ? "border-b-2 border-emerald-400/60 bg-emerald-500/[0.06] text-emerald-100"
                    : "text-slate-500 hover:text-slate-300"
                }`}
              >
                {t === "node" && "Node QA"}
                {t === "edges" && "Edges & weights"}
                {t === "agents" && "Agents"}
              </button>
            ))}
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            {rightTab === "node" && (
              <div className="space-y-4">
                {!detail && (
                  <p className="font-mono text-sm leading-relaxed text-slate-500">
                    Select a node on the canvas to read its explanation, see inbound and outbound neighbors, and
                    approve or reject the node for this graph.
                  </p>
                )}
                {detail && nodeBrief && (
                  <>
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="font-mono text-[10px] uppercase tracking-wider text-slate-500">node</p>
                        <p className="mt-1 font-mono text-xs leading-relaxed text-slate-300">{detail.node.label}</p>
                      </div>
                      <span className={statusPill(approvals.nodes[detail.node.id] ?? "pending")}>
                        {statusLabel(approvals.nodes[detail.node.id] ?? "pending")}
                      </span>
                    </div>

                    <div>
                      <p className="font-mono text-[10px] uppercase tracking-wider text-emerald-400/80">what it is</p>
                      <p className="mt-1 font-mono text-xs leading-relaxed text-slate-400">{nodeBrief.summary}</p>
                    </div>

                    <div>
                      <p className="font-mono text-[10px] uppercase tracking-wider text-emerald-400/80">
                        neighbors & meaning
                      </p>
                      <p className="mt-1 font-mono text-xs leading-relaxed text-slate-400">{nodeBrief.neighborContext}</p>
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        onClick={() => setNodeApproval(detail.node.id, "approved")}
                        className="rounded-lg bg-emerald-500/25 px-3 py-2 font-mono text-[11px] text-emerald-100"
                      >
                        Approve node
                      </button>
                      <button
                        type="button"
                        onClick={() => setNodeApproval(detail.node.id, "rejected")}
                        className="rounded-lg border border-red-400/35 px-3 py-2 font-mono text-[11px] text-red-200/90"
                      >
                        Reject node
                      </button>
                    </div>

                    {detail.outgoing.length > 0 && (
                      <div>
                        <p className="mb-2 font-mono text-[10px] uppercase tracking-wider text-slate-500">outgoing</p>
                        <ul className="space-y-2">
                          {detail.outgoing.map((n, i) => (
                            <li
                              key={`o-${i}`}
                              className="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2 font-mono text-[11px]"
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="text-slate-400">→ {n.node.label.slice(0, 72)}</span>
                                <span className="shrink-0 text-cyan-300/90">{(n.probability * 100).toFixed(0)}%</span>
                              </div>
                              <p className="mt-1 text-[10px] text-slate-600">
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
                        <p className="mb-2 font-mono text-[10px] uppercase tracking-wider text-slate-500">incoming</p>
                        <ul className="space-y-2">
                          {detail.incoming.map((n, i) => (
                            <li
                              key={`i-${i}`}
                              className="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2 font-mono text-[11px]"
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="text-slate-400">← {n.node.label.slice(0, 72)}</span>
                                <span className="shrink-0 text-violet-300/90">{(n.probability * 100).toFixed(0)}%</span>
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
              <div className="space-y-3">
                <p className="font-mono text-xs leading-relaxed text-slate-500">
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
                        className="rounded-xl border border-white/10 bg-white/[0.03] px-3 py-3 font-mono text-[11px]"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-slate-400">
                            {fromN?.label.slice(0, 40)} → {toN?.label.slice(0, 40)}
                          </span>
                          <span className={statusPill(st)}>{statusLabel(st)}</span>
                        </div>
                        <div className="mt-2 flex flex-wrap items-center gap-3 text-[10px] text-slate-500">
                          <span>weight {(e.probability * 100).toFixed(1)}%</span>
                          <span>tokens {e.token}</span>
                        </div>
                        <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-white/10">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-cyan-500/80 to-violet-500/80"
                            style={{ width: `${e.probability * 100}%` }}
                          />
                        </div>
                        <div className="mt-2 flex flex-wrap gap-2">
                          <button
                            type="button"
                            onClick={() => setEdgeApproval(e.from, e.to, "approved")}
                            className="rounded-md bg-emerald-500/20 px-2 py-1 text-[10px] text-emerald-200"
                          >
                            Approve weight
                          </button>
                          <button
                            type="button"
                            onClick={() => setEdgeApproval(e.from, e.to, "rejected")}
                            className="rounded-md border border-red-400/30 px-2 py-1 text-[10px] text-red-200/90"
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
                <p className="font-mono text-xs leading-relaxed text-slate-500">
                  Background workers for this graph (and cross-graph orchestration). Traces are illustrative until wired
                  to your task runner.
                </p>
                {agentsForGraph.map((a) => (
                  <article
                    key={a.id}
                    className="rounded-xl border border-white/10 bg-[#060616]/80 p-3 font-mono text-[11px]"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="text-emerald-200/95">{a.name}</p>
                        <p className="mt-0.5 text-[10px] text-slate-500">{a.role}</p>
                      </div>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] ${
                          a.status === "running"
                            ? "bg-cyan-500/15 text-cyan-200"
                            : a.status === "blocked"
                              ? "bg-amber-500/15 text-amber-200"
                              : a.status === "done"
                                ? "bg-emerald-500/15 text-emerald-200"
                                : "bg-slate-500/15 text-slate-400"
                        }`}
                      >
                        {a.status}
                      </span>
                    </div>
                    <p className="mt-3 text-[10px] uppercase tracking-wider text-slate-600">environment</p>
                    <ul className="mt-1 list-inside list-disc text-[10px] text-slate-400">
                      {a.environment.map((x) => (
                        <li key={x}>{x}</li>
                      ))}
                    </ul>
                    <p className="mt-3 text-[10px] uppercase tracking-wider text-slate-600">current task</p>
                    <p className="mt-1 text-slate-300">{a.currentTask}</p>
                    <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-white/10">
                      <div
                        className="h-full rounded-full bg-emerald-500/60"
                        style={{ width: `${Math.round(a.progress * 100)}%` }}
                      />
                    </div>
                    <p className="mt-3 text-[10px] uppercase tracking-wider text-slate-600">trace</p>
                    <ol className="mt-1 space-y-1 text-[10px] text-slate-500">
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
