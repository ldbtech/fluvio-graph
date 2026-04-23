import type { CodebaseModuleTree, GraphEdge, GraphNode } from "@/lib/types";

function nodeId(n: CodebaseModuleTree, parentId: string | null): string {
  if (n.path && n.path.length > 0) return n.path;
  if (parentId) return `${parentId}/${n.name}`;
  return n.name;
}

/** Turn a galaxy `TreeNode` subtree into force-graph nodes/edges (parent → child). */
export function moduleSubtreeToGraph(
  root: CodebaseModuleTree,
  maxNodes = 140,
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];

  function walk(n: CodebaseModuleTree, parentId: string | null): void {
    if (nodes.length >= maxNodes) return;
    const id = nodeId(n, parentId);
    const label =
      n.kind === "file"
        ? n.name
        : `${n.name} · ${n.language} · ${n.file_count} file${n.file_count === 1 ? "" : "s"}`;
    nodes.push({
      id,
      label,
      page: n.path || ".",
      source: "github",
    });
    if (parentId !== null) {
      edges.push({ from: parentId, to: id, token: 1, probability: 0.92 });
    }
    for (const c of n.children) {
      if (nodes.length >= maxNodes) break;
      walk(c, id);
    }
  }

  walk(root, null);
  return { nodes, edges };
}
