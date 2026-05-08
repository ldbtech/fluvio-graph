import { getKgEngineUrl } from "@/lib/constants";
import { postCodebaseClone } from "@/lib/fetchCodebaseClone";
import type { GraphEdge, GraphNode } from "@/lib/types";

const TWIN_TOKEN_KEY = "twin_token";
const TWIN_USER_ID_KEY = "twin_user_id";
const LEGACY_OWNER_ID_KEY = "twin_owner_id";

// ── Session (Bearer token) ───────────────────────────────────────────────────

export function setToken(token: string) {
  if (typeof window !== "undefined") {
    localStorage.setItem(TWIN_TOKEN_KEY, token);
  }
}

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(TWIN_TOKEN_KEY);
}

export function clearToken() {
  if (typeof window !== "undefined") {
    localStorage.removeItem(TWIN_TOKEN_KEY);
  }
}

/** User id from auth verify or sync from GET /twin/auth/me — used for Wallet issue-url and hardware orders. */
export function setTwinUserId(id: string) {
  if (typeof window !== "undefined") {
    localStorage.setItem(TWIN_USER_ID_KEY, id);
  }
}

export function getTwinUserId(): string | null {
  if (typeof window === "undefined") return null;
  let id = localStorage.getItem(TWIN_USER_ID_KEY);
  if (id) return id;
  const legacy = localStorage.getItem(LEGACY_OWNER_ID_KEY);
  if (legacy) {
    localStorage.setItem(TWIN_USER_ID_KEY, legacy);
    return legacy;
  }
  return null;
}

/** Clear token and stored user id (legacy owner key too). */
export function clearSession() {
  clearToken();
  if (typeof window !== "undefined") {
    localStorage.removeItem(TWIN_USER_ID_KEY);
    localStorage.removeItem(LEGACY_OWNER_ID_KEY);
  }
}

/** JSON request headers; includes Bearer when a session token exists. */
export function authHeaders(): HeadersInit {
  const token = getToken();
  return token
    ? { "Content-Type": "application/json", Authorization: `Bearer ${token}` }
    : { "Content-Type": "application/json" };
}

/** @deprecated Use getTwinUserId — same storage, includes legacy migration. */
export function getOwnerId(): string | null {
  return getTwinUserId();
}

/** @deprecated Use setTwinUserId */
export function setOwnerId(id: string) {
  setTwinUserId(id);
}

// ── Types ─────────────────────────────────────────────────────────────────────

export type FluvioAccount = {
  user_id:      string;
  owner_slug:   string;
  display_name: string;
  tagline:      string;
  email:        string;
  phone:        string;
  /** When the API provides a public tap URL; otherwise the UI shows a short fallback. */
  nfc_public_path?: string;
  /** Primary NFC card for Wallet QR / physical tags (`GET /twin/tap/:id`). */
  nfc_card_id?: string | null;
  graph_id:     string | null;
  documents: Array<{
    id:      string;
    title:   string;
    kind:    string;
    status:  string;
    excerpt: string;
  }>;
  connections: Array<{
    id:                string;
    name:              string;
    role:              string;
    how_we_met:        string;
    relation_strength: number;
    ingested_summary:  string;
  }>;
};

export type FluvioGraphPayload = {
  nodes: Array<{ id: string; label: string; page: string; source: string }>;
  edges: Array<{ from: string; to: string; token: number; probability: number; label: string }>;
};

export type SetupResponse = {
  user_id:  string;
  graph_id: string;
  card_id:  string;
  name:     string;
};

export type AuthRequestResponse = {
  ok:      boolean;
  email:   string;
  sent:    boolean;
  code?:   string;
  message: string;
};

export type AuthVerifyResponse = {
  ok:       boolean;
  token:    string;
  user_id:  string;
  name:     string;
  graph_id: string | null;
};

export type AuthMeResponse = {
  user_id:  string;
  name:     string;
  email:    string | null;
  graph_id: string | null;
};

export type TapResponse = {
  connected:     boolean;
  tapped_user:   { user_id: string; name: string; tagline: string; card_id: string };
  connection_id: string;
  zone:          number;
};

// ── Graph helpers ─────────────────────────────────────────────────────────────

export function toGraphNodesEdges(payload: FluvioGraphPayload): {
  nodes: GraphNode[];
  edges: GraphEdge[];
} {
  const nodes: GraphNode[] = payload.nodes.map((n) => ({
    id:     n.id,
    label:  n.label,
    page:   n.page,
    source: n.source,
  }));
  const edges: GraphEdge[] = payload.edges.map((e) => ({
    from:        e.from,
    to:          e.to,
    token:       e.token,
    probability: e.probability,
    label:       e.label,
  }));
  return { nodes, edges };
}

// ── API calls ─────────────────────────────────────────────────────────────────

/** Request email OTP (`POST /twin/auth/request`). Creates user if new. */
export async function postAuthRequest(body: {
  email: string;
  name?: string;
}): Promise<AuthRequestResponse> {
  const res = await fetch(`${getKgEngineUrl()}/twin/auth/request`, {
    method:  "POST",
    headers: { "Content-Type": "application/json" },
    body:    JSON.stringify(body),
  });
  if (!res.ok) {
    const t = await res.text().catch(() => "");
    throw new Error(t ? t.slice(0, 240) : `request code ${res.status}`);
  }
  return (await res.json()) as AuthRequestResponse;
}

