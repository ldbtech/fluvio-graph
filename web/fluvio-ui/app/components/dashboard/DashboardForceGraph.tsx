"use client";

import { useEffect, useReducer, useRef } from "react";
import * as d3 from "d3";
import type { GraphEdge, GraphNode } from "@/lib/types";

type SimNode = GraphNode & d3.SimulationNodeDatum;

type Props = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** When set, this node id is pinned to center */
  centerId?: string;
  onNodeClick?: (id: string, label: string) => void;
  className?: string;
};

export function DashboardForceGraph({ nodes, edges, centerId, onNodeClick, className = "" }: Props) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const [, bump] = useReducer((n: number) => n + 1, 0);

  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => bump());
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const svgEl = svgRef.current;
    if (!svgEl || nodes.length === 0) return;

    const svg = d3.select(svgEl);
    svg.selectAll("*").remove();

    const rect = svgEl.getBoundingClientRect();
    const W = Math.max(200, rect.width);
    const H = Math.max(220, rect.height);
    svg.attr("viewBox", `0 0 ${W} ${H}`).attr("width", W).attr("height", H);

    const gRoot = svg.append("g");
    svg.call(
      d3
        .zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.2, 4])
        .on("zoom", (ev) => gRoot.attr("transform", ev.transform)),
    );

    const simNodes: SimNode[] = nodes.map((n) => ({ ...n }));
    const idMap = new Map(simNodes.map((n) => [n.id, n]));
    type LinkD = GraphEdge & { source: SimNode; target: SimNode };
    const simLinks: LinkD[] = edges
      .map((e) => {
        const s = idMap.get(e.from);
        const t = idMap.get(e.to);
        if (!s || !t) return null;
        return { ...e, source: s, target: t } as LinkD;
      })
      .filter((l): l is LinkD => l !== null);

    if (centerId) {
      const c = simNodes.find((n) => n.id === centerId);
      if (c) {
        c.fx = W / 2;
        c.fy = H / 2;
      }
    }

    const sim = d3
      .forceSimulation(simNodes)
      .force(
        "link",
        d3
          .forceLink(simLinks)
          .id((d: d3.SimulationNodeDatum) => (d as SimNode).id)
          .distance((d) => {
            const l = d as LinkD;
            return 56 + (1 - l.probability) * 72;
          })
          .strength(0.5),
      )
      .force("charge", d3.forceManyBody().strength(-180))
      .force("center", d3.forceCenter(W / 2, H / 2))
      .force("collision", d3.forceCollide(28));

    const linkSel = gRoot
      .append("g")
      .attr("stroke", "#534AB7")
      .attr("stroke-opacity", 0.35)
      .selectAll("line")
      .data(simLinks)
      .join("line")
      .attr("stroke-width", 1);

    const nodeSel = gRoot
      .append("g")
      .selectAll("g")
      .data(simNodes)
      .join("g")
      .style("cursor", onNodeClick ? "pointer" : "default")
      .on("click", (_ev, d) => {
        if (onNodeClick) onNodeClick(d.id, d.label);
      });

    const isCenter = (d: SimNode) => centerId && d.id === centerId;

    nodeSel
      .append("circle")
      .attr("r", (d) => (isCenter(d) ? 20 : 11))
      .attr("fill", (d) => (isCenter(d) ? "#534AB7" : "#1A1828"))
      .attr("stroke", (d) => (isCenter(d) ? "#AFA9EC" : "#7F77DD"))
      .attr("stroke-width", (d) => (isCenter(d) ? 2 : 1))
      .attr("opacity", (d) => (isCenter(d) ? 1 : 0.92));

    nodeSel
      .append("text")
      .attr("text-anchor", "middle")
      .attr("dy", (d) => (isCenter(d) ? 34 : 22))
      .attr("fill", "#c4c0ba")
      .attr("font-size", (d) => (isCenter(d) ? 11 : 9))
      .text((d) => (d.label.length > 22 ? `${d.label.slice(0, 20)}…` : d.label));

    sim.on("tick", () => {
      linkSel
        .attr("x1", (d) => (d.source as SimNode).x!)
        .attr("y1", (d) => (d.source as SimNode).y!)
        .attr("x2", (d) => (d.target as SimNode).x!)
        .attr("y2", (d) => (d.target as SimNode).y!);

      nodeSel.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    return () => {
      sim.stop();
    };
  }, [nodes, edges, centerId, onNodeClick, bump]);

  return <svg ref={svgRef} className={`size-full touch-none bg-[#07060c] ${className}`} />;
}
