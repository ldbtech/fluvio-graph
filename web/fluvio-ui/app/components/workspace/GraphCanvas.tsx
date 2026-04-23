"use client";

import { useEffect } from "react";
import * as d3 from "d3";
import type { QaItemStatus } from "@/lib/qaTypes";
import { qaEdgeKey } from "@/lib/qaTypes";
import type { GraphEdge, GraphNode, SelectedNode } from "@/lib/types";

type Props = {
  svgRef: React.RefObject<SVGSVGElement | null>;
  nodes: GraphNode[];
  edges: GraphEdge[];
  onSelect: (s: SelectedNode | null) => void;
  /** When set, node and edge strokes reflect human QA state. */
  qaNodeStatus?: Record<string, QaItemStatus>;
  qaEdgeStatus?: Record<string, QaItemStatus>;
};

export function GraphCanvas({ svgRef, nodes, edges, onSelect, qaNodeStatus, qaEdgeStatus }: Props) {
  useEffect(() => {
    if (!svgRef.current) return;
    if (nodes.length === 0) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const W = svgRef.current.clientWidth || window.innerWidth;
    const H = svgRef.current.clientHeight || window.innerHeight;

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
    const links = edges
      .map((e) => {
        const source = nodeMap.get(e.from);
        const target = nodeMap.get(e.to);
        if (!source || !target) return null;
        return { ...e, source, target };
      })
      .filter((l): l is NonNullable<typeof l> => l !== null) as {
      source: GraphNode;
      target: GraphNode;
      from: string;
      to: string;
      token: number;
      probability: number;
    }[];

    const n = nodes.length;
    const charge = n > 400 ? -90 : n > 150 ? -160 : -250;
    const sim = d3
      .forceSimulation(nodes as d3.SimulationNodeDatum[])
      .force(
        "link",
        d3
          .forceLink(links)
          .id((d: d3.SimulationNodeDatum) => (d as GraphNode).id)
          .distance((d) => 60 + (1 - (d as { probability: number }).probability) * 100),
      )
      .force("charge", d3.forceManyBody().strength(charge))
      .force("center", d3.forceCenter(W / 2, H / 2))
      .force("collision", d3.forceCollide(32))
      .alphaDecay(n > 300 ? 0.14 : 0.05)
      .alphaMin(0.09);

    const maxTicks = n > 500 ? 220 : 420;
    let tickCount = 0;

    const edgeStroke = (d: (typeof links)[0]) => {
      const st = qaEdgeStatus?.[qaEdgeKey(d.from, d.to)];
      if (st === "approved") return "#34d399";
      if (st === "rejected") return "#f87171";
      return d.probability > 0.7 ? "#38bdf8" : d.probability > 0.5 ? "#a78bfa" : "#52525b";
    };

    const link = g
      .append("g")
      .selectAll<SVGLineElement, (typeof links)[0]>("line")
      .data(links)
      .join("line")
      .attr("class", (d) =>
        qaEdgeStatus?.[qaEdgeKey(d.from, d.to)] === "rejected" ? "" : "edge-glow-line",
      )
      .attr("stroke", (d) => edgeStroke(d))
      .attr("stroke-width", (d) => Math.max(1, d.probability * 2.5))
      .attr("stroke-opacity", (d) => {
        const st = qaEdgeStatus?.[qaEdgeKey(d.from, d.to)];
        if (st === "rejected") return 0.45;
        return 0.35 + d.probability * 0.55;
      })
      .attr("stroke-linecap", "round")
      .attr("stroke-dasharray", (d) =>
        qaEdgeStatus?.[qaEdgeKey(d.from, d.to)] === "rejected" ? "5 4" : "none",
      )
      .attr("style", (_d, i) => `animation-delay: ${(i % 28) * 0.08}s`)
      .attr("filter", (d) =>
        qaEdgeStatus?.[qaEdgeKey(d.from, d.to)] === "rejected" ? "none" : "url(#edgeGlow)",
      );

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
          .map((l) => ({ node: l.target, token: l.token, probability: l.probability }));
        onSelect({ node: d, neighbors });
      });

    const nodeInnerStroke = (d: GraphNode) => {
      const st = qaNodeStatus?.[d.id];
      if (st === "approved") return "#34d399";
      if (st === "rejected") return "#f87171";
      return "#a1a1aa";
    };

    node
      .append("circle")
      .attr("r", 20)
      .attr("fill", "none")
      .attr("stroke", (d) => nodeInnerStroke(d))
      .attr("stroke-width", 0.5)
      .attr("stroke-opacity", (d) => (qaNodeStatus?.[d.id] ? 0.28 : 0.15))
      .attr("filter", "url(#glow)");

    const nodeInnerFill = (d: GraphNode) => {
      const st = qaNodeStatus?.[d.id];
      if (st === "approved") return "#022c1f";
      if (st === "rejected") return "#2c0a0a";
      return "#18181b";
    };

    const nodeDotFill = (d: GraphNode) => {
      const st = qaNodeStatus?.[d.id];
      if (st === "approved") return "#6ee7b7";
      if (st === "rejected") return "#fca5a5";
      return "#38bdf8";
    };

    node
      .append("circle")
      .attr("r", 12)
      .attr("fill", (d) => nodeInnerFill(d))
      .attr("stroke", (d) => nodeInnerStroke(d))
      .attr("stroke-width", 1.2)
      .attr("filter", "url(#glow)");

    node
      .append("circle")
      .attr("r", 4)
      .attr("fill", (d) => nodeDotFill(d))
      .attr("class", "pulse-dot");

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
      node.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
      tickCount += 1;
      if (tickCount >= maxTicks) {
        sim.stop();
      }
    });

    return () => {
      sim.stop();
    };
  }, [svgRef, nodes, edges, onSelect, qaNodeStatus, qaEdgeStatus]);

  return <svg ref={svgRef} className="absolute inset-0 h-full w-full" />;
}
