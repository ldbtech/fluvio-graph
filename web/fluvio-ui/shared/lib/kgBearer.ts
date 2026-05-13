/** Minimal kg-engine Bearer header — avoids importing `fluvioDashboardApi` from low-level fetch helpers (cycles). */

const TWIN_TOKEN_KEY = "twin_token";

/** Same storage key as `fluvioDashboardApi` / onboarding. */
export function kgBearerHeaders(): HeadersInit {
  if (typeof window === "undefined") return {};
  const t = localStorage.getItem(TWIN_TOKEN_KEY);
  return t ? { Authorization: `Bearer ${t}` } : {};
}
