"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  startTransition,
} from "react";
import { BrainDomainTabs } from "./BrainDomainTabs";
import { BrainFusionLoadingMock } from "./BrainFusionLoadingMock";
import { ConnectorSidebar } from "./ConnectorSidebar";
import { GraphErrorBoundary } from "./GraphErrorBoundary";
import { GraphCanvas } from "./GraphCanvas";
import { GithubRepoFileTree } from "./GithubRepoFileTree";
import { WorkspaceSurfacePanel } from "./WorkspaceSurfacePanel";
import { WorkspaceRightPanel, type DesignPendingTool } from "./WorkspaceRightPanel";
import { ArchitectureLivePanel, type ArchitectureScene } from "./ArchitectureLivePanel";
import { GithubBrainSecurityPanel } from "./GithubBrainSecurityPanel";
import { WorkspaceTopChrome } from "./WorkspaceTopChrome";
import {
  approveArchitectureTool,
  discardArchitectureToolJob,
  ensureArchitectureToolsForMessage,
  type ToolJobStatus,
} from "@/lib/architectureToolAgent";
import { KG_URL } from "@/lib/constants";
import { fetchGraphMeta, fetchGraphWorkspace } from "@/lib/fetchGraphWorkspace";
import { filterGraphBySource, filterLiveEmailGraph } from "@/lib/graphFilters";
import { fetchCodebaseGalaxyTree } from "@/lib/fetchCodebaseGalaxy";
import { postCodebaseResolve } from "@/lib/fetchCodebaseResolve";
import { moduleSubtreeToGraph } from "@/lib/moduleTreeToGraph";
import { getMetaGraph, getMockGraph, getUnifiedGraph } from "@/lib/mockGraphs";
import { mockConnectorNarrative } from "@/lib/mockWorkspace";
import type {
  BrainTab,
  CodebaseCloneResult,
  CodebaseModuleTree,
  ConnectorId,
  ConnectorStatus,
  GraphEdge,
  GraphNode,
  SelectedNode,
  WorkspaceKind,
  WorkspaceSurface,
} from "@/lib/types";
import { DESIGN_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "@/lib/workspaceKinds";

const PDF_INPUT_ID = "pdf-workspace-upload";
const HEX_R = 80;

const hexPoints = (r: number) =>
  Array.from({ length: 6 }, (_, i) => {
    const a = (Math.PI / 3) * i - Math.PI / 6;
    const x = r + Math.cos(a) * r;
    const y = r + Math.sin(a) * r;
    return `${x.toFixed(4)},${y.toFixed(4)}`;
  }).join(" ");

type WorkspaceMode = "sources" | "brain";

export default function WorkspaceApp() {
  const svgRef = useRef<SVGSVGElement>(null);

  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);
  /** Full-graph counts from kg-engine (used for tab readiness; not limited to the graph sample). */
  const [graphSourceCounts, setGraphSourceCounts] = useState<Record<string, number>>({});
  const [graphTotals, setGraphTotals] = useState<{
    nodes: number;
    edges: number;
    returnedNodes: number;
    returnedEdges: number;
  } | null>(null);
  const [selected, setSelected] = useState<SelectedNode | null>(null);
  const [chatPrefill, setChatPrefill] = useState<string | null>(null);

  const [connectorStatus, setConnectorStatus] = useState<
    Partial<Record<ConnectorId, ConnectorStatus>>
  >({});
  /** Gmail OAuth token on disk (`GET /connect/gmail/status`), independent of ingested chunks. */
  const [gmailConnected, setGmailConnected] = useState(false);
  const [activity, setActivity] = useState<string | null>(null);
  const [centerPanel, setCenterPanel] = useState<WorkspaceSurface | null>(null);
  /** Last GitHub repo accepted for simple `/ingest` + `/parse` flow. */
  const [githubCloneInfo, setGithubCloneInfo] = useState<CodebaseCloneResult | null>(null);
  /** Galaxy tree for GitHub brain main graph (module tree → graph canvas when no resolve slice). */
  const [githubGalaxyTree, setGithubGalaxyTree] = useState<CodebaseModuleTree | null>(null);
  /** Repo-relative path from resolve / file focus — biases `/chat` toward that module. */
  const [githubChatFocusPath, setGithubChatFocusPath] = useState<string | null>(null);
  /** Import subgraph from `POST /codebase/resolve` when user clicks a file in the repo tree. */
  const [githubResolveSlice, setGithubResolveSlice] = useState<{
    nodes: GraphNode[];
    edges: GraphEdge[];
  } | null>(null);
  const [githubResolveBusy, setGithubResolveBusy] = useState(false);
  const [githubResolvePendingPath, setGithubResolvePendingPath] = useState<string | null>(null);
  const [githubResolveErr, setGithubResolveErr] = useState<string | null>(null);
  const githubResolveInFlightRef = useRef(false);

  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("sources");
  const [workspaceKind, setWorkspaceKind] = useState<WorkspaceKind>("personal");
  const [brainTab, setBrainTab] = useState<BrainTab>("documents");
  const [designScene, setDesignScene] = useState<ArchitectureScene | null>(null);
  const [designId, setDesignId] = useState<string | null>(null);
  const [designBusy, setDesignBusy] = useState(false);
  const [designError, setDesignError] = useState<string | null>(null);
  const [archToolJobStatus, setArchToolJobStatus] = useState<ToolJobStatus | null>(null);
  /** Room selected in ArchitectureLivePanel — sent as `selected_room_id` to POST /architecture/chat. */
  const [archFocusRoomId, setArchFocusRoomId] = useState<string | null>(null);
  const [archToolPending, setArchToolPending] = useState<DesignPendingTool | null>(null);
  const [archToolApproveBusy, setArchToolApproveBusy] = useState(false);
  const archToolPendingRef = useRef<DesignPendingTool | null>(null);
  const skipToolEnsureNextRef = useRef(false);

  /** Docked chat clears when this changes — include GitHub `owner/repo` so a new clone starts a fresh LLM thread. */
  const chatDomainKey = useMemo(() => {
    if (brainTab === "github" && githubCloneInfo) {
      return `github:${githubCloneInfo.owner}/${githubCloneInfo.repo}`;
    }
    if (brainTab === "github") return "github:no-clone";
    return brainTab;
  }, [brainTab, githubCloneInfo]);

  const [fusionReady, setFusionReady] = useState(true);
  const [metaReady, setMetaReady] = useState(true);

  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState<"idle" | "uploading" | "processing" | "finalizing">("idle");
  const [error, setError] = useState<string | null>(null);

  const [graphLoading, setGraphLoading] = useState(false);
  const [graphLoadProgress, setGraphLoadProgress] = useState({ message: "", percent: 0 });
  const [graphSampleNote, setGraphSampleNote] = useState<string | null>(null);

  const onSelectNode = useCallback((s: SelectedNode | null) => {
    setSelected(s);
  }, []);

  useEffect(() => {
    setSelected(null);
  }, [brainTab]);

  useEffect(() => {
    setGithubChatFocusPath(null);
  }, [brainTab, githubCloneInfo?.owner, githubCloneInfo?.repo]);

  useEffect(() => {
    if (brainTab !== "unified") {
      setFusionReady(true);
      return;
    }
    setFusionReady(false);
    const t = window.setTimeout(() => setFusionReady(true), 1400);
    return () => window.clearTimeout(t);
  }, [brainTab]);

  useEffect(() => {
    if (brainTab !== "meta") {
      setMetaReady(true);
      return;
    }
    setMetaReady(false);
    const t = window.setTimeout(() => setMetaReady(true), 800);
    return () => window.clearTimeout(t);
  }, [brainTab]);

  const consumeChatPrefill = useCallback(() => setChatPrefill(null), []);

  const runWorkspaceGraphLoad = useCallback(async (signal?: AbortSignal) => {
    setError(null);
    setGraphLoading(true);
    setGraphLoadProgress({ message: "Starting…", percent: 0 });
    try {
      const data = await fetchGraphWorkspace(KG_URL, (p) => setGraphLoadProgress(p), signal);
      startTransition(() => {
        setNodes(data.nodes);
        setEdges(data.edges);
        setGraphSourceCounts(data.source_counts);
        setGraphTotals({
          nodes: data.graph_total_nodes,
          edges: data.graph_total_edges,
          returnedNodes: data.nodes.length,
          returnedEdges: data.edges.length,
        });
      });
      const parts: string[] = [];
      if (data.nodes_capped) {
        parts.push(
          `Showing ${data.nodes.length.toLocaleString()} of ${data.graph_total_nodes.toLocaleString()} nodes in this view`,
        );
      }
      if (data.edges_truncated) {
        parts.push("Some edges were omitted for performance");
      }
      setGraphSampleNote(parts.length ? parts.join(" · ") : null);
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      setError(`cannot reach kg-engine at ${KG_URL} — is it running?`);
      setGraphSampleNote(null);
    } finally {
      setGraphLoading(false);
      setGraphLoadProgress({ message: "", percent: 0 });
    }
  }, []);

  /** Full nodes+edges load (PDF ingest, Gmail refresh, manual reload). */
  const fetchGraph = useCallback(async () => {
    await runWorkspaceGraphLoad(undefined);
  }, [runWorkspaceGraphLoad]);

  /** Counts only — keeps Sources fast; heavy load runs when you open Workspace brain. */
  const loadGraphMeta = useCallback(async () => {
    setError(null);
    try {
      const meta = await fetchGraphMeta(KG_URL);
      setGraphSourceCounts(meta.source_counts ?? {});
      setGraphTotals({
        nodes: meta.graph_total_nodes,
        edges: meta.graph_total_edges,
        returnedNodes: 0,
        returnedEdges: 0,
      });
    } catch {
      setError(`cannot reach kg-engine at ${KG_URL} — is it running?`);
    }
  }, []);

  /** After workspace snapshot / reset / load (kg-engine mutates graph on disk + in memory). */
  const onWorkspaceGraphChanged = useCallback(async () => {
    await loadGraphMeta();
    await runWorkspaceGraphLoad(undefined);
  }, [loadGraphMeta, runWorkspaceGraphLoad]);

  useEffect(() => {
    void loadGraphMeta();
  }, [loadGraphMeta]);

  useEffect(() => {
    if (workspaceMode !== "brain" || nodes.length > 0) return;
    const ac = new AbortController();
    void runWorkspaceGraphLoad(ac.signal);
    return () => ac.abort();
  }, [workspaceMode, nodes.length, runWorkspaceGraphLoad]);

  /** Keep Gmail sidebar chip in sync after browser OAuth tab completes. */
  useEffect(() => {
    const pollGmail = async () => {
      try {
        const res = await fetch(`${KG_URL}/connect/gmail/status`);
        if (!res.ok) return;
        const data = (await res.json()) as { connected: boolean };
        setGmailConnected(data.connected);
        setConnectorStatus((prev) => ({
          ...prev,
          gmail: data.connected ? "mock_on" : "off",
        }));
      } catch {
        /* offline or server down */
      }
    };
    void pollGmail();
    const onVis = () => {
      if (document.visibilityState === "visible") void pollGmail();
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, [KG_URL]);

  const onFileSelect = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      if (!file.name.endsWith(".pdf")) {
        setError("only PDF files are supported");
        return;
      }

      setCenterPanel(null);
      setError(null);
      setPhase("uploading");
      setProgress(0);

      const form = new FormData();
      form.append("file", file);

      try {
        await new Promise<void>((resolve, reject) => {
          const xhr = new XMLHttpRequest();
          let serverPulse: ReturnType<typeof setInterval> | null = null;

          const clearServerPulse = () => {
            if (serverPulse) {
              clearInterval(serverPulse);
              serverPulse = null;
            }
          };

          xhr.upload.onprogress = (ev) => {
            if (ev.lengthComputable && ev.total > 0) {
              const pct = Math.round((ev.loaded / ev.total) * 48);
              setProgress(pct);
            }
          };

          xhr.upload.onload = () => {
            setPhase("processing");
            let p = 50;
            serverPulse = setInterval(() => {
              p += Math.random() * 2.8 + 0.4;
              if (p >= 96) p = 96;
              setProgress((prev) => Math.max(prev, Math.round(p)));
            }, 320);
          };

          xhr.onload = () => {
            clearServerPulse();
            if (xhr.status >= 200 && xhr.status < 300) {
              setProgress(100);
              resolve();
            } else {
              reject(
                new Error(`server returned ${xhr.status}: ${xhr.responseText || xhr.statusText}`),
              );
            }
          };

          xhr.onerror = () => {
            clearServerPulse();
            reject(new Error(`network error — is kg-engine running? (${KG_URL})`));
          };

          xhr.onabort = () => {
            clearServerPulse();
            reject(new Error("upload aborted"));
          };

          xhr.open("POST", `${KG_URL}/ingest/pdf`);
          xhr.send(form);

          requestAnimationFrame(() => {
            setProgress((prev) => (prev === 0 ? 3 : prev));
          });
        });

        setPhase("finalizing");
        await fetchGraph();
        setPhase("idle");
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        setError(`upload failed: ${message}`);
        setPhase("idle");
        setProgress(0);
      } finally {
        e.target.value = "";
      }
    },
    [fetchGraph],
  );

  const onSelectSurface = useCallback((surface: WorkspaceSurface) => {
    setCenterPanel(surface);
    if (surface === "documents") {
      setActivity("PDF: live POST /ingest/pdf — other fields on this screen are placeholders for Rust config.");
    } else {
      setActivity(mockConnectorNarrative(surface));
    }
  }, []);

  const onOAuthPreviewComplete = useCallback((id: ConnectorId) => {
    setConnectorStatus((s) => ({ ...s, [id]: "mock_on" }));
    setActivity(
      `${mockConnectorNarrative(id)} · Rail shows preview-on — persist OAuth tokens and sync jobs in Rust next.`,
    );
  }, []);

  const onGithubPublicCloneSuccess = useCallback((info: CodebaseCloneResult) => {
    setGithubCloneInfo(info);
    setConnectorStatus((s) => ({ ...s, github: "mock_on" }));
    setActivity(
      `GitHub: cloned ${info.owner}/${info.repo} (${info.was_cloned ? "new" : "pull"}) — open Brain → GitHub, use the file tree to resolve imports, or browse the module graph below.`,
    );
  }, []);

  const livePdf = useMemo(
    () => filterGraphBySource(nodes, edges, "pdf"),
    [nodes, edges],
  );
  const liveEmail = useMemo(() => filterLiveEmailGraph(nodes, edges), [nodes, edges]);
  const liveArchitecture = useMemo(
    () => filterGraphBySource(nodes, edges, ["architecture", "tools"]),
    [nodes, edges],
  );

  const hasServerSourceCounts = Object.keys(graphSourceCounts).length > 0;
  const pdfNodeTotal = hasServerSourceCounts
    ? graphSourceCounts["pdf"] ?? 0
    : livePdf.nodes.length;
  const emailNodeTotal = hasServerSourceCounts
    ? (graphSourceCounts["email"] ?? 0) + (graphSourceCounts["gmail"] ?? 0)
    : liveEmail.nodes.length;

  const documentGraphReady = pdfNodeTotal > 0;

  useEffect(() => {
    if (brainTab !== "github" || !githubCloneInfo) {
      setGithubGalaxyTree(null);
      return;
    }
    let cancelled = false;
    void fetchCodebaseGalaxyTree(KG_URL, githubCloneInfo)
      .then((tree) => {
        if (!cancelled) setGithubGalaxyTree(tree);
      })
      .catch(() => {
        if (!cancelled) setGithubGalaxyTree(null);
      });
    return () => {
      cancelled = true;
    };
  }, [brainTab, githubCloneInfo]);

  useEffect(() => {
    if (brainTab !== "github" || !githubCloneInfo) {
      setGithubResolveSlice(null);
      setGithubResolveErr(null);
      setGithubResolveBusy(false);
      setGithubResolvePendingPath(null);
    }
  }, [brainTab, githubCloneInfo?.owner, githubCloneInfo?.repo]);

  const onGithubFileResolve = useCallback(
    async (relPath: string) => {
      if (!githubCloneInfo) return;
      if (githubResolveInFlightRef.current) {
        setActivity("GitHub import resolve is still running — wait for it to finish, then try again.");
        return;
      }
      githubResolveInFlightRef.current = true;
      const url = `${githubCloneInfo.owner}/${githubCloneInfo.repo}`;
      const pathNorm = relPath.replace(/\\/g, "/");
      setGithubResolveBusy(true);
      setGithubResolvePendingPath(pathNorm);
      setGithubResolveErr(null);
      setSelected(null);
      try {
        const res = await postCodebaseResolve(KG_URL, {
          url,
          path: pathNorm,
          max_depth: 2,
          max_files: 48,
        });
        const rawNodes = res.graph_nodes ?? [];
        const rawEdges = res.graph_edges ?? [];
        let nodes: GraphNode[] = rawNodes.map((n) => ({
          id: n.id,
          label: n.label,
          page: n.page,
          source: n.source || "github",
        }));
        if (nodes.length === 0 && res.resolved_paths?.length) {
          nodes = res.resolved_paths.map((p) => {
            const seg = p.replace(/\\/g, "/").split("/").pop() || p;
            return { id: p, label: seg, page: p, source: "github" };
          });
        }
        const edges: GraphEdge[] = rawEdges.map((e) => ({
          from: e.from,
          to: e.to,
          token: e.token ?? 1,
          probability: e.probability ?? 0.9,
          label: e.label,
        }));
        setGithubResolveSlice({ nodes, edges });
        setGithubChatFocusPath(pathNorm);
        setActivity(
          `GitHub: resolved ${res.resolved_paths.length} file(s) from ${pathNorm} (depth ${res.max_depth_reached}) → workspace graph updated.`,
        );
        // Macrotask so React commits `githubResolveSlice` before `fetchGraph` sets `graphLoading` — otherwise the
        // brain canvas can unmount briefly (galaxy tree not ready yet) and show an empty/black graph.
        await new Promise<void>((r) => {
          setTimeout(() => r(), 0);
        });
        void fetchGraph();
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setGithubResolveErr(msg);
        setActivity(`GitHub resolve failed: ${msg}`);
      } finally {
        githubResolveInFlightRef.current = false;
        setGithubResolveBusy(false);
        setGithubResolvePendingPath(null);
      }
    },
    [githubCloneInfo, fetchGraph],
  );

  const personalPreviewCount = useMemo(
    () => PERSONAL_CONNECTOR_IDS.filter((id) => connectorStatus[id] === "mock_on").length,
    [connectorStatus],
  );

  const designPreviewCount = useMemo(
    () => DESIGN_CONNECTOR_IDS.filter((id) => connectorStatus[id] === "mock_on").length,
    [connectorStatus],
  );

  const brainGraph = useMemo(() => {
    switch (brainTab) {
      case "unified":
        return getUnifiedGraph(workspaceKind, nodes, edges, connectorStatus, {
          gmailOAuthConnected: gmailConnected,
        });
      case "meta":
        return getMetaGraph(workspaceKind, documentGraphReady, connectorStatus);
      case "documents":
        return livePdf;
      case "github": {
        if (githubResolveSlice && githubResolveSlice.nodes.length > 0) {
          return githubResolveSlice;
        }
        if (!githubGalaxyTree) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
        return moduleSubtreeToGraph(githubGalaxyTree, 420);
      }
      case "des_arch_plans":
        return liveArchitecture;
      default: {
        const id = brainTab as ConnectorId;
        const st = connectorStatus[id];
        if (id === "gmail") {
          if (liveEmail.nodes.length > 0) return liveEmail;
          if (gmailConnected) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
          if (st === "mock_on") return getMockGraph(id);
          return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
        }
        if (st === "mock_on") return getMockGraph(id);
        return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
      }
    }
  }, [
    brainTab,
    workspaceKind,
    nodes,
    edges,
    connectorStatus,
    documentGraphReady,
    livePdf,
    liveEmail,
    liveArchitecture,
    gmailConnected,
    emailNodeTotal,
    githubGalaxyTree,
    githubResolveSlice,
  ]);

  const showBrainLoading =
    !graphLoading &&
    ((brainTab === "unified" && !fusionReady) || (brainTab === "meta" && !metaReady));

  const brainGraphEmpty =
    brainTab !== "unified" &&
    brainTab !== "meta" &&
    brainGraph.nodes.length === 0 &&
    !(brainTab === "github" && githubResolveBusy);

  const chatSource =
    workspaceKind === "personal" &&
    ((brainTab === "documents" && pdfNodeTotal > 0) ||
      (brainTab === "gmail" && emailNodeTotal > 0) ||
      (brainTab === "github" &&
        (githubGalaxyTree !== null ||
          (githubResolveSlice !== null && githubResolveSlice.nodes.length > 0))))
      ? "live"
      : "mock";

  const pickDefaultBrainDomain = useCallback((): BrainTab => {
    if (pdfNodeTotal > 0) return "documents";
    if (emailNodeTotal > 0) return "gmail";
    const first = PERSONAL_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on");
    return first ?? "documents";
  }, [pdfNodeTotal, emailNodeTotal, connectorStatus]);

  const pickDefaultDesignBrainTab = useCallback((): BrainTab => {
    const first = DESIGN_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on");
    return first ?? "des_arch_plans";
  }, [connectorStatus]);

  useEffect(() => {
    setBrainTab((prev) => {
      if (prev === "unified" || prev === "meta") return prev;

      if (workspaceKind === "design") {
        const invalid =
          prev === "documents" || PERSONAL_CONNECTOR_IDS.includes(prev as ConnectorId);
        if (invalid) return pickDefaultDesignBrainTab();
        return prev;
      }

      if (DESIGN_CONNECTOR_IDS.includes(prev as ConnectorId)) {
        if (pdfNodeTotal > 0) return "documents";
        if (emailNodeTotal > 0) return "gmail";
        return PERSONAL_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on") ?? "documents";
      }
      return prev;
    });
  }, [workspaceKind, pdfNodeTotal, emailNodeTotal, connectorStatus, pickDefaultDesignBrainTab]);

  const onWorkspaceKindChange = useCallback((kind: WorkspaceKind) => {
    setWorkspaceKind(kind);
    setCenterPanel((c) => {
      if (c === null) return c;
      if (kind === "design" && (c === "documents" || PERSONAL_CONNECTOR_IDS.includes(c))) return null;
      if (kind === "personal" && DESIGN_CONNECTOR_IDS.includes(c as ConnectorId)) return null;
      return c;
    });
  }, []);

  useEffect(() => {
    if (!designScene?.rooms?.length) {
      setArchFocusRoomId(null);
      return;
    }
    setArchFocusRoomId((prev) => {
      if (prev && designScene.rooms.some((r) => r.id === prev)) return prev;
      return designScene.rooms[0]?.id ?? null;
    });
  }, [designScene]);

  const handleDesignChatCommand = useCallback(
    async (question: string): Promise<string | null> => {
      const q = question.trim();
      const lower = q.toLowerCase();

      const designHelp = () =>
        [
          "Design commands (this sidebar):",
          "",
          "• With no design loaded yet: type your brief in plain text (any language) — same as POST /architecture/generate — or use /design generate <brief>",
          "• With a design loaded: plain text goes to POST /architecture/chat (edits). Or /modify …, /design chat …",
          "  Before each chat: POST /tools/detect → optional POST /tools/spawn + poll. Brand-new tools stay in generated/ until you click Approve in the chat panel (POST /tools/approve).",
          "  Room picker above the 3D view sets selected_room_id for chat.",
          "• JSON edits (advanced): /design modify or /modify with a payload starting with [",
          "",
          "JSON example:",
          '/design modify [{"update_room":{"room_id":"home_office","ops":[{"set_area_sqm":{"value":14}}]}}]',
          "",
          "Stacked two-story massing is not modeled in the viewer yet; `stories` in the brief affects program text only.",
        ].join("\n");

      const runDesignGenerate = async (brief: string): Promise<string> => {
        const b = brief.trim();
        if (!b) return "Usage: /design generate <brief text>";
        setDesignBusy(true);
        setDesignError(null);
        try {
          const res = await fetch(`${KG_URL}/architecture/generate`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ brief: b }),
          });
          const text = await res.text();
          if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
          const data = JSON.parse(text) as { design_id: string; scene: ArchitectureScene };
          setDesignId(data.design_id);
          setDesignScene(data.scene);
          setConnectorStatus((s) => ({ ...s, des_arch_plans: "mock_on" }));
          setActivity("Architecture design generated. Left graph now reflects architecture nodes.");
          await fetchGraph();
          return `Generated design ${data.design_id} with ${data.scene.rooms.length} rooms, ${data.scene.walls.length} walls, ${data.scene.openings.length} openings.`;
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e);
          setDesignError(msg);
          return `Design generate failed: ${msg}`;
        } finally {
          setDesignBusy(false);
        }
      };

      const runDesignModify = async (raw: string): Promise<string> => {
        if (!designId) return "No active design yet. Run /design generate <brief> first.";
        if (!raw) return "Usage: /design modify <json edits> or /modify <json edits>";
        setDesignBusy(true);
        setDesignError(null);
        try {
          const edits = JSON.parse(raw) as unknown;
          const res = await fetch(`${KG_URL}/architecture/modify`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ design_id: designId, edits }),
          });
          const text = await res.text();
          if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
          const data = JSON.parse(text) as { scene: ArchitectureScene; rooms: number; relationships: number };
          setDesignScene(data.scene);
          await fetchGraph();
          return `Applied edit. Design now has ${data.rooms} rooms and ${data.relationships} relationships.`;
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e);
          setDesignError(msg);
          return `Design modify failed: ${msg}`;
        } finally {
          setDesignBusy(false);
        }
      };

      const runArchitectureChat = async (message: string): Promise<string> => {
        if (!designId) return "No active design yet. Run /design generate <brief> first.";
        if (!message.trim()) return "Enter what you want to change.";
        setDesignBusy(true);
        setDesignError(null);
        setArchToolJobStatus(null);
        try {
          let toolPrefix = "";
          if (!skipToolEnsureNextRef.current) {
            const tools = await ensureArchitectureToolsForMessage(message.trim(), {
              onJobStatus: (status) => setArchToolJobStatus(status),
            });
            if (!tools.ok) {
              setDesignError(tools.error);
              return `Architecture tools sync failed: ${tools.error}`;
            }
            if (tools.pendingApproval) {
              const pend: DesignPendingTool = {
                ...tools.pendingApproval,
                replayMessage: message.trim(),
              };
              archToolPendingRef.current = pend;
              setArchToolPending(pend);
              setActivity("New architecture tool awaits approval in the chat panel.");
              return `New tool "${tools.pendingApproval.tool_name}" (${tools.pendingApproval.file_name}) is ready. Use Approve & continue below to copy it into fluvio-tools/src/tools and run architecture chat, or Discard to roll back.`;
            }
            if (!tools.skippedSpawn) {
              await fetchGraph();
              if (tools.userNote) {
                toolPrefix = `${tools.userNote}\n\n`;
                setActivity("Architecture tool catalog updated; graph refreshed.");
              }
            }
          } else {
            skipToolEnsureNextRef.current = false;
          }

          const res = await fetch(`${KG_URL}/architecture/chat`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              design_id: designId,
              selected_room_id: archFocusRoomId?.trim() || null,
              message: message.trim(),
            }),
          });
          const text = await res.text();
          if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
          const data = JSON.parse(text) as {
            answer: string;
            changes: unknown[];
            scene: ArchitectureScene;
          };
          setDesignScene(data.scene);
          await fetchGraph();
          const n = data.changes?.length ?? 0;
          const tail = n ? ` (${n} structured change${n === 1 ? "" : "s"} applied.)` : "";
          return `${toolPrefix}${data.answer}${tail}`;
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : String(e);
          setDesignError(msg);
          return `Architecture chat failed: ${msg}`;
        } finally {
          setDesignBusy(false);
          setArchToolJobStatus(null);
        }
      };

      const isJsonEditsPayload = (raw: string): boolean => {
        const t = raw.trim();
        return t.startsWith("[");
      };

      // Architecture tab, no leading slash: generate when no design yet (matches curl POST /architecture/generate); else chat.
      if (brainTab === "des_arch_plans" && !q.startsWith("/")) {
        if (!designId) return runDesignGenerate(q);
        return runArchitectureChat(q);
      }

      if (lower.startsWith("/design chat ")) {
        const msg = q.slice("/design chat ".length).trim();
        if (!msg) return "Usage: /design chat <message>";
        return runArchitectureChat(msg);
      }

      if (lower === "/modify") {
        return designHelp();
      }
      if (lower.startsWith("/modify ")) {
        const raw = q.slice("/modify ".length).trim();
        if (!raw) return "Usage: /modify <message> — natural language, or /modify <json array> starting with [";
        if (isJsonEditsPayload(raw)) {
          try {
            const parsed = JSON.parse(raw) as unknown;
            if (!Array.isArray(parsed)) {
              return "JSON after /modify must be an array of edits (same as /design modify).";
            }
          } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            return `Invalid JSON: ${msg}`;
          }
          return runDesignModify(raw);
        }
        return runArchitectureChat(raw);
      }

      if (lower === "/design" || lower === "/design help" || lower.startsWith("/design help")) {
        return designHelp();
      }

      if (!lower.startsWith("/design ")) return null;

      if (lower.startsWith("/design generate ")) {
        const brief = q.slice("/design generate ".length).trim();
        return runDesignGenerate(brief);
      }

      if (lower.startsWith("/design modify ")) {
        const raw = q.slice("/design modify ".length).trim();
        if (!raw) return "Usage: /design modify <message> — natural language, or JSON array starting with [";
        if (isJsonEditsPayload(raw)) {
          try {
            const parsed = JSON.parse(raw) as unknown;
            if (!Array.isArray(parsed)) {
              return "JSON after /design modify must be an array of edits.";
            }
          } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            return `Invalid JSON: ${msg}`;
          }
          return runDesignModify(raw);
        }
        return runArchitectureChat(raw);
      }

      return `Unknown /design command.\n\n${designHelp()}`;
    },
    [designId, fetchGraph, archFocusRoomId, brainTab],
  );

  const handleApproveArchTool = useCallback(async (): Promise<string | null> => {
    const p = archToolPendingRef.current;
    if (!p || !designId) return null;
    setArchToolApproveBusy(true);
    setDesignError(null);
    try {
      await approveArchitectureTool(p.file_name, p.job_id);
      archToolPendingRef.current = null;
      setArchToolPending(null);
      await fetchGraph();
      setActivity("Tool approved into catalog; finishing architecture chat.");
      skipToolEnsureNextRef.current = true;
      setDesignBusy(true);
      const res = await fetch(`${KG_URL}/architecture/chat`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          design_id: designId,
          selected_room_id: archFocusRoomId?.trim() || null,
          message: p.replayMessage,
        }),
      });
      const text = await res.text();
      if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
      const data = JSON.parse(text) as {
        answer: string;
        changes: unknown[];
        scene: ArchitectureScene;
      };
      setDesignScene(data.scene);
      await fetchGraph();
      const n = data.changes?.length ?? 0;
      const tail = n ? ` (${n} structured change${n === 1 ? "" : "s"} applied.)` : "";
      return `${data.answer}${tail}`;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setDesignError(msg);
      return `Architecture chat failed: ${msg}`;
    } finally {
      setArchToolApproveBusy(false);
      setDesignBusy(false);
    }
  }, [designId, archFocusRoomId, fetchGraph]);

  const handleDiscardArchTool = useCallback(async () => {
    const p = archToolPendingRef.current;
    if (!p) return;
    setArchToolApproveBusy(true);
    setDesignError(null);
    try {
      await discardArchitectureToolJob(p.job_id);
      archToolPendingRef.current = null;
      setArchToolPending(null);
      await fetchGraph();
      setActivity("Tool generation rolled back.");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setDesignError(msg);
      throw e;
    } finally {
      setArchToolApproveBusy(false);
    }
  }, [fetchGraph]);

  useEffect(() => {
    if (brainTab !== "des_arch_plans") {
      archToolPendingRef.current = null;
      setArchToolPending(null);
    }
  }, [brainTab]);

  const onWorkspaceModeChange = useCallback(
    (mode: WorkspaceMode) => {
      if (mode === "brain") {
        setBrainTab(workspaceKind === "design" ? pickDefaultDesignBrainTab() : pickDefaultBrainDomain());
        if (nodes.length === 0) {
          setGraphLoading(true);
          setGraphLoadProgress({ message: "Opening workspace brain…", percent: 1 });
        }
      }
      setWorkspaceMode(mode);
    },
    [workspaceKind, pickDefaultBrainDomain, pickDefaultDesignBrainTab, nodes.length],
  );

  const openSourcesFor = useCallback((surface: WorkspaceSurface | null) => {
    setWorkspaceMode("sources");
    if (surface) setCenterPanel(surface);
  }, []);

  const fillPct = progress / 100;
  const hexH = HEX_R * 2;
  const isIngesting = phase === "uploading" || phase === "processing" || phase === "finalizing";

  const ingestOverlay = isIngesting && (
    <div className="pointer-events-none fixed inset-0 z-[100] flex flex-col items-center justify-center bg-zinc-950/85 pt-12 backdrop-blur-md">
      <div className="relative mb-6">
        <svg width={HEX_R * 2} height={HEX_R * 2} viewBox={`0 0 ${HEX_R * 2} ${HEX_R * 2}`}>
          <defs>
            <linearGradient id="liquidBodyGrad" x1="0" y1="1" x2="0" y2="0">
              <stop offset="0%" stopColor="#023d4a" stopOpacity="0.95" />
              <stop offset="45%" stopColor="#0a7a8c" stopOpacity="0.75" />
              <stop offset="78%" stopColor="#00c8d4" stopOpacity="0.45" />
              <stop offset="100%" stopColor="#7df9ff" stopOpacity="0.25" />
            </linearGradient>
            <linearGradient id="liquidSurfaceGrad" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="#00fff2" stopOpacity="0" />
              <stop offset="50%" stopColor="#b8ffff" stopOpacity="0.85" />
              <stop offset="100%" stopColor="#00fff2" stopOpacity="0" />
            </linearGradient>
            <clipPath id="hex-clip">
              <polygon points={hexPoints(HEX_R - 2)} />
            </clipPath>
            <filter id="hexLiquidGlow" x="-40%" y="-40%" width="180%" height="180%">
              <feGaussianBlur stdDeviation="2.5" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
            <style>
              {`
                @keyframes meniscusShift {
                  0%, 100% { transform: translateX(-3px); opacity: 0.85; }
                  50% { transform: translateX(3px); opacity: 1; }
                }
                @keyframes bubbleDrift {
                  0% { transform: translateY(4px); opacity: 0; }
                  15% { opacity: 0.35; }
                  100% { transform: translateY(-${hexH * 0.35}px); opacity: 0; }
                }
                .meniscus { animation: meniscusShift 2.4s ease-in-out infinite; }
                .bubble-a { animation: bubbleDrift 3.2s ease-in infinite; animation-delay: 0s; }
                .bubble-b { animation: bubbleDrift 2.7s ease-in infinite; animation-delay: 1.1s; }
                .bubble-c { animation: bubbleDrift 3.6s ease-in infinite; animation-delay: 2s; }
              `}
            </style>
          </defs>

          <g clipPath="url(#hex-clip)">
            <rect
              x={0}
              y={hexH * (1 - fillPct)}
              width={HEX_R * 2}
              height={Math.max(0, hexH * fillPct + 3)}
              fill="url(#liquidBodyGrad)"
              style={{ transition: "y 0.35s ease-out, height 0.35s ease-out" }}
            />
            {fillPct > 0.04 && (
              <>
                <rect
                  className="meniscus"
                  x={-6}
                  y={hexH * (1 - fillPct) - 5}
                  width={HEX_R * 2 + 12}
                  height={7}
                  fill="url(#liquidSurfaceGrad)"
                  style={{
                    transition: "y 0.35s ease-out",
                    filter: "url(#hexLiquidGlow)",
                  }}
                />
                <ellipse
                  cx={HEX_R * 0.35}
                  cy={hexH * (1 - fillPct) + 10}
                  rx={3}
                  ry={2}
                  fill="#b8ffff"
                  fillOpacity={0.2}
                  className="bubble-a"
                />
                <ellipse
                  cx={HEX_R * 1.25}
                  cy={hexH * (1 - fillPct) + 24}
                  rx={2}
                  ry={1.5}
                  fill="#7df9ff"
                  fillOpacity={0.18}
                  className="bubble-b"
                />
                <ellipse
                  cx={HEX_R * 0.82}
                  cy={hexH * (1 - fillPct) + 40}
                  rx={2.5}
                  ry={2}
                  fill="#00fff2"
                  fillOpacity={0.15}
                  className="bubble-c"
                />
              </>
            )}
          </g>

          <polygon
            points={hexPoints(HEX_R - 2)}
            fill="none"
            stroke="#00fff2"
            strokeWidth="1.2"
            strokeOpacity={0.55}
            style={{ filter: "drop-shadow(0 0 10px #00fff2aa)" }}
          />
        </svg>

        <div className="absolute inset-0 flex items-center justify-center">
          <span
            className="font-mono text-xl font-bold tabular-nums text-[#d4fbff]"
            style={{ textShadow: "0 0 18px #00fff2aa" }}
          >
            {progress}%
          </span>
        </div>
      </div>

      <p className="text-sm font-medium text-zinc-400">
        {phase === "uploading" && "uploading PDF…"}
        {phase === "processing" && "embedding & building graph…"}
        {phase === "finalizing" && "wiring neuron view…"}
      </p>
    </div>
  );

  return (
    <main className="ui-main relative h-screen w-screen overflow-hidden pt-12 select-none">
      <input
        id={PDF_INPUT_ID}
        type="file"
        accept=".pdf"
        onChange={onFileSelect}
        className="sr-only"
      />

      <WorkspaceTopChrome
        mode={workspaceMode}
        onModeChange={onWorkspaceModeChange}
        workspaceKind={workspaceKind}
        onWorkspaceKindChange={onWorkspaceKindChange}
        documentGraphReady={documentGraphReady}
        personalPreviewCount={personalPreviewCount}
        designPreviewCount={designPreviewCount}
      />

      {workspaceMode === "sources" && (
        <div className="flex h-[calc(100vh-3rem)] min-h-0 w-full">
          <ConnectorSidebar
            workspaceKind={workspaceKind}
            onWorkspaceKindChange={onWorkspaceKindChange}
            pdfInputId={PDF_INPUT_ID}
            activeSurface={centerPanel}
            statusById={connectorStatus}
            onSelectSurface={onSelectSurface}
            activity={activity}
            kgUrl={KG_URL}
            onWorkspaceGraphChanged={onWorkspaceGraphChanged}
          />

          <div className="relative min-w-0 flex-1">
            {centerPanel !== null && (
              <WorkspaceSurfacePanel
                surface={centerPanel}
                onClose={() => setCenterPanel(null)}
                pdfInputId={PDF_INPUT_ID}
                kgUrl={KG_URL}
                graphNodes={nodes.length}
                graphEdges={edges.length}
                onOAuthPreviewComplete={onOAuthPreviewComplete}
                onGmailGraphRefresh={fetchGraph}
                onGithubPublicCloneSuccess={onGithubPublicCloneSuccess}
                onGithubCloneSessionStart={fetchGraph}
              />
            )}

            {centerPanel === null &&
              workspaceKind === "personal" &&
              documentGraphReady === false &&
              phase === "idle" && (
              <label
                htmlFor={PDF_INPUT_ID}
                className="absolute inset-0 flex cursor-pointer flex-col items-center justify-center gap-2 bg-zinc-950/50 backdrop-blur-sm"
              >
                <div className="relative mb-6">
                  <svg width={HEX_R * 2} height={HEX_R * 2} viewBox={`0 0 ${HEX_R * 2} ${HEX_R * 2}`}>
                    <polygon
                      points={hexPoints(HEX_R)}
                      fill="none"
                      stroke="#00fff2"
                      strokeWidth="1"
                      strokeOpacity="0.25"
                      className="transition-all duration-500 hover:stroke-opacity-60"
                      style={{ filter: "drop-shadow(0 0 12px #00fff240)" }}
                    />
                    <polygon
                      points={hexPoints(HEX_R - 8)}
                      fill="none"
                      stroke="#00fff2"
                      strokeWidth="0.5"
                      strokeOpacity="0.1"
                    />
                  </svg>
                  <div className="absolute inset-0 flex items-center justify-center">
                    <span
                      className="text-5xl text-zinc-600 transition duration-500 hover:text-sky-400/80"
                      style={{ filter: "drop-shadow(0 0 16px #00fff260)" }}
                    >
                      +
                    </span>
                  </div>
                </div>
                <p className="font-mono text-sm text-slate-400 transition hover:text-slate-300">
                  Sources · drop a PDF (live ingest)
                </p>
                <p className="max-w-sm px-6 text-center font-mono text-xs text-slate-600">
                  Graphs live in <span className="font-medium text-zinc-200">Workspace brain</span> — one tab per source. Connect
                  integrations from the rail, then open the brain to chat with each slice.
                </p>
                {error && <p className="mt-2 max-w-md px-6 text-center font-mono text-xs text-red-400/90">{error}</p>}
              </label>
            )}

            {centerPanel === null &&
              workspaceKind === "personal" &&
              documentGraphReady &&
              phase === "idle" && (
              <div className="absolute inset-0 flex flex-col items-center justify-center gap-6 p-8">
                <div className="max-w-md rounded-2xl border border-emerald-500/25 bg-emerald-500/[0.06] p-8 text-center shadow-[0_0_40px_rgba(16,185,129,0.08)]">
                  <p className="font-mono text-[10px] uppercase tracking-wider text-emerald-400/80">documents graph</p>
                  <p className="mt-2 text-2xl font-semibold text-emerald-100">
                    {nodes.length} nodes · {edges.length} edges
                  </p>
                  <p className="mt-3 text-sm leading-relaxed text-slate-400">
                    The PDF brain is ready. Open the workspace brain to explore and chat on the graph canvas — separate
                    from connector setup.
                  </p>
                  <div className="mt-6 flex flex-wrap justify-center gap-3">
                    <button
                      type="button"
                      onClick={() => onWorkspaceModeChange("brain")}
                      className="rounded-xl bg-zinc-100 px-6 py-2.5 text-sm font-semibold text-zinc-900 shadow-lg transition hover:bg-white"
                    >
                      Open workspace brain
                    </button>
                    <label
                      htmlFor={PDF_INPUT_ID}
                      className="cursor-pointer rounded-xl border border-emerald-400/30 px-5 py-2.5 font-mono text-sm text-emerald-200 transition hover:bg-emerald-500/10"
                    >
                      + add PDF
                    </label>
                  </div>
                </div>
                {error && <p className="font-mono text-xs text-red-400/90">{error}</p>}
              </div>
            )}

            {centerPanel === null && workspaceKind === "design" && phase === "idle" && (
              <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-zinc-950/40 px-8 text-center backdrop-blur-sm">
                <p className="max-w-md text-[15px] font-medium leading-relaxed text-zinc-300">
                  {
                    "Pick Architecture on the left, then in Brain use right chat: /design generate <brief> to create design + graph."
                  }
                </p>
                <p className="max-w-sm text-[12px] leading-relaxed text-zinc-600">
                  Design graph is created from architecture generation and updates as you modify the design.
                </p>
              </div>
            )}

            {centerPanel === null && !documentGraphReady && error && phase === "idle" && (
              <div className="pointer-events-none absolute bottom-8 left-1/2 max-w-lg -translate-x-1/2 rounded-lg border border-red-500/30 bg-red-950/50 px-4 py-2 font-mono text-xs text-red-300">
                {error}
              </div>
            )}

            {ingestOverlay}
          </div>
        </div>
      )}

      {workspaceMode === "brain" && (
        <div className="relative flex h-[calc(100vh-3rem)] w-full min-h-0 flex-col sm:flex-row">
          {brainTab === "github" && githubResolveBusy && (
            <div
              className="pointer-events-auto fixed inset-0 z-[210] flex items-center justify-center bg-zinc-950/75 px-4 backdrop-blur-sm"
              role="dialog"
              aria-modal="true"
              aria-busy="true"
              aria-label="Resolving file imports"
            >
              <div className="w-full max-w-md rounded-2xl border border-sky-500/35 bg-zinc-900/95 px-6 py-5 shadow-2xl">
                <div className="flex items-start gap-4">
                  <span
                    className="mt-0.5 inline-block size-8 shrink-0 animate-spin rounded-full border-2 border-sky-400/25 border-t-sky-400"
                    aria-hidden
                  />
                  <div className="min-w-0 flex-1">
                    <p className="text-[14px] font-semibold text-sky-100">Building import graph</p>
                    <p className="mt-1 truncate font-mono text-[12px] text-zinc-300" title={githubResolvePendingPath ?? ""}>
                      {githubResolvePendingPath ?? "…"}
                    </p>
                    <p className="mt-3 text-[12px] leading-relaxed text-zinc-500">
                      kg-engine is walking imports and updating the workspace graph. The UI is locked until this
                      finishes.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}
          {brainTab === "github" && connectorStatus.github === "mock_on" && (
            <GithubRepoFileTree
              kgUrl={KG_URL}
              cloneInfo={githubCloneInfo}
              className="flex max-h-[40vh] w-full min-h-0 shrink-0 flex-col border-b border-white/[0.06] sm:max-h-none sm:max-w-[min(100%,272px)] sm:border-b-0 sm:border-r"
              onResolveFile={onGithubFileResolve}
              resolveBusy={githubResolveBusy}
              resolvePendingPath={githubResolvePendingPath}
              resolveError={githubResolveErr}
              resolveSubgraphActive={
                githubResolveSlice !== null && githubResolveSlice.nodes.length > 0
              }
              onClearResolveSubgraph={() => {
                setGithubResolveSlice(null);
                setGithubResolveErr(null);
                setGithubResolvePendingPath(null);
              }}
            />
          )}
          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            <BrainDomainTabs
              workspaceKind={workspaceKind}
              active={brainTab}
              onChange={setBrainTab}
              documentGraphReady={documentGraphReady}
              gmailLiveReady={emailNodeTotal > 0}
              gmailOAuthConnected={gmailConnected}
              connectorStatus={connectorStatus}
            />
            {brainTab === "github" && connectorStatus.github === "mock_on" && (
              <GithubBrainSecurityPanel
                kgUrl={KG_URL}
                pdfReady={documentGraphReady}
                focusPathPrefix={githubChatFocusPath}
                onGraphRefresh={fetchGraph}
                onOpenSourcesDocuments={() => openSourcesFor("documents")}
              />
            )}
            <div
              className={`relative min-h-0 flex-1 ${workspaceKind === "design" ? "flex flex-col lg:flex-row" : ""}`}
            >
              <div className="relative min-h-[min(240px,38vh)] min-w-0 flex-1 overflow-hidden">
                {graphLoading &&
                  !(
                    brainTab === "github" &&
                    (githubGalaxyTree !== null ||
                      (githubResolveSlice !== null && githubResolveSlice.nodes.length > 0) ||
                      githubResolveBusy)
                  ) && (
                  <div className="absolute inset-0 z-[25] flex flex-col items-center justify-center gap-4 bg-zinc-950/90 backdrop-blur-md">
                  <div className="w-[min(380px,88vw)] rounded-2xl border border-white/[0.06] bg-zinc-900/60 p-5 shadow-xl">
                    <div className="mb-2 flex justify-between text-[12px] font-medium text-zinc-500">
                      <span>Loading graph</span>
                      <span className="tabular-nums text-zinc-200">{graphLoadProgress.percent}%</span>
                    </div>
                    <div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
                      <div
                        className="h-full rounded-full bg-zinc-100 transition-[width] duration-200 ease-out"
                        style={{ width: `${Math.max(0, Math.min(100, graphLoadProgress.percent))}%` }}
                      />
                    </div>
                    <p className="mt-4 text-center text-[13px] leading-relaxed text-zinc-400">
                      {graphLoadProgress.message || "…"}
                    </p>
                    <p className="mt-2 text-center text-[11px] leading-relaxed text-zinc-600">
                      Loading in chunks so the canvas can appear as data arrives.
                    </p>
                  </div>
                  </div>
                )}

                <div className={showBrainLoading ? "pointer-events-none opacity-[0.08]" : ""}>
                  {(!graphLoading ||
                    (brainTab === "github" &&
                      (githubGalaxyTree !== null ||
                        (githubResolveSlice !== null && githubResolveSlice.nodes.length > 0) ||
                        githubResolveBusy))) && (
                    <GraphErrorBoundary>
                      <GraphCanvas
                        key={`${brainTab}-${githubResolveSlice ? "slice" : "live"}-${brainGraph.nodes.length}-${brainGraph.edges.length}`}
                        svgRef={svgRef}
                        nodes={brainGraph.nodes}
                        edges={brainGraph.edges}
                        onSelect={onSelectNode}
                      />
                    </GraphErrorBoundary>
                  )}
                </div>

                {showBrainLoading && (
                  <BrainFusionLoadingMock
                    variant={brainTab === "meta" ? "meta" : "unified"}
                    workspaceKind={workspaceKind}
                  />
                )}

                {brainGraphEmpty && (
                  <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-4 bg-zinc-950/88 px-6 text-center backdrop-blur-md">
                    <p className="max-w-sm text-[15px] font-medium leading-relaxed text-zinc-300">
                    {brainTab === "documents"
                      ? "No PDF graph yet. Ingest a document from Sources, then return here."
                      : brainTab === "gmail" && gmailConnected && emailNodeTotal === 0
                        ? "Gmail is connected but nothing is ingested yet. Open Sources → Gmail and run Sync, then return here."
                        : brainTab === "github"
                          ? "No module graph yet. In Sources → GitHub, clone a public repo (kg-engine must reach your clone), then open the file tree and resolve a file to load a subgraph, or wait for the repo module graph."
                          : workspaceKind === "design" && brainTab === "des_arch_plans"
                            ? "No architecture graph yet. In right chat, run /design generate <brief> and the left graph will populate."
                          : `No ${brainTab} graph yet. Finish the preview connect flow in Sources to load mock nodes, or connect the live source in Rust.`}
                    </p>
                    <div className="flex flex-wrap justify-center gap-2">
                    <button
                      type="button"
                      onClick={() =>
                        openSourcesFor(brainTab === "documents" ? "documents" : (brainTab as ConnectorId))
                      }
                      className="rounded-xl border border-white/[0.1] bg-zinc-100 px-4 py-2.5 text-[13px] font-semibold text-zinc-900 transition hover:bg-white"
                    >
                      Open in Sources
                    </button>
                    {workspaceKind === "personal" && brainTab === "documents" && (
                      <label
                        htmlFor={PDF_INPUT_ID}
                        className="cursor-pointer rounded-lg border border-white/15 px-4 py-2 font-mono text-xs text-slate-300 hover:bg-white/5"
                      >
                        Quick upload PDF
                      </label>
                    )}
                    </div>
                  </div>
                )}

                {!brainGraphEmpty && !showBrainLoading && !graphLoading && phase === "idle" && (
                  <div className="pointer-events-none absolute left-1/2 top-3 z-10 flex max-w-[min(92vw,520px)] -translate-x-1/2 flex-col items-center gap-1 rounded-full border border-white/[0.08] bg-zinc-900/90 px-4 py-1.5 text-[11px] font-medium text-zinc-500 shadow-lg backdrop-blur-md">
                    <div className="flex flex-wrap items-center justify-center gap-3">
                    <span className="text-zinc-400">{brainTab}</span>
                    <span className="h-1 w-1 rounded-full bg-zinc-600" />
                    <span>
                      {brainGraph.nodes.length} nodes · {brainGraph.edges.length} edges
                      {graphTotals &&
                        graphTotals.returnedNodes > 0 &&
                        (graphTotals.returnedNodes < graphTotals.nodes ||
                          graphTotals.returnedEdges < graphTotals.edges) && (
                          <span className="text-amber-200/80">
                            {" "}
                            · sample of {graphTotals.returnedNodes}/{graphTotals.nodes} nodes
                          </span>
                        )}
                    </span>
                    </div>
                    {graphSampleNote && (
                      <span className="text-center text-[10px] leading-snug text-amber-200/70">{graphSampleNote}</span>
                    )}
                  </div>
                )}

                {selected && !brainGraphEmpty && !showBrainLoading && (
                  <div
                    className="pointer-events-auto absolute left-4 top-14 z-20 w-72 rounded-2xl border border-white/[0.08] bg-zinc-900/95 p-4 shadow-2xl backdrop-blur-xl"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <div className="mb-3 flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="h-2 w-2 rounded-full bg-sky-400" />
                        <span className="text-[12px] font-medium text-zinc-300">Segment {selected.node.page}</span>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelected(null)}
                        className="rounded-full px-2 py-0.5 text-[13px] text-zinc-500 transition hover:bg-white/[0.06] hover:text-zinc-200"
                      >
                        ✕
                      </button>
                    </div>

                    <p className="mb-4 line-clamp-5 border-l-2 border-white/[0.08] pl-3 text-[13px] leading-relaxed text-zinc-400">
                      {selected.node.label}
                    </p>

                    {selected.neighbors.length > 0 && (
                      <div className="mb-3">
                        <p className="mb-2 text-[11px] font-medium uppercase tracking-wide text-zinc-600">Connections</p>
                        <div className="space-y-1">
                          {selected.neighbors.map((n, i) => (
                            <div
                              key={i}
                              className="flex items-center justify-between rounded-xl bg-zinc-950/80 px-3 py-2 text-[12px]"
                            >
                              <div className="flex min-w-0 flex-col gap-0.5">
                                <span className="text-zinc-500">s.{n.node.page}</span>
                                {n.label ? (
                                  <span className="font-mono text-[10px] text-sky-400/90">{n.label}</span>
                                ) : null}
                              </div>
                              <div className="flex shrink-0 items-center gap-3">
                                <span className="tabular-nums text-zinc-400">{n.token}t</span>
                                <div className="flex items-center gap-1">
                                  <div className="h-1 w-12 overflow-hidden rounded-full bg-zinc-800">
                                    <div
                                      className="h-full rounded-full bg-zinc-300"
                                      style={{ width: `${n.probability * 100}%` }}
                                    />
                                  </div>
                                  <span className="w-8 text-right tabular-nums text-zinc-300">
                                    {(n.probability * 100).toFixed(0)}%
                                  </span>
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    <button
                      type="button"
                      onClick={() => {
                        setChatPrefill(
                          `Tell me about this part of the ${brainTab} graph: ${selected.node.label.slice(0, 60)}`,
                        );
                      }}
                      className="mt-1 w-full rounded-xl bg-zinc-100 py-2.5 text-[13px] font-semibold text-zinc-900 transition hover:bg-white"
                    >
                      Ask about this node
                    </button>
                  </div>
                )}

                {ingestOverlay}
              </div>

              {workspaceKind === "design" && (
                <ArchitectureLivePanel
                  scene={designScene}
                  designId={designId}
                  busy={designBusy}
                  error={designError}
                  toolJobStatus={archToolJobStatus}
                  focusRoomId={archFocusRoomId}
                  onFocusRoomChange={setArchFocusRoomId}
                />
              )}
            </div>
          </div>

          <WorkspaceRightPanel
            dock
            workspaceKind={workspaceKind}
            domainKey={chatDomainKey}
            chatSource={chatSource}
            brainTab={brainTab}
            graphEmpty={brainGraphEmpty}
            nodeCount={brainGraph.nodes.length}
            chatPrefill={chatPrefill}
            onConsumeChatPrefill={consumeChatPrefill}
            codebaseFocusPath={brainTab === "github" ? githubChatFocusPath : null}
            onDesignCommand={workspaceKind === "design" ? handleDesignChatCommand : undefined}
            designPendingTool={workspaceKind === "design" ? archToolPending : null}
            designApproveBusy={archToolApproveBusy}
            onApproveDesignTool={workspaceKind === "design" ? handleApproveArchTool : undefined}
            onDiscardDesignTool={workspaceKind === "design" ? handleDiscardArchTool : undefined}
          />
        </div>
      )}
    </main>
  );
}
