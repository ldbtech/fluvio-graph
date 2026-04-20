import type { ConnectorId, ConnectorStatus, GraphEdge, GraphNode, WorkspaceKind } from "./types";
import { filterGraphBySource, filterLiveEmailGraph } from "./graphFilters";
import { INVEST_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "./workspaceKinds";

/** Small deterministic graphs so each connector “brain” looks different in the UI. */
export function getMockGraph(domain: ConnectorId): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const mk = (id: string, label: string, page: string): GraphNode => ({
    id: `${domain}:${id}`,
    label,
    page,
    source: domain,
  });

  switch (domain) {
    case "gmail": {
      const nodes = [
        mk("t1", "Thread: Q4 planning — budget draft", "1"),
        mk("p1", "Person: Alex Rivera", "2"),
        mk("p2", "Person: Jordan Lee", "2"),
        mk("l1", "Label: work/finance", "3"),
        mk("m1", "Message snippet: “Can we lock scope by Friday?”", "1"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 12, probability: 0.82 },
        { from: nodes[0].id, to: nodes[2].id, token: 10, probability: 0.78 },
        { from: nodes[0].id, to: nodes[4].id, token: 24, probability: 0.91 },
        { from: nodes[3].id, to: nodes[0].id, token: 6, probability: 0.65 },
      ];
      return { nodes, edges };
    }
    case "spotify": {
      const nodes = [
        mk("tr1", "Track: Midnight Runner", "1"),
        mk("ar1", "Artist: Neon Tide", "2"),
        mk("pl1", "Playlist: Deep focus", "3"),
        mk("af1", "Audio feature cluster: high valence", "4"),
        mk("ss1", "Session: 52 min focus block", "1"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 8, probability: 0.88 },
        { from: nodes[2].id, to: nodes[0].id, token: 5, probability: 0.72 },
        { from: nodes[0].id, to: nodes[3].id, token: 14, probability: 0.7 },
        { from: nodes[4].id, to: nodes[0].id, token: 9, probability: 0.8 },
      ];
      return { nodes, edges };
    }
    case "github": {
      const nodes = [
        mk("repo", "Repo: kg-engine", "1"),
        mk("pr", "PR #204: ingestion registry", "2"),
        mk("sym", "Symbol: IngestionPipeline", "3"),
        mk("wf", "Workflow: ci.yml", "4"),
        mk("iss", "Issue: OAuth token refresh", "2"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[1].id, to: nodes[0].id, token: 11, probability: 0.85 },
        { from: nodes[2].id, to: nodes[0].id, token: 7, probability: 0.76 },
        { from: nodes[3].id, to: nodes[0].id, token: 5, probability: 0.68 },
        { from: nodes[4].id, to: nodes[1].id, token: 4, probability: 0.62 },
      ];
      return { nodes, edges };
    }
    case "calendar": {
      const nodes = [
        mk("ev1", "Event: Design review", "1"),
        mk("ev2", "Event: 1:1 with PM", "2"),
        mk("att", "Attendee cluster: product", "3"),
        mk("rm", "Room: Orion (3F)", "4"),
        mk("rec", "Recurrence: weekly standup", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[2].id, token: 6, probability: 0.74 },
        { from: nodes[1].id, to: nodes[2].id, token: 5, probability: 0.71 },
        { from: nodes[0].id, to: nodes[3].id, token: 3, probability: 0.58 },
        { from: nodes[4].id, to: nodes[0].id, token: 8, probability: 0.69 },
      ];
      return { nodes, edges };
    }
    case "whatsapp": {
      const nodes = [
        mk("ch", "Chat: Founders circle", "1"),
        mk("c1", "Contact: Sam", "2"),
        mk("c2", "Contact: Riley", "2"),
        mk("msg", "Message: “Ship the preview tonight”", "1"),
        mk("task", "Extracted task: lock changelog", "3"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[3].id, to: nodes[0].id, token: 10, probability: 0.86 },
        { from: nodes[0].id, to: nodes[1].id, token: 4, probability: 0.66 },
        { from: nodes[0].id, to: nodes[2].id, token: 4, probability: 0.64 },
        { from: nodes[3].id, to: nodes[4].id, token: 9, probability: 0.77 },
      ];
      return { nodes, edges };
    }
    case "slack": {
      const nodes = [
        mk("ch", "#eng-knowledge", "1"),
        mk("th", "Thread: graph rollout", "2"),
        mk("u1", "User: @you", "3"),
        mk("rx", "Reaction cluster: eyes", "4"),
        mk("lnk", "Link unfurl: notion.so/…", "2"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[1].id, to: nodes[0].id, token: 5, probability: 0.79 },
        { from: nodes[1].id, to: nodes[2].id, token: 6, probability: 0.73 },
        { from: nodes[1].id, to: nodes[3].id, token: 3, probability: 0.55 },
        { from: nodes[4].id, to: nodes[1].id, token: 7, probability: 0.67 },
      ];
      return { nodes, edges };
    }
    case "notion": {
      const nodes = [
        mk("db", "Database: Roadmap", "1"),
        mk("pg", "Page: Q2 goals", "2"),
        mk("rel", "Relation: depends_on → Epic", "3"),
        mk("blk", "Block: KPI table", "2"),
        mk("usr", "Mention: @design", "4"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[1].id, to: nodes[0].id, token: 8, probability: 0.81 },
        { from: nodes[1].id, to: nodes[2].id, token: 5, probability: 0.7 },
        { from: nodes[1].id, to: nodes[3].id, token: 4, probability: 0.63 },
        { from: nodes[3].id, to: nodes[4].id, token: 3, probability: 0.52 },
      ];
      return { nodes, edges };
    }
    case "web": {
      const nodes = [
        mk("root", "Page: / (shell + TLS metadata)", "1"),
        mk("api", "Route: POST /api/login (session cookie)", "2"),
        mk("csp", "Finding: CSP missing frame-ancestors", "3"),
        mk("dep", "Asset graph: CDN script → SRI gap", "4"),
        mk("pdf1", "PDF learnings: OWASP CSRF cheat sheet (attached)", "5"),
        mk("pdf2", "PDF learnings: internal security baseline v3 (attached)", "5"),
        mk("xlnk", "Cross-edge: baseline ↔ CSP gap (mock)", "3"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 9, probability: 0.84 },
        { from: nodes[0].id, to: nodes[3].id, token: 4, probability: 0.68 },
        { from: nodes[1].id, to: nodes[2].id, token: 6, probability: 0.72 },
        { from: nodes[4].id, to: nodes[2].id, token: 3, probability: 0.58 },
        { from: nodes[5].id, to: nodes[2].id, token: 3, probability: 0.55 },
        { from: nodes[5].id, to: nodes[6].id, token: 2, probability: 0.5 },
        { from: nodes[6].id, to: nodes[2].id, token: 2, probability: 0.62 },
      ];
      return { nodes, edges };
    }
    case "equities": {
      const nodes = [
        mk("t1", "Ticker: ORCL · sector Technology", "1"),
        mk("lv", "Level: 200-day VWAP cluster", "2"),
        mk("er", "Event: earnings beat Q3 (mock)", "3"),
        mk("an", "Analyst note: PT raise cluster", "4"),
        mk("rs", "Risk: concentration vs benchmark", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 7, probability: 0.8 },
        { from: nodes[2].id, to: nodes[0].id, token: 6, probability: 0.76 },
        { from: nodes[3].id, to: nodes[2].id, token: 4, probability: 0.64 },
        { from: nodes[4].id, to: nodes[0].id, token: 5, probability: 0.7 },
      ];
      return { nodes, edges };
    }
    case "futures": {
      const nodes = [
        mk("c1", "Contract: ESZ5 · E-mini S&P", "1"),
        mk("cr", "Curve: contango vs backwardation", "2"),
        mk("mg", "Margin: SPAN estimate (mock)", "3"),
        mk("rl", "Roll: front → next liquidity", "4"),
        mk("mc", "Macro: FOMC path node", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 8, probability: 0.82 },
        { from: nodes[0].id, to: nodes[2].id, token: 5, probability: 0.68 },
        { from: nodes[1].id, to: nodes[3].id, token: 4, probability: 0.6 },
        { from: nodes[4].id, to: nodes[1].id, token: 3, probability: 0.55 },
      ];
      return { nodes, edges };
    }
    case "cryptocurrencies": {
      const nodes = [
        mk("p1", "Pair: ETH/USDT · venue A", "1"),
        mk("ch", "Chain: L2 bridge exposure", "2"),
        mk("df", "DeFi: pool IL vs spot hedge", "3"),
        mk("fl", "Flow: whale wallet cluster (mock)", "4"),
        mk("rs", "Risk: funding rate spike", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 6, probability: 0.78 },
        { from: nodes[0].id, to: nodes[2].id, token: 5, probability: 0.72 },
        { from: nodes[3].id, to: nodes[0].id, token: 4, probability: 0.65 },
        { from: nodes[4].id, to: nodes[0].id, token: 5, probability: 0.7 },
      ];
      return { nodes, edges };
    }
    case "fin_news": {
      const nodes = [
        mk("h1", "Wire: Fed signals · Reuters (mock)", "1"),
        mk("h2", "Headline: tech selloff breadth", "2"),
        mk("sn", "Sentiment: bearish cluster 24h", "3"),
        mk("en", "Entity: ORCL co-mentioned", "4"),
        mk("lk", "Link: futures correlation edge", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[2].id, token: 5, probability: 0.74 },
        { from: nodes[1].id, to: nodes[2].id, token: 4, probability: 0.7 },
        { from: nodes[1].id, to: nodes[3].id, token: 3, probability: 0.58 },
        { from: nodes[4].id, to: nodes[1].id, token: 3, probability: 0.52 },
      ];
      return { nodes, edges };
    }
    case "fin_market_data": {
      const nodes = [
        mk("v1", "Vendor A: consolidated tape (mock)", "1"),
        mk("v2", "Vendor B: L2 order book depth", "2"),
        mk("oh", "OHLCV: 1m bars fused", "3"),
        mk("iv", "Implied vol surface node", "4"),
        mk("al", "Alt data: credit card spend index", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[2].id, token: 6, probability: 0.8 },
        { from: nodes[1].id, to: nodes[2].id, token: 6, probability: 0.79 },
        { from: nodes[2].id, to: nodes[3].id, token: 4, probability: 0.66 },
        { from: nodes[4].id, to: nodes[2].id, token: 3, probability: 0.54 },
      ];
      return { nodes, edges };
    }
    case "fin_research": {
      const nodes = [
        mk("bk", "Book: Graham & Dodd · ch.8 (PDF ingest mock)", "1"),
        mk("fm", "Factor model: HML loadings", "2"),
        mk("ct", "Citation: risk parity paper → node", "3"),
        mk("nt", "Note: internal desk memo 2024-Q4", "4"),
        mk("xg", "Cross: book claim ↔ live ticker", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[2].id, token: 4, probability: 0.62 },
        { from: nodes[0].id, to: nodes[1].id, token: 5, probability: 0.68 },
        { from: nodes[3].id, to: nodes[1].id, token: 3, probability: 0.55 },
        { from: nodes[4].id, to: nodes[0].id, token: 2, probability: 0.48 },
      ];
      return { nodes, edges };
    }
    default:
      return { nodes: [], edges: [] };
  }
}

