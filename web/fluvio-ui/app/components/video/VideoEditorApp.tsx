"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { GraphCanvas } from "@/app/components/workspace/GraphCanvas";
import { GraphErrorBoundary } from "@/app/components/workspace/GraphErrorBoundary";
import { getKgEngineUrl } from "@/lib/constants";
import { type ChatMessage, type VideoNode, useVideoStore } from "@/lib/videoStore";
import type { GraphEdge, GraphNode, SelectedNode } from "@/lib/types";

const ENTITY_COLORS: Record<string, string> = {
  person: "#f472b6",
  object: "#fbbf24",
  scene: "#38bdf8",
  text: "#00d17a",
  effect: "#7b61ff",
  Flower: "#f43f5e",
};

/** Read duration and display size from a blob/object URL (no server required). */
function probeVideoMetadata(objectUrl: string): Promise<{ duration: number; width: number; height: number }> {
  return new Promise((resolve) => {
    const v = document.createElement("video");
    v.muted = true;
    v.preload = "metadata";
    const done = () => {
      const duration = Number.isFinite(v.duration) && v.duration > 0 ? v.duration : 0;
      const width = v.videoWidth || 0;
      const height = v.videoHeight || 0;
      try {
        v.removeAttribute("src");
        v.load();
      } catch {
        /* ignore */
      }
      resolve({ duration, width, height });
    };
    const t = window.setTimeout(done, 14_000);
    v.onloadedmetadata = () => {
      window.clearTimeout(t);
      done();
    };
    v.onerror = () => {
      window.clearTimeout(t);
      done();
    };
    v.src = objectUrl;
  });
}

function formatResolution(w: number, h: number) {
  if (w > 0 && h > 0) return `${w}×${h}`;
  return "—";
}

function clamp(n: number, lo: number, hi: number) {
  return Math.min(hi, Math.max(lo, n));
}

const LAYOUT_LS = "fluvio-video-layout-v1";
const SIDEBAR_W = { min: 280, max: 560, def: 400 } as const;
const VIDEO_W = { min: 260, max: 800, def: 400 } as const;

type RightDockTab = "agents" | "chat" | "files";

function formatTime(seconds: number) {
  const min = Math.floor(seconds / 60);
  const sec = Math.floor(seconds % 60);
  const ms = Math.floor((seconds - Math.floor(seconds)) * 1000);
  return `${String(min).padStart(2, "0")}:${String(sec).padStart(2, "0")}.${String(ms).padStart(3, "0")}`;
}

/** Prior turns for `POST /chat` (excludes the latest user line; drops tool-gen rows). */
function kgChatHistoryFromMessages(messages: ChatMessage[]): { role: string; content: string }[] {
  const eligible = messages.filter(
    (m) =>
      (m.role === "user" || m.role === "assistant") &&
      m.type !== "tool-generated" &&
      String(m.content ?? "").trim().length > 0,
  );
  return eligible.slice(0, -1).map((m) => ({ role: m.role, content: m.content }));
}

function videoGraphFromNodes(videoNodes: VideoNode[]): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = videoNodes.map((n) => ({
    id: n.id,
    label: n.label,
    page: `${n.time_start.toFixed(1)}s–${n.time_end.toFixed(1)}s`,
    source: "video",
  }));
  const uriToId = new Map<string, string>();
  for (const n of videoNodes) {
    uriToId.set(n.source_uri, n.id);
    uriToId.set(n.id, n.id);
  }
  const edges: GraphEdge[] = [];
  for (const n of videoNodes) {
    for (const e of n.edges) {
      let toId = uriToId.get(e.to_uri);
      if (!toId) {
        const hit = videoNodes.find((v) => v.source_uri === e.to_uri || v.id === e.to_uri);
        toId = hit?.id;
      }
      if (!toId) continue;
      edges.push({
        from: n.id,
        to: toId,
        token: 1,
        probability: e.prob,
        label: e.label,
      });
    }
  }
  return { nodes, edges };
}

/** `POST /ingest/video` JSON body (snake_case). */
type VideoIngestPostResponse = {
  video_id: string;
  duration: number;
  fps: number;
  resolution: string;
  codec?: string;
  scenes?: number;
  status?: string;
};

/** `GET /video/{id}` scene row. */
type SceneApiRow = {
  scene_index: number;
  time_start: number;
  time_end: number;
  duration: number;
  sample_time: number;
  score: number;
  understanding: string;
  /** Present when `understanding` is `failed` (Ollama down, timeout, etc.). */
  understanding_error?: string | null;
  description: string | null;
  source_uri: string;
};

/** `GET /video/{id}/status` — LLaVA-over-Ollama progress for this clip’s scene nodes. */
type VideoVisionStatus = {
  video_id: string;
  total: number;
  complete: number;
  failed: number;
  pending: number;
  percent: number;
  processed_percent: number;
  done: boolean;
};

type VideoDetailResponse = {
  video_id: string;
  duration: number;
  fps: number;
  resolution: string;
  codec: string;
  scene_count: number;
  has_audio: boolean;
  scenes: SceneApiRow[];
};

function scenesToVideoNodes(scenes: SceneApiRow[], fps: number): VideoNode[] {
  const f = fps > 0 ? fps : 30;
  return scenes.map((s) => {
    const firstLine =
      s.description?.trim().split(/\r?\n/).find((l) => l.length > 0)?.slice(0, 120) ?? null;
    const u = s.understanding;
    const label =
      u === "failed"
        ? `Scene ${s.scene_index} (vision failed)`
        : firstLine ??
          (u === "pending" || u === ""
            ? `Scene ${s.scene_index} (understanding…)`
            : `Scene ${s.scene_index}`);
    const semantic =
      u === "failed"
        ? (s.understanding_error?.trim() || "Ollama/LLaVA could not describe this scene (see server logs).")
        : (s.description ?? s.understanding);
    return {
      id: `scene-${s.scene_index}`,
      source_uri: s.source_uri,
      entity_type: "scene" as const,
      label,
      time_start: s.time_start,
      time_end: s.time_end,
      frame_start: Math.floor(s.time_start * f),
      frame_end: Math.ceil(s.time_end * f),
      bbox: null,
      mask_path: null,
      confidence: Number.isFinite(s.score) ? s.score : 0,
      semantic,
      edges: [],
    };
  });
}

async function fetchVideoGraphFromServer(videoId: string): Promise<VideoNode[]> {
  const r = await fetch(`${getKgEngineUrl()}/video/${encodeURIComponent(videoId)}`);
  if (!r.ok) return [];
  const json = (await r.json()) as VideoDetailResponse;
  return scenesToVideoNodes(json.scenes ?? [], json.fps ?? 0);
}

function ingestSteps(progress: number) {
  return [
    { name: "Extracting keyframes...", percent: Math.min(progress, 25), note: "" },
    {
      name: "Understanding frames...",
      percent: Math.max(0, Math.min(progress - 25, 35)),
      note: "LLaVA via Ollama (server) describes each scene — objects, actions, setting, text in frame",
    },
    {
      name: "Tracking objects...",
      percent: Math.max(0, Math.min(progress - 60, 25)),
      note: "SAM2 tracking entities across frames",
    },
    {
      name: "Building knowledge graph...",
      percent: Math.max(0, Math.min(progress - 85, 10)),
      note: "Wiring temporal + spatial edges",
    },
    { name: "Finishing", percent: Math.max(0, Math.min(progress - 95, 5)), note: "" },
  ];
}

type ToolJobState = {
  toolName: string;
  phase: string;
  percent: number;
  done: boolean;
};

type PublishPlatform = "youtube" | "instagram" | "tiktok" | "linkedin";

