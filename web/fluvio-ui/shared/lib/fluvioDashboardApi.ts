import { getKgEngineUrl } from "@/shared/lib/constants";
import { postCodebaseClone } from "@/shared/lib/fetchCodebaseClone";
import type { GraphEdge, GraphNode } from "@/shared/lib/types";

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

/** GET / HEAD etc. — Bearer only (no `Content-Type`). */
export function authBearerHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/** Headers for multipart uploads — include Bearer only (do not set `Content-Type`). */
export function authMultipartHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
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
  /** Stable physical / NFC scope UUID (`users.physical_id`); same string as `metadata.owner_physical_id` on ingested nodes. */
  physical_id: string | null;
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

export type PeerGraphStatus = {
  viewer_user_id:         string;
  peer_user_id:           string;
  peer_name:              string;
  connected:              boolean;
  zone:                   number | null;
  surreal_rows_in_zone:   number;
  surreal_workspace_rows: number;
  pg_user_upload_rows:    number;
  pg_user_upload_kinds:   string[];
  card_based_connection:  boolean;
};

export type SetupResponse = {
  user_id:     string;
  physical_id: string;
  card_id:     string;
  name:        string;
};

export type AuthRequestResponse = {
  ok:      boolean;
  email:   string;
  sent:    boolean;
  code?:   string;
  message: string;
};

export type AuthVerifyResponse = {
  ok:          boolean;
  token:       string;
  user_id:     string;
  name:        string;
  physical_id: string | null;
};

export type AuthMeResponse = {
  user_id:     string;
  name:        string;
  email:       string | null;
  physical_id: string | null;
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

/** Status/debug for selected connection — confirms Surreal rows visible in your current zone. */
export async function fetchPeerGraphStatus(
  id:      string,
  signal?: AbortSignal,
): Promise<PeerGraphStatus> {
  const res = await fetch(
    `${getKgEngineUrl()}/twin/network/${encodeURIComponent(id)}/status`,
    { headers: authHeaders(), signal },
  );
  if (!res.ok) throw new Error(`connection status ${res.status}`);
  return (await res.json()) as PeerGraphStatus;
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
  const res = await fetch(`${getKgEngineUrl()}/ingest/pdf`, {
    method:  "POST",
    headers: authMultipartHeaders(),
    body:    fd,
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `PDF ingest HTTP ${res.status}`);
  }
  return (await res.json()) as { nodes: number; edges: number };
}

export type UserUploadRow = {
  id:            string;
  user_id:       string;
  kind:          string;
  file_name:     string;
  document_id:   string | null;
  graph_nodes:   number | null;
  graph_edges:   number | null;
  created_at:    string;
};

/** Recent PDF / video uploads for the signed-in user (`GET /user/uploads`). */
export async function fetchUserUploads(signal?: AbortSignal): Promise<UserUploadRow[]> {
  const res = await fetch(`${getKgEngineUrl()}/user/uploads`, {
    method:  "GET",
    headers: authBearerHeaders(),
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `user/uploads HTTP ${res.status}`);
  }
  return (await res.json()) as UserUploadRow[];
}

/** `DELETE /user/uploads/:id` — removes the library row, matching graph nodes, Surreal records, and video files when applicable. */
export async function deleteUserUpload(
  uploadId: string,
  signal?: AbortSignal,
): Promise<{ removed_graph_nodes: number; removed_surreal_nodes: number }> {
  const res = await fetch(`${getKgEngineUrl()}/user/uploads/${encodeURIComponent(uploadId)}`, {
    method:  "DELETE",
    headers: authBearerHeaders(),
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `delete upload HTTP ${res.status}`);
  }
  const j = (await res.json()) as {
    removed_graph_nodes?: number;
    removed_surreal_nodes?: number;
  };
  return {
    removed_graph_nodes:   j.removed_graph_nodes ?? 0,
    removed_surreal_nodes: j.removed_surreal_nodes ?? 0,
  };
}

/**
 * `POST /ingest/pdf/stream` — NDJSON lines: `start`, `progress` (% of PDF→graph), `done`, or `error`.
 */
export async function postWorkspaceIngestPdfStream(
  file: File,
  onProgress: (p: { percent: number; phase?: string; pages?: number }) => void,
  signal?: AbortSignal,
): Promise<{ nodes: number; edges: number }> {
  const fd = new FormData();
  fd.append("file", file);
  const res = await fetch(`${getKgEngineUrl()}/ingest/pdf/stream`, {
    method:  "POST",
    headers: authMultipartHeaders(),
    body:    fd,
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `PDF stream ingest HTTP ${res.status}`);
  }
  const reader = res.body?.getReader();
  if (!reader) throw new Error("PDF stream: empty response body");

  const dec = new TextDecoder();
  let buf = "";
  let last: { nodes: number; edges: number } | null = null;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    for (;;) {
      const nl = buf.indexOf("\n");
      if (nl < 0) break;
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line) continue;
      let j: Record<string, unknown>;
      try {
        j = JSON.parse(line) as Record<string, unknown>;
      } catch {
        continue;
      }
      const ev = j.event;
      if (ev === "start" && typeof j.pages === "number") {
        onProgress({ percent: 0, phase: "start", pages: j.pages });
      }
      if (ev === "progress" && typeof j.percent === "number") {
        onProgress({
          percent: j.percent,
          phase:   typeof j.phase === "string" ? j.phase : undefined,
          pages:   typeof j.pages === "number" ? j.pages : undefined,
        });
      }
      if (ev === "done" && typeof j.nodes === "number" && typeof j.edges === "number") {
        last = { nodes: j.nodes, edges: j.edges };
      }
      if (ev === "error") {
        const msg = typeof j.message === "string" ? j.message : "PDF ingest failed";
        throw new Error(msg);
      }
    }
  }

  if (!last) throw new Error("PDF stream: no completion from server");
  return last;
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
  const res = await fetch(`${getKgEngineUrl()}/ingest/video`, {
    method:  "POST",
    headers: authMultipartHeaders(),
    body:    fd,
    signal,
  });
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