/** Merges subgraphs around a fusion hub (personal: +PDF; invest: markets-only). */
export function getUnifiedGraph(
  kind: WorkspaceKind,
  liveNodes: GraphNode[],
  liveEdges: GraphEdge[],
  connectorStatus: Partial<Record<ConnectorId, ConnectorStatus>>,
  opts?: { gmailOAuthConnected?: boolean },
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const pdfLive = filterGraphBySource(liveNodes, liveEdges, "pdf");
  const emailLive = filterLiveEmailGraph(liveNodes, liveEdges);
  const gmailSkipMock =
    emailLive.nodes.length > 0 || Boolean(opts?.gmailOAuthConnected);

  const hubId = "fusion:workspace-hub";
  const connectorLoop = kind === "invest" ? INVEST_CONNECTOR_IDS : PERSONAL_CONNECTOR_IDS;
  const nodes: GraphNode[] = [
    {
      id: hubId,
      label:
        kind === "invest"
          ? "Markets fusion hub · multi-vendor + research (mock)"
          : "Fusion hub · cross-domain join layer (mock)",
      page: "Ω",
      source: "unified",
    },
  ];
  const edges: GraphEdge[] = [];

  if (kind === "personal" && pdfLive.nodes.length > 0) {
    for (const n of pdfLive.nodes) {
      const id = `pdf:${n.id}`;
      nodes.push({ ...n, id, source: "pdf" });
    }
    for (const e of pdfLive.edges) {
      edges.push({ ...e, from: `pdf:${e.from}`, to: `pdf:${e.to}` });
    }
    edges.push({ from: hubId, to: `pdf:${pdfLive.nodes[0].id}`, token: 6, probability: 0.78 });
  }

  if (kind === "personal" && emailLive.nodes.length > 0) {
    for (const n of emailLive.nodes) {
      const id = `email:${n.id}`;
      nodes.push({ ...n, id, source: "email" });
    }
    for (const e of emailLive.edges) {
      edges.push({ ...e, from: `email:${e.from}`, to: `email:${e.to}` });
    }
    edges.push({
      from: hubId,
      to: `email:${emailLive.nodes[0].id}`,
      token: 5,
      probability: 0.76,
    });
  }

  for (const id of connectorLoop) {
    if (id === "gmail" && gmailSkipMock) continue;
    if (connectorStatus[id] !== "mock_on") continue;
    const g = getMockGraph(id);
    for (const n of g.nodes) nodes.push(n);
    for (const e of g.edges) edges.push(e);
    if (g.nodes[0]) {
      edges.push({ from: hubId, to: g.nodes[0].id, token: 5, probability: 0.7 });
    }
  }

  if (kind === "personal") {
    const gmailMsg = nodes.find((n) => n.id === "gmail:m1");
    const spotifyTr = nodes.find((n) => n.id === "spotify:tr1");
    if (gmailMsg && spotifyTr) {
      edges.push({
        from: gmailMsg.id,
        to: spotifyTr.id,
        token: 2,
        probability: 0.42,
      });
    }

    const pdfHead = pdfLive.nodes[0] ? `pdf:${pdfLive.nodes[0].id}` : null;
    const ghHead = nodes.find((n) => n.id === "github:repo");
    if (pdfHead && ghHead) {
      edges.push({ from: pdfHead, to: ghHead.id, token: 3, probability: 0.48 });
    }

    const webCsp = nodes.find((n) => n.id === "web:csp");
    if (pdfHead && webCsp) {
      edges.push({ from: pdfHead, to: webCsp.id, token: 2, probability: 0.44 });
    }
  }

  if (kind === "invest") {
    const eq = nodes.find((n) => n.id === "equities:t1");
    const nw = nodes.find((n) => n.id === "fin_news:h2");
    if (eq && nw) {
      edges.push({ from: nw.id, to: eq.id, token: 3, probability: 0.5 });
    }
    const cr = nodes.find((n) => n.id === "cryptocurrencies:p1");
    const fu = nodes.find((n) => n.id === "futures:c1");
    if (cr && fu) {
      edges.push({ from: cr.id, to: fu.id, token: 2, probability: 0.41 });
    }
    const bk = nodes.find((n) => n.id === "fin_research:bk");
    if (eq && bk) {
      edges.push({ from: bk.id, to: eq.id, token: 2, probability: 0.46 });
    }
  }

  if (nodes.length === 1) {
    for (const id of connectorLoop) {
      const ghostId = `fusion:await-${id}`;
      nodes.push({
        id: ghostId,
        label: `Awaiting ${id} subgraph…`,
        page: "—",
        source: "unified",
      });
      edges.push({ from: hubId, to: ghostId, token: 1, probability: 0.28 });
    }
  }

  return { nodes, edges };
}

