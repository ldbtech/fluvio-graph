"use client";

import { create } from "zustand";

export interface VideoNode {
  id: string;
  source_uri: string;
  entity_type: "person" | "object" | "scene" | "text" | "effect" | "Flower";
  label: string;
  time_start: number;
  time_end: number;
  frame_start: number;
  frame_end: number;
  bbox: { x: number; y: number; w: number; h: number } | null;
  mask_path: string | null;
  confidence: number;
  semantic: string;
  edges: { to_uri: string; label: string; prob: number }[];
}

export interface EffectBlock {
  id: string;
  tool_name: string;
  time_start: number;
  time_end: number;
  params: Record<string, unknown>;
  color: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  thinking?: string[];
  nodes_used?: string[];
  frames_affected?: [number, number];
  tool_used?: string;
  preview_path?: string;
  type?: "result" | "tool-generated";
}

/** One uploaded or demo video plus its editor session (graph, chat, timeline). */
export interface VideoLibraryItem {
  id: string;
  fileName: string;
  videoId: string;
  videoUrl: string;
  duration: number;
  fps: number;
  resolution: string;
  uploadedAt: number;
  allNodes: VideoNode[];
  messages: ChatMessage[];
  effects: EffectBlock[];
  currentTime: number;
  isPlaying: boolean;
  playbackSpeed: number;
}

interface VideoStore {
  videoId: string | null;
  videoUrl: string | null;
  duration: number;
  fps: number;
  resolution: string;
  currentTime: number;
  isPlaying: boolean;
  playbackSpeed: number;
  allNodes: VideoNode[];
  nodesAtTime: VideoNode[];
  selectedNode: VideoNode | null;
  activeEffects: EffectBlock[];
  messages: ChatMessage[];
  isProcessing: boolean;
  zoomLevel: number;
  effects: EffectBlock[];
  library: VideoLibraryItem[];
  activeLibraryId: string | null;
  setVideoMeta: (input: {
    videoId: string;
    videoUrl: string;
    duration: number;
    fps: number;
    resolution: string;
  }) => void;
  setCurrentTime: (time: number) => void;
  setIsPlaying: (playing: boolean) => void;
  setPlaybackSpeed: (speed: number) => void;
  setAllNodes: (nodes: VideoNode[]) => void;
  selectNode: (node: VideoNode | null) => void;
  setMessages: (messages: ChatMessage[]) => void;
  addMessage: (message: ChatMessage) => void;
  setIsProcessing: (busy: boolean) => void;
  addEffect: (effect: EffectBlock) => void;
  setZoomLevel: (zoomLevel: number) => void;
  flushActiveToLibrary: () => void;
  selectLibraryVideo: (id: string) => void;
  addLibraryItem: (item: Omit<VideoLibraryItem, "id" | "uploadedAt">) => string;
  removeLibraryItem: (id: string) => void;
  clearAllVideos: () => void;
  /** Save session to library and return to the upload / library hub (videos stay listed). */
  dockToLibrary: () => void;
  /** Merge fields into a library row (e.g. refresh `allNodes` after background ingest). */
  patchLibraryItem: (id: string, patch: Partial<Omit<VideoLibraryItem, "id" | "uploadedAt">>) => void;
}

const filterNodesAtTime = (nodes: VideoNode[], currentTime: number) =>
  nodes.filter((node) => currentTime >= node.time_start && currentTime <= node.time_end);

function patchActiveLibraryItem(
  library: VideoLibraryItem[],
  activeLibraryId: string | null,
  patch: Partial<Omit<VideoLibraryItem, "id" | "fileName" | "uploadedAt">>,
): VideoLibraryItem[] {
  if (!activeLibraryId) return library;
  return library.map((e) => (e.id === activeLibraryId ? { ...e, ...patch } : e));
}

