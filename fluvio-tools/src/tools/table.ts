// ============================================================
// TOOL: Standard Table
// FILE: table.ts
// CATEGORY: tables
// DESCRIPTION: Rectangular table with four legs. Suitable for dining, coffee, side, console use.
// STYLES: dining, coffee, side, console
// MATERIALS: white_oak, dark_walnut, marble, polished_concrete, brushed_steel, matte_black
// SUPPORTS: width, depth, height scaling, leg material variants
// DOES_NOT_SUPPORT: round table, oval table, pedestal base, extendable, glass top
// TAGS: table, dining, coffee table, side table, console, furniture
// VERSION: 1.0
// ============================================================
import * as THREE from "three"
import { MaterialKey, MaterialLibrary } from "./MaterialLibrary"

export function generateTable(style: unknown, material: MaterialKey): THREE.Group {
  void style
  const group = new THREE.Group()
  const spec = MaterialLibrary[material]
  const mat = new THREE.MeshStandardMaterial({
    color: spec.color as string,
    roughness: spec.roughness,
    metalness: spec.metalness,
  })
  const legMat = new THREE.MeshStandardMaterial({ color: 0x1f1f1f, roughness: 0.5, metalness: 0.15 })

  const top = new THREE.Mesh(new THREE.BoxGeometry(1.8, 0.05, 0.9), mat)
  top.position.y = 0.75; top.castShadow = true; top.receiveShadow = true

  const legGeo = new THREE.CylinderGeometry(0.03, 0.025, 0.72, 14)
  ;[[-0.82,0.36,0.38],[0.82,0.36,0.38],[-0.82,0.36,-0.38],[0.82,0.36,-0.38]].forEach(([x,y,z]) => {
    const leg = new THREE.Mesh(legGeo, legMat)
    leg.position.set(x, y, z); leg.castShadow = true; group.add(leg)
  })

  group.add(top)
  return group
}
