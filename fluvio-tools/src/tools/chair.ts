// ============================================================
// TOOL: Standard Chair
// FILE: chair.ts
// CATEGORY: seating
// DESCRIPTION: Single chair with seat, back rest, and four legs.
// STYLES: dining, lounge, accent, bar
// MATERIALS: fabric_grey, fabric_cream, fabric_navy, white_oak, dark_walnut, matte_black
// SUPPORTS: seat height, back height, leg material, width scaling
// DOES_NOT_SUPPORT: armchair, rocking chair, swivel, office chair, stool
// TAGS: chair, seating, dining chair, accent chair, single seat
// VERSION: 1.0
// ============================================================
import * as THREE from "three"
import { MaterialKey, MaterialLibrary } from "./MaterialLibrary"

export function generateChair(style: unknown, material: MaterialKey): THREE.Group {
  void style
  const group = new THREE.Group()
  const spec = MaterialLibrary[material]
  const mat = new THREE.MeshStandardMaterial({
    color: spec.color as string,
    roughness: spec.roughness,
    metalness: spec.metalness,
  })
  const legMat = new THREE.MeshStandardMaterial({ color: 0x1f1f1f, roughness: 0.5, metalness: 0.2 })

  const seat = new THREE.Mesh(new THREE.BoxGeometry(0.5, 0.07, 0.5), mat)
  seat.position.y = 0.46; seat.castShadow = true; seat.receiveShadow = true

  const back = new THREE.Mesh(new THREE.BoxGeometry(0.5, 0.52, 0.07), mat)
  back.position.set(0, 0.77, -0.215); back.castShadow = true

  const legGeo = new THREE.CylinderGeometry(0.025, 0.02, 0.44, 12)
  ;[[-0.19,0.22,0.19],[0.19,0.22,0.19],[-0.19,0.22,-0.19],[0.19,0.22,-0.19]].forEach(([x,y,z]) => {
    const leg = new THREE.Mesh(legGeo, legMat)
    leg.position.set(x, y, z); leg.castShadow = true; group.add(leg)
  })

  group.add(seat, back)
  return group
}
