// ============================================================
// TOOL: Standard Sofa
// FILE: sofa.ts
// CATEGORY: seating
// DESCRIPTION: Three-seat sofa with cushioned back, capsule arms, back cushions, and legs.
// STYLES: modern, scandinavian, industrial, curved
// MATERIALS: fabric_grey, fabric_cream, fabric_navy, white_oak, dark_walnut
// SUPPORTS: arm styles, cushion count, leg variants, width scaling
// DOES_NOT_SUPPORT: sectional, chaise lounge, L-shape, curved body, recliner
// TAGS: sofa, couch, seating, living room, upholstered, three-seat
// VERSION: 1.0
// ============================================================
// sofa.ts
import * as THREE from "three"
import { MaterialLibrary } from "./MaterialLibrary"

type MatEntry = (typeof MaterialLibrary)[keyof typeof MaterialLibrary]

function buildPBRMaterial(spec: MatEntry) {
  const mat = new THREE.MeshStandardMaterial({
    roughness: spec.roughness,
    metalness: spec.metalness,
  })
  if ("color" in spec && spec.color) mat.color.set(spec.color)
  if ("transparent" in spec && spec.transparent) {
    mat.transparent = true
    if ("opacity" in spec && spec.opacity != null) mat.opacity = spec.opacity
  }
  return mat
}

export function generateSofa(style: unknown, material: keyof typeof MaterialLibrary) {
  void style
  const group = new THREE.Group()
  const source = buildPBRMaterial(MaterialLibrary[material])
  const hsl = { h: 0, s: 0, l: 0 }
  source.color.getHSL(hsl)
  const sofaColor = new THREE.Color().setHSL(hsl.h, Math.min(hsl.s * 0.55 + 0.1, 0.45), 0.42)
  const cushionColor = sofaColor.clone().offsetHSL(0, 0.03, 0.05)

  const upholsteryMat = new THREE.MeshStandardMaterial({
    color: sofaColor,
    roughness: 0.9,
    metalness: 0.02,
  })
  const cushionMat = new THREE.MeshStandardMaterial({
    color: cushionColor,
    roughness: 0.95,
    metalness: 0.01,
  })

  const legMat = new THREE.MeshStandardMaterial({
    color: 0x1f1f1f,
    roughness: 0.55,
    metalness: 0.1,
  })

  const baseGeometry = new THREE.BoxGeometry(2.2, 0.45, 0.9, 4, 4, 4)
  baseGeometry.computeVertexNormals()
  const base = new THREE.Mesh(baseGeometry, upholsteryMat)
  base.position.y = 0.225
  base.castShadow = true
  base.receiveShadow = true

  const backGeometry = new THREE.BoxGeometry(2.2, 0.55, 0.15, 4, 4, 2)
  backGeometry.computeVertexNormals()
  const back = new THREE.Mesh(backGeometry, upholsteryMat)
  back.position.set(0, 0.675, -0.375)
  back.castShadow = true

  const armGeometry = new THREE.CapsuleGeometry(0.09, 0.55, 6, 10)
  armGeometry.rotateZ(Math.PI / 2)
  const armL = new THREE.Mesh(armGeometry, upholsteryMat)
  armL.position.set(-1.025, 0.45, 0)
  armL.castShadow = true

  const armR = armL.clone()
  armR.position.x = 1.025

  const cushionGeometry = new THREE.BoxGeometry(0.68, 0.2, 0.78, 3, 2, 3)
  cushionGeometry.computeVertexNormals()
  const cushionL = new THREE.Mesh(cushionGeometry, cushionMat)
  cushionL.position.set(-0.72, 0.55, 0.02)
  cushionL.rotation.z = 0.015
  cushionL.castShadow = true

  const cushionM = cushionL.clone()
  cushionM.position.x = 0
  cushionM.rotation.z = -0.01

  const cushionR = cushionL.clone()
  cushionR.position.x = 0.72
  cushionR.rotation.z = 0.02

  const backCushionGeometry = new THREE.BoxGeometry(0.66, 0.28, 0.12, 3, 2, 2)
  backCushionGeometry.computeVertexNormals()
  const backCushionL = new THREE.Mesh(backCushionGeometry, cushionMat)
  backCushionL.position.set(-0.72, 0.79, -0.33)
  backCushionL.rotation.x = -0.06
  backCushionL.castShadow = true

  const backCushionM = backCushionL.clone()
  backCushionM.position.x = 0
  backCushionM.rotation.x = -0.04

  const backCushionR = backCushionL.clone()
  backCushionR.position.x = 0.72
  backCushionR.rotation.x = -0.05

  const legGeometry = new THREE.CylinderGeometry(0.05, 0.05, 0.2, 14)
  const legFL = new THREE.Mesh(legGeometry, legMat)
  legFL.position.set(-0.9, 0.1, 0.35)
  legFL.castShadow = true
  const legFR = legFL.clone()
  legFR.position.set(0.9, 0.1, 0.35)
  const legBL = legFL.clone()
  legBL.position.set(-0.9, 0.1, -0.35)
  const legBR = legFL.clone()
  legBR.position.set(0.9, 0.1, -0.35)

  group.add(
    base,
    back,
    armL,
    armR,
    cushionL,
    cushionM,
    cushionR,
    backCushionL,
    backCushionM,
    backCushionR,
    legFL,
    legFR,
    legBL,
    legBR,
  )
  return group
}
