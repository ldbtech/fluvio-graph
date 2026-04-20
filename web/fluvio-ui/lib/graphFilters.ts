import type { GraphEdge, GraphNode } from "./types";

/** Keep nodes whose `source` is in `sources` and edges whose endpoints are both kept. */
export function filterGraphBySource(
  nodes: GraphNode[],
  edges: GraphEdge[],
  source: string | readonly string[],
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const sources = typeof source === "string" ? [source] : source;
  const allow = new Set(sources);
  const ids = new Set(nodes.filter((n) => allow.has(n.source)).map((n) => n.id));
  const filteredNodes = nodes.filter((n) => ids.has(n.id));
  const filteredEdges = edges.filter((e) => ids.has(e.from) && ids.has(e.to));
  return { nodes: filteredNodes, edges: filteredEdges };
}

/** Live Gmail chunks from kg-engine used `"gmail"` before normalizer used `"email"`. */
export function filterLiveEmailGraph(
  nodes: GraphNode[],
  edges: GraphEdge[],
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  return filterGraphBySource(nodes, edges, ["email", "gmail"]);
}
