/** POST /codebase/clone — shallow clone or git pull before POST /ingest. */

export type CodebaseCloneResponse = {
  owner: string;
  repo: string;
  local_path: string;
  was_cloned: boolean;
};

function kgBase(kgUrl: string): string {
  return kgUrl.trim().replace(/\/+$/, "");
}

/**
 * Shallow clone or git pull. Tries `/codebase/clone` then `/sync/codebase/clone` (older docs / proxies).
 */
export async function postCodebaseClone(kgUrl: string, url: string): Promise<CodebaseCloneResponse> {
  const base = kgBase(kgUrl);
  const body = JSON.stringify({ url: url.trim() });

  for (const path of ["/codebase/clone", "/sync/codebase/clone"] as const) {
    const res = await fetch(`${base}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
    });
    const text = await res.text();
    if (res.status === 404) continue;
    if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
    return JSON.parse(text) as CodebaseCloneResponse;
  }

  throw new Error(
    "clone endpoint not found (HTTP 404). Restart kg-engine so it includes POST /codebase/clone, or set NEXT_PUBLIC_KG_URL to the API base (e.g. http://localhost:8001).",
  );
}