/** Verify OTP and persist session token + user id. */
export async function postAuthVerify(email: string, code: string): Promise<AuthVerifyResponse> {
  const res = await fetch(`${getKgEngineUrl()}/twin/auth/verify`, {
    method:  "POST",
    headers: { "Content-Type": "application/json" },
    body:    JSON.stringify({ email: email.trim().toLowerCase(), code: code.trim() }),
  });
  if (!res.ok) {
    const t = await res.text().catch(() => "");
    throw new Error(t ? t.slice(0, 240) : `verify ${res.status}`);
  }
  const data = (await res.json()) as AuthVerifyResponse;
  setToken(data.token);
  setTwinUserId(data.user_id);
  return data;
}

/** Validate Bearer token and return user (`GET /twin/auth/me`). */
export async function fetchAuthMe(signal?: AbortSignal): Promise<AuthMeResponse | null> {
  const token = getToken();
  if (!token) return null;
  const res = await fetch(`${getKgEngineUrl()}/twin/auth/me`, {
    headers: authHeaders(),
    signal,
  });
  if (res.status === 401) {
    clearSession();
    return null;
  }
  if (!res.ok) throw new Error(`auth/me ${res.status}`);
  const me = (await res.json()) as AuthMeResponse;
  setTwinUserId(me.user_id);
  return me;
}

/** Log out (revokes server session and clears local storage). */
export async function logoutAuthSession(): Promise<void> {
  const token = getToken();
  if (!token) {
    clearSession();
    return;
  }
  try {
    await fetch(`${getKgEngineUrl()}/twin/auth/session`, {
      method:  "DELETE",
      headers: authHeaders(),
    });
  } finally {
    clearSession();
  }
}

/** Create account + NFC card. Stores user id for Wallet (no Bearer until email auth). */
export async function postTwinSetup(body: {
  name: string;
  email?: string;
  phone?: string;
}): Promise<SetupResponse> {
  const res = await fetch(`${getKgEngineUrl()}/twin/setup`, {
    method:  "POST",
    headers: { "Content-Type": "application/json" },
    body:    JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`setup ${res.status}`);
  const data = (await res.json()) as SetupResponse;
  setTwinUserId(data.user_id);
  return data;
}

/** Fetch owner profile + documents + connections. */
export async function fetchFluvioAccount(signal?: AbortSignal): Promise<FluvioAccount> {
  const res = await fetch(`${getKgEngineUrl()}/twin/me`, {
    headers: authHeaders(),
    signal,
  });
  if (res.status === 401) {
    clearSession();
    throw new Error("Session expired — sign in again.");
  }
  if (!res.ok) throw new Error(`account ${res.status}`);
  const data = (await res.json()) as FluvioAccount;
  return {
    ...data,
    email: data.email ?? "",
    phone: data.phone ?? "",
    nfc_card_id: data.nfc_card_id ?? null,
  };
}

/** Update name, email, or phone. */
export async function postFluvioAccountProfile(body: {
  name?:  string;
  email?: string;
  phone?: string;
}): Promise<{ ok: boolean }> {
  const res = await fetch(`${getKgEngineUrl()}/twin/me/profile`, {
    method:  "POST",
    headers: authHeaders(),
    body:    JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`profile ${res.status}`);
  return (await res.json()) as { ok: boolean };
}

/** Called when user taps an NFC card. card_id from the NFC tag URL. */
export async function tapCard(cardId: string): Promise<TapResponse> {
  const res = await fetch(`${getKgEngineUrl()}/twin/tap/${encodeURIComponent(cardId)}`, {
    headers: authHeaders(),
  });
  if (!res.ok) throw new Error(`tap ${res.status}`);
  return (await res.json()) as TapResponse;
}

/** Fetch the owner's social/network graph. */
export async function fetchFluvioSocialGraph(
  signal?: AbortSignal,
): Promise<FluvioGraphPayload> {
  const res = await fetch(`${getKgEngineUrl()}/twin/network`, {
    headers: authHeaders(),
    signal,
  });
  if (!res.ok) throw new Error(`network ${res.status}`);
  return (await res.json()) as FluvioGraphPayload;
}

/** Fetch the mini graph for one connection. */
export async function fetchFluvioConnectionGraph(
  id:      string,
  signal?: AbortSignal,
): Promise<FluvioGraphPayload> {
  const res = await fetch(
    `${getKgEngineUrl()}/twin/network/${encodeURIComponent(id)}`,
    { headers: authHeaders(), signal },
  );
  if (!res.ok) throw new Error(`connection graph ${res.status}`);
  return (await res.json()) as FluvioGraphPayload;
}

/** Ingest a note or document into the twin's graph. */
export async function postFluvioIngest(payload: {
  title?: string;
  body?:  string;
  kind?:  string;
}): Promise<{ ok: boolean }> {
  const res = await fetch(`${getKgEngineUrl()}/twin/ingest`, {
    method:  "POST",
    headers: authHeaders(),
    body:    JSON.stringify(payload),
  });
  if (!res.ok) throw new Error(`ingest ${res.status}`);
  return (await res.json()) as { ok: boolean };
}

/** Update zone for a connection (1 = public, 2 = closer). */
export async function updateConnectionZone(
  userId: string,
  zone:   1 | 2,
): Promise<{ ok: boolean }> {
  const res = await fetch(`${getKgEngineUrl()}/twin/zone/${encodeURIComponent(userId)}`, {
    method:  "PUT",
    headers: authHeaders(),
    body:    JSON.stringify({ zone }),
  });
  if (!res.ok) throw new Error(`zone ${res.status}`);
  return (await res.json()) as { ok: boolean };
}

// ── Workspace graph ingests (same kg-engine pipeline as Map / workspace) ─────

/** `POST /ingest/pdf` — multipart `file` field. */
export async function postWorkspaceIngestPdf(
  file: File,
  signal?: AbortSignal,
): Promise<{ nodes: number; edges: number }> {
  const fd = new FormData();
  fd.append("file", file);
  const res = await fetch(`${getKgEngineUrl()}/ingest/pdf`, { method: "POST", body: fd, signal });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `PDF ingest HTTP ${res.status}`);
  }
  return (await res.json()) as { nodes: number; edges: number };
}

