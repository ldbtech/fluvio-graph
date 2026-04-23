"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import { GraphCanvas } from "./GraphCanvas";
import { fetchCodebaseGalaxyTree } from "@/lib/fetchCodebaseGalaxy";
import { moduleSubtreeToGraph } from "@/lib/moduleTreeToGraph";
import type {
  CodebaseCloneResult,
  CodebaseIngestResult,
  CodebaseModuleTree,
  GraphEdge,
  GraphNode,
} from "@/lib/types"; 

type Props = {
  kgUrl: string;
  cloneInfo: CodebaseCloneResult | null;
  className?: string;
  /** After a planet’s subtree is ingested into the KG — refresh workspace graph from the server. */
  onPlanetIngestComplete?: () => void | Promise<void>;
};

function repoRelativeModulePrefix(m: CodebaseModuleTree): string {
  const p = (m.path || "").replace(/\\/g, "/").replace(/^\.\//, "").trim();
  if (p.length > 0) return p;
  return m.name.trim();
}

/** Cool alloy + slight teal drift — reads like painted hardware in flight lighting. */
function analysisBodyColor(lang: string): number {
  let h = 2166136261;
  const s = lang.toLowerCase();
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  const t = ((h >>> 0) % 1000) / 1000;
  const lo = new THREE.Color(0x1e2d38);
  const hi = new THREE.Color(0x3d5a6e);
  return lo.clone().lerp(hi, t).getHex();
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function formatMissionElapsed(totalSec: number): string {
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function makeAnalysisPlanet(hex: number): THREE.MeshStandardMaterial {
  const c = new THREE.Color(hex);
  return new THREE.MeshStandardMaterial({
    color: c,
    metalness: 0.88,
    roughness: 0.22,
    emissive: new THREE.Color(0x020810),
    emissiveIntensity: 0.12,
  });
}

function applyOrthoFrustum(cam: THREE.OrthographicCamera, cw: number, ch: number, extentY: number) {
  const aspect = cw / Math.max(1, ch);
  const halfH = extentY;
  const halfW = halfH * aspect;
  cam.left = -halfW;
  cam.right = halfW;
  cam.top = halfH;
  cam.bottom = -halfH;
  cam.updateProjectionMatrix();
}

function createOrbitTrack(radius: number): THREE.LineLoop {
  const curve = new THREE.EllipseCurve(0, 0, radius, radius, 0, Math.PI * 2, false, 0);
  const pts = curve.getPoints(144).map((p) => new THREE.Vector3(p.x, 0, p.y));
  const geom = new THREE.BufferGeometry().setFromPoints(pts);
  const mat = new THREE.LineBasicMaterial({
    color: 0x3ecfff,
    transparent: true,
    opacity: 0.38,
    depthWrite: false,
  });
  return new THREE.LineLoop(geom, mat);
}

function createGroundGrid(maxR: number): THREE.LineSegments {
  const v: number[] = [];
  const spokes = 12;
  for (let i = 0; i < spokes; i++) {
    const a = (i / spokes) * Math.PI * 2;
    v.push(0, 0, 0, Math.cos(a) * maxR, 0, Math.sin(a) * maxR);
  }
  const rings = 5;
  for (let ri = 1; ri <= rings; ri++) {
    const r = (ri / rings) * maxR;
    for (let i = 0; i < 72; i++) {
      const a0 = (i / 72) * Math.PI * 2;
      const a1 = ((i + 1) / 72) * Math.PI * 2;
      v.push(Math.cos(a0) * r, 0, Math.sin(a0) * r, Math.cos(a1) * r, 0, Math.sin(a1) * r);
    }
  }
  const geom = new THREE.BufferGeometry();
  geom.setAttribute("position", new THREE.Float32BufferAttribute(v, 3));
  const mat = new THREE.LineBasicMaterial({
    color: 0x1a6a8a,
    transparent: true,
    opacity: 0.22,
    depthWrite: false,
  });
  return new THREE.LineSegments(geom, mat);
}

function createStarfield(count: number, spread: number): THREE.Points {
  const positions = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    positions[i * 3] = (Math.random() - 0.5) * spread;
    positions[i * 3 + 1] = (Math.random() - 0.35) * spread * 0.85;
    positions[i * 3 + 2] = (Math.random() - 0.5) * spread;
  }
  const geom = new THREE.BufferGeometry();
  geom.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  const mat = new THREE.PointsMaterial({
    color: 0x9bd4ff,
    size: 0.055,
    transparent: true,
    opacity: 0.72,
    depthWrite: false,
    sizeAttenuation: true,
  });
  return new THREE.Points(geom, mat);
}

export function SolarSystemCanvas({
  kgUrl,
  cloneInfo,
  className = "",
  onPlanetIngestComplete,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const subSvgRef = useRef<SVGSVGElement>(null);
  const [tree, setTree] = useState<CodebaseModuleTree | null>(null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [focusedModule, setFocusedModule] = useState<CodebaseModuleTree | null>(null);
  const focusRef = useRef<CodebaseModuleTree | null>(null);
  const [metSec, setMetSec] = useState(0);
  const [planetIngestBusy, setPlanetIngestBusy] = useState(false);
  const [planetIngestErr, setPlanetIngestErr] = useState<string | null>(null);
  const [planetIngestSummary, setPlanetIngestSummary] = useState<string | null>(null);
  const planetIngestGen = useRef(0);
  const planetIngestCache = useRef(new Set<string>());

  useEffect(() => {
    planetIngestCache.current.clear();
  }, [cloneInfo?.owner, cloneInfo?.repo]);

  const planetKey = useMemo(() => {
    if (!focusedModule || !cloneInfo) return null;
    return `${cloneInfo.owner}/${cloneInfo.repo}:${repoRelativeModulePrefix(focusedModule)}`;
  }, [cloneInfo?.owner, cloneInfo?.repo, focusedModule]);

  useEffect(() => {
    if (!cloneInfo || !focusedModule || !planetKey) {
      setPlanetIngestBusy(false);
      setPlanetIngestErr(null);
      setPlanetIngestSummary(null);
      return;
    }
    const pathPrefix = repoRelativeModulePrefix(focusedModule);
    if (!pathPrefix) {
      setPlanetIngestErr("Could not derive a module path for indexing.");
      return;
    }

    const cacheKey = planetKey;
    if (planetIngestCache.current.has(cacheKey)) {
      setPlanetIngestBusy(false);
      setPlanetIngestErr(null);
      setPlanetIngestSummary("This module is already in the workspace graph.");
      return;
    }

    const gen = ++planetIngestGen.current;
    let cancelled = false;
    setPlanetIngestBusy(true);
    setPlanetIngestErr(null);
    setPlanetIngestSummary(null);

    void (async () => {
      try {
        const coordUrl = `${cloneInfo.owner}/${cloneInfo.repo}`;
        const res = await fetch(`${kgUrl}/ingest`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: coordUrl, path: pathPrefix }),
        });
        const text = await res.text();
        if (!res.ok) {
          throw new Error(text || `planet ingest HTTP ${res.status}`);
        }
        let data: CodebaseIngestResult;
        try {
          data = JSON.parse(text) as CodebaseIngestResult;
        } catch {
          throw new Error(text || "invalid JSON from server");
        }
        if (cancelled || gen !== planetIngestGen.current) return;

        planetIngestCache.current.add(cacheKey);
        setPlanetIngestSummary(
          `${data.chunks} chunks ingested · ${data.nodes} total nodes · ${data.edges} total edges`,
        );
        await onPlanetIngestComplete?.();
      } catch (e: unknown) {
        if (!cancelled && gen === planetIngestGen.current) {
          setPlanetIngestErr(e instanceof Error ? e.message : String(e));
        }
      } finally {
        if (!cancelled && gen === planetIngestGen.current) {
          setPlanetIngestBusy(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [planetKey, kgUrl, onPlanetIngestComplete]);

  useEffect(() => {
    const id = window.setInterval(() => setMetSec((s) => s + 1), 1000);
    return () => window.clearInterval(id);
  }, []);

  const setFocus = useCallback((m: CodebaseModuleTree | null) => {
    focusRef.current = m;
    setFocusedModule(m);
  }, []);

  useEffect(() => {
    if (!cloneInfo) {
      setTree(null);
      setErr(null);
      setFocus(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setErr(null);
    void fetchCodebaseGalaxyTree(kgUrl, cloneInfo)
      .then((t) => {
        if (!cancelled) {
          setTree(t);
          setFocus(null);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) setErr(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [kgUrl, cloneInfo?.owner, cloneInfo?.repo, setFocus]);

  const subgraph = useMemo<{ nodes: GraphNode[]; edges: GraphEdge[] }>(() => {
    if (!focusedModule) return { nodes: [], edges: [] };
    return moduleSubtreeToGraph(focusedModule);
  }, [focusedModule]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host || !tree) return;

    const w = host.clientWidth || 640;
    const h = host.clientHeight || 220;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x020408);

    const viewExtent = { current: 10.5 };
    const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.4, 220);
    applyOrthoFrustum(camera, w, h, viewExtent.current);
    camera.position.set(22, 16, 22);
    camera.lookAt(0, 0, 0);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(w, h);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 0.95;
    host.appendChild(renderer.domElement);

    const stars = createStarfield(2200, 95);
    scene.add(stars);

    scene.add(new THREE.AmbientLight(0x4a6a8a, 0.18));
    const hemi = new THREE.HemisphereLight(0x7a9ec8, 0x020308, 0.38);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xe8f4ff, 0.78);
    key.position.set(14, 22, 12);
    scene.add(key);
    const fill = new THREE.DirectionalLight(0x2a8cc4, 0.26);
    fill.position.set(-12, 8, -10);
    scene.add(fill);
    const rim = new THREE.DirectionalLight(0x00c8ff, 0.12);
    rim.position.set(0, -6, 18);
    scene.add(rim);

    const sunGroup = new THREE.Group();
    const sunCoreGeom = new THREE.SphereGeometry(0.2, 28, 28);
    const sunCoreMat = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      emissive: 0xc8e8ff,
      emissiveIntensity: 0.55,
      metalness: 0.35,
      roughness: 0.18,
    });
    const sunCore = new THREE.Mesh(sunCoreGeom, sunCoreMat);
    sunGroup.add(sunCore);
    const torusMeshes: THREE.Mesh[] = [];
    for (let k = 0; k < 3; k++) {
      const tGeo = new THREE.TorusGeometry(0.38 + k * 0.26, 0.012, 8, 100);
      const tMat = new THREE.MeshBasicMaterial({
        color: 0x22b8ff,
        transparent: true,
        opacity: 0.42 - k * 0.08,
        depthWrite: false,
      });
      const tr = new THREE.Mesh(tGeo, tMat);
      tr.rotation.x = Math.PI / 2;
      tr.rotation.z = k * 0.35;
      sunGroup.add(tr);
      torusMeshes.push(tr);
    }
    scene.add(sunGroup);

    const grid = createGroundGrid(9.2);
    scene.add(grid);

    const modules = tree.children.filter((c) => c.kind === "module").slice(0, 36);
    const orbitRadius = 5.2;
    const usedRadii = new Set<number>();
    modules.forEach((_, i) => usedRadii.add(orbitRadius + (i % 3) * 0.35));
    const orbitTracks: THREE.LineLoop[] = [];
    usedRadii.forEach((r) => {
      const track = createOrbitTrack(r);
      scene.add(track);
      orbitTracks.push(track);
    });

    const planetMeshes: THREE.Mesh[] = [];
    const moonGroups: THREE.Group[] = [];
    const baseMaterials: THREE.MeshStandardMaterial[] = [];
    const moonMat = new THREE.MeshStandardMaterial({
      color: 0x3a5568,
      metalness: 0.72,
      roughness: 0.35,
      emissive: new THREE.Color(0x041018),
      emissiveIntensity: 0.22,
    });

    modules.forEach((mod, i) => {
      const n = Math.max(1, modules.length);
      const base = (i / n) * Math.PI * 2;
      const bodyHex = analysisBodyColor(mod.language);
      const pr = clamp(0.32 + Math.log10(mod.file_count + 1) * 0.1, 0.28, 0.82);
      const geom = new THREE.SphereGeometry(pr, 36, 36);
      const mat = makeAnalysisPlanet(bodyHex);
      const mesh = new THREE.Mesh(geom, mat);
      mesh.userData.baseAngle = base;
      mesh.userData.speed = 0.28 + (i % 5) * 0.03;
      mesh.userData.orbitR = orbitRadius + (i % 3) * 0.35;
      mesh.userData.moduleIndex = i;
      scene.add(mesh);
      planetMeshes.push(mesh);
      baseMaterials.push(mat);

      const moonOrbit = new THREE.Group();
      scene.add(moonOrbit);
      moonGroups.push(moonOrbit);

      const files = mod.children.filter((c) => c.kind === "file").slice(0, 8);
      files.forEach((_file, j) => {
        const mg = new THREE.SphereGeometry(0.065, 10, 10);
        const moon = new THREE.Mesh(mg, moonMat);
        const mr = 0.72 + j * 0.16;
        moon.position.set(mr, 0, 0);
        moon.userData.orbitR = mr;
        moon.userData.speed = 1.1 + j * 0.12;
        moon.userData.phase = j * 0.7;
        moonOrbit.add(moon);
      });
    });

    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    const zoomFocusDist = { current: 5.2 };

    const onClick = (e: MouseEvent) => {
      const rect = renderer.domElement.getBoundingClientRect();
      pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hits = raycaster.intersectObjects(planetMeshes, false);
      if (hits.length > 0) {
        const mesh = hits[0].object as THREE.Mesh;
        const idx = typeof mesh.userData.moduleIndex === "number" ? mesh.userData.moduleIndex : -1;
        if (idx >= 0 && modules[idx]) {
          setFocus(modules[idx]);
          return;
        }
      }
      setFocus(null);
    };

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (focusRef.current) {
        zoomFocusDist.current = clamp(zoomFocusDist.current + e.deltaY * 0.012, 2.4, 14);
      } else {
        viewExtent.current = clamp(viewExtent.current + e.deltaY * 0.032, 5.5, 22);
        applyOrthoFrustum(camera, host.clientWidth || w, host.clientHeight || h, viewExtent.current);
      }
    };

    renderer.domElement.addEventListener("click", onClick);
    renderer.domElement.addEventListener("wheel", onWheel, { passive: false });

    let raf = 0;
    const t0 = performance.now();
    const restCam = new THREE.Vector3(22, 14, 22);
    const tmpDesired = new THREE.Vector3();

    const tick = (now: number) => {
      const t = (now - t0) / 1000;
      sunGroup.rotation.y = t * 0.12;
      stars.rotation.y = t * 0.008;

      planetMeshes.forEach((mesh) => {
        const base = mesh.userData.baseAngle as number;
        const sp = mesh.userData.speed as number;
        const R = mesh.userData.orbitR as number;
        const a = base + t * sp * 0.22;
        mesh.position.set(Math.cos(a) * R, Math.sin(a * 0.35) * 0.4, Math.sin(a) * R);
      });

      moonGroups.forEach((grp, i) => {
        const planet = planetMeshes[i];
        if (planet) {
          grp.position.copy(planet.position);
          grp.rotation.y = t * 0.9;
        }
        grp.children.forEach((ch) => {
          if (!(ch instanceof THREE.Mesh)) return;
          const mr = ch.userData.orbitR as number;
          const sp = ch.userData.speed as number;
          const ph = ch.userData.phase as number;
          const a = t * sp + ph;
          ch.position.set(Math.cos(a) * mr, Math.sin(a * 2) * 0.15, Math.sin(a) * mr);
        });
      });

      const focus = focusRef.current;
      const focusIdx = focus ? modules.indexOf(focus) : -1;

      planetMeshes.forEach((mesh, i) => {
        const mat = baseMaterials[i];
        if (!mat) return;
        const sel = i === focusIdx;
        if (sel) {
          mat.emissive.setHex(0x006688);
          mat.emissiveIntensity = 0.62;
        } else {
          mat.emissive.setHex(0x020810);
          mat.emissiveIntensity = 0.12;
        }
      });

      if (focus && focusIdx >= 0) {
        const planet = planetMeshes[focusIdx];
        if (planet) {
          const target = planet.position;
          tmpDesired.copy(target).add(
            new THREE.Vector3(1.35, 0.85, 1.35).normalize().multiplyScalar(zoomFocusDist.current),
          );
          camera.position.lerp(tmpDesired, 0.1);
          camera.lookAt(target);
        }
      } else {
        const ang = t * 0.052;
        restCam.set(Math.cos(ang) * 24, 14, Math.sin(ang) * 24);
        camera.position.lerp(restCam, 0.07);
        camera.lookAt(0, 0, 0);
      }

      renderer.render(scene, camera);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    const ro = new ResizeObserver(() => {
      const cw = host.clientWidth || w;
      const ch = host.clientHeight || h;
      applyOrthoFrustum(camera, cw, ch, viewExtent.current);
      renderer.setSize(cw, ch);
    });
    ro.observe(host);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      renderer.domElement.removeEventListener("click", onClick);
      renderer.domElement.removeEventListener("wheel", onWheel);
      renderer.dispose();
      sunCoreGeom.dispose();
      sunCoreMat.dispose();
      torusMeshes.forEach((tm) => {
        tm.geometry.dispose();
        (tm.material as THREE.Material).dispose();
      });
      grid.geometry.dispose();
      (grid.material as THREE.Material).dispose();
      stars.geometry.dispose();
      (stars.material as THREE.Material).dispose();
      orbitTracks.forEach((tr) => {
        tr.geometry.dispose();
        (tr.material as THREE.Material).dispose();
      });
      planetMeshes.forEach((m) => {
        m.geometry.dispose();
        (m.material as THREE.Material).dispose();
      });
      moonGroups.forEach((g) => {
        g.children.forEach((ch) => {
          if (ch instanceof THREE.Mesh) {
            ch.geometry.dispose();
          }
        });
      });
      moonMat.dispose();
      host.removeChild(renderer.domElement);
    };
  }, [tree, setFocus]);

  if (!cloneInfo) {
    return (
      <div
        className={`flex h-full min-h-[120px] flex-col items-center justify-center border-b border-cyan-900/30 bg-[#020408] px-4 text-center ${className}`}
      >
        <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-cyan-600/90">Flight deck offline</p>
        <p className="mt-2 max-w-md text-[13px] leading-relaxed text-slate-500">
          Clone a public repository in Sources to activate the module orbit and telemetry feed.
        </p>
      </div>
    );
  }

  const hudFrame = (
    <>
      <div
        className="pointer-events-none absolute left-2 top-2 z-[6] h-7 w-7 border-l-2 border-t-2 border-cyan-400/45"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute right-2 top-2 z-[6] h-7 w-7 border-r-2 border-t-2 border-cyan-400/45"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute bottom-2 left-2 z-[6] h-7 w-7 border-b-2 border-l-2 border-cyan-400/35"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute bottom-2 right-2 z-[6] h-7 w-7 border-b-2 border-r-2 border-cyan-400/35"
        aria-hidden
      />
    </>
  );

  return (
    <div
      className={`relative flex h-full min-h-0 flex-col border-b border-cyan-900/25 bg-[#020408] ${className}`}
    >
      <header className="relative z-20 flex h-9 shrink-0 items-center justify-between gap-3 border-b border-cyan-500/15 bg-gradient-to-r from-black/80 via-[#061018]/95 to-black/80 px-3 font-mono text-[10px] uppercase tracking-wide text-cyan-100/90">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <span className="shrink-0 text-cyan-400/90">Orbit</span>
          <span className="truncate text-slate-400/95 normal-case tracking-normal">
            {cloneInfo.owner}/{cloneInfo.repo}
          </span>
          <span className="hidden text-slate-600 sm:inline">·</span>
          <span className="hidden text-slate-600 sm:inline">GET /parse + POST /ingest</span>
        </div>
        <div className="flex shrink-0 items-center gap-3 tabular-nums">
          <span className="text-cyan-500/80">
            MET <span className="text-cyan-200/95">{formatMissionElapsed(metSec)}</span>
          </span>
          {loading ? (
            <span className="rounded border border-amber-500/35 bg-amber-950/40 px-1.5 py-0.5 text-[9px] font-semibold tracking-wider text-amber-200/90">
              ACQ
            </span>
          ) : err ? (
            <span className="rounded border border-red-500/40 bg-red-950/50 px-1.5 py-0.5 text-[9px] font-semibold tracking-wider text-red-200/90">
              FLT ERR
            </span>
          ) : (
            <span className="rounded border border-emerald-500/35 bg-emerald-950/35 px-1.5 py-0.5 text-[9px] font-semibold tracking-wider text-emerald-200/90">
              LINK OK
            </span>
          )}
        </div>
      </header>

      <div className="relative min-h-[96px] w-full flex-1 shrink-0 overflow-hidden">
        {hudFrame}
        <div
          className="pointer-events-none absolute inset-0 z-[4] opacity-[0.035]"
          style={{
            backgroundImage:
              "repeating-linear-gradient(0deg, transparent, transparent 1px, rgba(56, 189, 248, 0.45) 1px, rgba(56, 189, 248, 0.45) 2px)",
          }}
          aria-hidden
        />
        <div className="pointer-events-none absolute left-3 top-2.5 z-[8] max-w-[min(100%,20rem)] text-[10px] leading-snug text-slate-500">
          <span className="font-mono text-cyan-600/80">Orthographic situation</span>
          <span className="mt-0.5 block font-mono text-[9px] text-slate-600">
            Click planet · KG indexes that module (loading below) · tree graph + /chat can use those nodes
          </span>
        </div>
        {focusedModule && (
          <button
            type="button"
            onClick={() => setFocus(null)}
            className="absolute right-3 top-2 z-20 rounded border border-cyan-500/35 bg-black/70 px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-wider text-cyan-100 shadow-[0_0_12px_rgba(34,211,238,0.12)] backdrop-blur-sm pointer-events-auto transition hover:border-cyan-400/55 hover:bg-cyan-950/40"
          >
            Resume orbit
          </button>
        )}
        {loading && (
          <div className="absolute inset-0 z-[14] flex flex-col items-center justify-center gap-2 bg-[#020408]/88 font-mono text-[12px] text-cyan-200/90 backdrop-blur-[2px]">
            <span className="text-[10px] uppercase tracking-[0.35em] text-cyan-500/80">Acquiring telemetry</span>
            <span className="inline-flex h-1 w-24 overflow-hidden rounded-full bg-cyan-950">
              <span className="h-full w-1/3 animate-pulse rounded-full bg-cyan-400/70" />
            </span>
          </div>
        )}
        {err && !loading && (
          <div className="absolute inset-0 z-[14] flex items-center justify-center bg-[#020408]/92 px-4 text-center font-mono text-[12px] leading-relaxed text-amber-200/95">
            {err}
          </div>
        )}
        <div ref={hostRef} className="absolute inset-0 min-h-[96px]" />
      </div>

      {focusedModule && (
        <div className="relative flex min-h-[120px] flex-1 flex-col border-t border-cyan-900/30 bg-[#030508]">
          <div className="flex shrink-0 flex-col gap-1 border-b border-cyan-500/10 bg-black/50 px-3 py-1.5 font-mono text-[10px] uppercase tracking-wide text-slate-400">
            <div className="flex items-center justify-between gap-2">
              <span className="min-w-0 truncate normal-case tracking-normal">
                <span className="text-cyan-600/90">
                  {planetIngestBusy ? "Indexing module → KG" : "Structure"}
                </span>{" "}
                <span className="text-slate-500">·</span> {focusedModule.path || focusedModule.name}{" "}
                <span className="text-slate-600">·</span> {focusedModule.language}{" "}
                <span className="text-slate-600">·</span> {subgraph.nodes.length} tree nodes
              </span>
              <span className="hidden shrink-0 text-slate-600 sm:inline normal-case">
                Drag graph · scroll wheel zoom
              </span>
            </div>
            {planetIngestSummary && !planetIngestBusy && (
              <p className="normal-case text-[9px] font-normal tracking-normal text-emerald-200/85">
                {planetIngestSummary}
              </p>
            )}
            {planetIngestErr && !planetIngestBusy && (
              <p className="normal-case text-[9px] font-normal tracking-normal text-red-300/90">
                {planetIngestErr}
              </p>
            )}
          </div>
          <div className="relative min-h-0 flex-1 bg-[#020408]">
            {planetIngestBusy && (
              <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 bg-[#020408]/90 font-mono text-[11px] text-cyan-100/95 backdrop-blur-sm">
                <span className="text-[10px] uppercase tracking-[0.28em] text-cyan-500/90">Building KG for this planet</span>
                <span className="max-w-[18rem] px-3 text-center text-[10px] font-normal normal-case tracking-normal text-slate-400">
                  Parsing, embedding, and merging into the workspace graph — safe to wait; re-open uses cache.
                </span>
                <span className="inline-flex h-1 w-28 overflow-hidden rounded-full bg-cyan-950">
                  <span className="h-full w-2/5 animate-pulse rounded-full bg-cyan-400/75" />
                </span>
              </div>
            )}
            {subgraph.nodes.length > 0 ? (
              <GraphCanvas
                svgRef={subSvgRef}
                nodes={subgraph.nodes}
                edges={subgraph.edges}
                onSelect={() => {}}
              />
            ) : (
              <div className="flex h-full min-h-[100px] items-center justify-center px-4 text-center font-mono text-[10px] text-slate-600">
                No file children in this module for the tree graph.
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
