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

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/**
 * If the user asks about someone by name but did not tap the graph, map text to a connection
 * node id so kg-engine receives `graph_owner_id` and loads their Surreal ingests.
 */
export function inferPeerOwnerIdFromMessage(
  message: string,
  graph:       TwinGraphPayload,
  selfUserId:  string | null,
): string | undefined {
  const self = selfUserId?.trim().toLowerCase();
  const t = message.toLowerCase();
  const others = graph.nodes.filter((n) => {
    const id = n.id.trim();
    if (!UUID_RE.test(id)) return false;
    if (self && id.toLowerCase() === self) return false;
    return true;
  });
  if (others.length === 0) return undefined;

  const scored: { id: string; score: number }[] = [];
  for (const n of others) {
    const labelClean = n.label.replace(/\s*\(you\)\s*$/i, "").trim();
    const words = labelClean.split(/\s+/).filter((w) => w.length >= 2);
    let score = 0;
    for (const w of words) {
      if (t.includes(w.toLowerCase())) score += w.length;
    }
    if (score > 0) scored.push({ id: n.id, score });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored[0]?.id;
}

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
