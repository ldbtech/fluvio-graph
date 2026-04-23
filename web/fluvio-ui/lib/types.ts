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
  | "fin_research"
  | "des_bim"
  | "des_arch_plans"
  | "des_structural"
  | "des_civil_site"
  | "des_building_codes"
  | "des_physics_sim";

/** Personal, markets, or architecture / civil design workspace. */
export type WorkspaceKind = "personal" | "invest" | "design";

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

/** Frontend GitHub repo identity used by the workspace UI. */
export type CodebaseCloneResult = {
  owner: string;
  repo: string;
  local_path: string;
  was_cloned: boolean;
};

/** Response from `POST /ingest` (kg-engine). */
export type CodebaseIngestResult = {
  chunks: number;
  nodes: number;
  edges: number;
};

/** Legacy scoped ingest response (deprecated with simplified `/ingest`). */
export type CodebasePlanetIngestResult = {
  path_prefix: string;
  chunks_in_scope: number;
  chunks_skipped_existing: number;
  nodes_added: number;
  structured_edges: number;
  graph_nodes: number;
  graph_edges: number;
};

/** Legacy file-list response (deprecated with simplified codebase flow). */
export type CodebaseFilesResponse = {
  paths: string[];
  truncated: boolean;
};

/** Legacy galaxy tree node used by GitHub mock visuals. */
export type CodebaseModuleKind = "repo" | "module" | "file";

export type CodebaseModuleTree = {
  name: string;
  path: string;
  kind: CodebaseModuleKind;
  size_bytes: number;
  file_count: number;
  language: string;
  depth: number;
  children: CodebaseModuleTree[];
};
