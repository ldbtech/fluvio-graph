import type { ConnectorId, WorkspaceKind } from "./types";

export const PERSONAL_CONNECTOR_IDS: ConnectorId[] = [
  "gmail",
  "github",
  "calendar",
  "whatsapp",
  "slack",
  "notion",
];

/** Architecture design slice (live-backed). */
export const DESIGN_CONNECTOR_IDS: ConnectorId[] = ["des_arch_plans"];

export function connectorsForKind(kind: WorkspaceKind): ConnectorId[] {
  if (kind === "design") return DESIGN_CONNECTOR_IDS;
  return PERSONAL_CONNECTOR_IDS;
}
