const DEFAULT_PORT = 8001;

function trimTrailingSlash(url: string): string {
  return url.replace(/\/+$/, "");
}

/** Server-side overrides (never use "infer from browser host" for Node fetches). */
function envKgBase(): string | null {
  if (typeof process === "undefined") return null;
  for (const key of ["KG_ENGINE_URL", "NEXT_PUBLIC_KG_URL", "API_URL"] as const) {
    const raw = process.env[key]?.trim();
    if (raw) return trimTrailingSlash(raw);
  }
  return null;
}

/**
 * Resolve kg-engine base URL at call time so browser traffic from a LAN/IP host
 * (e.g. http://192.168.x.x:3000) reaches the API on the same host (:8001) instead of "localhost".
 */
export function getKgEngineUrl(): string {
  const fromEnv = envKgBase();
  if (fromEnv) return fromEnv;

  if (typeof window !== "undefined" && window.location?.hostname) {
    const { protocol, hostname } = window.location;
    if (hostname === "localhost" || hostname === "127.0.0.1") {
      return `http://127.0.0.1:${DEFAULT_PORT}`;
    }
    return `${protocol}//${hostname}:${DEFAULT_PORT}`;
  }

  return `http://127.0.0.1:${DEFAULT_PORT}`;
}
