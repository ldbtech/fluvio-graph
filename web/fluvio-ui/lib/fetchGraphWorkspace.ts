import type { GraphEdge, GraphNode } from "./types";

export type GraphLoadProgress = { message: string; percent: number };

export type GraphMetaResult = {
  graph_total_nodes: number;
  graph_total_edges: number;
  source_counts: Record<string, number>;
};

export type GraphWorkspaceResult = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  source_counts: Record<string, number>;
  graph_total_nodes: number;
  graph_total_edges: number;
  edges_truncated: boolean;
  nodes_capped: boolean;
};

/** Keeps D3 layout + JSON.parse within a tolerable range on consumer hardware. */
const MAX_UI_NODES = 2_200;
const NODE_PAGE = 250;
/** Match server default; explicit so responses stay small enough to parse without freezing. */
const MAX_EDGES_IN_RESPONSE = 36_000;

function yieldToMain(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

/** Tiny payload for Sources tab readiness (counts only). */
export async function fetchGraphMeta(kgUrl: string): Promise<GraphMetaResult> {
  const metaRes = await fetch(`${kgUrl}/graph/meta`);
  if (!metaRes.ok) throw new Error(`graph/meta HTTP ${metaRes.status}`);
  return (await metaRes.json()) as GraphMetaResult;
}

function aborted(signal: AbortSignal | undefined) {
  return Boolean(signal?.aborted);
}

/**
 * Loads workspace graph in pages plus a capped edge subset. Yields between steps so the UI
 * can paint progress instead of freezing on one huge JSON.parse.
 */
export async function fetchGraphWorkspace(
  kgUrl: string,
  onProgress: (p: GraphLoadProgress) => void,
  signal?: AbortSignal,
): Promise<GraphWorkspaceResult> {
  const req = (input: RequestInfo | URL, init?: RequestInit) =>
    fetch(input, { ...init, signal });

  onProgress({ message: "Fetching graph summary…", percent: 2 });
  const metaRes = await req(`${kgUrl}/graph/meta`);
  if (!metaRes.ok) throw new Error(`graph/meta HTTP ${metaRes.status}`);
  const meta = (await metaRes.json()) as GraphMetaResult;
  if (aborted(signal)) throw new DOMException("Aborted", "AbortError");
  await yieldToMain();

  const total = meta.graph_total_nodes;
  const target = Math.min(total, MAX_UI_NODES);
  const nodes_capped = total > MAX_UI_NODES;

  onProgress({ message: `Loading nodes (0 / ${target})…`, percent: 6 });

  const nodes: GraphNode[] = [];
  let offset = 0;
  while (offset < target) {
    if (aborted(signal)) throw new DOMException("Aborted", "AbortError");
    const lim = Math.min(NODE_PAGE, target - offset);
    const r = await req(`${kgUrl}/graph/nodes?offset=${offset}&limit=${lim}`);
    if (!r.ok) throw new Error(`graph/nodes HTTP ${r.status}`);
    const chunk = (await r.json()) as {
      nodes: GraphNode[];
      returned: number;
      done: boolean;
      total_nodes: number;
    };
    nodes.push(...(chunk.nodes ?? []));
    offset += chunk.returned;
    if (chunk.returned === 0) break;
    const pct = 6 + Math.min(44, Math.floor(44 * (nodes.length / Math.max(1, target))));
    onProgress({
      message: `Loading nodes (${nodes.length} / ${target})…`,
      percent: pct,
    });
    await yieldToMain();
    if (chunk.done) break;
  }

  onProgress({ message: "Loading edges for visible nodes…", percent: 52 });
  const ids = nodes.map((n) => n.id);
  const er = await req(`${kgUrl}/graph/edges_subset`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ids, max_edges: MAX_EDGES_IN_RESPONSE }),
  });
  if (!er.ok) throw new Error(`graph/edges_subset HTTP ${er.status}`);
  const ej = (await er.json()) as {
    edges: GraphEdge[];
    truncated: boolean;
    returned_edges: number;
  };
  if (aborted(signal)) throw new DOMException("Aborted", "AbortError");
  await yieldToMain();

  onProgress({ message: "Finalizing…", percent: 96 });

  return {
    nodes,
    edges: ej.edges ?? [],
    source_counts: meta.source_counts ?? {},
    graph_total_nodes: meta.graph_total_nodes,
    graph_total_edges: meta.graph_total_edges,
    edges_truncated: Boolean(ej.truncated),
    nodes_capped,
  };
}
