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

export function connectorsForKind(kind: WorkspaceKind): ConnectorId[] {
  return kind === "invest" ? INVEST_CONNECTOR_IDS : PERSONAL_CONNECTOR_IDS;
}
