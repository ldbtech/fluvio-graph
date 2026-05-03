// ============================================================
// TOOL: Red Car for Parking Lot
// FILE: car_parking_lot_maybe.ts
// CATEGORY: structure
// DESCRIPTION: Generates a red passenger car suitable for placement in parking lots and driveways.
// STYLES: modern, realistic, automotive
// MATERIALS: matte_black, brushed_steel, glass
// SUPPORTS: parking lots, driveways, urban scenes
// DOES_NOT_SUPPORT: off-road vehicles, trucks, motorcycles
// TAGS: car, vehicle, red car, parking, transportation, automobile, sedan
// VERSION: 1.0
// ============================================================

import * as THREE from "three"
import { MaterialLibrary, MaterialKey } from "./MaterialLibrary"

export function generateRedCarForParkingLot(style: string, material: MaterialKey): THREE.Group {
  const group = new THREE.Group()
  group.userData = { tool: "car_parking_lot_maybe", style, material }

  // Materials
  const bodyMaterial = new THREE.MeshStandardMaterial({
    color: '#cc0000',
    roughness: 0.3,
    metalness: 0.7,
  })

  const glassMaterial = new THREE.MeshStandardMaterial({
    color: '#87ceeb',
    roughness: 0.1,
    metalness: 0.0,
    transparent: true,
    opacity: 0.6,
  })

  const tireMaterial = new THREE.MeshStandardMaterial({
    color: '#1a1a1a',
    roughness: 0.9,
    metalness: 0.0,
  })

  const rimMaterial = new THREE.MeshStandardMaterial({
    color: MaterialLibrary.brushed_steel?.color ?? '#888888',
    roughness: MaterialLibrary.brushed_steel?.roughness ?? 0.3,
    metalness: MaterialLibrary.brushed_steel?.metalness ?? 0.8,
  })

  // Car body
  const bodyGeometry = new THREE.BoxGeometry(4.5, 1.4, 1.8)
  const bodyMesh = new THREE.Mesh(bodyGeometry, bodyMaterial)
  bodyMesh.position.set(0, 0.7, 0)
  bodyMesh.castShadow = true
  bodyMesh.receiveShadow = true
  bodyMesh.name = "car_body"
  group.add(bodyMesh)

  // Front windshield
  const frontWindshieldGeometry = new THREE.BoxGeometry(1.2, 0.8, 0.05)
  const frontWindshieldMesh = new THREE.Mesh(frontWindshieldGeometry, glassMaterial)
  frontWindshieldMesh.position.set(1.0, 1.1, 0)
  frontWindshieldMesh.rotation.x = -0.2
  frontWindshieldMesh.castShadow = true
  frontWindshieldMesh.receiveShadow = true
  frontWindshieldMesh.name = "front_windshield"
  group.add(frontWindshieldMesh)

  // Rear windshield
  const rearWindshieldGeometry = new THREE.BoxGeometry(1.0, 0.6, 0.05)
  const rearWindshieldMesh = new THREE.Mesh(rearWindshieldGeometry, glassMaterial)
  rearWindshieldMesh.position.set(-1.2, 1.0, 0)
  rearWindshieldMesh.rotation.x = 0.2
  rearWindshieldMesh.castShadow = true
  rearWindshieldMesh.receiveShadow = true
  rearWindshieldMesh.name = "rear_windshield"
  group.add(rearWindshieldMesh)

  // Side windows (left)
  const sideWindowGeometry = new THREE.BoxGeometry(0.4, 0.3, 0.05)
  const leftFrontWindowMesh = new THREE.Mesh(sideWindowGeometry, glassMaterial)
  leftFrontWindowMesh.position.set(0.4, 1.0, 0.9)
  leftFrontWindowMesh.castShadow = true
  leftFrontWindowMesh.receiveShadow = true
  leftFrontWindowMesh.name = "left_front_window"
  group.add(leftFrontWindowMesh)

  const leftRearWindowMesh = new THREE.Mesh(sideWindowGeometry, glassMaterial)
  leftRearWindowMesh.position.set(-0.4, 1.0, 0.9)
  leftRearWindowMesh.castShadow = true
  leftRearWindowMesh.receiveShadow = true
  leftRearWindowMesh.name = "left_rear_window"
  group.add(leftRearWindowMesh)

  // Side windows (right)
  const rightFrontWindowMesh = new THREE.Mesh(sideWindowGeometry, glassMaterial)
  rightFrontWindowMesh.position.set(0.4, 1.0, -0.9)
  rightFrontWindowMesh.castShadow = true
  rightFrontWindowMesh.receiveShadow = true
  rightFrontWindowMesh.name = "right_front_window"
  group.add(rightFrontWindowMesh)

  const rightRearWindowMesh = new THREE.Mesh(sideWindowGeometry, glassMaterial)
  rightRearWindowMesh.position.set(-0.4, 1.0, -0.9)
  rightRearWindowMesh.castShadow = true
  rightRearWindowMesh.receiveShadow = true
  rightRearWindowMesh.name = "right_rear_window"
  group.add(rightRearWindowMesh)

  // Front bumper
  const frontBumperGeometry = new THREE.BoxGeometry(0.2, 0.3, 1.8)
  const frontBumperMesh = new THREE.Mesh(frontBumperGeometry, bodyMaterial)
  frontBumperMesh.position.set(2.35, 0.35, 0)
  frontBumperMesh.castShadow = true
  frontBumperMesh.receiveShadow = true
  frontBumperMesh.name = "front_bumper"
  group.add(frontBumperMesh)

  // Rear bumper
  const rearBumperGeometry = new THREE.BoxGeometry(0.2, 0.3, 1.8)
  const rearBumperMesh = new THREE.Mesh(rearBumperGeometry, bodyMaterial)
  rearBumperMesh.position.set(-2.35, 0.35, 0)
  rearBumperMesh.castShadow = true
  rearBumperMesh.receiveShadow = true
  rearBumperMesh.name = "rear_bumper"
  group.add(rearBumperMesh)

  // Wheels
  const tireGeometry = new THREE.CylinderGeometry(0.325, 0.325, 0.2, 16)
  const rimGeometry = new THREE.CylinderGeometry(0.2, 0.2, 0.22, 16)

  // Front left wheel
  const frontLeftTire = new THREE.Mesh(tireGeometry, tireMaterial)
  frontLeftTire.position.set(1.5, 0.325, 0.95)
  frontLeftTire.rotation.z = Math.PI / 2
  frontLeftTire.castShadow = true
  frontLeftTire.receiveShadow = true
  frontLeftTire.name = "front_left_tire"
  group.add(frontLeftTire)

  const frontLeftRim = new THREE.Mesh(rimGeometry, rimMaterial)
  frontLeftRim.position.set(1.5, 0.325, 0.95)
  frontLeftRim.rotation.z = Math.PI / 2
  frontLeftRim.castShadow = true
  frontLeftRim.receiveShadow = true
  frontLeftRim.name = "front_left_rim"
  group.add(frontLeftRim)

  // Front right wheel
  const frontRightTire = new THREE.Mesh(tireGeometry, tireMaterial)
  frontRightTire.position.set(1.5, 0.325, -0.95)
  frontRightTire.rotation.z = Math.PI / 2
  frontRightTire.castShadow = true
  frontRightTire.receiveShadow = true
  frontRightTire.name = "front_right_tire"
  group.add(frontRightTire)

  const frontRightRim = new THREE.Mesh(rimGeometry, rimMaterial)
  frontRightRim.position.set(1.5, 0.325, -0.95)
  frontRightRim.rotation.z = Math.PI / 2
  frontRightRim.castShadow = true
  frontRightRim.receiveShadow = true
  frontRightRim.name = "front_right_rim"
  group.add(frontRightRim)

  // Rear left wheel
  const rearLeftTire = new THREE.Mesh(tireGeometry, tireMaterial)
  rearLeftTire.position.set(-1.5, 0.325, 0.95)
  rearLeftTire.rotation.z = Math.PI / 2
  rearLeftTire.castShadow = true
  rearLeftTire.receiveShadow = true
  rearLeftTire.name = "rear_left_tire"
  group.add(rearLeftTire)

  const rearLeftRim = new THREE.Mesh(rimGeometry, rimMaterial)
  rearLeftRim.position.set(-1.5, 0.325, 0.95)
  rearLeftRim.rotation.z = Math.PI / 2
  rearLeftRim.castShadow = true
  rearLeftRim.receiveShadow = true
  rearLeftRim.name = "rear_left_rim"
  group.add(rearLeftRim)

  // Rear right wheel
  const rearRightTire = new THREE.Mesh(tireGeometry, tireMaterial)
  rearRightTire.position.set(-1.5, 0.325, -0.95)
  rearRightTire.rotation.z = Math.PI / 2
  rearRightTire.castShadow = true
  rearRightTire.receiveShadow = true
  rearRightTire.name = "rear_right_tire"
  group.add(rearRightTire)

  const rearRightRim = new THREE.Mesh(rimGeometry, rimMaterial)
  rearRightRim.position.set(-1.5, 0.325, -0.95)
  rearRightRim.rotation.z = Math.PI / 2
  rearRightRim.castShadow = true
  rearRightRim.receiveShadow = true
  rearRightRim.name = "rear_right_rim"
  group.add(rearRightRim)

  return group
}