export type WorkspaceCodebaseIngestProgress = {
  phase: "clone" | "ingest";
  /** Rough milestone 0–100 for UI (clone ≈ first half, ingest second). */
  pct: number;
};

/** Clone (or pull) then `POST /ingest` for `path` under the repo URL. */
export async function ingestWorkspaceCodebasePrefix(
  repoUrl: string,
  pathPrefix: string,
  signal?: AbortSignal,
  onProgress?: (p: WorkspaceCodebaseIngestProgress) => void,
): Promise<WorkspaceCodebaseIngestResponse> {
  onProgress?.({ phase: "clone", pct: 8 });
  await postCodebaseClone(getKgEngineUrl(), repoUrl);
  onProgress?.({ phase: "ingest", pct: 48 });
  const res = await fetch(`${getKgEngineUrl()}/ingest`, {
    method:  "POST",
    headers: authHeaders(),
    body:    JSON.stringify({ url: repoUrl.trim(), path: pathPrefix.trim() }),
    signal,
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(detail ? detail.slice(0, 280) : `Codebase ingest HTTP ${res.status}`);
  }
  const body = (await res.json()) as WorkspaceCodebaseIngestResponse;
  onProgress?.({ phase: "ingest", pct: 100 });
  return body;
}

export async function fetchGmailConnected(signal?: AbortSignal): Promise<boolean> {
  const res = await fetch(`${getKgEngineUrl()}/connect/gmail/status`, {
    headers: authBearerHeaders(),
    signal,
  });
  if (!res.ok) throw new Error(`Gmail status HTTP ${res.status}`);
  const j = (await res.json()) as { connected?: boolean };
  return !!j.connected;
}

/** Latest inbox messages from Gmail (metadata). Default 10; max 50 on server. */
export type GmailRecentMail = {
  id: string;
  thread_id: string;
  snippet?: string | null;
  subject?: string | null;
  from?: string | null;
  date_header?: string | null;
  internal_date_ms?: number | null;
  /** Present when Gmail History reported this message as newly added this poll. */
  is_new?: boolean | null;
};

export async function fetchGmailRecentInbox(opts?: {
  limit?: number;
  signal?: AbortSignal;
}): Promise<GmailRecentMail[]> {
  const lim = Math.min(50, Math.max(1, opts?.limit ?? 10));
  const res = await fetch(`${getKgEngineUrl()}/gmail/recent?limit=${lim}`, {
    headers: authBearerHeaders(),
    signal: opts?.signal,
  });
  const text = await res.text().catch(() => "");
  if (!res.ok) {
    throw new Error(text ? text.slice(0, 280) : `Gmail recent HTTP ${res.status}`);
  }
  const rows = (text ? JSON.parse(text) : []) as GmailRecentMail[];
  return Array.isArray(rows) ? rows : [];
}

/** Sender allow-list (normalized on server). Empty = all inbox senders in recent list. */
export async function fetchGmailFocus(signal?: AbortSignal): Promise<string[]> {
  const res = await fetch(`${getKgEngineUrl()}/gmail/focus`, {
    headers: authBearerHeaders(),
    signal,
  });
  const text = await res.text().catch(() => "");
  if (!res.ok) throw new Error(text ? text.slice(0, 280) : `Gmail focus HTTP ${res.status}`);
  const j = (text ? JSON.parse(text) : {}) as { senders?: string[] };
  return Array.isArray(j.senders) ? j.senders : [];
}

export async function putGmailFocus(senders: string[], signal?: AbortSignal): Promise<string[]> {
  const res = await fetch(`${getKgEngineUrl()}/gmail/focus`, {
    method:  "PUT",
    headers: authHeaders(),
    body:    JSON.stringify({ senders }),
    signal,
  });
  const text = await res.text().catch(() => "");
  if (!res.ok) throw new Error(text ? text.slice(0, 280) : `Gmail focus save HTTP ${res.status}`);
  const j = (text ? JSON.parse(text) : {}) as { senders?: string[] };
  return Array.isArray(j.senders) ? j.senders : [];
}

/** `POST /connect/gmail/start` — returns Google's consent URL; requires Bearer session. */
export async function postGmailConnectStart(
  opts?: { forceConsent?: boolean; signal?: AbortSignal },
): Promise<string> {
  const res = await fetch(`${getKgEngineUrl()}/connect/gmail/start`, {
    method:  "POST",
    headers: authHeaders(),
    body:    JSON.stringify({ force_consent: !!opts?.forceConsent }),
    signal:  opts?.signal,
  });
  if (!res.ok) {
    const t = await res.text().catch(() => "");
    throw new Error(t ? t.slice(0, 280) : `Gmail OAuth start HTTP ${res.status}`);
  }
  const j = (await res.json()) as { url?: string };
  if (!j.url) throw new Error("OAuth start missing url");
  return j.url;
}