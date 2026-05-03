import type { ConnectorId, WorkspaceKind } from "./types";

const MOCK_REPLIES = [
  "That connector is not wired to the graph engine yet — once it is, I will route tasks through your live workspace graph.",
  "Preview mode: I can outline the workflow, but there is no ingested data behind this answer yet.",
  "When this source is connected, agents will be able to pull structured facts here and attach them as nodes.",
];

/** `brainContext` is a BrainTab string: documents, gmail, …, unified, meta */
export function mockAssistantReply(
  question: string,
  brainContext?: string,
  workspaceKind: WorkspaceKind = "personal",
): string {
  const ctx = brainContext ?? "";

  if (ctx === "unified") {
    const q = question.toLowerCase();
    if (workspaceKind === "design") {
      if (q.includes("physics") || q.includes("load") || q.includes("struct"))
        return "[design unified] Fusion joins BIM, structural envelopes, civil site facts, code clauses, and solver outputs so agents can answer “does this geometry violate constraints?” before steel is ordered (mock).";
      if (q.includes("agent") || q.includes("deploy"))
        return "[design unified] Orchestrator would route BIM deltas, analysis revisions, and code checks in parallel — deploy validation workers from Agents (mock).";
      if (q.includes("graph") || q.includes("node"))
        return "[design unified] Materialized join across architecture + civil slices; Rust would expose POST /graph/fusion/design with version pins per model revision.";
      return `[design unified] Cross-discipline retrieval is mocked — planner would fan out to IFC, FEM, geotech, and code subgraphs with provenance on every edge.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
    }
    if (q.includes("agent") || q.includes("deploy"))
      return "[unified fusion] Orchestrator agents fan out to each subgraph; deploy from the Agents tab to simulate mesh workers.";
    if (q.includes("graph") || q.includes("node"))
      return "[unified fusion] This view is a materialized join across domains. Rust will expose POST /graph/fusion/rebuild and stream version tokens to clients.";
    return `[unified fusion] Cross-domain retrieval is mocked — your question routes to a planner that would fan out to Gmail, PDF, GitHub, … subgraphs in parallel.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }

  if (ctx === "meta") {
    const q = question.toLowerCase();
    if (workspaceKind === "design") {
      if (q.includes("agent") || q.includes("mesh"))
        return "[design meta] Mesh registers solvers, clash engines, and code parsers; policy gates who may mutate structural nodes.";
      if (q.includes("health") || q.includes("status"))
        return "[design meta] Capsules track IFC federation checksums, solver convergence, and amendment packs — wire to CI + simulation runners in Rust.";
      return `[design meta] Control-plane for BIM, loads, and physics contracts — no live IFC in this mock.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
    }
    if (q.includes("agent") || q.includes("mesh"))
      return "[meta-graph] The agent mesh node is where autoscale workers register; policies decide which subgraphs they may touch.";
    if (q.includes("health") || q.includes("status"))
      return "[meta-graph] Each capsule reflects connector health + sync lag; wire this to Prometheus / internal heartbeats in Rust.";
    return `[meta-graph] Control-plane only — answers describe how domains attach to the orchestrator, not row-level facts.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }

  if (workspaceKind === "design" && ctx === "des_arch_plans") {
    return `[architecture preview] Live design generation + scene updates are available through the design chat commands.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }

  const prefix =
    ctx && ctx !== "documents"
      ? `[${ctx} preview brain] `
      : ctx === "documents"
        ? "[documents graph empty — mock reply] "
        : "";

  const q = question.toLowerCase();
  if (q.includes("agent") || q.includes("deploy"))
    return `${prefix}${MOCK_REPLIES[2]} Try the Agents tab — deployments are simulated for now, but the UX is real.`;
  if (q.includes("gmail") || q.includes("email") || q.includes("github"))
    return `${prefix}${MOCK_REPLIES[0]} For grounded answers on PDFs, open the PDF tab with a live ingested graph.`;
  if (q.includes("graph") || q.includes("node"))
    return `${prefix}${MOCK_REPLIES[1]} Each tab is a separate graph slice; Rust will serve GET /graph?domain=… per source.`;
  const pick = MOCK_REPLIES[Math.floor(Math.random() * MOCK_REPLIES.length)];
  return `${prefix}${pick}\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
}

export function mockConnectorNarrative(id: ConnectorId): string {
  switch (id) {
    case "gmail":
      return "OAuth + scoped read would stream threads into message entities linked to people and projects.";
    case "github":
      return "Repos, PRs, and symbols map to a codebase layer you can query like an architecture doc.";
    case "calendar":
      return "Events anchor time; recurring patterns become availability and commitment edges.";
    case "whatsapp":
      return "Chats (with consent) fold into conversation graphs tied to contacts and tasks.";
    case "slack":
      return "Channels and threads become team signal; bots can subscribe to deltas.";
    case "notion":
      return "Pages and databases flatten into typed nodes with back-links preserved.";
    case "equities":
      return "Equities tape: quotes, fundamentals, and calendar events become ticker-centric nodes with cross-links to news and research.";
    case "futures":
      return "Futures: continuous contracts, roll schedules, and margin estimates as graph edges for macro overlays.";
    case "cryptocurrencies":
      return "Crypto: per-venue pairs, on-chain exposure, and funding risk folded into a separate risk subgraph.";
    case "fin_news":
      return "News: multiple wire APIs deduped into headline → entity edges that point into equities and futures slices.";
    case "fin_market_data":
      return "Market data: vendor A/B bars and depth fused with conflict resolution and provenance on every edge.";
    case "fin_research":
      return "Research: desk PDFs and books ingest like documents; citations link to live tickers in the markets brain.";
    case "des_arch_plans":
      return "Architecture: generate from brief and modify in realtime; Three.js view updates from `/architecture/generate` and `/architecture/modify`.";
    default:
      return "";
  }
}
