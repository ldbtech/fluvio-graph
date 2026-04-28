// MaterialLibrary.ts
// Architectural PBR material definitions.
// Each entry maps to a THREE.MeshStandardMaterial config.
// roughness 0.0 = mirror, 1.0 = fully rough
// metalness 0.0 = dielectric, 1.0 = fully metallic

export const MaterialLibrary = {
  // ── Wood ──────────────────────────────────────────────────────────────────
  white_oak: {
    roughness: 0.50, metalness: 0.00,
    color: "#c8a87a",
  },
  dark_walnut: {
    roughness: 0.55, metalness: 0.00,
    color: "#4a3728",
  },
  pine: {
    roughness: 0.65, metalness: 0.00,
    color: "#d4a96a",
  },
  // ── Stone & concrete ──────────────────────────────────────────────────────
  polished_concrete: {
    roughness: 0.80, metalness: 0.00,
    color: "#9a9a9a",
  },
  raw_concrete: {
    roughness: 0.95, metalness: 0.00,
    color: "#7a7a7a",
  },
  marble: {
    roughness: 0.10, metalness: 0.05,
    color: "#f0ece4",
  },
  slate: {
    roughness: 0.90, metalness: 0.02,
    color: "#4a4e54",
  },
  terracotta: {
    roughness: 0.90, metalness: 0.00,
    color: "#c2714f",
  },
  // ── Fabric & upholstery ───────────────────────────────────────────────────
  fabric_grey: {
    roughness: 1.00, metalness: 0.00,
    color: "#8a8a8a",
  },
  fabric_cream: {
    roughness: 1.00, metalness: 0.00,
    color: "#e8dcc8",
  },
  fabric_navy: {
    roughness: 1.00, metalness: 0.00,
    color: "#2a3a5a",
  },
  // ── Metal ─────────────────────────────────────────────────────────────────
  brushed_brass: {
    roughness: 0.40, metalness: 0.90,
    color: "#b8942a",
  },
  brushed_steel: {
    roughness: 0.35, metalness: 0.95,
    color: "#9a9ea8",
  },
  matte_black: {
    roughness: 0.80, metalness: 0.30,
    color: "#1a1a1a",
  },
  // ── Glass ─────────────────────────────────────────────────────────────────
  glass: {
    roughness: 0.00, metalness: 0.10,
    color: "#88bbff",
    transparent: true,
    opacity: 0.15,
  },
} as const

export type MaterialKey = keyof typeof MaterialLibrary
export type MatSpec = (typeof MaterialLibrary)[MaterialKey]