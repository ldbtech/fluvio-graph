"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  startTransition,
  type KeyboardEvent,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { BrainDomainTabs } from "./components/workspace/BrainDomainTabs";
import { BrainFusionLoadingMock } from "./components/workspace/BrainFusionLoadingMock";
import { ConnectorSidebar } from "./components/workspace/ConnectorSidebar";
import { GraphErrorBoundary } from "./components/workspace/GraphErrorBoundary";
import { GraphCanvas } from "./components/workspace/GraphCanvas";
import { GithubRepoFileTree } from "./components/workspace/GithubRepoFileTree";
import { SolarSystemCanvas } from "./components/workspace/SolarSystemCanvas";
import { WorkspaceSurfacePanel } from "./components/workspace/WorkspaceSurfacePanel";
import { WorkspaceRightPanel } from "./components/workspace/WorkspaceRightPanel";
import { DesignBlueprintMock } from "./components/workspace/DesignBlueprintMock";
import { WorkspaceTopChrome } from "./components/workspace/WorkspaceTopChrome";
import { KG_URL } from "@/lib/constants";
import { fetchGraphMeta, fetchGraphWorkspace } from "@/lib/fetchGraphWorkspace";
import { filterGraphBySource, filterLiveEmailGraph } from "@/lib/graphFilters";
import { fetchCodebaseGalaxyTree } from "@/lib/fetchCodebaseGalaxy";
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
import { DESIGN_CONNECTOR_IDS, INVEST_CONNECTOR_IDS, PERSONAL_CONNECTOR_IDS } from "@/lib/workspaceKinds";

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

