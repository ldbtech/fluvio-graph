export interface GraphNode {
  id: string;
  label: string;
  page: string;
  source: string;
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

export interface GraphEdge {
  from: string;
  to: string;
  token: number;
  probability: number;
  source?: GraphNode;
  target?: GraphNode;
}

export interface SelectedNode {
  node: GraphNode;
  neighbors: { node: GraphNode; token: number; probability: number }[];
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

/** OAuth / integration sources (sidebar + full-screen connect previews). */
export type ConnectorId =
  | "gmail"
  | "spotify"
  | "github"
  | "calendar"
  | "whatsapp"
  | "slack"
  | "notion"
  | "web"
  | "equities"
  | "futures"
  | "cryptocurrencies"
  | "fin_news"
  | "fin_market_data"
  | "fin_research";

/** Personal vs markets / portfolio workspace (split dashboard). */
export type WorkspaceKind = "personal" | "invest";

/** Main canvas mode: graph home vs connector-specific setup (production-shaped UI). */
export type WorkspaceSurface = "documents" | ConnectorId;

/** Brain canvas tab: per-domain slice, fused view, or control-plane meta graph. */
export type BrainTab = WorkspaceSurface | "unified" | "meta";

export type ConnectorStatus = "off" | "connecting" | "mock_on";

export interface ConnectorDef {
  id: ConnectorId;
  name: string;
  blurb: string;
  accent: string;
}

export interface MockAgent {
  id: string;
  name: string;
  description: string;
  icon: string;
}