/** Control-plane meta graph: personal (PDF + apps) or invest (ledger + feeds). */
export function getMetaGraph(
  kind: WorkspaceKind,
  documentGraphReady: boolean,
  connectorStatus: Partial<Record<ConnectorId, ConnectorStatus>>,
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const cp = "meta:orchestrator";
  const nodes: GraphNode[] = [
    {
      id: cp,
      label:
        kind === "invest"
          ? "Markets meta · routing & entitlements (mock)"
          : "Meta-graph · orchestrator & policy (mock)",
      page: "M",
      source: "meta",
    },
  ];
  const edges: GraphEdge[] = [];

  const domainOrder = kind === "invest" ? INVEST_CONNECTOR_IDS : PERSONAL_CONNECTOR_IDS;

  if (kind === "personal") {
    const pdfId = "meta:domain:pdf";
    nodes.push({
      id: pdfId,
      label: documentGraphReady ? "PDF subgraph (materialized)" : "PDF subgraph (empty slot)",
      page: "1",
      source: "meta",
    });
    edges.push({ from: cp, to: pdfId, token: 4, probability: documentGraphReady ? 0.9 : 0.4 });

    for (const id of domainOrder) {
      const on = connectorStatus[id] === "mock_on";
      const nid = `meta:domain:${id}`;
      nodes.push({
        id: nid,
        label: `${id} · ${on ? "preview stream on" : "disconnected"}`,
        page: "2",
        source: "meta",
      });
      edges.push({ from: cp, to: nid, token: 3, probability: on ? 0.85 : 0.38 });
    }

    edges.push({
      from: pdfId,
      to: "meta:domain:github",
      token: 2,
      probability: documentGraphReady && connectorStatus.github === "mock_on" ? 0.55 : 0.22,
    });
    if (documentGraphReady && connectorStatus.web === "mock_on") {
      edges.push({
        from: pdfId,
        to: "meta:domain:web",
        token: 3,
        probability: 0.52,
      });
    }
    edges.push({
      from: "meta:domain:calendar",
      to: "meta:domain:gmail",
      token: 2,
      probability: 0.33,
    });
  } else {
    const ledgerId = "meta:domain:ledger";
    nodes.push({
      id: ledgerId,
      label: documentGraphReady
        ? "Research PDFs · shared engine slot (optional)"
        : "Research PDFs slot (empty — ingest from Personal or bind graph_id)",
      page: "1",
      source: "meta",
    });
    edges.push({ from: cp, to: ledgerId, token: 4, probability: documentGraphReady ? 0.75 : 0.35 });

    for (const id of domainOrder) {
      const on = connectorStatus[id] === "mock_on";
      const nid = `meta:domain:${id}`;
      nodes.push({
        id: nid,
        label: `${id} · ${on ? "API stream on" : "disconnected"}`,
        page: "2",
        source: "meta",
      });
      edges.push({ from: cp, to: nid, token: 3, probability: on ? 0.86 : 0.36 });
    }

    edges.push({
      from: "meta:domain:fin_news",
      to: "meta:domain:equities",
      token: 3,
      probability: 0.44,
    });
    edges.push({
      from: "meta:domain:fin_market_data",
      to: "meta:domain:cryptocurrencies",
      token: 2,
      probability: 0.4,
    });
    if (documentGraphReady) {
      edges.push({
        from: ledgerId,
        to: "meta:domain:fin_research",
        token: 2,
        probability: 0.48,
      });
    }
  }

  nodes.push({
    id: "meta:agent-mesh",
    label: "Agent mesh · autoscale workers (mock)",
    page: "A",
    source: "meta",
  });
  edges.push({ from: cp, to: "meta:agent-mesh", token: 5, probability: 0.72 });

  return { nodes, edges };
}
