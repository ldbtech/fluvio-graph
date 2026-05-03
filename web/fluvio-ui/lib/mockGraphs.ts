import type { ConnectorId, ConnectorStatus, GraphEdge, GraphNode, WorkspaceKind } from "./types";
import { filterGraphBySource, filterLiveEmailGraph } from "./graphFilters";
import { DESIGN_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "./workspaceKinds";

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
    case "des_bim": {
      const nodes = [
        mk("ifc", "IFC federation: Tower A · LOD350", "1"),
        mk("lvl", "Level L12 · slab edge profile", "2"),
        mk("clh", "Clash: MEP duct vs beam flange (mock)", "3"),
        mk("gv", "Gridline intersection G-4 / 12", "4"),
        mk("mat", "Material catalog: concrete C40/50", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 8, probability: 0.84 },
        { from: nodes[2].id, to: nodes[1].id, token: 6, probability: 0.76 },
        { from: nodes[3].id, to: nodes[1].id, token: 4, probability: 0.68 },
        { from: nodes[4].id, to: nodes[0].id, token: 5, probability: 0.72 },
      ];
      return { nodes, edges };
    }
    case "des_arch_plans": {
      const nodes = [
        mk("sh", "Sheet A-101: floor plan L12", "1"),
        mk("rm", "Room program: core labs cluster", "2"),
        mk("env", "Envelope U-value target wall-N", "3"),
        mk("acc", "Accessibility path · stair B", "4"),
        mk("can", "Canopy datum vs finish floor", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 7, probability: 0.8 },
        { from: nodes[0].id, to: nodes[2].id, token: 5, probability: 0.7 },
        { from: nodes[1].id, to: nodes[3].id, token: 4, probability: 0.64 },
        { from: nodes[0].id, to: nodes[4].id, token: 3, probability: 0.58 },
      ];
      return { nodes, edges };
    }
    case "des_structural": {
      const nodes = [
        mk("mdl", "ETABS model: lateral system core", "1"),
        mk("bm", "Beam B-1204 · W24x62", "2"),
        mk("col", "Column C-08 · axial + moment envelope", "3"),
        mk("conn", "Connection: moment frame knee (mock)", "4"),
        mk("drft", "Drift check: 0.68% interstory (wind)", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[1].id, to: nodes[0].id, token: 9, probability: 0.86 },
        { from: nodes[2].id, to: nodes[0].id, token: 8, probability: 0.83 },
        { from: nodes[3].id, to: nodes[1].id, token: 5, probability: 0.71 },
        { from: nodes[0].id, to: nodes[4].id, token: 6, probability: 0.75 },
      ];
      return { nodes, edges };
    }
    case "des_civil_site": {
      const nodes = [
        mk("pad", "Pad grade: east plaza finish ±0.15m", "1"),
        mk("util", "Storm line crossing column line 4", "2"),
        mk("geo", "Boring B-12: SPT N=18 @ -6m", "3"),
        mk("sw", "SWPPP control: silt fence run 140m", "4"),
        mk("eas", "Easement: utility corridor north edge", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[1].id, to: nodes[0].id, token: 6, probability: 0.77 },
        { from: nodes[2].id, to: nodes[0].id, token: 5, probability: 0.7 },
        { from: nodes[3].id, to: nodes[0].id, token: 4, probability: 0.62 },
        { from: nodes[4].id, to: nodes[1].id, token: 3, probability: 0.55 },
      ];
      return { nodes, edges };
    }
    case "des_building_codes": {
      const nodes = [
        mk("ibc", "IBC 2021 Ch.16 · wind procedure", "1"),
        mk("asce", "ASCE 7-22: risk category II", "2"),
        mk("snow", "Ground snow pg = 1.2 kPa (mock)", "3"),
        mk("fire", "Fire separation: occupancy B → A-2", "4"),
        mk("loc", "Local amendment: parapet height", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[1].id, token: 7, probability: 0.82 },
        { from: nodes[1].id, to: nodes[2].id, token: 5, probability: 0.69 },
        { from: nodes[0].id, to: nodes[3].id, token: 4, probability: 0.61 },
        { from: nodes[4].id, to: nodes[0].id, token: 3, probability: 0.54 },
      ];
      return { nodes, edges };
    }
    case "des_physics_sim": {
      const nodes = [
        mk("cfd", "CFD: pedestrian wind comfort · corner vortex", "1"),
        mk("th", "Thermal bridge: curtain wall mullion", "2"),
        mk("dyn", "Modal mass participation > 75% (mock)", "3"),
        mk("sett", "Settlement sensitivity: pad vs piles", "4"),
        mk("pass", "Pass/fail gate: drift + accel combined", "5"),
      ];
      const edges: GraphEdge[] = [
        { from: nodes[0].id, to: nodes[4].id, token: 6, probability: 0.74 },
        { from: nodes[1].id, to: nodes[4].id, token: 5, probability: 0.68 },
        { from: nodes[2].id, to: nodes[4].id, token: 5, probability: 0.71 },
        { from: nodes[3].id, to: nodes[4].id, token: 4, probability: 0.63 },
      ];
      return { nodes, edges };
    }
    default:
      return { nodes: [], edges: [] };
  }
}

