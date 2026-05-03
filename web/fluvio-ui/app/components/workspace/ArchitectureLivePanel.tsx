"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import { buildPlaceholderArtifact, buildToolMesh } from "@/lib/architectureToolMeshRegistry";

type Props = {
  scene: ArchitectureScene | null;
  designId: string | null;
  busy?: boolean;
  error?: string | null;
  toolJobStatus?: { phase: string; percent: number; message: string; done: boolean } | null;
  className?: string;
  /** When set with `onFocusRoomChange`, room focus is controlled by the parent (e.g. for /architecture/chat). */
  focusRoomId?: string | null;
  onFocusRoomChange?: (roomId: string | null) => void;
};

type SceneRoom = {
  id: string;
  name: string;
  position: [number, number, number];
  dimensions: [number, number, number];
  material: string;
  zone: string;
  node_id: string;
};

type SceneWall = {
  from_room: string;
  to_room: string;
  position: [number, number, number];
  dimensions: [number, number, number];
  rotation_y: number;
  material: string;
};

type SceneOpening = {
  kind: string;
  from_room: string;
  to_room: string;
  position: [number, number, number];
  dimensions: [number, number, number];
  label: string;
};

/** Catalog mesh placement from POST /architecture/chat (`merge_llm_artifacts_into_scene`). */
export type SceneArtifact = {
  tool_file: string;
  tool_name?: string;
  room_id: string;
  position: [number, number, number];
  rotation_y?: number;
  scale?: number;
  style?: string;
  material?: string;
};

export type ArchitectureScene = {
  design_id: string;
  rooms: SceneRoom[];
  walls: SceneWall[];
  openings: SceneOpening[];
  camera: { position: [number, number, number]; target: [number, number, number]; fov: number };
  bounds: [number, number, number, number];
  total_area: number;
  artifacts?: SceneArtifact[];
};

function roomColor(material: string): number {
  if (material === "tile_white") return 0xd4d4d8;
  if (material === "tile_light") return 0xd6d3d1;
  if (material === "grass") return 0x14532d;
  if (material === "concrete_grey") return 0x52525b;
  if (material === "plaster_warm") return 0x78716c;
  return 0xe4e4e7;
}