export default function Home() {
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
  /** Galaxy tree for GitHub brain main graph (no mock graph). */
  const [githubGalaxyTree, setGithubGalaxyTree] = useState<CodebaseModuleTree | null>(null);
  /** Vertical pixel height of the solar-system band vs graph (GitHub brain only); drag the handle between them. */
  const [githubGalaxyBandPx, setGithubGalaxyBandPx] = useState(280);
  const githubBrainColumnRef = useRef<HTMLDivElement>(null);

  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("sources");
  const [workspaceKind, setWorkspaceKind] = useState<WorkspaceKind>("personal");
  const [brainTab, setBrainTab] = useState<BrainTab>("documents");

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
      `GitHub: cloned ${info.owner}/${info.repo} (${info.was_cloned ? "new" : "pull"}) — open brain → GitHub for the solar system; click a planet to zoom and open the small module graph.`,
    );
  }, []);

  const onGithubGalaxySplitterDown = useCallback((e: ReactMouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const col = githubBrainColumnRef.current;
    if (!col) return;
    const startY = e.clientY;
    const startPx = githubGalaxyBandPx;
    const onMove = (ev: globalThis.MouseEvent) => {
      const rect = col.getBoundingClientRect();
      const reserveTabsAndGraph = 200;
      const maxPx = Math.max(140, rect.height - reserveTabsAndGraph);
      const minPx = 120;
      const next = Math.min(maxPx, Math.max(minPx, startPx + (ev.clientY - startY)));
      setGithubGalaxyBandPx(next);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }, [githubGalaxyBandPx]);

  const livePdf = useMemo(
    () => filterGraphBySource(nodes, edges, "pdf"),
    [nodes, edges],
  );
  const liveEmail = useMemo(() => filterLiveEmailGraph(nodes, edges), [nodes, edges]);

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

  const personalPreviewCount = useMemo(
    () => PERSONAL_CONNECTOR_IDS.filter((id) => connectorStatus[id] === "mock_on").length,
    [connectorStatus],
  );

  const investPreviewCount = useMemo(
    () => INVEST_CONNECTOR_IDS.filter((id) => connectorStatus[id] === "mock_on").length,
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
        if (!githubGalaxyTree) return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
        return moduleSubtreeToGraph(githubGalaxyTree, 420);
      }
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
    gmailConnected,
    emailNodeTotal,
    githubGalaxyTree,
  ]);

  const showBrainLoading =
    !graphLoading &&
    ((brainTab === "unified" && !fusionReady) || (brainTab === "meta" && !metaReady));

  const brainGraphEmpty =
    brainTab !== "unified" &&
    brainTab !== "meta" &&
    brainGraph.nodes.length === 0;

  const chatSource =
    workspaceKind === "personal" &&
    ((brainTab === "documents" && pdfNodeTotal > 0) ||
      (brainTab === "gmail" && emailNodeTotal > 0) ||
      (brainTab === "github" && githubGalaxyTree !== null))
      ? "live"
      : "mock";

  const pickDefaultBrainDomain = useCallback((): BrainTab => {
    if (pdfNodeTotal > 0) return "documents";
    if (emailNodeTotal > 0) return "gmail";
    const first = PERSONAL_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on");
    return first ?? "documents";
  }, [pdfNodeTotal, emailNodeTotal, connectorStatus]);

  const pickDefaultInvestBrainTab = useCallback((): BrainTab => {
    const first = INVEST_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on");
    return first ?? "equities";
  }, [connectorStatus]);

  const pickDefaultDesignBrainTab = useCallback((): BrainTab => {
    const first = DESIGN_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on");
    return first ?? "des_bim";
  }, [connectorStatus]);

  useEffect(() => {
    setBrainTab((prev) => {
      if (prev === "unified" || prev === "meta") return prev;

      if (workspaceKind === "invest") {
        const invalid =
          prev === "documents" ||
          PERSONAL_CONNECTOR_IDS.includes(prev as ConnectorId) ||
          DESIGN_CONNECTOR_IDS.includes(prev as ConnectorId);
        if (invalid) return pickDefaultInvestBrainTab();
        return prev;
      }

      if (workspaceKind === "design") {
        const invalid =
          prev === "documents" ||
          PERSONAL_CONNECTOR_IDS.includes(prev as ConnectorId) ||
          INVEST_CONNECTOR_IDS.includes(prev as ConnectorId);
        if (invalid) return pickDefaultDesignBrainTab();
        return prev;
      }

      if (INVEST_CONNECTOR_IDS.includes(prev as ConnectorId) || DESIGN_CONNECTOR_IDS.includes(prev as ConnectorId)) {
        if (pdfNodeTotal > 0) return "documents";
        if (emailNodeTotal > 0) return "gmail";
        return PERSONAL_CONNECTOR_IDS.find((id) => connectorStatus[id] === "mock_on") ?? "documents";
      }
      return prev;
    });
  }, [
    workspaceKind,
    pdfNodeTotal,
    emailNodeTotal,
    connectorStatus,
    pickDefaultInvestBrainTab,
    pickDefaultDesignBrainTab,
  ]);

  const onWorkspaceKindChange = useCallback((kind: WorkspaceKind) => {
    setWorkspaceKind(kind);
    setCenterPanel((c) => {
      if (c === null) return c;
      if (kind === "invest" && (c === "documents" || PERSONAL_CONNECTOR_IDS.includes(c))) return null;
      if (kind === "design" && (c === "documents" || PERSONAL_CONNECTOR_IDS.includes(c) || INVEST_CONNECTOR_IDS.includes(c)))
        return null;
      if (kind === "personal" && (INVEST_CONNECTOR_IDS.includes(c as ConnectorId) || DESIGN_CONNECTOR_IDS.includes(c as ConnectorId)))
        return null;
      return c;
    });
  }, []);

  const onWorkspaceModeChange = useCallback(
    (mode: WorkspaceMode) => {
      if (mode === "brain") {
        setBrainTab(
          workspaceKind === "invest"
            ? pickDefaultInvestBrainTab()
            : workspaceKind === "design"
              ? pickDefaultDesignBrainTab()
              : pickDefaultBrainDomain(),
        );
        if (nodes.length === 0) {
          setGraphLoading(true);
          setGraphLoadProgress({ message: "Opening workspace brain…", percent: 1 });
        }
      }
      setWorkspaceMode(mode);
    },
    [workspaceKind, pickDefaultBrainDomain, pickDefaultInvestBrainTab, pickDefaultDesignBrainTab, nodes.length],
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
    <main className="relative h-screen w-screen overflow-hidden bg-zinc-950 pt-12 text-zinc-200 select-none">
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
        investPreviewCount={investPreviewCount}
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

            {centerPanel === null &&
              (workspaceKind === "invest" || workspaceKind === "design") &&
              phase === "idle" && (
                <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-zinc-950/40 px-8 text-center backdrop-blur-sm">
                  <p className="max-w-md text-[15px] font-medium leading-relaxed text-zinc-300">
                    {workspaceKind === "invest"
                      ? "Pick a markets feed on the left to open its setup panel, then enable previews for graph tabs."
                      : "Pick a design source on the left — BIM, structural, civil, codes, or physics — then open Brain to explore mock graphs for each slice."}
                  </p>
                  <p className="max-w-sm text-[12px] leading-relaxed text-zinc-600">
                    Live ingestion for this workspace is not wired yet; the UI sketches how architecture and civil
                    knowledge will fuse so future models can prove loads and physics before anything is built.
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
        <div className="flex h-[calc(100vh-3rem)] w-full min-h-0">
          {brainTab === "github" && connectorStatus.github === "mock_on" && (
            <GithubRepoFileTree
              kgUrl={KG_URL}
              cloneInfo={githubCloneInfo}
              className="hidden w-[min(100%,272px)] shrink-0 sm:flex"
            />
          )}
          <div ref={githubBrainColumnRef} className="flex min-h-0 min-w-0 flex-1 flex-col">
            {brainTab === "github" && connectorStatus.github === "mock_on" && (
              <>
                <div
                  className="flex min-h-0 shrink-0 flex-col overflow-hidden border-b border-white/[0.06]"
                  style={{ height: githubGalaxyBandPx }}
                >
                  <SolarSystemCanvas
                    kgUrl={KG_URL}
                    cloneInfo={githubCloneInfo}
                    className="min-h-0 flex-1"
                    onPlanetIngestComplete={fetchGraph}
                  />
                </div>
                <div
                  role="separator"
                  aria-orientation="horizontal"
                  aria-label="Resize solar system and graph"
                  tabIndex={0}
                  onMouseDown={onGithubGalaxySplitterDown}
                  onKeyDown={(ke: KeyboardEvent<HTMLDivElement>) => {
                    const col = githubBrainColumnRef.current;
                    if (!col) return;
                    const rect = col.getBoundingClientRect();
                    const maxPx = Math.max(140, rect.height - 200);
                    const step = 20;
                    if (ke.key === "ArrowUp" || ke.key === "ArrowDown") {
                      ke.preventDefault();
                      setGithubGalaxyBandPx((p) =>
                        ke.key === "ArrowUp"
                          ? Math.min(maxPx, p + step)
                          : Math.max(120, p - step),
                      );
                    }
                  }}
                  className="group relative z-30 flex h-3 shrink-0 cursor-row-resize items-center justify-center border-b border-white/[0.06] bg-zinc-900/80 outline-none hover:bg-zinc-800 focus-visible:ring-2 focus-visible:ring-sky-500/40"
                >
                  <span className="h-1 w-12 rounded-full bg-zinc-600 transition group-hover:bg-zinc-400" />
                </div>
              </>
            )}
            <BrainDomainTabs
              workspaceKind={workspaceKind}
              active={brainTab}
              onChange={setBrainTab}
              documentGraphReady={documentGraphReady}
              gmailLiveReady={emailNodeTotal > 0}
              gmailOAuthConnected={gmailConnected}
              connectorStatus={connectorStatus}
            />
            <div
              className={`relative min-h-0 flex-1 ${workspaceKind === "design" ? "flex flex-col lg:flex-row" : ""}`}
            >
              <div className="relative min-h-0 min-w-0 flex-1 overflow-hidden">
                {graphLoading && (
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
                  {!graphLoading && (
                    <GraphErrorBoundary>
                      <GraphCanvas
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
                          ? "No module graph yet. In Sources → GitHub, clone a public repo (kg-engine must reach your clone), then return here for the galaxy, file tree, and live tree graph."
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
                              <span className="text-zinc-500">s.{n.node.page}</span>
                              <div className="flex items-center gap-3">
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

              {workspaceKind === "design" && <DesignBlueprintMock brainTab={brainTab} />}
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
          />
        </div>
      )}
    </main>
  );
}
