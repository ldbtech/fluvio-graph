"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { fetchCodebaseGalaxyTree } from "@/lib/fetchCodebaseGalaxy";
import type { CodebaseCloneResult, CodebaseModuleTree } from "@/lib/types";

/** One row in the sidebar file tree; `repoPath` is set for real files from the clone. */
export type FileTreeRow = { name: string; repoPath?: string; children?: FileTreeRow[] };

function isTreeFileNode(node: CodebaseModuleTree): boolean {
  const k = String(node.kind).toLowerCase();
  return k === "file";
}

/** Leaf paths that look like source files (covers API quirks where `kind` is missing). */
const SOURCE_FILE_PATH =
  /\.(rs|toml|py|pyi|ts|tsx|js|jsx|mjs|cjs|json|go|java|kt|kts|swift|c|h|cpp|cc|hpp|cs|fs|rb|php|zig|vue|svelte|md|yaml|yml|sh)$/i;

function pathLooksLikeSourceFile(relPath: string): boolean {
  const base = relPath.replace(/\\/g, "/").split("/").pop() ?? "";
  if (base === "Dockerfile" || base === "Makefile") return true;
  return SOURCE_FILE_PATH.test(relPath);
}

function moduleTreeToUiTree(root: CodebaseModuleTree | null): FileTreeRow[] | null {
  if (!root) return null;
  const walk = (node: CodebaseModuleTree): FileTreeRow => {
    const nKids = node.children?.length ?? 0;
    const hasChildren = nKids > 0;
    const norm = (node.path ?? "").replace(/\\/g, "/");
    if (!hasChildren && norm && (isTreeFileNode(node) || pathLooksLikeSourceFile(norm))) {
      return { name: node.name, repoPath: norm };
    }
    if (!hasChildren) return { name: node.name };
    return { name: node.name, children: node.children!.map(walk) };
  };
  return root.children.map(walk);
}