export const useVideoStore = create<VideoStore>((set, get) => ({
  videoId: null,
  videoUrl: null,
  duration: 0,
  fps: 30,
  resolution: "0x0",
  currentTime: 0,
  isPlaying: false,
  playbackSpeed: 1,
  allNodes: [],
  nodesAtTime: [],
  selectedNode: null,
  activeEffects: [],
  messages: [],
  isProcessing: false,
  zoomLevel: 1,
  effects: [],
  library: [],
  activeLibraryId: null,

  flushActiveToLibrary: () => {
    const s = get();
    if (!s.activeLibraryId) return;
    set({
      library: s.library.map((e) =>
        e.id === s.activeLibraryId
          ? {
              ...e,
              videoId: s.videoId ?? e.videoId,
              videoUrl: s.videoUrl ?? e.videoUrl,
              duration: s.duration,
              fps: s.fps,
              resolution: s.resolution,
              allNodes: s.allNodes,
              messages: s.messages,
              effects: s.effects,
              currentTime: s.currentTime,
              isPlaying: s.isPlaying,
              playbackSpeed: s.playbackSpeed,
            }
          : e,
      ),
    });
  },

  selectLibraryVideo: (id) => {
    const s = get();
    if (!s.library.some((e) => e.id === id)) return;
    get().flushActiveToLibrary();
    const target = get().library.find((e) => e.id === id);
    if (!target) return;
    const bounded = Math.max(0, Math.min(target.currentTime, target.duration || target.currentTime));
    set({
      activeLibraryId: id,
      videoId: target.videoId,
      videoUrl: target.videoUrl,
      duration: target.duration,
      fps: target.fps,
      resolution: target.resolution,
      allNodes: target.allNodes,
      messages: target.messages,
      effects: target.effects,
      activeEffects: target.effects,
      currentTime: bounded,
      isPlaying: false,
      playbackSpeed: target.playbackSpeed,
      selectedNode: null,
      nodesAtTime: filterNodesAtTime(target.allNodes, bounded),
    });
  },

  addLibraryItem: (item) => {
    const id = crypto.randomUUID();
    const uploadedAt = Date.now();
    const entry: VideoLibraryItem = { ...item, id, uploadedAt };
    get().flushActiveToLibrary();
    const bounded = Math.max(0, Math.min(entry.currentTime, entry.duration || entry.currentTime));
    set({
      library: [...get().library, entry],
      activeLibraryId: id,
      videoId: entry.videoId,
      videoUrl: entry.videoUrl,
      duration: entry.duration,
      fps: entry.fps,
      resolution: entry.resolution,
      allNodes: entry.allNodes,
      messages: entry.messages,
      effects: entry.effects,
      activeEffects: entry.effects,
      currentTime: bounded,
      isPlaying: false,
      playbackSpeed: entry.playbackSpeed,
      selectedNode: null,
      nodesAtTime: filterNodesAtTime(entry.allNodes, bounded),
    });
    return id;
  },

  removeLibraryItem: (id) => {
    const s = get();
    const entry = s.library.find((e) => e.id === id);
    if (entry?.videoUrl.startsWith("blob:")) {
      try {
        URL.revokeObjectURL(entry.videoUrl);
      } catch {
        /* ignore */
      }
    }
    const nextLibrary = s.library.filter((e) => e.id !== id);
    if (s.activeLibraryId === id) {
      if (nextLibrary.length === 0) {
        set({
          library: [],
          activeLibraryId: null,
          videoId: null,
          videoUrl: null,
          duration: 0,
          fps: 30,
          resolution: "0x0",
          currentTime: 0,
          isPlaying: false,
          playbackSpeed: 1,
          allNodes: [],
          nodesAtTime: [],
          selectedNode: null,
          messages: [],
          effects: [],
          activeEffects: [],
        });
        return;
      }
      const first = nextLibrary[0]!;
      const bounded = Math.max(0, Math.min(first.currentTime, first.duration || first.currentTime));
      set({
        library: nextLibrary,
        activeLibraryId: first.id,
        videoId: first.videoId,
        videoUrl: first.videoUrl,
        duration: first.duration,
        fps: first.fps,
        resolution: first.resolution,
        allNodes: first.allNodes,
        messages: first.messages,
        effects: first.effects,
        activeEffects: first.effects,
        currentTime: bounded,
        isPlaying: false,
        playbackSpeed: first.playbackSpeed,
        selectedNode: null,
        nodesAtTime: filterNodesAtTime(first.allNodes, bounded),
      });
      return;
    }
    set({ library: nextLibrary });
  },

  clearAllVideos: () => {
    const s = get();
    for (const e of s.library) {
      if (e.videoUrl.startsWith("blob:")) {
        try {
          URL.revokeObjectURL(e.videoUrl);
        } catch {
          /* ignore */
        }
      }
    }
    set({
      library: [],
      activeLibraryId: null,
      videoId: null,
      videoUrl: null,
      duration: 0,
      fps: 30,
      resolution: "0x0",
      currentTime: 0,
      isPlaying: false,
      playbackSpeed: 1,
      allNodes: [],
      nodesAtTime: [],
      selectedNode: null,
      messages: [],
      effects: [],
      activeEffects: [],
    });
  },

  dockToLibrary: () => {
    get().flushActiveToLibrary();
    set({
      videoId: null,
      videoUrl: null,
      duration: 0,
      fps: 30,
      resolution: "0x0",
      currentTime: 0,
      isPlaying: false,
      playbackSpeed: 1,
      allNodes: [],
      nodesAtTime: [],
      selectedNode: null,
      messages: [],
      effects: [],
      activeEffects: [],
    });
  },

  patchLibraryItem: (id, patch) => {
    const s = get();
    const nextLibrary = s.library.map((e) => (e.id === id ? { ...e, ...patch } : e));
    const hit = nextLibrary.find((e) => e.id === id);
    const isActive = s.activeLibraryId === id;
    set({
      library: nextLibrary,
      ...(isActive && hit
        ? {
            ...(patch.videoId !== undefined ? { videoId: patch.videoId } : {}),
            ...(patch.videoUrl !== undefined ? { videoUrl: patch.videoUrl } : {}),
            ...(patch.duration !== undefined ? { duration: patch.duration } : {}),
            ...(patch.fps !== undefined ? { fps: patch.fps } : {}),
            ...(patch.resolution !== undefined ? { resolution: patch.resolution } : {}),
            ...(patch.allNodes !== undefined
              ? {
                  allNodes: patch.allNodes,
                  nodesAtTime: filterNodesAtTime(patch.allNodes, s.currentTime),
                }
              : {}),
            ...(patch.messages !== undefined ? { messages: patch.messages } : {}),
            ...(patch.effects !== undefined ? { effects: patch.effects, activeEffects: patch.effects } : {}),
          }
        : {}),
    });
  },

  setVideoMeta: ({ videoId, videoUrl, duration, fps, resolution }) => {
    const s = get();
    set({
      videoId,
      videoUrl,
      duration,
      fps,
      resolution,
      currentTime: 0,
      isPlaying: false,
      library: patchActiveLibraryItem(s.library, s.activeLibraryId, {
        videoId,
        videoUrl,
        duration,
        fps,
        resolution,
        currentTime: 0,
        isPlaying: false,
      }),
    });
  },

  setCurrentTime: (time) => {
    const { allNodes, duration } = get();
    const bounded = Math.max(0, Math.min(time, duration || time));
    set({
      currentTime: bounded,
      nodesAtTime: filterNodesAtTime(allNodes, bounded),
    });
  },

  setIsPlaying: (playing) => set({ isPlaying: playing }),

  setPlaybackSpeed: (speed) => set({ playbackSpeed: speed }),

  setAllNodes: (nodes) => {
    const { currentTime, activeLibraryId, library } = get();
    set({
      allNodes: nodes,
      nodesAtTime: filterNodesAtTime(nodes, currentTime),
      library: patchActiveLibraryItem(library, activeLibraryId, { allNodes: nodes }),
    });
  },

  selectNode: (node) => set({ selectedNode: node }),

  setMessages: (messages) => {
    const { activeLibraryId, library } = get();
    set({
      messages,
      library: patchActiveLibraryItem(library, activeLibraryId, { messages }),
    });
  },

  addMessage: (message) =>
    set((state) => {
      const messages = [...state.messages, message];
      return {
        messages,
        library: patchActiveLibraryItem(state.library, state.activeLibraryId, { messages }),
      };
    }),

  setIsProcessing: (busy) => set({ isProcessing: busy }),

  addEffect: (effect) =>
    set((state) => {
      const effects = [...state.effects, effect];
      const activeEffects = [...state.activeEffects, effect];
      return {
        effects,
        activeEffects,
        library: patchActiveLibraryItem(state.library, state.activeLibraryId, {
          effects,
        }),
      };
    }),

  setZoomLevel: (zoomLevel) => set({ zoomLevel }),
}));
