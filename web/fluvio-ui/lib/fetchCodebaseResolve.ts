import type { CodebaseResolveResult } from "@/lib/types";

export type CodebaseResolveRequest = {
  url: string;
  path: string;
  max_depth?: number;
  max_files?: number;
};

export async function postCodebaseResolve(
  kgUrl: string,
  body: CodebaseResolveRequest,
): Promise<CodebaseResolveResult> {
  const res = await fetch(`${kgUrl.replace(/\/$/, "")}/codebase/resolve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      url: body.url,
      path: body.path,
      max_depth: body.max_depth ?? 2,
      max_files: body.max_files ?? 40,
    }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `resolve ${res.status}`);
  }
  return (await res.json()) as CodebaseResolveResult;
}
