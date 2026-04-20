import type { GraphEdge, GraphNode } from "./types";

export type QaItemStatus = "pending" | "approved" | "rejected";

/** Stable key for an undirected display edge (canvas uses directed edges). */
export function qaEdgeKey(from: string, to: string): string {
  return `${from}\u2192${to}`;
}

export type QaNodeBrief = {
  summary: string;
  /** How this node relates to the graph and its outgoing neighbors. */
  neighborContext: string;
};

export type QaGraphBundle = {
  id: string;
  title: string;
  subtitle: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
  nodeQa: Record<string, QaNodeBrief>;
};

export type QaAgentStatus = "idle" | "running" | "blocked" | "done";

export type QaAgent = {
  id: string;
  graphId: string;
  name: string;
  role: string;
  status: QaAgentStatus;
  /** Runtime context the agent sees (tools, corpus, constraints). */
  environment: string[];
  currentTask: string;
  progress: number;
  /** Recent steps toward the task. */
  trace: string[];
};

export type QaGraphApprovals = {
  graph: QaItemStatus;
  nodes: Record<string, QaItemStatus>;
  edges: Record<string, QaItemStatus>;
};
