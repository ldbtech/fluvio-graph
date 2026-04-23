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
    if (workspaceKind === "invest") {
      if (q.includes("agent") || q.includes("deploy"))
        return "[markets unified] Desk agents attach to the fusion hub; spin Tape librarian / Roll scheduler from Agents (mock).";
      if (q.includes("graph") || q.includes("node"))
        return "[markets unified] Joins equities, futures, crypto, news, vendor bars, and research PDF nodes — POST /graph/fusion/markets later.";
      return `[markets unified] Multi-vendor retrieval is mocked — planner would query each feed API in parallel with rate limits.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
    }
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

  if (ctx === "web") {
    const q = question.toLowerCase();
    if (q.includes("pdf") || q.includes("attach") || q.includes("merge"))
      return "[web crawl graph] PDFs attach to the same graph_id as the site crawl so CVE notes and policy docs can edge-link to routes, headers, and forms (mock).";
    if (q.includes("vuln") || q.includes("security") || q.includes("csrf") || q.includes("xss"))
      return "[web crawl graph] Cross-link literature nodes to findings; Rust would run scanners + LLM triage with human-in-the-loop before any auto-fix.";
    return `[web crawl graph] Site structure + linked learning materials — mock only until POST /ingest/web/* ships.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }

  if (ctx === "meta") {
    const q = question.toLowerCase();
    if (workspaceKind === "invest") {
      if (q.includes("agent") || q.includes("mesh"))
        return "[markets meta] Agent mesh registers desk workers; entitlements decide which vendor APIs each worker may call.";
      if (q.includes("health") || q.includes("status"))
        return "[markets meta] Capsules mirror API keys, quota, and last successful sync per feed — hook to real metrics in Rust.";
      return `[markets meta] Control-plane for feeds + optional research PDF slot — no live ticks in this mock.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
    }
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

  if (workspaceKind === "invest" && ctx === "equities") {
    return `[equities preview] Mock tape + events graph — wire Polygon / IBKR / internal OMS when ready.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "invest" && ctx === "futures") {
    return `[futures preview] Curves, rolls, and margin are synthetic — Rust would normalize contract codes + point to SPAN snapshots.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "invest" && ctx === "cryptocurrencies") {
    return `[crypto preview] Pair + flow nodes are illustrative — real venues need signed websocket feeds and venue-specific IDs.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "invest" && ctx === "fin_news") {
    return `[news preview] Headlines from multiple wires merge into sentiment clusters — no live wire in this UI.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "invest" && ctx === "fin_market_data") {
    return `[market data preview] Vendor A/B fusion is mocked — production would dedupe ticks and clock-sync with NTP offsets.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "invest" && ctx === "fin_research") {
    return `[research preview] Books + memos share the same PDF engine as Personal; bind graph_id for cross-links to tickers.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }

  if (workspaceKind === "design" && ctx === "des_bim") {
    return `[BIM preview] IFC spaces, clashes, and materials as graph nodes — Rust would stream federation deltas from your authoring tool.\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "design" && ctx === "des_arch_plans") {
    return `[arch plans preview] Sheets and room programs become queryable intent — link envelope targets to structural and energy subgraphs (mock).\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "design" && ctx === "des_structural") {
    return `[structural preview] Members, load envelopes, and drift checks as first-class nodes — FEM exports would attach provenance edges (mock).\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "design" && ctx === "des_civil_site") {
    return `[civil preview] Grading, utilities, and borings as site constraints feeding foundations and lateral earth pressure nodes (mock).\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "design" && ctx === "des_building_codes") {
    return `[codes preview] Clause graph with amendments — agents would diff your model assumptions vs adopted code edition (mock).\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
  }
  if (workspaceKind === "design" && ctx === "des_physics_sim") {
    return `[physics preview] Wind, thermal, and dynamics gates — pass/fail nodes link back to structural and architecture revisions (mock).\n\nYou asked: “${question.slice(0, 200)}${question.length > 200 ? "…" : ""}”`;
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
  if (q.includes("gmail") || q.includes("email") || q.includes("spotify") || q.includes("github"))
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
    case "spotify":
      return "Playlists, listens, and audio features become taste clusters — great for mood + focus subgraphs.";
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
    case "web":
      return "Crawl a site into a graph, attach PDFs for grounded learning (e.g. security playbooks), then run agents for triage and suggested fixes.";
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
    case "des_bim":
      return "BIM / IFC: federated models, clashes, and grids become nodes so agents can trace a change from sheet to steel order.";
    case "des_arch_plans":
      return "Architectural plans: room programs, envelopes, and accessibility paths as intent nodes tied to structural and code slices.";
    case "des_structural":
      return "Structural: analysis models, member schedules, and connection details as a graph that must stay consistent with physics checks.";
    case "des_civil_site":
      return "Civil & site: grading, utilities, geotech borings, and easements as constraints feeding foundations and storm design.";
    case "des_building_codes":
      return "Codes & loads: adopted editions, load combinations, and local amendments as typed nodes linked to every load path.";
    case "des_physics_sim":
      return "Physics & simulation: CFD, thermal bridges, dynamics, and settlement sensitivity as validation gates on the design graph.";
    default:
      return "";
  }
}
