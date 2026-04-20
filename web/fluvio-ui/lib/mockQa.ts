import { getMockGraph } from "./mockGraphs";
import type { QaAgent, QaGraphApprovals, QaGraphBundle, QaItemStatus } from "./qaTypes";
import { qaEdgeKey } from "./qaTypes";

function briefFromLabel(label: string): { summary: string; neighborContext: string } {
  const summary =
    label.length > 120 ? `${label.slice(0, 117)}…` : `Extracted entity: ${label}`;
  return {
    summary,
    neighborContext:
      "Neighbors are ranked by model confidence (token cost and edge probability). Approving an edge records human agreement with that weight for downstream training and retrieval.",
  };
}

function attachQa(
  g: { nodes: QaGraphBundle["nodes"]; edges: QaGraphBundle["edges"] },
): Record<string, QaGraphBundle["nodeQa"][string]> {
  const nodeQa: QaGraphBundle["nodeQa"] = {};
  for (const n of g.nodes) {
    nodeQa[n.id] = briefFromLabel(n.label);
  }
  return nodeQa;
}

export const QA_GRAPHS: QaGraphBundle[] = [
  {
    id: "gmail-q4",
    title: "Gmail · Q4 planning",
    subtitle: "Thread + people + labels",
    ...(() => {
      const { nodes, edges } = getMockGraph("gmail");
      return { nodes, edges, nodeQa: attachQa({ nodes, edges }) };
    })(),
  },
  {
    id: "github-ingest",
    title: "GitHub · kg-engine",
    subtitle: "PRs, symbols, CI",
    ...(() => {
      const { nodes, edges } = getMockGraph("github");
      return { nodes, edges, nodeQa: attachQa({ nodes, edges }) };
    })(),
  },
  {
    id: "spotify-focus",
    title: "Spotify · Focus session",
    subtitle: "Audio + playlist graph",
    ...(() => {
      const { nodes, edges } = getMockGraph("spotify");
      return { nodes, edges, nodeQa: attachQa({ nodes, edges }) };
    })(),
  },
  {
    id: "equities-watch",
    title: "Markets · Equities watchlist",
    subtitle: "Positions + news edges",
    ...(() => {
      const { nodes, edges } = getMockGraph("equities");
      return { nodes, edges, nodeQa: attachQa({ nodes, edges }) };
    })(),
  },
];

export const QA_AGENTS: QaAgent[] = [
  {
    id: "ag-gmail-1",
    graphId: "gmail-q4",
    name: "Ingest-Clerk",
    role: "Normalize mail → graph nodes",
    status: "running",
    environment: ["Gmail API (read)", "token bucket", "PII scrubber"],
    currentTask: "Resolve thread participants vs. CC noise for edge weights",
    progress: 0.62,
    trace: [
      "Fetched thread t1 headers",
      "Proposed person edges with confidence 0.78–0.82",
      "Waiting on human QA for label→thread link",
    ],
  },
  {
    id: "ag-github-1",
    graphId: "github-ingest",
    name: "Repo-Mapper",
    role: "Link PRs and symbols",
    status: "blocked",
    environment: ["GitHub GraphQL", "default branch main", "Rust analyzer hints"],
    currentTask: "Disambiguate IngestionPipeline symbol across crates",
    progress: 0.35,
    trace: ["Mapped PR #204 to repo", "Stuck: duplicate symbol path in meta graph"],
  },
  {
    id: "ag-qa-1",
    graphId: "*",
    name: "QA-Orchestrator",
    role: "Cross-graph consistency",
    status: "idle",
    environment: ["QA queue", "policy: no PII in eval exports"],
    currentTask: "Stand by for graph-level approvals",
    progress: 0,
    trace: ["Subscribed to 4 active graphs"],
  },
  {
    id: "ag-spotify-1",
    graphId: "spotify-focus",
    name: "Session-Analyst",
    role: "Session ↔ track attribution",
    status: "running",
    environment: ["Listening history API", "session window 52m"],
    currentTask: "Confirm playlist membership edge direction",
    progress: 0.78,
    trace: ["Built session node ss1", "Cross-checked valence cluster af1"],
  },
];

export function initialApprovalsFor(bundle: QaGraphBundle): QaGraphApprovals {
  const nodes: Record<string, QaItemStatus> = {};
  const edges: Record<string, QaItemStatus> = {};
  for (const n of bundle.nodes) nodes[n.id] = "pending";
  for (const e of bundle.edges) edges[qaEdgeKey(e.from, e.to)] = "pending";
  return { graph: "pending", nodes, edges };
}

export function emptyApprovalsMap(): Record<string, QaGraphApprovals> {
  const m: Record<string, QaGraphApprovals> = {};
  for (const g of QA_GRAPHS) m[g.id] = initialApprovalsFor(g);
  return m;
}