/** Merges subgraphs around a fusion hub (personal vs design slices). */
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
  const connectorLoop = kind === "design" ? DESIGN_CONNECTOR_IDS : PERSONAL_CONNECTOR_IDS;
  const nodes: GraphNode[] = [
    {
      id: hubId,
      label:
        kind === "design"
          ? "Design fusion hub · architecture + civil + physics checks (mock)"
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
    const pdfHead = pdfLive.nodes[0] ? `pdf:${pdfLive.nodes[0].id}` : null;
    const ghHead = nodes.find((n) => n.id === "github:repo");
    if (pdfHead && ghHead) {
      edges.push({ from: pdfHead, to: ghHead.id, token: 3, probability: 0.48 });
    }
  }

  if (kind === "design") {
    const bim = nodes.find((n) => n.id === "des_bim:ifc");
    const arch = nodes.find((n) => n.id === "des_arch_plans:sh");
    const struct = nodes.find((n) => n.id === "des_structural:mdl");
    const civil = nodes.find((n) => n.id === "des_civil_site:pad");
    const codes = nodes.find((n) => n.id === "des_building_codes:asce");
    const phys = nodes.find((n) => n.id === "des_physics_sim:pass");
    if (bim && arch) edges.push({ from: arch.id, to: bim.id, token: 4, probability: 0.58 });
    if (struct && bim) edges.push({ from: struct.id, to: bim.id, token: 5, probability: 0.66 });
    if (codes && struct) edges.push({ from: codes.id, to: struct.id, token: 4, probability: 0.62 });
    if (civil && struct) edges.push({ from: civil.id, to: struct.id, token: 3, probability: 0.52 });
    if (phys && struct) edges.push({ from: phys.id, to: struct.id, token: 4, probability: 0.6 });
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

/** Control-plane meta graph: personal (PDF + apps) or design (codes + contracts). */
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
        kind === "design"
          ? "Design meta · codes, loads, and solver contracts (mock)"
          : "Meta-graph · orchestrator & policy (mock)",
      page: "M",
      source: "meta",
    },
  ];
  const edges: GraphEdge[] = [];

  const domainOrder = kind === "design" ? DESIGN_CONNECTOR_IDS : PERSONAL_CONNECTOR_IDS;

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
    edges.push({
      from: "meta:domain:calendar",
      to: "meta:domain:gmail",
      token: 2,
      probability: 0.33,
    });
  } else if (kind === "design") {
    const specsId = "meta:domain:spec_pdfs";
    nodes.push({
      id: specsId,
      label: documentGraphReady
        ? "Spec PDFs · shared engine slot (optional)"
        : "Spec PDFs slot (empty — ingest calc books in Personal)",
      page: "1",
      source: "meta",
    });
    edges.push({ from: cp, to: specsId, token: 4, probability: documentGraphReady ? 0.78 : 0.36 });

    for (const id of domainOrder) {
      const on = connectorStatus[id] === "mock_on";
      const nid = `meta:domain:${id}`;
      nodes.push({
        id: nid,
        label: `${id} · ${on ? "preview graph on" : "disconnected"}`,
        page: "2",
        source: "meta",
      });
      edges.push({ from: cp, to: nid, token: 3, probability: on ? 0.87 : 0.37 });
    }

    edges.push({
      from: "meta:domain:des_building_codes",
      to: "meta:domain:des_structural",
      token: 3,
      probability: 0.5,
    });
    edges.push({
      from: "meta:domain:des_arch_plans",
      to: "meta:domain:des_bim",
      token: 2,
      probability: 0.44,
    });
    edges.push({
      from: "meta:domain:des_civil_site",
      to: "meta:domain:des_structural",
      token: 2,
      probability: 0.42,
    });
    edges.push({
      from: "meta:domain:des_physics_sim",
      to: "meta:domain:des_structural",
      token: 3,
      probability: 0.48,
    });
    if (documentGraphReady) {
      edges.push({
        from: specsId,
        to: "meta:domain:des_building_codes",
        token: 2,
        probability: 0.45,
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