export type WorkspaceVideoIngestResponse = {
  video_id: string;
  duration: number;
  fps: number;
  resolution: string;
  codec?: string;
  scenes?: number;
  /** Scene nodes ingested by this upload (not full graph totals). */
  nodes?: number;
  edges?: number;
  has_audio?: boolean;
  status?: string;
};

/** `POST /ingest/video` — multipart `file` field. */
export async function postWorkspaceIngestVideo(
  file: File,
  signal?: AbortSignal,
): Promise<WorkspaceVideoIngestResponse> {
  const fd = new FormData();
  fd.append("file", file);
  const res = await fetch(`${getKgEngineUrl()}/ingest/video`, { method: "POST", body: fd, signal });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `Video ingest HTTP ${res.status}`);
  }
  return (await res.json()) as WorkspaceVideoIngestResponse;
}

export type WorkspaceCodebaseIngestResponse = {
  chunks: number;
  nodes: number;
  edges: number;
};

/** Clone (or pull) then `POST /ingest` for `path` under the repo URL. */
export async function ingestWorkspaceCodebasePrefix(
  repoUrl: string,
  pathPrefix: string,
  signal?: AbortSignal,
): Promise<WorkspaceCodebaseIngestResponse> {
  await postCodebaseClone(getKgEngineUrl(), repoUrl);
  const res = await fetch(`${getKgEngineUrl()}/ingest`, {
    method:  "POST",
    headers: { "Content-Type": "application/json" },
    body:    JSON.stringify({ url: repoUrl.trim(), path: pathPrefix.trim() }),
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `Codebase ingest HTTP ${res.status}`);
  }
  return (await res.json()) as WorkspaceCodebaseIngestResponse;
}

export type GmailSyncProgressSnapshot = {
  running: boolean;
  mode: string;
  phase: string;
  threads_done: number;
  threads_total: number;
  percent: number | null;
  chunks: number;
  error?: string | null;
  result?: {
    chunks: number;
    nodes_added: number;
    structured_edges: number;
    graph_nodes: number;
    graph_edges: number;
  } | null;
};

export async function fetchGmailConnected(signal?: AbortSignal): Promise<boolean> {
  const res = await fetch(`${getKgEngineUrl()}/connect/gmail/status`, { signal });
  if (!res.ok) throw new Error(`Gmail status HTTP ${res.status}`);
  const j = (await res.json()) as { connected?: boolean };
  return !!j.connected;
}

/** Opens in a new tab: browser OAuth, callback to kg-engine. */
export function gmailOAuthStartUrl(): string {
  return `${getKgEngineUrl()}/connect/gmail?redirect=1`;
}

/** Returns 202 Accepted; poll [`fetchGmailSyncProgress`]. */
export async function postGmailSync(signal?: AbortSignal): Promise<void> {
  const res = await fetch(`${getKgEngineUrl()}/sync/gmail`, {
    method:  "POST",
    headers: { "Content-Type": "application/json" },
    body:    JSON.stringify({ mode: "incremental" }),
    signal,
  });
  if (res.status === 409) throw new Error("Sync already running — wait or refresh progress.");
  if (!res.ok && res.status !== 202) {
    const t = await res.text().catch(() => "");
    throw new Error(t ? t.slice(0, 240) : `Gmail sync HTTP ${res.status}`);
  }
}

export async function fetchGmailSyncProgress(
  signal?: AbortSignal,
): Promise<GmailSyncProgressSnapshot> {
  const res = await fetch(`${getKgEngineUrl()}/sync/gmail/progress`, { signal });
  if (!res.ok) throw new Error(`Gmail progress HTTP ${res.status}`);
  return (await res.json()) as GmailSyncProgressSnapshot;
}