type PublishAgent = {
  id: string;
  platform: PublishPlatform;
  label: string;
  connected: boolean;
  status: "idle" | "running" | "done" | "error";
  lastMessage?: string;
};

export default function VideoEditorApp() {
  const {
    videoId,
    videoUrl,
    duration,
    fps,
    resolution,
    currentTime,
    isPlaying,
    playbackSpeed,
    nodesAtTime,
    allNodes,
    selectedNode,
    messages,
    isProcessing,
    effects,
    library,
    activeLibraryId,
    setCurrentTime,
    setIsPlaying,
    setPlaybackSpeed,
    setAllNodes,
    selectNode,
    setMessages,
    addMessage,
    setIsProcessing,
    addEffect,
    addLibraryItem,
    patchLibraryItem,
    selectLibraryVideo,
    removeLibraryItem,
    clearAllVideos,
    dockToLibrary,
  } = useVideoStore();

  const videoRef = useRef<HTMLVideoElement | null>(null);
  const overlayRef = useRef<HTMLCanvasElement | null>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const addMoreVideosRef = useRef<HTMLInputElement | null>(null);
  const thinkingStartedRef = useRef<Set<string>>(new Set());

  const [showOverlay, setShowOverlay] = useState(true);
  const [visibleTypes, setVisibleTypes] = useState<Record<string, boolean>>({
    person: true,
    object: true,
    scene: true,
    text: true,
    effect: true,
    Flower: true,
  });
  const [messageInput, setMessageInput] = useState("");
  const [thinkingVisible, setThinkingVisible] = useState<Record<string, number>>({});
  const [ingestProgress, setIngestProgress] = useState(0);
  const [isIngesting, setIsIngesting] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [saveNote, setSaveNote] = useState<string | null>(null);
  const [toolJob, setToolJob] = useState<ToolJobState | null>(null);
  const [pendingGeneratedTool, setPendingGeneratedTool] = useState<string | null>(null);
  const [uploadingName, setUploadingName] = useState<string | null>(null);
  const [uploadIndex, setUploadIndex] = useState<{ current: number; total: number } | null>(null);
  const [publishAgents, setPublishAgents] = useState<PublishAgent[]>([
    { id: "yt", platform: "youtube", label: "YouTube / Shorts", connected: false, status: "idle" },
    { id: "ig", platform: "instagram", label: "Instagram Reels", connected: false, status: "idle" },
    { id: "tt", platform: "tiktok", label: "TikTok", connected: false, status: "idle" },
    { id: "li", platform: "linkedin", label: "LinkedIn Video", connected: false, status: "idle" },
  ]);
  const [dragOver, setDragOver] = useState(false);
  const [rightDockTab, setRightDockTab] = useState<RightDockTab>("chat");
  const [sidebarWidthPx, setSidebarWidthPx] = useState<number>(SIDEBAR_W.def);
  const [videoWidthPx, setVideoWidthPx] = useState<number>(VIDEO_W.def);
  const [graphPanelHidden, setGraphPanelHidden] = useState(false);
  const [sidePanelHidden, setSidePanelHidden] = useState(false);
  const [isLgViewport, setIsLgViewport] = useState(false);
  const [visionStatus, setVisionStatus] = useState<VideoVisionStatus | null>(null);

  const { nodes: graphNodes, edges: graphEdges } = useMemo(() => videoGraphFromNodes(allNodes), [allNodes]);

  /** Poll local LLaVA (Ollama) scene understanding; refreshes graph nodes from `GET /video/{id}`. */
  useEffect(() => {
    if (!videoId) {
      setVisionStatus(null);
      return;
    }
    let cancelled = false;
    let timeoutId: number | undefined;
    const tick = async () => {
      if (cancelled) return;
      try {
        const stRes = await fetch(`${getKgEngineUrl()}/video/${encodeURIComponent(videoId)}/status`);
        if (!stRes.ok) {
          setVisionStatus(null);
          timeoutId = window.setTimeout(tick, 4000);
          return;
        }
        const status = (await stRes.json()) as VideoVisionStatus;
        setVisionStatus(status);
        const nodes = await fetchVideoGraphFromServer(videoId);
        const { library } = useVideoStore.getState();
        const row = library.find((e) => e.videoId === videoId);
        if (row) {
          useVideoStore.getState().patchLibraryItem(row.id, { allNodes: nodes });
        }
        if (status.done) return;
      } catch {
        setVisionStatus(null);
      }
      timeoutId = window.setTimeout(tick, 2500);
    };
    void tick();
    return () => {
      cancelled = true;
      if (timeoutId !== undefined) window.clearTimeout(timeoutId);
    };
  }, [videoId]);

  useEffect(() => {
    const mq = window.matchMedia("(min-width: 1024px)");
    const apply = () => setIsLgViewport(mq.matches);
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(LAYOUT_LS);
      if (!raw) return;
      const o = JSON.parse(raw) as { s?: number; v?: number };
      if (typeof o.s === "number") setSidebarWidthPx(clamp(o.s, SIDEBAR_W.min, SIDEBAR_W.max));
      if (typeof o.v === "number") setVideoWidthPx(clamp(o.v, VIDEO_W.min, VIDEO_W.max));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_LS, JSON.stringify({ s: sidebarWidthPx, v: videoWidthPx }));
    } catch {
      /* ignore */
    }
  }, [sidebarWidthPx, videoWidthPx]);

  const startSidebarWidthDrag = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidthPx;
    const onMove = (ev: MouseEvent) => {
      setSidebarWidthPx(clamp(startW + (startX - ev.clientX), SIDEBAR_W.min, SIDEBAR_W.max));
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [sidebarWidthPx]);

  const startVideoWidthDrag = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = videoWidthPx;
    const onMove = (ev: MouseEvent) => {
      // Dragging the handle right moves the split right → video column narrows (graph gains space).
      setVideoWidthPx(clamp(startW + (startX - ev.clientX), VIDEO_W.min, VIDEO_W.max));
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [videoWidthPx]);

  useEffect(() => {
    if (rightDockTab !== "chat") return;
    const el = messagesRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [rightDockTab, messages, isProcessing]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    video.playbackRate = playbackSpeed;
    if (isPlaying) void video.play();
    else video.pause();
  }, [isPlaying, playbackSpeed]);

  useEffect(() => {
    const id = window.setInterval(() => {
      const video = videoRef.current;
      if (!video || !isPlaying) return;
      setCurrentTime(video.currentTime);
    }, 60);
    return () => window.clearInterval(id);
  }, [isPlaying, setCurrentTime]);

  useEffect(() => {
    const cnv = overlayRef.current;
    const video = videoRef.current;
    if (!cnv || !video) return;
    const ctx = cnv.getContext("2d");
    if (!ctx) return;

    cnv.width = video.clientWidth;
    cnv.height = video.clientHeight;
    ctx.clearRect(0, 0, cnv.width, cnv.height);
    if (!showOverlay) return;

    nodesAtTime
      .filter((node) => node.bbox && visibleTypes[node.entity_type])
      .forEach((node) => {
        if (!node.bbox) return;
        const sx = cnv.width / 1920;
        const sy = cnv.height / 1080;
        const x = node.bbox.x * sx;
        const y = node.bbox.y * sy;
        const w = node.bbox.w * sx;
        const h = node.bbox.h * sy;
        ctx.strokeStyle = ENTITY_COLORS[node.entity_type] ?? "#ffffff";
        ctx.lineWidth = selectedNode?.id === node.id ? 3 : 2;
        ctx.strokeRect(x, y, w, h);
        ctx.fillStyle = ctx.strokeStyle;
        ctx.font = "12px var(--font-geist-mono), ui-monospace, monospace";
        ctx.fillText(node.label, x, Math.max(y - 8, 12));
      });
  }, [nodesAtTime, selectedNode, showOverlay, visibleTypes, currentTime]);

  useEffect(() => {
    messages.forEach((msg) => {
      if (msg.role !== "assistant" || !msg.thinking?.length) return;
      if (thinkingStartedRef.current.has(msg.id)) return;
      thinkingStartedRef.current.add(msg.id);
      let step = 0;
      const timer = window.setInterval(() => {
        step += 1;
        setThinkingVisible((prev) => ({ ...prev, [msg.id]: Math.min(step, msg.thinking!.length) }));
        if (step >= msg.thinking!.length) window.clearInterval(timer);
      }, 300);
    });
  }, [messages]);

  useEffect(() => {
    if (messagesRef.current) messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
  }, [messages, isProcessing]);

  useEffect(() => {
    const el = composerRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 220)}px`;
  }, [messageInput]);

  const onGraphSelect = useCallback(
    (s: SelectedNode | null) => {
      if (!s) {
        selectNode(null);
        return;
      }
      const vn = allNodes.find((n) => n.id === s.node.id);
      if (vn) {
        selectNode(vn);
        setCurrentTime(vn.time_start);
        if (videoRef.current) videoRef.current.currentTime = vn.time_start;
      }
    },
    [allNodes, selectNode, setCurrentTime],
  );

  const ingestFileCore = async (file: File) => {
    setIngestProgress(5);
    const pulse = window.setInterval(() => {
      setIngestProgress((prev) => Math.min(prev + 3, 97));
    }, 350);
    const objectUrl = URL.createObjectURL(file);
    const local = await probeVideoMetadata(objectUrl);
    const fallbackDuration = local.duration > 0 ? local.duration : 0;
    const fallbackResolution = formatResolution(local.width, local.height);
    const fallbackFps = 30;

    const revokeAndFail = (message: string) => {
      try {
        URL.revokeObjectURL(objectUrl);
      } catch {
        /* ignore */
      }
      setUploadError(message);
      setIngestProgress(0);
    };

    try {
      const fd = new FormData();
      fd.append("file", file);
      const resp = await fetch(`${getKgEngineUrl()}/ingest/video`, { method: "POST", body: fd });
      if (!resp.ok) {
        const detail = await resp.text().catch(() => "");
        revokeAndFail(
          detail
            ? `Ingest failed (HTTP ${resp.status}): ${detail.slice(0, 400)}`
            : `Ingest failed: server returned HTTP ${resp.status}. Is kg-engine running at ${getKgEngineUrl()}?`,
        );
        return;
      }

      const json = (await resp.json()) as VideoIngestPostResponse;
      const vid = json.video_id;
      if (!vid) {
        revokeAndFail("Ingest response missing video_id.");
        return;
      }

      const graphNodes = await fetchVideoGraphFromServer(vid);
      if (graphNodes.length === 0) {
        setUploadError(
          `Uploaded as ${vid}, but GET /video/${vid} returned no scenes. Graph may populate after a short delay.`,
        );
      } else {
        setUploadError(null);
      }

      const dur = json.duration > 0 ? json.duration : fallbackDuration;
      const fps = json.fps > 0 ? json.fps : fallbackFps;
      const res = json.resolution || fallbackResolution;

      addLibraryItem({
        fileName: file.name,
        videoId: vid,
        videoUrl: objectUrl,
        duration: dur > 0 ? dur : Math.max(fallbackDuration, 1),
        fps,
        resolution: res !== "—" && res ? res : fallbackResolution,
        allNodes: graphNodes,
        messages: [],
        effects: [],
        currentTime: 0,
        isPlaying: false,
        playbackSpeed: 1,
      });
      setIngestProgress(100);
    } catch (e) {
      try {
        URL.revokeObjectURL(objectUrl);
      } catch {
        /* ignore */
      }
      const msg = e instanceof Error ? e.message : String(e);
      setUploadError(`Cannot reach ${getKgEngineUrl()} or ingest failed: ${msg}`);
      setIngestProgress(0);
    } finally {
      window.clearInterval(pulse);
    }
  };

  const onUploadFiles = async (files: FileList | File[]) => {
    const list = Array.from(files).filter((f) => f.size > 0);
    if (list.length === 0) return;
    setUploadError(null);
    setUploadIndex({ current: 0, total: list.length });
    setIsIngesting(true);
    try {
      for (let i = 0; i < list.length; i += 1) {
        const file = list[i]!;
        setUploadIndex({ current: i + 1, total: list.length });
        setUploadingName(file.name);
        await ingestFileCore(file);
      }
    } finally {
      setIsIngesting(false);
      setUploadingName(null);
      setUploadIndex(null);
      setIngestProgress(0);
    }
  };

  const runMockToolJob = useCallback((toolName: string, onComplete: () => void) => {
    const phases: { phase: string; percent: number }[] = [
      { phase: "Writing spec", percent: 22 },
      { phase: "Generating", percent: 58 },
      { phase: "Validating", percent: 86 },
      { phase: "Ready", percent: 100 },
    ];
    setToolJob({ toolName, phase: phases[0].phase, percent: 0, done: false });
    let i = 0;
    const step = () => {
      if (i >= phases.length) {
        setToolJob((prev) => (prev ? { ...prev, done: true, percent: 100 } : null));
        onComplete();
        return;
      }
      const p = phases[i];
      i += 1;
      setToolJob({ toolName, phase: p.phase, percent: p.percent, done: false });
      window.setTimeout(step, 720);
    };
    window.setTimeout(step, 200);
  }, []);

  const sendChat = async () => {
    const instruction = messageInput.trim();
    if (!instruction || !videoId) return;
    addMessage({ id: `u-${Date.now()}`, role: "user", content: instruction });
    setMessageInput("");
    setIsProcessing(true);

    if (instruction.toLowerCase().includes("film grain")) {
      runMockToolJob("film_grain", () => {
        setPendingGeneratedTool("film_grain");
        setRightDockTab("chat");
        addMessage({
          id: `tool-${Date.now()}`,
          role: "assistant",
          content:
            "Tool 'film_grain' was not in the registry — a new edit tool was generated. Approve to register it, or reject to roll back.",
          type: "tool-generated",
          thinking: ["Writing spec →", "Generating →", "Validating →", "Ready"],
        });
        setIsProcessing(false);
      });
      return;
    }

    const history = kgChatHistoryFromMessages(useVideoStore.getState().messages);
    try {
      const resp = await fetch(`${getKgEngineUrl()}/chat`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ question: instruction, history }),
      });
      const text = await resp.text();
      if (!resp.ok) {
        addMessage({
          id: `a-${Date.now()}`,
          role: "assistant",
          content: `Could not reach the model (HTTP ${resp.status}). ${text.slice(0, 280)}`,
        });
        setIsProcessing(false);
        return;
      }
      let answer: string;
      try {
        const json = JSON.parse(text) as { answer?: string };
        answer = typeof json.answer === "string" ? json.answer : text;
      } catch {
        answer = text;
      }
      addMessage({ id: `a-${Date.now()}`, role: "assistant", content: answer });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      addMessage({
        id: `a-${Date.now()}`,
        role: "assistant",
        content: `Network error talking to kg-engine at ${getKgEngineUrl()}: ${msg}`,
      });
    }
    setIsProcessing(false);
  };

  const saveVideo = () => {
    if (!videoUrl) return;
    setSaveNote(null);
    const a = document.createElement("a");
    a.href = videoUrl;
    a.download = `${videoId ?? "video"}.mp4`;
    a.rel = "noopener";
    document.body.appendChild(a);
    a.click();
    a.remove();
    setSaveNote("Download started (browser may rename cross-origin sources).");
    window.setTimeout(() => setSaveNote(null), 4000);
  };

  const saveSnapshot = () => {
    const payload = {
      saved_at: new Date().toISOString(),
      video_id: videoId,
      current_time: currentTime,
      duration,
      fps,
      resolution,
      selected_node_id: selectedNode?.id ?? null,
      nodes: allNodes,
      messages,
      effects,
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `video-snapshot-${videoId ?? "draft"}-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setSaveNote("Snapshot JSON saved.");
    window.setTimeout(() => setSaveNote(null), 4000);
  };

  const toggleAgentConnected = useCallback((agentId: string) => {
    setPublishAgents((prev) =>
      prev.map((a) => (a.id === agentId ? { ...a, connected: !a.connected, status: "idle" as const } : a)),
    );
  }, []);

  const runPublishAgent = useCallback((agent: PublishAgent) => {
    if (!agent.connected) {
      addMessage({
        id: `sys-${Date.now()}`,
        role: "assistant",
        content: `Connect **${agent.label}** in Publish agents first (OAuth would run server-side in production).`,
      });
      return;
    }
    setPublishAgents((prev) =>
      prev.map((a) => (a.id === agent.id ? { ...a, status: "running" as const, lastMessage: undefined } : a)),
    );
    window.setTimeout(() => {
      const { videoId: rootVid, library: lib, activeLibraryId: active } = useVideoStore.getState();
      const resolvedId = rootVid ?? lib.find((e) => e.id === active)?.videoId ?? null;
      const ok = Math.random() > 0.08;
      setPublishAgents((prev) =>
        prev.map((a) =>
          a.id === agent.id
            ? {
                ...a,
                status: ok ? ("done" as const) : ("error" as const),
                lastMessage: ok
                  ? `Uploaded draft for "${resolvedId ?? "clip"}" (mock).`
                  : "Rate limit / token expired (mock).",
              }
            : a,
        ),
      );
      addMessage({
        id: `pub-${Date.now()}`,
        role: "assistant",
        content: ok
          ? `Publish agent finished for **${agent.label}** — ${resolvedId ? `asset \`${resolvedId}\`` : "video"} queued (mock).`
          : `Publish agent failed on **${agent.label}**. Retry after refreshing the connection.`,
      });
      window.setTimeout(() => {
        setPublishAgents((prev) =>
          prev.map((a) => (a.id === agent.id && a.status !== "running" ? { ...a, status: "idle" as const } : a)),
        );
      }, 4000);
    }, 2200 + Math.random() * 800);
  }, [addMessage]);

  if (!videoUrl) {
    return (
      <main className="flex min-h-screen flex-col bg-zinc-950 pt-14 text-zinc-100 selection:bg-violet-500/30">
        <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,rgba(139,92,246,0.12),transparent)]" />
        <header className="fixed left-0 right-0 top-0 z-50 flex h-14 items-center justify-between border-b border-white/[0.06] bg-zinc-950/80 px-4 backdrop-blur-xl">
          <div className="flex items-center gap-2 text-sm">
            <Link href="/product" className="font-semibold tracking-tight text-white transition hover:text-violet-200">
              Fluvio
            </Link>
            <span className="text-zinc-600">/</span>
            <span className="text-zinc-400">Video</span>
          </div>
          <Link
            href="/workspace"
            className="rounded-full border border-white/[0.08] px-4 py-2 text-xs font-medium text-zinc-300 transition hover:border-white/[0.14] hover:bg-white/[0.04] hover:text-white"
          >
            Workspace
          </Link>
        </header>

        <section className="relative z-10 flex flex-1 flex-col items-center px-4 pb-16 pt-8 sm:px-6">
          {library.length > 0 && (
            <div className="mb-10 w-full max-w-5xl">
              <div className="mb-4 flex items-end justify-between gap-4">
                <div>
                  <h2 className="text-lg font-semibold tracking-tight text-white">Library</h2>
                  <p className="mt-1 text-sm text-zinc-500">{library.length} clip{library.length === 1 ? "" : "s"}</p>
                </div>
                <button
                  type="button"
                  onClick={() => clearAllVideos()}
                  className="rounded-full px-3 py-1.5 text-xs font-medium text-zinc-500 transition hover:bg-rose-500/10 hover:text-rose-300"
                >
                  Clear library
                </button>
              </div>
              <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {library.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => selectLibraryVideo(item.id)}
                      className="group w-full overflow-hidden rounded-2xl bg-zinc-900/50 text-left ring-1 ring-white/[0.06] transition duration-200 hover:-translate-y-0.5 hover:ring-violet-500/25"
                    >
                      <div className="relative aspect-video bg-black">
                        <video
                          src={item.videoUrl}
                          className="h-full w-full object-cover opacity-90 transition group-hover:opacity-100"
                          muted
                          playsInline
                          preload="metadata"
                        />
                        <span className="absolute bottom-2 right-2 rounded-md bg-black/60 px-2 py-0.5 text-[10px] font-medium tabular-nums text-zinc-200 backdrop-blur-sm">
                          {item.resolution}
                        </span>
                      </div>
                      <div className="p-3">
                        <p className="truncate text-sm font-medium text-white" title={item.fileName}>
                          {item.fileName}
                        </p>
                        <p className="mt-1 truncate font-mono text-[11px] text-zinc-500">{item.videoId}</p>
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div
            className={`w-full max-w-lg rounded-3xl p-px transition ${
              dragOver ? "bg-gradient-to-b from-violet-500/50 to-fuchsia-500/30" : "bg-gradient-to-b from-white/[0.12] to-white/[0.04]"
            }`}
            onDragEnter={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={(e) => {
              e.preventDefault();
              if (!e.currentTarget.contains(e.relatedTarget as Node)) setDragOver(false);
            }}
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              e.preventDefault();
              setDragOver(false);
              const dropped = e.dataTransfer.files;
              if (dropped?.length) void onUploadFiles(dropped);
            }}
          >
            <div className="flex flex-col items-center rounded-[1.4rem] bg-zinc-950/95 px-8 py-12 text-center sm:px-12 sm:py-14">
              <input
                id="video-hub-multi-input"
                type="file"
                multiple
                accept=".mp4,.mov,.avi,.webm,video/mp4,video/webm"
                className="sr-only"
                onChange={(e) => {
                  const fl = e.target.files;
                  if (fl?.length) void onUploadFiles(fl);
                  e.target.value = "";
                }}
              />
              <p className="text-xs font-medium uppercase tracking-[0.2em] text-violet-400/90">Import</p>
              <h1 className="mt-3 max-w-sm text-balance text-2xl font-semibold tracking-tight text-white sm:text-3xl">
                Add your videos
              </h1>
              <p className="mt-3 max-w-sm text-pretty text-sm leading-relaxed text-zinc-400">
                Drag files here or browse. Each file is sent to <span className="font-mono text-zinc-500">POST /ingest/video</span>{" "}
                on <span className="font-mono text-zinc-500">{getKgEngineUrl()}</span>; playback uses your local blob until the clip
                is added. Scene nodes load from <span className="font-mono text-zinc-500">GET /video/{"{id}"}</span>; local LLaVA
                progress is on <span className="font-mono text-zinc-500">GET /video/{"{id}"}/status</span> (Ollama{" "}
                <span className="font-mono text-zinc-500">OLLAMA_URL</span> on the server).
              </p>
              <label htmlFor="video-hub-multi-input">
                <span className="mt-8 inline-flex cursor-pointer rounded-full bg-white px-8 py-3.5 text-sm font-semibold text-zinc-950 shadow-lg shadow-black/20 transition hover:bg-zinc-100 active:scale-[0.98]">
                  Choose files
                </span>
              </label>
              <p className="mt-4 text-xs text-zinc-600">MP4 · MOV · WebM · AVI</p>
              {uploadError && (
                <p className="mt-6 max-w-sm text-pretty rounded-2xl border border-amber-500/20 bg-amber-500/5 px-4 py-3 text-left text-xs leading-relaxed text-amber-200/90">
                  {uploadError}
                </p>
              )}
              {uploadIndex && (
                <p className="mt-4 text-xs text-zinc-500">
                  File {uploadIndex.current} of {uploadIndex.total}
                  {uploadingName ? (
                    <>
                      {" · "}
                      <span className="font-medium text-zinc-300">{uploadingName}</span>
                    </>
                  ) : null}
                </p>
              )}
              {isIngesting && (
                <div className="mt-8 w-full max-w-sm space-y-3 text-left">
                  {ingestSteps(ingestProgress).map((step) => (
                    <div key={step.name} className="rounded-xl bg-zinc-900/80 px-4 py-3 ring-1 ring-white/[0.05]">
                      <div className="text-xs font-medium text-zinc-200">{step.name}</div>
                      <div className="mt-2 h-1 overflow-hidden rounded-full bg-zinc-800">
                        <div
                          className="h-full rounded-full bg-gradient-to-r from-violet-500 to-fuchsia-500 transition-all duration-300"
                          style={{ width: `${Math.min(step.percent * 4, 100)}%` }}
                        />
                      </div>
                      {step.note ? <div className="mt-2 text-[11px] leading-snug text-zinc-500">{step.note}</div> : null}
                    </div>
                  ))}
                </div>
              )}
              <p className="mt-10 font-mono text-[10px] text-zinc-600">
                API <span className="text-zinc-500">{getKgEngineUrl()}</span>
              </p>
            </div>
          </div>
        </section>
      </main>
    );
  }

  return (
    <main className="relative h-screen w-screen overflow-hidden bg-zinc-950 pt-14 text-zinc-100 selection:bg-violet-500/30">
      <header className="fixed left-0 right-0 top-0 z-50 flex h-14 items-center gap-3 border-b border-white/[0.06] bg-zinc-950/90 px-3 backdrop-blur-xl sm:px-4">
        <div className="flex min-w-0 shrink-0 items-center gap-2 text-xs sm:text-sm">
          <Link href="/product" className="shrink-0 font-semibold text-white transition hover:text-violet-200">
            Fluvio
          </Link>
          <span className="text-zinc-600">/</span>
          <Link href="/workspace" className="hidden text-zinc-500 transition hover:text-zinc-300 sm:inline">
            Workspace
          </Link>
          <span className="hidden text-zinc-600 sm:inline">/</span>
          <span className="truncate text-zinc-400">Video</span>
        </div>

        <input
          ref={addMoreVideosRef}
          type="file"
          multiple
          accept=".mp4,.mov,.avi,.webm,video/mp4,video/webm"
          className="sr-only"
          onChange={(e) => {
            const fl = e.target.files;
            if (fl?.length) void onUploadFiles(fl);
            e.target.value = "";
          }}
        />

        <div className="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto py-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          <button
            type="button"
            title="Library & upload"
            onClick={() => dockToLibrary()}
            className="shrink-0 rounded-full border border-white/[0.08] bg-zinc-900/80 px-3 py-1.5 text-xs font-medium text-zinc-300 transition hover:border-white/[0.12] hover:bg-zinc-800 hover:text-white"
          >
            Library
          </button>
          {library.map((item) => (
            <div key={item.id} className="flex shrink-0 items-stretch rounded-full ring-1 ring-white/[0.06]">
              <button
                type="button"
                title={item.fileName}
                onClick={() => selectLibraryVideo(item.id)}
                className={`max-w-[120px] truncate rounded-l-full px-3 py-1.5 text-xs font-medium transition sm:max-w-[200px] ${
                  item.id === activeLibraryId
                    ? "bg-white/[0.1] text-white"
                    : "text-zinc-400 hover:bg-white/[0.05] hover:text-zinc-200"
                }`}
              >
                {item.fileName}
              </button>
              <button
                type="button"
                title="Remove"
                onClick={() => removeLibraryItem(item.id)}
                className="rounded-r-full px-2 py-1.5 text-xs text-zinc-500 transition hover:bg-rose-500/15 hover:text-rose-300"
              >
                ×
              </button>
            </div>
          ))}
          <button
            type="button"
            title="Add videos"
            onClick={() => addMoreVideosRef.current?.click()}
            className="shrink-0 rounded-full border border-dashed border-white/15 px-3 py-1.5 text-xs font-medium text-zinc-500 transition hover:border-violet-400/40 hover:text-violet-200"
          >
            + Add
          </button>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          <button
            type="button"
            title={graphPanelHidden ? "Show graph" : "Hide graph"}
            onClick={() => setGraphPanelHidden((v) => !v)}
            className={`inline-flex rounded-full border px-2.5 py-1.5 text-[10px] font-medium transition ${
              graphPanelHidden
                ? "border-violet-500/40 bg-violet-500/15 text-violet-200"
                : "border-white/[0.1] text-zinc-400 hover:bg-white/[0.06] hover:text-zinc-200"
            }`}
          >
            Graph
          </button>
          <button
            type="button"
            title={sidePanelHidden ? "Show side panel" : "Hide side panel"}
            onClick={() => setSidePanelHidden((v) => !v)}
            className={`inline-flex rounded-full border px-2.5 py-1.5 text-[10px] font-medium transition ${
              sidePanelHidden
                ? "border-violet-500/40 bg-violet-500/15 text-violet-200"
                : "border-white/[0.1] text-zinc-400 hover:bg-white/[0.06] hover:text-zinc-200"
            }`}
          >
            Panel
          </button>
          {saveNote && (
            <span className="hidden max-w-[88px] truncate text-[10px] text-emerald-400/90 lg:inline">{saveNote}</span>
          )}
          <button
            type="button"
            onClick={saveVideo}
            className="rounded-full bg-white px-3 py-1.5 text-xs font-semibold text-zinc-950 transition hover:bg-zinc-100"
          >
            Export
          </button>
          <button
            type="button"
            onClick={saveSnapshot}
            className="hidden rounded-full border border-white/[0.1] px-3 py-1.5 text-xs font-medium text-zinc-400 transition hover:bg-white/[0.05] hover:text-white sm:inline"
          >
            JSON
          </button>
        </div>
      </header>

      {graphPanelHidden && (
        <button
          type="button"
          onClick={() => setGraphPanelHidden(false)}
          className="fixed left-0 top-1/2 z-[60] flex -translate-y-1/2 rounded-r-lg border border-l-0 border-white/10 bg-zinc-900/95 py-4 pl-1 pr-1.5 text-[10px] font-semibold uppercase tracking-wide text-zinc-300 shadow-xl backdrop-blur-md hover:bg-zinc-800"
          title="Show graph"
        >
          Graph
        </button>
      )}
      {sidePanelHidden && (
        <button
          type="button"
          onClick={() => setSidePanelHidden(false)}
          className="fixed right-0 top-1/2 z-[60] flex -translate-y-1/2 rounded-l-lg border border-r-0 border-white/10 bg-zinc-900/95 py-4 pl-1.5 pr-1 text-[10px] font-semibold uppercase tracking-wide text-zinc-300 shadow-xl backdrop-blur-md hover:bg-zinc-800"
          title="Show panel"
        >
          Panel
        </button>
      )}

      <div className="relative flex h-[calc(100vh-3.5rem)] w-full min-h-0 flex-col lg:flex-row">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <div className="relative flex min-h-0 min-w-0 flex-1 flex-col lg:flex-row">
            {!graphPanelHidden && (
            <div className="relative min-h-[min(200px,36vh)] min-w-0 flex-1 overflow-hidden border-b border-white/[0.06] bg-zinc-950 lg:min-h-0 lg:min-w-[160px] lg:border-b-0 lg:border-r lg:border-white/[0.06]">
              {graphNodes.length > 0 ? (
                <div className="pointer-events-none absolute left-1/2 top-3 z-10 flex max-w-[min(92vw,520px)] -translate-x-1/2 flex-col items-center gap-1 rounded-2xl border border-white/[0.08] bg-zinc-900/90 px-4 py-2 text-[11px] font-medium text-zinc-500 shadow-lg backdrop-blur-md">
                  <div className="flex flex-wrap items-center justify-center gap-3">
                    <span className="text-zinc-400">Temporal graph</span>
                    <span className="h-1 w-1 rounded-full bg-zinc-600" />
                    <span>
                      <span className="tabular-nums text-zinc-200">{graphNodes.length}</span> nodes ·{" "}
                      <span className="tabular-nums text-zinc-200">{graphEdges.length}</span> edges
                    </span>
                  </div>
                  {visionStatus && visionStatus.total > 0 ? (
                    <div className="w-full min-w-[200px] max-w-sm border-t border-white/[0.06] pt-1.5 text-center">
                      <p className="text-[10px] leading-snug text-zinc-500">
                        LLaVA (Ollama on server:{" "}
                        <span className="font-mono text-zinc-400">OLLAMA_URL</span>) —{" "}
                        <span className="tabular-nums text-zinc-300">{visionStatus.complete}</span>/
                        <span className="tabular-nums text-zinc-400">{visionStatus.total}</span> scenes described
                        {(visionStatus.failed ?? 0) > 0 ? (
                          <>
                            {" · "}
                            <span className="tabular-nums text-amber-400/90">{visionStatus.failed}</span> failed
                          </>
                        ) : null}
                        {visionStatus.done ? (
                          <span className="text-emerald-400/90"> · done</span>
                        ) : (
                          <>
                            {" · "}
                            <span className="tabular-nums text-zinc-400">
                              {visionStatus.processed_percent ?? visionStatus.percent}%
                            </span>{" "}
                            processed
                          </>
                        )}
                      </p>
                      {!visionStatus.done ? (
                        <div className="mx-auto mt-1.5 h-1 w-full max-w-[220px] overflow-hidden rounded-full bg-zinc-800">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-violet-500 to-sky-500 transition-all duration-300"
                            style={{
                              width: `${Math.min(100, visionStatus.processed_percent ?? visionStatus.percent)}%`,
                            }}
                          />
                        </div>
                      ) : null}
                      <p className="mt-1 font-mono text-[9px] text-zinc-600">
                        Track: GET {getKgEngineUrl()}/video/{"{"}id{"}"}/status
                      </p>
                    </div>
                  ) : null}
                </div>
              ) : (
                <div className="pointer-events-none absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 px-6 text-center">
                  <p className="text-sm font-medium text-zinc-400">No graph yet</p>
                  <p className="max-w-xs text-xs leading-relaxed text-zinc-600">
                    After a successful ingest, scenes from the server appear here. Playback still uses your uploaded file
                    in the browser.
                  </p>
                </div>
              )}
              <GraphErrorBoundary>
                <GraphCanvas
                  key={`video-graph-${graphNodes.length}-${graphEdges.length}`}
                  svgRef={svgRef}
                  nodes={graphNodes}
                  edges={graphEdges}
                  onSelect={onGraphSelect}
                />
              </GraphErrorBoundary>
              {selectedNode && (
                <div
                  className="pointer-events-auto absolute bottom-3 left-3 right-3 z-20 max-h-[38%] overflow-y-auto rounded-2xl border border-white/[0.08] bg-zinc-900/95 p-3 shadow-2xl ring-1 ring-black/40 backdrop-blur-xl sm:left-4 sm:right-auto sm:max-w-sm"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div className="mb-1 flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className="h-2 w-2 rounded-full bg-sky-400" />
                      <span className="text-[12px] font-medium text-zinc-300">{selectedNode.label}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => selectNode(null)}
                      className="rounded-full px-2 py-0.5 text-[13px] text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-200"
                    >
                      ✕
                    </button>
                  </div>
                  <p className="font-mono text-[10px] text-sky-500/80">{selectedNode.source_uri}</p>
                  <p className="mt-2 text-[12px] leading-relaxed text-zinc-400">{selectedNode.semantic}</p>
                  {selectedNode.edges.length > 0 && (
                    <div className="mt-2 border-t border-white/[0.06] pt-2">
                      <p className="mb-1 text-[11px] font-medium uppercase tracking-wide text-zinc-600">Edges</p>
                      <ul className="space-y-1">
                        {selectedNode.edges.map((e, i) => (
                          <li key={i} className="flex justify-between gap-2 font-mono text-[10px]">
                            <span className="truncate text-zinc-500">{e.label}</span>
                            <span className="shrink-0 text-zinc-600">{Math.round(e.prob * 100)}%</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}
            </div>
            )}

            {!graphPanelHidden && (
              <div
                role="separator"
                aria-orientation="vertical"
                aria-label="Resize graph and preview"
                className="hidden w-2 shrink-0 cursor-col-resize items-stretch border-l border-r border-transparent bg-zinc-950 hover:border-violet-500/20 hover:bg-violet-500/5 lg:flex"
                onMouseDown={startVideoWidthDrag}
              >
                <div className="mx-auto my-auto h-10 w-px rounded-full bg-zinc-600" />
              </div>
            )}

            <aside
              className={`relative flex min-h-[220px] flex-col border-t border-white/[0.06] bg-zinc-950 lg:h-full lg:min-h-0 lg:border-l lg:border-t-0 ${
                graphPanelHidden ? "min-h-0 w-full flex-1 lg:min-w-0" : "w-full lg:shrink-0"
              }`}
              style={
                isLgViewport && !graphPanelHidden
                  ? {
                      width: videoWidthPx,
                      minWidth: VIDEO_W.min,
                      maxWidth: VIDEO_W.max,
                    }
                  : undefined
              }
            >
              <div className="border-b border-white/[0.06] px-4 py-4">
                <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-violet-400/90">Preview</p>
                <p className="mt-1 text-sm font-medium text-white">Timeline</p>
                <p className="mt-1 text-xs leading-relaxed text-zinc-500">
                  Scrub and review. Ask the assistant on the right for edits.
                </p>
              </div>
              <div className="space-y-2 border-b border-white/[0.06] px-4 py-3 text-xs text-zinc-500">
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  <span className="text-zinc-600">ID</span>
                  <span className="truncate font-mono text-[11px] text-zinc-300">{videoId}</span>
                </div>
                <p className="tabular-nums text-zinc-400">
                  {formatTime(currentTime)} / {formatTime(duration)} · {fps} fps · {resolution}
                </p>
              </div>

              {toolJob && !pendingGeneratedTool && (
                <div className="border-b border-white/[0.06] bg-zinc-900/60 px-4 py-2 text-[11px] text-zinc-300">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-semibold uppercase tracking-wide text-zinc-500">Generating tool</span>
                    <span className="tabular-nums text-zinc-500">{Math.round(toolJob.percent)}%</span>
                  </div>
                  <p className="mt-1">
                    <span className="font-mono text-sky-400/80">{toolJob.toolName}</span>
                    {" — "}
                    {toolJob.phase}
                    {toolJob.done ? " · complete" : ""}
                  </p>
                  <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
                    <div
                      className="h-full rounded-full bg-sky-400 transition-[width] duration-300"
                      style={{ width: `${Math.min(100, toolJob.percent)}%` }}
                    />
                  </div>
                </div>
              )}

              <div className="relative min-h-0 flex-1 p-3">
                <div className="relative h-full min-h-[180px] overflow-hidden rounded-2xl bg-black ring-1 ring-white/[0.08]">
                  <video
                    ref={videoRef}
                    src={videoUrl}
                    className="h-full w-full object-contain"
                    onTimeUpdate={(e) => setCurrentTime(e.currentTarget.currentTime)}
                    onLoadedMetadata={(e) => setCurrentTime(e.currentTarget.currentTime)}
                  />
                  <canvas ref={overlayRef} className="pointer-events-none absolute inset-0 h-full w-full" />
                </div>
              </div>

              <div className="shrink-0 space-y-3 border-t border-white/[0.06] px-3 py-3">
                <div className="flex flex-wrap items-center gap-2">
                  <button
                    type="button"
                    onClick={() => setIsPlaying(!isPlaying)}
                    className="rounded-full bg-white px-4 py-2 text-xs font-semibold text-zinc-950 transition hover:bg-zinc-100"
                  >
                    {isPlaying ? "Pause" : "Play"}
                  </button>
                  <span className="font-mono text-[11px] tabular-nums text-zinc-500">{formatTime(currentTime)}</span>
                  {[0.25, 0.5, 1, 2, 4].map((speed) => (
                    <button
                      key={speed}
                      type="button"
                      onClick={() => setPlaybackSpeed(speed)}
                      className={`rounded-full px-2.5 py-1 text-[11px] font-medium transition ${
                        playbackSpeed === speed
                          ? "bg-violet-500/20 text-violet-200 ring-1 ring-violet-500/30"
                          : "text-zinc-500 hover:bg-white/[0.06] hover:text-zinc-300"
                      }`}
                    >
                      {speed}x
                    </button>
                  ))}
                  <input
                    type="range"
                    min={0}
                    max={Math.max(duration || 0, 0.01)}
                    step={0.01}
                    value={currentTime}
                    onChange={(e) => {
                      const t = Number(e.target.value);
                      setCurrentTime(t);
                      if (videoRef.current) videoRef.current.currentTime = t;
                    }}
                    className="ml-auto min-w-[100px] max-w-[200px] flex-1 accent-violet-500"
                    aria-label="Seek"
                  />
                </div>
                <div className="flex flex-wrap gap-1.5">
                  <button
                    type="button"
                    onClick={() => setShowOverlay((v) => !v)}
                    className="rounded-full border border-white/[0.08] px-3 py-1 text-[11px] text-zinc-400 transition hover:bg-white/[0.05]"
                  >
                    {showOverlay ? "Hide overlays" : "Show overlays"}
                  </button>
                  {Object.keys(ENTITY_COLORS).map((type) => (
                    <button
                      key={type}
                      type="button"
                      onClick={() => setVisibleTypes((prev) => ({ ...prev, [type]: !prev[type] }))}
                      className={`rounded-full border px-2.5 py-1 text-[11px] transition ${
                        visibleTypes[type]
                          ? "border-transparent bg-white/[0.08]"
                          : "border-white/[0.06] text-zinc-600 hover:text-zinc-400"
                      }`}
                      style={{ color: visibleTypes[type] ? ENTITY_COLORS[type] : undefined }}
                    >
                      {type}
                    </button>
                  ))}
                </div>
              </div>
            </aside>
          </div>
        </div>

        {!sidePanelHidden && (
          <>
            <div
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize editor and side panel"
              className="hidden w-2 shrink-0 cursor-col-resize items-stretch border-l border-r border-transparent bg-zinc-950 hover:border-violet-500/25 hover:bg-violet-500/5 lg:flex"
              onMouseDown={startSidebarWidthDrag}
            >
              <div className="mx-auto my-auto h-14 w-px rounded-full bg-zinc-600/90" />
            </div>
            <aside
              className="flex max-h-[min(520px,58vh)] w-full shrink-0 flex-col overflow-hidden border-t border-white/[0.06] bg-zinc-950 lg:h-full lg:max-h-none lg:min-h-0 lg:border-l lg:border-t-0 lg:shrink-0"
              style={
                isLgViewport
                  ? { width: sidebarWidthPx, minWidth: SIDEBAR_W.min, maxWidth: SIDEBAR_W.max }
                  : undefined
              }
            >
              <div className="flex shrink-0 gap-0.5 border-b border-white/[0.06] bg-zinc-950/98 px-2 pt-2">
                {(["chat", "agents", "files"] as const).map((tab) => (
                  <button
                    key={tab}
                    type="button"
                    onClick={() => setRightDockTab(tab)}
                    className={`min-w-0 flex-1 truncate rounded-t-lg px-1.5 py-2.5 text-xs font-medium transition sm:px-2 ${
                      rightDockTab === tab
                        ? "bg-zinc-900 text-white ring-1 ring-white/[0.08] ring-b-0"
                        : "text-zinc-500 hover:text-zinc-300"
                    }`}
                  >
                    {tab === "chat" ? "Chat" : tab === "agents" ? "Agents" : "Files"}
                  </button>
                ))}
              </div>

              <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                {rightDockTab === "agents" && (
                  <div className="min-h-0 flex-1 overflow-y-auto p-4">
                    <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Publishing</p>
                    <p className="mt-1 text-xs leading-relaxed text-zinc-600">
                      Connect channels, then queue an upload of the current clip (simulated).
                    </p>
                    <ul className="mt-4 space-y-2">
                      {publishAgents.map((agent) => (
                        <li
                          key={agent.id}
                          className="rounded-xl bg-zinc-900/60 px-3 py-2.5 ring-1 ring-white/[0.05]"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="min-w-0">
                              <div className="flex items-center gap-2">
                                {agent.status === "running" ? (
                                  <span
                                    className="inline-block size-3.5 shrink-0 animate-spin rounded-full border-2 border-violet-400 border-t-transparent"
                                    aria-hidden
                                  />
                                ) : (
                                  <span className="size-1.5 shrink-0 rounded-full bg-zinc-600" aria-hidden />
                                )}
                                <span className="truncate text-xs font-medium text-zinc-100">{agent.label}</span>
                              </div>
                              <p className="mt-1 pl-5 text-[10px] text-zinc-500">
                                {agent.connected ? (
                                  <span className="text-emerald-400/90">Connected</span>
                                ) : (
                                  "Not connected"
                                )}
                              </p>
                              {agent.lastMessage && agent.status !== "running" ? (
                                <p className="mt-1 pl-5 text-[10px] text-zinc-500">{agent.lastMessage}</p>
                              ) : null}
                            </div>
                          </div>
                          <div className="mt-2 flex flex-wrap gap-2 pl-5">
                            <button
                              type="button"
                              onClick={() => toggleAgentConnected(agent.id)}
                              className="rounded-full border border-white/[0.1] px-3 py-1 text-[10px] font-medium text-zinc-400 transition hover:bg-white/[0.06]"
                            >
                              {agent.connected ? "Disconnect" : "Connect"}
                            </button>
                            <button
                              type="button"
                              disabled={agent.status === "running"}
                              onClick={() => runPublishAgent(agent)}
                              className="rounded-full bg-white px-3 py-1 text-[10px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-40"
                            >
                              {agent.status === "running" ? "Working…" : "Queue"}
                            </button>
                          </div>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {rightDockTab === "files" && (
                  <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                    <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
                      <div>
                        <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Active</p>
                        <p className="mt-1 truncate text-sm font-medium text-white">
                          {library.find((x) => x.id === activeLibraryId)?.fileName ?? "—"}
                        </p>
                        <p className="mt-1 truncate font-mono text-[11px] text-zinc-500">{videoId}</p>
                      </div>
                      <button
                        type="button"
                        onClick={() => addMoreVideosRef.current?.click()}
                        className="w-full rounded-full bg-white py-2.5 text-xs font-semibold text-zinc-950 transition hover:bg-zinc-100"
                      >
                        Add videos…
                      </button>
                      <button
                        type="button"
                        onClick={() => dockToLibrary()}
                        className="w-full rounded-full border border-white/[0.1] py-2.5 text-xs font-medium text-zinc-300 transition hover:bg-white/[0.05]"
                      >
                        Open library hub
                      </button>
                      <div>
                        <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Clips</p>
                        <ul className="mt-2 space-y-1">
                          {library.map((item) => (
                            <li
                              key={item.id}
                              className="flex items-center justify-between gap-2 rounded-lg bg-zinc-900/50 px-2 py-1.5 text-xs ring-1 ring-white/[0.05]"
                            >
                              <button
                                type="button"
                                className="min-w-0 flex-1 truncate text-left text-zinc-200 hover:text-white"
                                onClick={() => selectLibraryVideo(item.id)}
                              >
                                {item.fileName}
                              </button>
                              <button
                                type="button"
                                className="shrink-0 text-zinc-500 hover:text-rose-400"
                                title="Remove"
                                onClick={() => removeLibraryItem(item.id)}
                              >
                                ×
                              </button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    </div>
                  </div>
                )}

                {rightDockTab === "chat" && (
                  <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                    <div className="shrink-0 border-b border-white/[0.06] px-4 py-2">
                      <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-violet-400/90">Assistant</p>
                      <p className="text-xs text-zinc-500">
                        Graph: <span className="tabular-nums text-zinc-300">{graphNodes.length}</span> nodes
                      </p>
                    </div>
                    {pendingGeneratedTool && (
                      <div className="shrink-0 border-b border-amber-500/25 bg-amber-950/45 px-3 py-2.5">
                        <p className="text-[11px] font-semibold uppercase tracking-wide text-amber-200/90">Tool awaiting approval</p>
                        <p className="mt-1 text-[12px] leading-snug text-zinc-300">
                          <span className="font-medium text-zinc-100">{pendingGeneratedTool}</span> — register for{" "}
                          <span className="font-mono text-zinc-500">the edit registry</span> (demo) or discard.
                        </p>
                        <div className="mt-2 flex flex-wrap gap-2">
                          <button
                            type="button"
                            className="rounded-lg bg-amber-500 px-3 py-1.5 text-[12px] font-semibold text-zinc-950 hover:bg-amber-400"
                            onClick={() => {
                              const name = pendingGeneratedTool;
                              setPendingGeneratedTool(null);
                              setToolJob(null);
                              addMessage({
                                id: `approve-${Date.now()}`,
                                role: "assistant",
                                content: `Approved tool '${name ?? "tool"}'. It is now available for graph-grounded edits.`,
                              });
                            }}
                          >
                            Approve
                          </button>
                          <button
                            type="button"
                            className="rounded-lg border border-white/[0.12] bg-zinc-900/80 px-3 py-1.5 text-[12px] font-medium text-zinc-300 hover:bg-zinc-800"
                            onClick={() => {
                              setPendingGeneratedTool(null);
                              setToolJob(null);
                              addMessage({
                                id: `rej-${Date.now()}`,
                                role: "assistant",
                                content: "Discarded generated tool (mock rollback).",
                              });
                            }}
                          >
                            Reject
                          </button>
                        </div>
                      </div>
                    )}
                    <div ref={messagesRef} className="min-h-0 flex-1 space-y-3 overflow-y-auto p-3 select-text">
                      {messages.length === 0 && (
                        <div className="flex flex-col items-center justify-center gap-2 py-12 text-center">
                          <p className="text-sm font-medium text-zinc-400">Start a conversation</p>
                          <p className="max-w-[280px] px-2 text-xs leading-relaxed text-zinc-600">
                            Ask about the clip graph or your ingest — messages go to{" "}
                            <span className="font-mono text-zinc-500">POST /chat</span> on{" "}
                            <span className="font-mono text-zinc-500">{getKgEngineUrl()}</span>. Mention{" "}
                            <span className="font-mono text-zinc-500">film grain</span> for the tool-generation demo.
                          </p>
                        </div>
                      )}
                      {messages.map((msg) => (
                        <div key={msg.id}>
                          {msg.role === "user" ? (
                            <div className="ml-2 rounded-[1.25rem] rounded-br-md bg-violet-600/90 px-4 py-2.5 text-[13px] leading-relaxed text-white shadow-sm">
                              {msg.content}
                            </div>
                          ) : (
                            <div className="mr-1 rounded-[1.25rem] rounded-bl-md bg-zinc-900 px-4 pb-2.5 pt-2.5 text-[13px] leading-relaxed text-zinc-300 ring-1 ring-white/[0.06]">
                              {msg.thinking?.slice(0, thinkingVisible[msg.id] ?? 0).map((line, i) => (
                                <div key={`${msg.id}-t-${i}`} className="mb-1 font-mono text-[11px] text-zinc-500">
                                  → {line}
                                </div>
                              ))}
                              <div className="whitespace-pre-wrap break-words">{msg.content}</div>
                              {msg.nodes_used?.length ? (
                                <button
                                  type="button"
                                  className="mt-2 text-left text-[11px] text-sky-400/90 underline"
                                  onClick={() => {
                                    const node = allNodes.find((n) => n.id === msg.nodes_used?.[0]);
                                    if (node) {
                                      selectNode(node);
                                      setCurrentTime(node.time_start);
                                      if (videoRef.current) videoRef.current.currentTime = node.time_start;
                                    }
                                  }}
                                >
                                  Applied to [{msg.nodes_used[0]} →]
                                </button>
                              ) : null}
                            </div>
                          )}
                        </div>
                      ))}
                      {isProcessing && (
                        <div className="mr-1 rounded-[1.25rem] bg-zinc-900/80 px-4 py-3 text-[13px] text-zinc-500 ring-1 ring-white/[0.05]">
                          <span className="inline-flex items-center gap-2">
                            <span className="size-2 animate-pulse rounded-full bg-violet-400" />
                            Thinking…
                          </span>
                        </div>
                      )}
                    </div>
                    <div className="flex shrink-0 gap-2 border-t border-white/[0.06] bg-zinc-950/80 p-3">
                      <textarea
                        ref={composerRef}
                        value={messageInput}
                        onChange={(e) => setMessageInput(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" && !e.shiftKey) {
                            e.preventDefault();
                            void sendChat();
                          }
                        }}
                        rows={1}
                        placeholder="Message…"
                        className="max-h-[220px] min-h-[44px] flex-1 resize-none overflow-y-auto rounded-2xl border border-white/[0.08] bg-zinc-900/90 px-4 py-3 text-[13px] leading-relaxed text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-violet-500/35 focus:ring-2 focus:ring-violet-500/20"
                      />
                      <button
                        type="button"
                        onClick={() => void sendChat()}
                        disabled={isProcessing || !messageInput.trim()}
                        className="shrink-0 self-end rounded-2xl bg-white px-5 py-3 text-[13px] font-semibold text-zinc-950 transition enabled:hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-30"
                      >
                        Send
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </aside>
          </>
        )}

      </div>
    </main>
  );
}
