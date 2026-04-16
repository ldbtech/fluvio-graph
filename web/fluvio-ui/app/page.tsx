"use client";
import { useEffect, useRef, useState, useCallback } from "react";
import * as d3 from "d3";

const KG_URL = "http://localhost:8001";

interface Node {
  id: string;
  label: string;
  page: string;
  source: string;
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

interface Edge {
  from: string;
  to: string;
  token: number;
  probability: number;
  source?: Node;
  target?: Node;
}

interface SelectedNode {
  node: Node;
  neighbors: { node: Node; token: number; probability: number }[];
}

interface Message {
  role: "user" | "assistant";
  content: string;
}

export default function Home() {
  const svgRef       = useRef<SVGSVGElement>(null);
  const inputRef     = useRef<HTMLInputElement>(null);
  const messagesRef  = useRef<HTMLDivElement>(null);

  const [nodes,     setNodes]     = useState<Node[]>([]);
  const [edges,     setEdges]     = useState<Edge[]>([]);
  const [selected,  setSelected]  = useState<SelectedNode | null>(null);
  const [chatOpen,  setChatOpen]  = useState(false);
  const [messages,  setMessages]  = useState<Message[]>([]);
  const [input,     setInput]     = useState("");
  const [loading,   setLoading]   = useState(false);
  const [progress,  setProgress]  = useState(0);       // 0-100
  const [phase,     setPhase]     = useState<
    "idle" | "uploading" | "processing" | "finalizing"
  >("idle");
  const [error,     setError]     = useState<string|null>(null);

  // Auto-scroll chat
  useEffect(() => {
    if (messagesRef.current) {
      messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
    }
  }, [messages, loading]);

  const fetchGraph = useCallback(async () => {
    try {
      const res  = await fetch(`${KG_URL}/graph`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      console.log("Graph fetched:", data.nodes.length, "nodes", data.edges.length, "edges");
      setNodes(data.nodes ?? []);
      setEdges(data.edges ?? []);
    } catch (e) {
      console.error("fetchGraph failed:", e);
      setError("cannot reach kg-engine at :8001 — is it running?");
    }
  }, []);

  useEffect(() => { fetchGraph(); }, [fetchGraph]);

  // ── File upload with animated progress ───────────────────────────────────
  const onFileSelect = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.name.endsWith(".pdf")) {
      setError("only PDF files are supported");
      return;
    }

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

        // 0–48%: bytes on the wire. After upload, 50–96%: server embedding (same HTTP request).
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
            reject(new Error(`server returned ${xhr.status}: ${xhr.responseText || xhr.statusText}`));
          }
        };

        xhr.onerror = () => {
          clearServerPulse();
          reject(new Error("network error — is kg-engine running on :8001?"));
        };

        xhr.onabort = () => {
          clearServerPulse();
          reject(new Error("upload aborted"));
        };

        xhr.open("POST", `${KG_URL}/ingest/pdf`);
        xhr.send(form);

        // If the browser never reports upload size, still show motion after a tick
        requestAnimationFrame(() => {
          setProgress((prev) => (prev === 0 ? 3 : prev));
        });
      });

      setPhase("finalizing");
      await fetchGraph();
      setPhase("idle");
    } catch (err: unknown) {
      console.error("Upload failed:", err);
      const message = err instanceof Error ? err.message : String(err);
      setError(`upload failed: ${message}`);
      setPhase("idle");
      setProgress(0);
    } finally {
      e.target.value = "";
    }
  }, [fetchGraph]);

  // ── D3 brain graph ────────────────────────────────────────────────────────
  useEffect(() => {
    if (!svgRef.current) return;
    if (nodes.length === 0) return;

    console.log("Rendering D3 graph:", nodes.length, "nodes");

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const W = svgRef.current.clientWidth  || window.innerWidth;
    const H = svgRef.current.clientHeight || window.innerHeight;

    const defs = svg.append("defs");

    // Node glow
    const gf = defs.append("filter").attr("id", "glow");
    gf.append("feGaussianBlur").attr("stdDeviation", "4").attr("result", "blur");
    const gm = gf.append("feMerge");
    gm.append("feMergeNode").attr("in", "blur");
    gm.append("feMergeNode").attr("in", "SourceGraphic");

    // Edge glow (stronger “synapse” look)
    const ef = defs.append("filter").attr("id", "edgeGlow").attr("x", "-50%").attr("y", "-50%").attr("width", "200%").attr("height", "200%");
    ef.append("feGaussianBlur").attr("stdDeviation", "3.5").attr("result", "blur");
    const em = ef.append("feMerge");
    em.append("feMergeNode").attr("in", "blur");
    em.append("feMergeNode").attr("in", "SourceGraphic");

    defs.append("style").text(`
      @keyframes nodePulse {
        0%, 100% { opacity: 1; transform: scale(1); }
        50%       { opacity: 0.45; transform: scale(1.35); }
      }
      .pulse-dot { transform-box: fill-box; transform-origin: center; animation: nodePulse 2.2s ease-in-out infinite; }
      @keyframes edgeFlow {
        0%   { stroke-opacity: 0.35; }
        50%  { stroke-opacity: 0.95; }
        100% { stroke-opacity: 0.35; }
      }
      .edge-glow-line { animation: edgeFlow 2.8s ease-in-out infinite; }
    `);

    const g = svg.append("g");

    svg.call(
      d3.zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.1, 4])
        .on("zoom", (ev) => g.attr("transform", ev.transform))
    );

    // Same object references as `nodes` — required so forceSimulation updates x/y on the
    // objects that forceLink uses (copies from `{ ...n }` break links: endpoints never move).
    const nodeMap = new Map(nodes.map((n) => [n.id, n]));
    const links = edges
      .map((e) => {
        const source = nodeMap.get(e.from);
        const target = nodeMap.get(e.to);
        if (!source || !target) return null;
        return { ...e, source, target };
      })
      .filter((l): l is NonNullable<typeof l> => l !== null) as any[];

    const sim = d3
      .forceSimulation(nodes as any)
      .force("link",      d3.forceLink(links).id((d: any) => d.id).distance((d: any) => 60 + (1 - d.probability) * 100))
      .force("charge",    d3.forceManyBody().strength(-250))
      .force("center",    d3.forceCenter(W / 2, H / 2))
      .force("collision", d3.forceCollide(32))
      .alphaDecay(0.02);

    // ── Edges (glowing “synapse” links) ──
    const link = g.append("g")
      .selectAll<SVGLineElement, any>("line")
      .data(links)
      .join("line")
      .attr("class", "edge-glow-line")
      .attr("stroke", (d) => d.probability > 0.7 ? "#5dfff8" : d.probability > 0.5 ? "#9b85ff" : "#4a3d7a")
      .attr("stroke-width",   (d) => Math.max(1, d.probability * 2.5))
      .attr("stroke-opacity", (d) => 0.35 + d.probability * 0.55)
      .attr("stroke-linecap", "round")
      .attr("style", (_d: unknown, i: number) => `animation-delay: ${(i % 28) * 0.08}s`)
      .attr("filter", "url(#edgeGlow)");

    // ── Nodes ──
    const node = g.append("g")
      .selectAll<SVGGElement, Node>("g")
      .data(nodes)
      .join("g")
      .attr("cursor", "pointer")
      .call(
        d3.drag<SVGGElement, Node>()
          .on("start", (ev, d) => { if (!ev.active) sim.alphaTarget(0.3).restart(); (d as any).fx = d.x; (d as any).fy = d.y; })
          .on("drag",  (ev, d) => { (d as any).fx = ev.x; (d as any).fy = ev.y; })
          .on("end",   (ev, d) => { if (!ev.active) sim.alphaTarget(0); (d as any).fx = null; (d as any).fy = null; })
      )
      .on("click", (ev, d) => {
        ev.stopPropagation();
        const neighbors = links
          .filter((l) => l.source.id === d.id)
          .map((l) => ({ node: l.target, token: l.token, probability: l.probability }));
        setSelected({ node: d, neighbors });
      });

    // Outer halo
    node.append("circle")
      .attr("r", 20).attr("fill", "none")
      .attr("stroke", "#00fff2").attr("stroke-width", 0.5)
      .attr("stroke-opacity", 0.15).attr("filter", "url(#glow)");

    // Mid ring
    node.append("circle")
      .attr("r", 12).attr("fill", "#060616")
      .attr("stroke", "#00fff2").attr("stroke-width", 1.2)
      .attr("filter", "url(#glow)");

    // Pulse core (neuron body)
    node.append("circle")
      .attr("r", 4).attr("fill", "#00fff2")
      .attr("class", "pulse-dot");

    // Page label
    node.append("text")
      .text((d) => `p${d.page}`)
      .attr("x", 16).attr("y", 4)
      .attr("fill", "#4a7a9b")
      .attr("font-size", "9px")
      .attr("font-family", "monospace")
      .attr("pointer-events", "none");

    // Click background to deselect
    svg.on("click", () => setSelected(null));

    sim.on("tick", () => {
      link
        .attr("x1", (d) => d.source.x ?? 0).attr("y1", (d) => d.source.y ?? 0)
        .attr("x2", (d) => d.target.x ?? 0).attr("y2", (d) => d.target.y ?? 0);
      node.attr("transform", (d: any) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    return () => { sim.stop(); };
  }, [nodes, edges]);

  // ── Chat ──────────────────────────────────────────────────────────────────
  const sendMessage = useCallback(async () => {
    if (!input.trim() || loading) return;
    const question = input.trim();
    setInput("");
    setMessages((m) => [...m, { role: "user", content: question }]);
    setLoading(true);
    try {
      const res  = await fetch(`${KG_URL}/chat`, {
        method:  "POST",
        headers: { "Content-Type": "application/json" },
        body:    JSON.stringify({ question, history: messages }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setMessages((m) => [...m, { role: "assistant", content: data.answer }]);
    } catch (err: any) {
      setMessages((m) => [...m, { role: "assistant", content: `error: ${err.message}` }]);
    } finally {
      setLoading(false);
    }
  }, [input, loading, messages]);

  // ── Hexagon fill path (SVG) ───────────────────────────────────────────────
  const HEX_R = 80;
  // Fixed precision so SSR and browser produce identical `points` (avoids hydration mismatch).
  const hexPoints = (r: number) =>
    Array.from({ length: 6 }, (_, i) => {
      const a = (Math.PI / 3) * i - Math.PI / 6;
      const x = r + Math.cos(a) * r;
      const y = r + Math.sin(a) * r;
      return `${x.toFixed(4)},${y.toFixed(4)}`;
    }).join(" ");

  const fillPct = progress / 100;
  const hexH = HEX_R * 2;

  // ── Render ────────────────────────────────────────────────────────────────
  const isIngesting =
    phase === "uploading" || phase === "processing" || phase === "finalizing";

  return (
    <main className="w-screen h-screen bg-[#04040f] overflow-hidden relative select-none">

      {/* File input — visually hidden but accessible */}
      <input
        id="pdf-upload"
        type="file"
        accept=".pdf"
        onChange={onFileSelect}
        style={{
          position: "absolute",
          width: "1px",
          height: "1px",
          padding: 0,
          margin: "-1px",
          overflow: "hidden",
          clip: "rect(0, 0, 0, 0)",
          whiteSpace: "nowrap",
          border: 0,
        }}
      />

      {/* Graph canvas — always mounted */}
      <svg ref={svgRef} className="w-full h-full" />

      {/* ── IDLE EMPTY STATE ── */}
      {phase === "idle" && nodes.length === 0 && (
        <label
          htmlFor="pdf-upload"
          className="absolute inset-0 flex flex-col items-center justify-center cursor-pointer group"
        >
          <div className="relative mb-8">
            <svg width={HEX_R * 2} height={HEX_R * 2} viewBox={`0 0 ${HEX_R*2} ${HEX_R*2}`}>
              <polygon
                points={hexPoints(HEX_R)}
                fill="none"
                stroke="#00fff2"
                strokeWidth="1"
                strokeOpacity="0.25"
                className="group-hover:stroke-opacity-60 transition-all duration-500"
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
                className="text-[#00fff2]/40 text-5xl group-hover:text-[#00fff2]/80
                           transition-all duration-500"
                style={{ filter: "drop-shadow(0 0 16px #00fff260)" }}
              >
                +
              </span>
            </div>
          </div>
          <p className="text-[#7b9fc4]/60 text-sm font-mono group-hover:text-[#7b9fc4] transition-colors">
            click to upload a PDF
          </p>
          <p className="text-[#7b9fc4]/20 text-xs font-mono mt-2">
            transforms your document into a knowledge graph
          </p>
          {error && (
            <p className="mt-4 text-red-400/80 text-xs font-mono px-4 text-center">{error}</p>
          )}
        </label>
      )}

      {/* ── INGESTING STATE — hex tank fills like liquid / oxygen ── */}
      {isIngesting && (
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none z-10">
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
                className="text-[#d4fbff] text-xl font-mono font-bold tabular-nums"
                style={{ textShadow: "0 0 18px #00fff2aa" }}
              >
                {progress}%
              </span>
            </div>
          </div>

          <p className="text-[#7df9ff]/90 text-sm font-mono">
            {phase === "uploading" && "uploading PDF…"}
            {phase === "processing" && "embedding & building graph…"}
            {phase === "finalizing" && "wiring neuron view…"}
          </p>
          <p className="text-[#7b9fc4]/40 text-xs font-mono mt-1">
            fill completes, then the graph lights up
          </p>
        </div>
      )}

      {/* ── GRAPH EXISTS — top bar ── */}
      {nodes.length > 0 && phase === "idle" && (
        <div className="absolute top-4 left-1/2 -translate-x-1/2 flex items-center gap-3
                        px-4 py-2 rounded-full bg-[#0a0a1a]/80 border border-[#00fff2]/20
                        backdrop-blur-sm pointer-events-none">
          <span className="w-2 h-2 rounded-full bg-[#00fff2] animate-pulse" />
          <span className="text-[#7b9fc4] text-xs font-mono">
            {nodes.length} nodes · {edges.length} edges
          </span>
        </div>
      )}

      {/* Add PDF button when graph exists */}
      {nodes.length > 0 && phase === "idle" && (
        <label
          htmlFor="pdf-upload"
          className="absolute top-4 right-4 px-3 py-1.5 rounded-full cursor-pointer
                     bg-[#0a0a1a]/80 border border-[#00fff2]/20 text-[#00fff2]
                     text-xs font-mono hover:border-[#00fff2]/60 hover:bg-[#00fff2]/10
                     transition-all backdrop-blur-sm"
        >
          + add PDF
        </label>
      )}

      {/* Error banner */}
      {error && phase === "idle" && nodes.length > 0 && (
        <div className="absolute bottom-24 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg
                        bg-red-900/40 border border-red-500/30 text-red-400 text-xs font-mono">
          {error}
        </div>
      )}

      {/* ── NODE DETAIL PANEL ── */}
      {selected && (
        <div className="absolute top-16 left-4 w-72 bg-[#060616]/95 border border-[#00fff2]/20
                        rounded-xl p-4 backdrop-blur-md font-mono"
             onClick={(e) => e.stopPropagation()}>
          <div className="flex justify-between items-center mb-3">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-[#00fff2]"
                    style={{ boxShadow: "0 0 6px #00fff2" }} />
              <span className="text-[#00fff2] text-xs">page {selected.node.page}</span>
            </div>
            <button onClick={() => setSelected(null)}
                    className="text-[#7b9fc4]/50 hover:text-white transition-colors text-xs">
              ✕
            </button>
          </div>

          <p className="text-[#8ab4cc] text-xs leading-relaxed mb-4 line-clamp-5 border-l
                        border-[#00fff2]/20 pl-3">
            {selected.node.label}
          </p>

          {selected.neighbors.length > 0 && (
            <div className="mb-3">
              <p className="text-[#7b9fc4]/60 text-xs mb-2 uppercase tracking-wider">
                connections
              </p>
              <div className="space-y-1">
                {selected.neighbors.map((n, i) => (
                  <div key={i} className="flex items-center justify-between
                                          bg-[#ffffff06] rounded-lg px-3 py-1.5 text-xs">
                    <span className="text-[#8ab4cc]">p.{n.node.page}</span>
                    <div className="flex items-center gap-3">
                      <span className="text-[#7b61ff]">{n.token}t</span>
                      <div className="flex items-center gap-1">
                        <div className="w-12 h-1 rounded-full bg-[#ffffff10] overflow-hidden">
                          <div
                            className="h-full rounded-full bg-[#00fff2]"
                            style={{ width: `${n.probability * 100}%`,
                                     boxShadow: "0 0 4px #00fff2" }}
                          />
                        </div>
                        <span className="text-[#00fff2] w-8 text-right">
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
            onClick={() => {
              setChatOpen(true);
              setInput(`Tell me about page ${selected.node.page}: ${selected.node.label.slice(0, 60)}`);
            }}
            className="w-full py-2 rounded-lg border border-[#00fff2]/30 text-[#00fff2]
                       text-xs hover:bg-[#00fff2]/10 transition-colors mt-1"
          >
            ask about this node →
          </button>
        </div>
      )}

      {/* ── CHAT TOGGLE ── */}
      <button
        onClick={() => setChatOpen((o) => !o)}
        className="absolute bottom-6 right-6 w-14 h-14 rounded-full flex items-center
                   justify-center text-[#04040f] font-bold text-lg transition-transform
                   hover:scale-110 active:scale-95"
        style={{
          background: "linear-gradient(135deg, #00fff2, #7b61ff)",
          boxShadow: "0 0 24px #00fff260, 0 0 48px #7b61ff30",
        }}
      >
        {chatOpen ? "✕" : "💬"}
      </button>

      {/* ── CHAT PANEL ── */}
      {chatOpen && (
        <div className="absolute bottom-24 right-6 w-80 h-[440px] flex flex-col
                        bg-[#060616]/98 border border-[#00fff2]/20 rounded-2xl
                        backdrop-blur-xl overflow-hidden"
             style={{ boxShadow: "0 0 40px #00fff210" }}>

          <div className="px-4 py-3 border-b border-[#ffffff08] flex items-center justify-between
                          bg-[#00fff208]">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-[#00fff2] animate-pulse" />
              <span className="text-[#00fff2] text-xs font-mono">knowledge graph chat</span>
            </div>
            <span className="text-[#7b9fc4]/40 text-xs font-mono">{nodes.length} nodes</span>
          </div>

          <div ref={messagesRef} className="flex-1 overflow-y-auto p-3 space-y-3">
            {messages.length === 0 && (
              <div className="flex flex-col items-center justify-center h-full gap-2 opacity-40">
                <span className="text-4xl">⬡</span>
                <p className="text-[#7b9fc4] text-xs font-mono text-center">
                  ask anything about<br/>your document
                </p>
              </div>
            )}
            {messages.map((m, i) => (
              <div key={i} className={`text-xs font-mono rounded-xl px-3 py-2.5
                                       leading-relaxed break-words ${
                m.role === "user"
                  ? "bg-[#00fff2]/10 text-[#00fff2] ml-8 border border-[#00fff2]/10"
                  : "bg-[#ffffff06] text-[#c2d4e8] mr-8 border border-[#ffffff08]"
              }`}>
                {m.content}
              </div>
            ))}
            {loading && (
              <div className="bg-[#ffffff06] text-[#7b9fc4] text-xs font-mono
                              rounded-xl px-3 py-2.5 mr-8 border border-[#ffffff08]">
                <span className="animate-pulse">thinking</span>
                <span className="animate-bounce">...</span>
              </div>
            )}
          </div>

          <div className="p-3 border-t border-[#ffffff08] flex gap-2 bg-[#00fff205]">
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendMessage(); } }}
              placeholder="ask your graph..."
              className="flex-1 bg-[#ffffff08] rounded-xl px-3 py-2 text-xs font-mono
                         text-[#c2d4e8] placeholder-[#7b9fc4]/25 outline-none
                         border border-[#ffffff10] focus:border-[#00fff2]/40
                         transition-colors"
            />
            <button
              onClick={sendMessage}
              disabled={loading || !input.trim()}
              className="px-3 py-2 rounded-xl text-xs font-mono font-bold
                         disabled:opacity-20 transition-all active:scale-95"
              style={{
                background: loading || !input.trim()
                  ? "transparent"
                  : "linear-gradient(135deg, #00fff2, #7b61ff)",
                color: loading || !input.trim() ? "#7b9fc4" : "#04040f",
                border: "1px solid #00fff230",
              }}
            >
              →
            </button>
          </div>
        </div>
      )}
    </main>
  );
}