export function ArchitectureLivePanel({
  scene,
  designId,
  busy = false,
  error = null,
  toolJobStatus = null,
  className = "",
  focusRoomId: focusRoomIdProp,
  onFocusRoomChange,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const [internalRoomId, setInternalRoomId] = useState<string | null>(null);
  const liftFocus = onFocusRoomChange != null;
  const selectedRoomId = liftFocus ? (focusRoomIdProp ?? null) : internalRoomId;
  const setSelectedRoomId = (id: string | null) => {
    if (liftFocus) onFocusRoomChange?.(id);
    else setInternalRoomId(id);
  };

  const selectedRoom = useMemo(
    () => scene?.rooms.find((r) => r.id === selectedRoomId) ?? null,
    [scene, selectedRoomId],
  );

  useEffect(() => {
    if (!selectedRoom) return;
  }, [selectedRoom?.id]);

  useEffect(() => {
    if (liftFocus) return;
    if (!scene?.rooms.length) {
      setInternalRoomId(null);
      return;
    }
    if (!internalRoomId || !scene.rooms.some((r) => r.id === internalRoomId)) {
      setInternalRoomId(scene.rooms[0].id);
    }
  }, [scene, internalRoomId, liftFocus]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host || !scene) return;

    const width = Math.max(320, host.clientWidth || 320);
    const height = Math.max(220, host.clientHeight || 220);
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(width, height);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.appendChild(renderer.domElement);

    const s = new THREE.Scene();
    s.background = new THREE.Color(0x020617);
    s.add(new THREE.AmbientLight(0xffffff, 0.42));
    const key = new THREE.DirectionalLight(0xe2e8f0, 0.8);
    key.position.set(14, 24, 14);
    s.add(key);

    const [minX, minZ, maxX, maxZ] = scene.bounds;
    const cx = (minX + maxX) / 2;
    const cz = (minZ + maxZ) / 2;
    const span = Math.max(maxX - minX, maxZ - minZ, 12);
    const camera = new THREE.PerspectiveCamera(scene.camera.fov || 50, width / height, 0.1, 500);
    camera.position.set(cx + span * 1.1, span * 0.8, cz + span * 1.1);
    camera.lookAt(cx, 0, cz);

    const grid = new THREE.GridHelper(span * 2.2, 28, 0x334155, 0x1e293b);
    grid.position.set(cx, 0, cz);
    s.add(grid);

    const roomMeshes: THREE.Mesh[] = [];
    for (const room of scene.rooms) {
      const [w, h, d] = room.dimensions;
      const geom = new THREE.BoxGeometry(w, h, d);
      const mat = new THREE.MeshStandardMaterial({
        color: roomColor(room.material),
        metalness: 0.08,
        roughness: 0.88,
        transparent: true,
        opacity: selectedRoomId && selectedRoomId !== room.id ? 0.46 : 0.95,
      });
      const mesh = new THREE.Mesh(geom, mat);
      mesh.position.set(room.position[0], room.position[1], room.position[2]);
      if (selectedRoomId === room.id) {
        const edge = new THREE.LineSegments(
          new THREE.EdgesGeometry(geom),
          new THREE.LineBasicMaterial({ color: 0x22d3ee }),
        );
        mesh.add(edge);
      }
      s.add(mesh);
      roomMeshes.push(mesh);
    }

    const wallMat = new THREE.MeshStandardMaterial({
      color: 0x94a3b8,
      metalness: 0.15,
      roughness: 0.9,
      transparent: true,
      opacity: 0.62,
    });
    for (const wall of scene.walls) {
      const [w, h, d] = wall.dimensions;
      const geom = new THREE.BoxGeometry(w, h, d);
      const mesh = new THREE.Mesh(geom, wallMat);
      mesh.position.set(wall.position[0], wall.position[1], wall.position[2]);
      mesh.rotation.y = wall.rotation_y;
      s.add(mesh);
    }

    const openingMat = new THREE.MeshStandardMaterial({
      color: 0xf59e0b,
      metalness: 0.15,
      roughness: 0.35,
      emissive: 0x78350f,
      emissiveIntensity: 0.28,
    });
    for (const opening of scene.openings) {
      const [w, h, d] = opening.dimensions;
      const geom = new THREE.BoxGeometry(w, h, d);
      const mesh = new THREE.Mesh(geom, openingMat);
      mesh.position.set(opening.position[0], opening.position[1], opening.position[2]);
      s.add(mesh);
    }

    const artifactRoots: THREE.Object3D[] = [];
    for (const art of scene.artifacts ?? []) {
      const meshInput = {
        tool_file: art.tool_file,
        tool_name: art.tool_name,
        position: art.position,
        rotation_y: art.rotation_y ?? 0,
        scale: art.scale ?? 1,
        style: art.style ?? "modern",
        material: art.material ?? "matte_black",
      };
      const built = buildToolMesh(meshInput) ?? buildPlaceholderArtifact(meshInput);
      built.position.set(art.position[0], art.position[1], art.position[2]);
      built.rotation.y = meshInput.rotation_y;
      built.scale.setScalar(meshInput.scale);
      s.add(built);
      artifactRoots.push(built);
    }

    let raf = 0;
    const tick = () => {
      renderer.render(s, camera);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    const ro = new ResizeObserver(() => {
      const w = Math.max(320, host.clientWidth || 320);
      const h = Math.max(220, host.clientHeight || 220);
      renderer.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    });
    ro.observe(host);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      roomMeshes.forEach((m) => {
        m.geometry.dispose();
        (m.material as THREE.Material).dispose();
      });
      artifactRoots.forEach((root) => {
        root.traverse((obj) => {
          if (obj instanceof THREE.Mesh) {
            obj.geometry?.dispose();
            const mat = obj.material;
            if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
            else (mat as THREE.Material | undefined)?.dispose?.();
          }
        });
      });
      renderer.dispose();
      if (host.contains(renderer.domElement)) host.removeChild(renderer.domElement);
    };
  }, [scene, selectedRoomId]);

  return (
    <aside
      className={`relative flex flex-col border-t border-white/[0.08] bg-zinc-950/85 lg:h-full lg:min-h-0 lg:w-[min(100%,480px)] lg:shrink-0 lg:border-l lg:border-t-0 ${className}`}
    >
      <div className="border-b border-white/[0.06] px-4 py-3">
        <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-600">Three.js architecture</p>
        <p className="mt-1 text-[13px] text-zinc-300">Driven from right chat</p>
      </div>

      <div className="space-y-1 border-b border-white/[0.06] px-4 py-3 font-mono text-[11px] text-zinc-500">
        <p>
          <span className="text-zinc-400">/design generate</span> {"<brief>"}
        </p>
        <p>
          <span className="text-zinc-400">plain text</span> or <span className="text-zinc-400">/modify</span> —{" "}
          <span className="text-zinc-400">POST /architecture/chat</span> (scene may include <span className="font-mono text-zinc-500">artifacts</span>{" "}
          for catalog meshes from <span className="font-mono text-zinc-500">fluvio-tools/src/tools</span>)
        </p>
        {designId && <p>design_id: {designId}</p>}
      </div>

      {scene && (
        <div className="space-y-2 border-b border-white/[0.06] px-4 py-3">
          <label className="text-[11px] text-zinc-500">Room</label>
          <select
            value={selectedRoomId ?? ""}
            onChange={(e) => setSelectedRoomId(e.target.value || null)}
            className="w-full rounded-xl border border-white/[0.08] bg-zinc-900/70 px-3 py-2 text-[12px] text-zinc-100 outline-none"
          >
            {scene.rooms.map((room) => (
              <option key={room.id} value={room.id}>
                {room.name} ({room.id})
              </option>
            ))}
          </select>
          <p className="text-[10px] text-zinc-600">
            Room above is sent as <span className="font-mono text-zinc-500">selected_room_id</span> with natural-language
            edits.
          </p>
        </div>
      )}

      <div ref={hostRef} className="min-h-[220px] flex-1" />

      {!scene && (
        <div className="pointer-events-none absolute inset-x-0 bottom-4 flex justify-center">
          <p className="rounded-full border border-white/[0.08] bg-black/40 px-3 py-1 text-[10px] text-zinc-500">
            Use right chat: /design generate ...
          </p>
        </div>
      )}
      {(error || busy || toolJobStatus) && (
        <div className="border-t border-white/[0.08] bg-zinc-900/60 px-4 py-2 text-[11px] text-zinc-300">
          {error ? (
            error
          ) : toolJobStatus && !toolJobStatus.done ? (
            <span>
              Generating tool in background: {toolJobStatus.phase} {Math.max(0, Math.min(100, Math.round(toolJobStatus.percent)))}%
              {toolJobStatus.message ? ` - ${toolJobStatus.message}` : ""}
            </span>
          ) : busy ? (
            "Applying design command..."
          ) : null}
        </div>
      )}
    </aside>
  );
}
