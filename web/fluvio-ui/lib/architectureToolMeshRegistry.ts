"use client";

import * as THREE from "three";

export type SceneArtifactForMesh = {
  tool_file: string;
  tool_name?: string;
  position: [number, number, number];
  rotation_y: number;
  scale: number;
  style: string;
  material: string;
};

/**
 * Runtime-generated catalog files cannot be statically imported by the web bundle.
 * Return null so callers always use a safe placeholder when no client-side mesh exists.
 */
export function buildToolMesh(art: SceneArtifactForMesh): THREE.Group | null {
  void art;
  return null;
}

/** Fallback when the server references a tool the client bundle does not map yet. */
export function buildPlaceholderArtifact(art: SceneArtifactForMesh): THREE.Group {
  const g = new THREE.Group();
  const box = new THREE.Mesh(
    new THREE.BoxGeometry(1.2, 0.8, 2.0),
    new THREE.MeshStandardMaterial({ color: 0xf97316, roughness: 0.75, metalness: 0.1 }),
  );
  box.position.y = 0.4;
  g.add(box);
  g.userData.placeholder = true;
  g.userData.tool_file = art.tool_file;
  g.userData.tool_name = art.tool_name ?? art.tool_file;
  return g;
}
