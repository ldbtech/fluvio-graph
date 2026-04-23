import type { ConnectorId, WorkspaceKind } from "./types";

export const PERSONAL_CONNECTOR_IDS: ConnectorId[] = [
  "gmail",
  "spotify",
  "github",
  "calendar",
  "whatsapp",
  "slack",
  "notion",
  "web",
];

export const INVEST_CONNECTOR_IDS: ConnectorId[] = [
  "equities",
  "futures",
  "cryptocurrencies",
  "fin_news",
  "fin_market_data",
  "fin_research",
];

/** Architecture + civil engineering design slices (mock until Rust ingestion). */
export const DESIGN_CONNECTOR_IDS: ConnectorId[] = [
  "des_bim",
  "des_arch_plans",
  "des_structural",
  "des_civil_site",
  "des_building_codes",
  "des_physics_sim",
];

export function connectorsForKind(kind: WorkspaceKind): ConnectorId[] {
  if (kind === "invest") return INVEST_CONNECTOR_IDS;
  if (kind === "design") return DESIGN_CONNECTOR_IDS;
  return PERSONAL_CONNECTOR_IDS;
}
