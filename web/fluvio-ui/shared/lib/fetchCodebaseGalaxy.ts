import type { CodebaseCloneResult, CodebaseModuleTree } from "@/shared/lib/types";
import { kgBearerHeaders } from "@/shared/lib/kgBearer";

export async function fetchCodebaseGalaxyTree(
  kgUrl: string,
  info: CodebaseCloneResult,
): Promise<CodebaseModuleTree> {
  const params = new URLSearchParams({ owner: info.owner, repo: info.repo });
  const res = await fetch(`${kgUrl}/tree?${params.toString()}`, {
    method: "GET",
    headers: { ...kgBearerHeaders(), Accept: "application/json" },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `tree ${res.status}`);
  }
  return (await res.json()) as CodebaseModuleTree;
}
