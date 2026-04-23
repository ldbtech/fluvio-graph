"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { fetchCodebaseGalaxyTree } from "@/lib/fetchCodebaseGalaxy";
import type { CodebaseCloneResult, CodebaseModuleTree } from "@/lib/types";

type TreeNode = { name: string; children?: TreeNode[] };

function moduleTreeToUiTree(root: CodebaseModuleTree | null): TreeNode[] | null {
  if (!root) return null;
  const walk = (node: CodebaseModuleTree): TreeNode => {
    if (!node.children?.length) return { name: node.name };
    return { name: node.name, children: node.children.map(walk) };
  };
  return root.children.map(walk);
}

function expandDepth(nodes: TreeNode[], maxOpenDepth: number, prefix = "", depth = 0): Set<string> {
  const s = new Set<string>();
  if (depth >= maxOpenDepth) return s;
  for (const n of nodes) {
    if (!n.children?.length) continue;
    const id = prefix ? `${prefix}/${n.name}` : n.name;
    s.add(id);
    if (depth + 1 < maxOpenDepth) {
      const inner = expandDepth(n.children, maxOpenDepth, id, depth + 1);
      inner.forEach((x) => s.add(x));
    }
  }
  return s;
}

/** Static tree when no clone metadata (Sources preview only). */
const MOCK_PREVIEW_TREE: TreeNode[] = [
  { name: "README.md" },
  {
    name: "src",
    children: [{ name: "main.rs" }, { name: "lib.rs" }],
  },
];

function TreeRows({
  nodes,
  prefix,
  depth,
  openSet,
  toggle,
}: {
  nodes: TreeNode[];
  prefix: string;
  depth: number;
  openSet: Set<string>;
  toggle: (path: string) => void;
}) {
  return (
    <>
      {nodes.map((node) => {
        const hasKids = Boolean(node.children?.length);
        const id = prefix ? `${prefix}/${node.name}` : node.name;
        const open = openSet.has(id);
        return (
          <div key={id} className="select-text">
            <button
              type="button"
              onClick={() => hasKids && toggle(id)}
              className={`flex w-full items-center gap-1 rounded-lg px-1.5 py-1 text-left text-[12px] ${
                hasKids ? "text-zinc-300 hover:bg-white/[0.05]" : "cursor-default text-zinc-600"
              }`}
              style={{ paddingLeft: 6 + depth * 10 }}
              disabled={!hasKids}
            >
              <span className="w-3 shrink-0 text-center text-slate-600">
                {hasKids ? (open ? "▾" : "▸") : "·"}
              </span>
              <span className={hasKids ? "text-violet-200/90" : "text-slate-400"}>{node.name}</span>
            </button>
            {hasKids && open && node.children && (
              <TreeRows
                nodes={node.children}
                prefix={id}
                depth={depth + 1}
                openSet={openSet}
                toggle={toggle}
              />
            )}
          </div>
        );
      })}
    </>
  );
}

type Props = {
  kgUrl: string;
  cloneInfo: CodebaseCloneResult | null;
  className?: string;
};

export function GithubRepoFileTree({ kgUrl, cloneInfo, className = "" }: Props) {
  const [openSet, setOpenSet] = useState<Set<string>>(() => expandDepth(MOCK_PREVIEW_TREE, 2));
  const [liveTree, setLiveTree] = useState<CodebaseModuleTree | null>(null);
  const [loading, setLoading] = useState(false);
  const [remoteErr, setRemoteErr] = useState<string | null>(null);

  const treeFromClone = useMemo(() => moduleTreeToUiTree(liveTree), [liveTree]);

  useEffect(() => {
    if (!cloneInfo?.owner || !cloneInfo?.repo) {
      setLiveTree(null);
      setRemoteErr(null);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setRemoteErr(null);
    void fetchCodebaseGalaxyTree(kgUrl, cloneInfo)
      .then((tree) => {
        if (!cancelled) setLiveTree(tree);
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setLiveTree(null);
          setRemoteErr(e instanceof Error ? e.message : String(e));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [cloneInfo?.owner, cloneInfo?.repo, kgUrl]);

  useEffect(() => {
    if (!cloneInfo) {
      setOpenSet(expandDepth(MOCK_PREVIEW_TREE, 2));
      return;
    }
    if (treeFromClone?.length) {
      setOpenSet(expandDepth(treeFromClone, 4));
    }
  }, [cloneInfo, treeFromClone]);

  const toggle = useCallback((path: string) => {
    setOpenSet((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const title = cloneInfo ? `${cloneInfo.owner}/${cloneInfo.repo}` : "preview · sample tree";
  const pathHint = cloneInfo?.local_path ?? "~/.fluvio/repos/<owner>/<repo>/";

  const rowsToRender = useMemo((): TreeNode[] | null => {
    if (!cloneInfo) return MOCK_PREVIEW_TREE;
    if (remoteErr) return null;
    if (loading && liveTree === null) return null;
    if (!loading && liveTree !== null && (!treeFromClone || treeFromClone.length === 0)) return null;
    if (treeFromClone && treeFromClone.length > 0) return treeFromClone;
    return MOCK_PREVIEW_TREE;
  }, [cloneInfo, remoteErr, loading, liveTree, treeFromClone]);

  const isLiveTree = Boolean(cloneInfo && liveTree && treeFromClone && treeFromClone.length > 0);

  return (
    <aside
      className={`flex min-h-0 flex-col border-r border-white/[0.06] bg-zinc-950/90 backdrop-blur-xl ${className}`}
    >
      <div className="shrink-0 border-b border-white/[0.06] px-4 py-3">
        <p className="text-[11px] font-medium uppercase tracking-wide text-zinc-600">Repository</p>
        <p className="mt-1 truncate text-[13px] font-semibold tracking-tight text-zinc-100" title={title}>
          {title}
        </p>
        <p className="mt-1 truncate font-mono text-[11px] text-zinc-600" title={pathHint}>
          {pathHint}
        </p>
        {cloneInfo && (
          <p className="mt-2 text-[11px] font-medium text-emerald-400/90">
            {cloneInfo.was_cloned ? "New clone" : "Updated"}
          </p>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 py-3">
        <p className="mb-2 px-2 text-[11px] font-medium text-zinc-500">
          {isLiveTree ? "Files" : "Preview"}
        </p>
        {cloneInfo && loading && (
          <p className="px-2 text-[12px] text-zinc-500">Loading file list…</p>
        )}
        {cloneInfo && remoteErr && (
          <p className="mb-2 px-2 text-[12px] text-red-400/90">{remoteErr}</p>
        )}
        {cloneInfo && !loading && liveTree && (!treeFromClone || treeFromClone.length === 0) && !remoteErr && (
          <p className="px-2 text-[12px] text-zinc-500">No files in clone.</p>
        )}
        {rowsToRender && rowsToRender.length > 0 && (
          <TreeRows nodes={rowsToRender} prefix="" depth={0} openSet={openSet} toggle={toggle} />
        )}
        {!cloneInfo && (
          <p className="mt-4 px-2 text-[12px] leading-relaxed text-zinc-600">
            Clone a public repo in Sources to load the real tree from your machine via kg-engine.
          </p>
        )}
      </div>
    </aside>
  );
}
