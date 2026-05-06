/** Graph shape shared by Twin workspace UI and kg-engine `/twin/network`. */

export type TwinGraphNode = {
  id: string;
  label: string;
  page: string;
  source: string;
};

export type TwinGraphEdge = {
  from: string;
  to: string;
  token: number;
  probability: number;
  label: string;
};

export type TwinGraphPayload = {
  nodes: TwinGraphNode[];
  edges: TwinGraphEdge[];
};

export function buildTwinGraphContext(
  graph: TwinGraphPayload,
  selected: { id: string; label: string } | null,
): string {
  const nodeLines = graph.nodes.map((n) => `- ${n.label} (id=${n.id}, ${n.page}/${n.source})`).join("\n");
  const edgeLines = graph.edges
    .map((e) => `- ${e.from} → ${e.to} [${e.label}, p=${Number(e.probability).toFixed(2)}]`)
    .join("\n");
  const focus = selected
    ? `Focused node in the UI: "${selected.label}" (id=${selected.id}). Prefer this entity when the user says "them", "this person", or "here".`
    : "No single node is focused; use the full graph. If the user asks who to talk to for X, pick plausible nodes from the list.";
  return [
    "RELATIONSHIP GRAPH (from kg-engine /twin/network — your NFC connections and profile hub):",
    "NODES:",
    nodeLines || "(none)",
    "EDGES:",
    edgeLines || "(none)",
    focus,
  ].join("\n");
}
