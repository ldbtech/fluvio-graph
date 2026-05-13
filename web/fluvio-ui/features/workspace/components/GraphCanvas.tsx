"use client";

import { useEffect, useReducer } from "react";
import * as d3 from "d3";
import type { GraphEdge, GraphNode, SelectedNode } from "@/shared/lib/types";

type Props = {
  svgRef: React.RefObject<SVGSVGElement | null>;
  nodes: GraphNode[];
  edges: GraphEdge[];
  onSelect: (s: SelectedNode | null) => void;
};

export function GraphCanvas({ svgRef, nodes, edges, onSelect }: Props) {
  /** Bumps when the SVG layout box changes so we re-center in real viewport coords (not window size). */
  const [layoutRev, bumpLayout] = useReducer((n: number) => n + 1, 0);

  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => bumpLayout());
    ro.observe(el);
    return () => ro.disconnect();
  }, [svgRef]);

  useEffect(() => {
    if (!svgRef.current) return;
    if (nodes.length === 0) {
      d3.select(svgRef.current).selectAll("*").remove();
      return;
    }

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const rect = svgRef.current.getBoundingClientRect();
    const W = Math.max(8, rect.width || svgRef.current.clientWidth, 320);
    const H = Math.max(8, rect.height || svgRef.current.clientHeight, 240);

    const defs = svg.append("defs");
    const gf = defs.append("filter").attr("id", "glow");
    gf.append("feGaussianBlur").attr("stdDeviation", "4").attr("result", "blur");
    const gm = gf.append("feMerge");
    gm.append("feMergeNode").attr("in", "blur");
    gm.append("feMergeNode").attr("in", "SourceGraphic");

    const ef = defs
      .append("filter")
      .attr("id", "edgeGlow")
      .attr("x", "-50%")
      .attr("y", "-50%")
      .attr("width", "200%")
      .attr("height", "200%");
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
      d3
        .zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.1, 4])
        .on("zoom", (ev) => g.attr("transform", ev.transform)),
    );

    const nodeMap = new Map(nodes.map((n) => [n.id, n]));
    type LinkDatum = GraphEdge & {
      source: GraphNode;
      target: GraphNode;
    };

    const links = edges
      .map((e) => {
        const source = nodeMap.get(e.from);
        const target = nodeMap.get(e.to);
        if (!source || !target) return null;
        return { ...e, source, target } as LinkDatum;
      })
      .filter((l): l is LinkDatum => l !== null);

    const n = nodes.length;
    const charge = n > 400 ? -90 : n > 150 ? -160 : -250;
    const sim = d3
      .forceSimulation(nodes as d3.SimulationNodeDatum[])
      .force(
        "link",
        d3
          .forceLink(links)
          .id((d: d3.SimulationNodeDatum) => (d as GraphNode).id)
          .distance((d) => 60 + (1 - (d as LinkDatum).probability) * 100),
      )
      .force("charge", d3.forceManyBody().strength(charge))
      .force("center", d3.forceCenter(W / 2, H / 2))
      .force("collision", d3.forceCollide(32))
      .alphaDecay(n > 300 ? 0.14 : 0.05)
      .alphaMin(0.09);

    const maxTicks = n > 500 ? 220 : 420;
    let tickCount = 0;

    const edgeStroke = (d: LinkDatum) => {
      const lab = (d.label ?? "").toLowerCase();
      if (lab === "imports") return "#22d3ee";
      if (lab === "contains") return "#c084fc";
      if (lab === "legacy") return "#78716c";
      if (lab === "tree") return "#64748b";
      if (lab === "semantic_neighbor") {
        return d.probability > 0.7 ? "#38bdf8" : d.probability > 0.5 ? "#a78bfa" : "#52525b";
      }
      if (lab) return "#94a3b8";
      return d.probability > 0.7 ? "#38bdf8" : d.probability > 0.5 ? "#a78bfa" : "#52525b";
    };

    const edgeTitle = (d: LinkDatum) => {
      const lab = d.label?.trim() || "edge";
      return `${lab} · p=${d.probability.toFixed(3)} · tok=${d.token}`;
    };

    const showEdgeMidLabel = (d: LinkDatum) => {
      const lab = (d.label ?? "").toLowerCase();
      if (!lab || lab === "semantic_neighbor") return false;
      if (nodes.length > 120 && lab === "legacy") return false;
      return true;
    };

    const edgeMidText = (d: LinkDatum) => {
      const raw = (d.label ?? "").trim();
      if (!raw) return "";
      if (raw.length > 14) return `${raw.slice(0, 12)}…`;
      return raw;
    };

    const linkLayer = g.append("g").attr("class", "links");

    const link = linkLayer
      .selectAll<SVGLineElement, LinkDatum>("line")
      .data(links)
      .join("line")
      .attr("class", "edge-glow-line")
      .attr("stroke", (d) => edgeStroke(d))
      .attr("stroke-width", (d) => Math.max(1, d.probability * 2.5))
      .attr("stroke-opacity", (d) => 0.35 + d.probability * 0.55)
      .attr("stroke-linecap", "round")
      .attr("stroke-dasharray", "none")
      .attr("style", (_d, i) => `animation-delay: ${(i % 28) * 0.08}s`)
      .attr("filter", "url(#edgeGlow)");

    link.each(function (d) {
      d3.select(this).append("title").text(edgeTitle(d));
    });

    const linkLabels = g
      .append("g")
      .attr("class", "edge-labels")
      .selectAll<SVGTextElement, LinkDatum>("text")
      .data(links.filter(showEdgeMidLabel))
      .join("text")
      .attr("fill", "#a1a1aa")
      .attr("font-size", "9px")
      .attr("font-family", "ui-monospace, monospace")
      .attr("text-anchor", "middle")
      .attr("pointer-events", "none")
      .attr("opacity", 0.88)
      .text((d) => edgeMidText(d));

    const node = g
      .append("g")
      .selectAll<SVGGElement, GraphNode>("g")
      .data(nodes)
      .join("g")
      .attr("cursor", "pointer")
      .call(
        d3
          .drag<SVGGElement, GraphNode>()
          .on("start", (ev, d) => {
            if (!ev.active) sim.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
          })
          .on("drag", (ev, d) => {
            d.fx = ev.x;
            d.fy = ev.y;
          })
          .on("end", (ev, d) => {
            if (!ev.active) sim.alphaTarget(0);
            d.fx = null;
            d.fy = null;
          }),
      )
      .on("click", (ev, d) => {
        ev.stopPropagation();
        const neighbors = links
          .filter((l) => l.source.id === d.id)
          .map((l) => ({
            node: l.target,
            token: l.token,
            probability: l.probability,
            label: l.label,
          }));
        onSelect({ node: d, neighbors });
      });

    node
      .append("circle")
      .attr("r", 20)
      .attr("fill", "none")
      .attr("stroke", "#a1a1aa")
      .attr("stroke-width", 0.5)
      .attr("stroke-opacity", 0.15)
      .attr("filter", "url(#glow)");

    node
      .append("circle")
      .attr("r", 12)
      .attr("fill", "#18181b")
      .attr("stroke", "#a1a1aa")
      .attr("stroke-width", 1.2)
      .attr("filter", "url(#glow)");

    node.append("circle").attr("r", 4).attr("fill", "#38bdf8").attr("class", "pulse-dot");

    node
      .append("text")
      .text((d) => `p${d.page}`)
      .attr("x", 16)
      .attr("y", 4)
      .attr("fill", "#71717a")
      .attr("font-size", "9px")
      .attr("font-family", "monospace")
      .attr("pointer-events", "none");

    svg.on("click", () => onSelect(null));

    sim.on("tick", () => {
      link
        .attr("x1", (d) => d.source.x ?? 0)
        .attr("y1", (d) => d.source.y ?? 0)
        .attr("x2", (d) => d.target.x ?? 0)
        .attr("y2", (d) => d.target.y ?? 0);
      linkLabels
        .attr("x", (d) => ((d.source.x ?? 0) + (d.target.x ?? 0)) / 2)
        .attr("y", (d) => ((d.source.y ?? 0) + (d.target.y ?? 0)) / 2 - 3);
      node.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
      tickCount += 1;
      if (tickCount >= maxTicks) {
        sim.stop();
      }
    });

    return () => {
      sim.stop();
    };
  }, [svgRef, nodes, edges, onSelect, layoutRev]);

  return <svg ref={svgRef} className="absolute inset-0 h-full w-full" />;
}
