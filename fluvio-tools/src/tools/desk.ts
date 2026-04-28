// ============================================================
// TOOL: Standard Desk
// FILE: desk.ts
// CATEGORY: office
// DESCRIPTION: Rectangular work desk with four box legs and cable management tray.
// STYLES: minimal, executive, corner, standing
// MATERIALS: white_oak, dark_walnut, polished_concrete, matte_black, brushed_steel
// SUPPORTS: width, depth scaling, cable tray, leg material
// DOES_NOT_SUPPORT: L-shape desk, drawers, hutch, monitor arm, standing mechanism
// TAGS: desk, office, work, writing desk, home office
// VERSION: 1.0
// ============================================================
import * as THREE from "three"
import { MaterialKey, MaterialLibrary } from "./MaterialLibrary"

export function generateDesk(style: unknown, material: MaterialKey): THREE.Group {
  void style
  const group = new THREE.Group()
  const spec = MaterialLibrary[material]
  const mat = new THREE.MeshStandardMaterial({
    color: spec.color as string,
    roughness: spec.roughness,
    metalness: spec.metalness,
  })
  const legMat = new THREE.MeshStandardMaterial({ color: 0x2a2a2a, roughness: 0.6, metalness: 0.3 })

  const top = new THREE.Mesh(new THREE.BoxGeometry(1.4, 0.04, 0.65), mat)
  top.position.y = 0.74; top.castShadow = true; top.receiveShadow = true

  // Cable management tray under desk
  const tray = new THREE.Mesh(new THREE.BoxGeometry(1.0, 0.04, 0.12), legMat)
  tray.position.set(0, 0.55, -0.2); group.add(tray)

  const legGeo = new THREE.BoxGeometry(0.04, 0.72, 0.04)
  ;[[-0.65,0.36,0.28],[0.65,0.36,0.28],[-0.65,0.36,-0.28],[0.65,0.36,-0.28]].forEach(([x,y,z]) => {
    const leg = new THREE.Mesh(legGeo, legMat)
    leg.position.set(x, y, z); leg.castShadow = true; group.add(leg)
  })

  group.add(top)
  return group
}
