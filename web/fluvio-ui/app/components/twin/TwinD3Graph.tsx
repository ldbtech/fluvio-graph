"use client";

import { useEffect, useId, useReducer, useRef } from "react";
import * as d3 from "d3";
import type { TwinGraphPayload } from "@/lib/twinGraphStore";

type GNode = {
  id: string;
  label: string;
  group: "center" | "satellite";
} & d3.SimulationNodeDatum;

type Props = {
  graph: TwinGraphPayload;
  selectedId: string | null;
  onSelectNode: (id: string, label: string) => void;
};

export function TwinD3Graph({ graph, selectedId, onSelectNode }: Props) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const highlightRef = useRef<(id: string | null) => void>(() => {});
  const selectedIdRef = useRef<string | null>(null);
  const reactId = useId().replace(/:/g, "");
  const [, bumpLayout] = useReducer((n: number) => n + 1, 0);

  selectedIdRef.current = selectedId;

  useEffect(() => {
    highlightRef.current(selectedIdRef.current);
  }, [selectedId]);

  useEffect(() => {
    const svgEl = svgRef.current;
    if (!svgEl) return;
    const ro = new ResizeObserver(() => bumpLayout());
    ro.observe(svgEl);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const svgEl = svgRef.current;
    if (!svgEl || graph.nodes.length === 0) return;

    const filterId = `twin-d3-glow-${reactId}`;

    const svg = d3.select(svgEl);
    svg.selectAll("*").remove();

    const rect = svgEl.getBoundingClientRect();
    const W = Math.max(200, rect.width);
    const H = Math.max(200, rect.height);

    svg.attr("viewBox", `0 0 ${W} ${H}`).attr("width", W).attr("height", H);

    const defs = svg.append("defs");
    const glow = defs.append("filter").attr("id", filterId);
    glow.append("feGaussianBlur").attr("stdDeviation", "3").attr("result", "b");
    const mg = glow.append("feMerge");
    mg.append("feMergeNode").attr("in", "b");
    mg.append("feMergeNode").attr("in", "SourceGraphic");

    defs.append("style").text(`
      @keyframes twinD3Pulse {
        0%, 100% { opacity: 0.55; }
        50% { opacity: 1; }
      }
      .twin-d3-pulse { animation: twinD3Pulse 2.4s ease-in-out infinite; }
    `);

    const gRoot = svg.append("g");
    svg.call(
      d3
        .zoom<SVGSVGElement, unknown>()
        .scaleExtent([0.35, 3])
        .on("zoom", (ev) => gRoot.attr("transform", ev.transform)),
    );

    const simNodes: GNode[] = graph.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      group: n.id === "ali" ? ("center" as const) : ("satellite" as const),
    }));
    const idToNode = new Map(simNodes.map((n) => [n.id, n]));
    const simLinks = graph.edges
      .map((l) => {
        const s = idToNode.get(l.from);
        const t = idToNode.get(l.to);
        if (!s || !t) return null;
        return { source: s, target: t };
      })
      .filter((x): x is { source: GNode; target: GNode } => x !== null);

    const center = simNodes.find((n) => n.id === "ali");
    if (center) {
      center.fx = W / 2;
      center.fy = H / 2;
    }

    const sim = d3
      .forceSimulation(simNodes)
      .force(
        "link",
        d3
          .forceLink(simLinks)
          .id((d) => (d as GNode).id)
          .distance(64)
          .strength(0.5),
      )
      .force("charge", d3.forceManyBody().strength(-200))
      .force("center", d3.forceCenter(W / 2, H / 2))
      .force("collision", d3.forceCollide(36));

    const linkLayer = gRoot.append("g").attr("stroke", "#534AB7").attr("stroke-opacity", 0.35);
    const linkSel = linkLayer
      .selectAll("line")
      .data(simLinks)
      .join("line")
      .attr("stroke-width", 1.2);

    const nodeRoot = gRoot.append("g");
    const nodeSel = nodeRoot
      .selectAll("g")
      .data(simNodes)
      .join("g")
      .style("cursor", "pointer")
      .on("click", (_ev, d) => {
        onSelectNode(d.id, d.label);
      });

    const applyHighlight = (selId: string | null) => {
      nodeSel.select("circle").each(function (d) {
        const el = d3.select(this);
        const gn = d as GNode;
        const on = selId !== null && gn.id === selId;
        el.attr("stroke-width", on ? (gn.group === "center" ? 3 : 2.6) : gn.group === "center" ? 2 : 1.2).attr(
          "stroke",
          on ? "#E8E4FF" : gn.group === "center" ? "#7F77DD" : "#534AB7",
        );
      });
    };

    highlightRef.current = applyHighlight;

    nodeSel
      .append("circle")
      .attr("r", (d) => (d.group === "center" ? 22 : 14))
      .attr("fill", (d) => (d.group === "center" ? "#534AB7" : "#1A1828"))
      .attr("stroke", (d) => (d.group === "center" ? "#7F77DD" : "#534AB7"))
      .attr("stroke-width", (d) => (d.group === "center" ? 2 : 1.2))
      .attr("filter", `url(#${filterId})`)
      .attr("class", "twin-d3-pulse");

    nodeSel
      .append("text")
      .attr("text-anchor", "middle")
      .attr("dy", (d) => (d.group === "center" ? 36 : 28))
      .attr("fill", "#AFA9EC")
      .attr("font-size", (d) => (d.group === "center" ? 12 : 10))
      .text((d) => d.label);

    applyHighlight(selectedIdRef.current);

    sim.on("tick", () => {
      linkSel
        .attr("x1", (d) => d.source.x!)
        .attr("y1", (d) => d.source.y!)
        .attr("x2", (d) => d.target.x!)
        .attr("y2", (d) => d.target.y!);

      nodeSel.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    return () => {
      highlightRef.current = () => {};
      sim.stop();
    };
  }, [graph, bumpLayout, onSelectNode, reactId]);

  return (
    <svg
      ref={svgRef}
      className="block size-full min-h-[min(160px,30svh)] touch-none bg-[#07060c]"
      role="img"
      aria-label="Connection graph — pinch or drag to pan and zoom"
    />
  );
}
