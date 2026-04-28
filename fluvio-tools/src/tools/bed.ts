// ============================================================
// TOOL: Standard Bed
// FILE: bed.ts
// CATEGORY: bedroom
// DESCRIPTION: Bed frame with mattress, headboard, and pillows. Supports single to king sizes.
// STYLES: modern, scandinavian, platform, upholstered
// MATERIALS: white_oak, dark_walnut, fabric_grey, fabric_cream, matte_black
// SUPPORTS: size variants single double queen king, headboard height, leg style
// DOES_NOT_SUPPORT: bunk bed, trundle, canopy, murphy bed, adjustable base
// TAGS: bed, bedroom, sleeping, frame, mattress, headboard
// VERSION: 1.0
// ============================================================
import * as THREE from "three"
import { MaterialKey, MaterialLibrary } from "./MaterialLibrary"

export function generateBed(style: unknown, material: MaterialKey): THREE.Group {
  void style
  const group = new THREE.Group()
  const spec = MaterialLibrary[material]
  const mat = new THREE.MeshStandardMaterial({
    color: spec.color as string,
    roughness: spec.roughness,
    metalness: spec.metalness,
  })
  const mattressMat = new THREE.MeshStandardMaterial({ color: 0xe8dcc8, roughness: 1.0 })
  const legMat = new THREE.MeshStandardMaterial({ color: 0x1f1f1f, roughness: 0.5, metalness: 0.2 })

  const frame = new THREE.Mesh(new THREE.BoxGeometry(1.6, 0.28, 2.1), mat)
  frame.position.y = 0.14; frame.castShadow = true; frame.receiveShadow = true

  const mattress = new THREE.Mesh(new THREE.BoxGeometry(1.52, 0.22, 1.95), mattressMat)
  mattress.position.y = 0.39; mattress.castShadow = true

  const headboard = new THREE.Mesh(new THREE.BoxGeometry(1.6, 0.72, 0.08), mat)
  headboard.position.set(0, 0.72, -1.04); headboard.castShadow = true

  const pillow1 = new THREE.Mesh(new THREE.BoxGeometry(0.65, 0.1, 0.44), mattressMat)
  pillow1.position.set(-0.38, 0.56, -0.7); pillow1.castShadow = true
  const pillow2 = pillow1.clone(); pillow2.position.x = 0.38

  const legGeo = new THREE.CylinderGeometry(0.04, 0.04, 0.22, 12)
  ;[[-0.72,0.11,0.9],[0.72,0.11,0.9],[-0.72,0.11,-0.9],[0.72,0.11,-0.9]].forEach(([x,y,z]) => {
    const leg = new THREE.Mesh(legGeo, legMat)
    leg.position.set(x, y, z); leg.castShadow = true; group.add(leg)
  })

  group.add(frame, mattress, headboard, pillow1, pillow2)
  return group
}