function expandDepth(nodes: FileTreeRow[], maxOpenDepth: number, prefix = "", depth = 0): Set<string> {
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
const MOCK_PREVIEW_TREE: FileTreeRow[] = [
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
  onResolveFile,
  resolveBusy,
}: {
  nodes: FileTreeRow[];
  prefix: string;
  depth: number;
  openSet: Set<string>;
  toggle: (path: string) => void;
  onResolveFile?: (repoPath: string) => void | Promise<void>;
  resolveBusy?: boolean;
}) {
  return (
    <>
      {nodes.map((node) => {
        const hasKids = Boolean(node.children?.length);
        const id = prefix ? `${prefix}/${node.name}` : node.name;
        const open = openSet.has(id);
        const canResolve = Boolean(!hasKids && node.repoPath && onResolveFile);
        return (
          <div key={id} className="select-text">
            <button
              type="button"
              onClick={(ev) => {
                ev.preventDefault();
                ev.stopPropagation();
                if (hasKids) {
                  toggle(id);
                  return;
                }
                if (!canResolve) return;
                if (resolveBusy) return;
                void onResolveFile?.(node.repoPath!);
              }}
              disabled={hasKids ? false : !canResolve}
              className={`flex w-full items-center gap-1 rounded-lg px-1.5 py-1 text-left text-[12px] ${
                hasKids
                  ? "text-zinc-300 hover:bg-white/[0.05]"
                  : canResolve
                    ? "cursor-pointer text-sky-300/90 hover:bg-sky-500/10"
                    : "cursor-default text-zinc-600"
              } ${resolveBusy && canResolve ? "opacity-70" : ""}`}
              style={{ paddingLeft: 6 + depth * 10 }}
            >
              <span className="pointer-events-none w-3 shrink-0 text-center text-slate-600">
                {hasKids ? (open ? "▾" : "▸") : "·"}
              </span>
              <span
                className={`pointer-events-none min-w-0 flex-1 truncate ${hasKids ? "text-violet-200/90" : canResolve ? "text-sky-200/90" : "text-slate-400"}`}
              >
                {node.name}
              </span>
            </button>
            {hasKids && open && node.children && (
              <TreeRows
                nodes={node.children}
                prefix={id}
                depth={depth + 1}
                openSet={openSet}
                toggle={toggle}
                onResolveFile={onResolveFile}
                resolveBusy={resolveBusy}
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
  onResolveFile?: (repoPath: string) => void | Promise<void>;
  resolveBusy?: boolean;
  /** Repo path currently being resolved (for status line). */
  resolvePendingPath?: string | null;
  resolveError?: string | null;
  resolveSubgraphActive?: boolean;
  onClearResolveSubgraph?: () => void;
};

export function GithubRepoFileTree({
  kgUrl,
  cloneInfo,
  className = "",
  onResolveFile,
  resolveBusy = false,
  resolvePendingPath = null,
  resolveError = null,
  resolveSubgraphActive = false,
  onClearResolveSubgraph,
}: Props) {
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

  const rowsToRender = useMemo((): FileTreeRow[] | null => {
    if (!cloneInfo) return MOCK_PREVIEW_TREE;
    if (remoteErr) return null;
    if (loading && liveTree === null) return null;
    if (!loading && liveTree !== null && (!treeFromClone || treeFromClone.length === 0)) return null;
    if (treeFromClone && treeFromClone.length > 0) return treeFromClone;
    // Clone is set but tree not ready — do not show mock files (they have no repoPath and look "broken").
    if (cloneInfo) return null;
    return MOCK_PREVIEW_TREE;
  }, [cloneInfo, remoteErr, loading, liveTree, treeFromClone]);

  const isLiveTree = Boolean(cloneInfo && liveTree && treeFromClone && treeFromClone.length > 0);

  return (
    <aside
      className={`relative z-[80] flex min-h-0 flex-col border-r border-white/[0.06] bg-zinc-950/90 backdrop-blur-xl ${className}`}
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
        <div className="mb-2 flex items-center justify-between gap-2 px-2">
          <div className="flex min-w-0 items-center gap-2">
            <p className="text-[11px] font-medium text-zinc-500">{isLiveTree ? "Files" : "Preview"}</p>
            {resolveBusy && (
              <span className="inline-flex min-w-0 max-w-[min(100%,11rem)] flex-col gap-0.5 text-[10px] font-medium text-sky-400/95">
                <span className="inline-flex items-center gap-1.5">
                  <span
                    className="inline-block size-2.5 shrink-0 animate-spin rounded-full border-2 border-sky-400/30 border-t-sky-400"
                    aria-hidden
                  />
                  Resolving…
                </span>
                {resolvePendingPath ? (
                  <span className="truncate font-mono text-[9px] font-normal text-zinc-500" title={resolvePendingPath}>
                    {resolvePendingPath}
                  </span>
                ) : null}
              </span>
            )}
          </div>
          {isLiveTree && resolveSubgraphActive && onClearResolveSubgraph && (
            <button
              type="button"
              onClick={onClearResolveSubgraph}
              className="shrink-0 rounded-md px-2 py-0.5 text-[10px] font-medium text-zinc-400 transition hover:bg-white/[0.06] hover:text-zinc-200"
            >
              Tree view
            </button>
          )}
        </div>
        {cloneInfo && loading && <p className="px-2 text-[12px] text-zinc-500">Loading file list…</p>}
        {cloneInfo && remoteErr && <p className="mb-2 px-2 text-[12px] text-red-400/90">{remoteErr}</p>}
        {resolveError && <p className="mb-2 px-2 text-[12px] text-red-400/90">{resolveError}</p>}
        {cloneInfo && !loading && liveTree && (!treeFromClone || treeFromClone.length === 0) && !remoteErr && (
          <p className="px-2 text-[12px] text-zinc-500">No files in clone.</p>
        )}
        {isLiveTree && onResolveFile && (
          <p className="mb-2 px-2 text-[10px] leading-relaxed text-zinc-600">
            Click a file to resolve imports and show that slice on the graph.
          </p>
        )}
        {rowsToRender && rowsToRender.length > 0 && (
          <TreeRows
            nodes={rowsToRender}
            prefix=""
            depth={0}
            openSet={openSet}
            toggle={toggle}
            onResolveFile={cloneInfo ? onResolveFile : undefined}
            resolveBusy={resolveBusy}
          />
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
