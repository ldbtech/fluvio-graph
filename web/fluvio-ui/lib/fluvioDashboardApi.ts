import { getKgEngineUrl } from "@/lib/constants";
import type { GraphEdge, GraphNode } from "@/lib/types";

// ── Owner ID ──────────────────────────────────────────────────────────────────
// Stored in localStorage after POST /twin/setup.
// Sent as X-Owner-ID header on every request.

export function getOwnerId(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("twin_owner_id");
}

export function setOwnerId(id: string) {
  if (typeof window !== "undefined") {
    localStorage.setItem("twin_owner_id", id);
  }
}

function ownerHeaders(): HeadersInit {
  const id = getOwnerId();
  return id
    ? { "Content-Type": "application/json", "X-Owner-ID": id }
    : { "Content-Type": "application/json" };
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

/** Create account + NFC card. Stores owner_id in localStorage. */
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
  setOwnerId(data.user_id);
  return data;
}

/** Fetch owner profile + documents + connections. */
export async function fetchFluvioAccount(signal?: AbortSignal): Promise<FluvioAccount> {
  const res = await fetch(`${getKgEngineUrl()}/twin/me`, {
    headers: ownerHeaders(),
    signal,
  });
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
    headers: ownerHeaders(),
    body:    JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`profile ${res.status}`);
  return (await res.json()) as { ok: boolean };
}

/** Called when user taps an NFC card. card_id from the NFC tag URL. */
export async function tapCard(cardId: string): Promise<TapResponse> {
  const res = await fetch(`${getKgEngineUrl()}/twin/tap/${encodeURIComponent(cardId)}`, {
    headers: ownerHeaders(),
  });
  if (!res.ok) throw new Error(`tap ${res.status}`);
  return (await res.json()) as TapResponse;
}

/** Fetch the owner's social/network graph. */
export async function fetchFluvioSocialGraph(
  signal?: AbortSignal,
): Promise<FluvioGraphPayload> {
  const res = await fetch(`${getKgEngineUrl()}/twin/network`, {
    headers: ownerHeaders(),
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
    { headers: ownerHeaders(), signal },
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
    headers: ownerHeaders(),
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
    headers: ownerHeaders(),
    body:    JSON.stringify({ zone }),
  });
  if (!res.ok) throw new Error(`zone ${res.status}`);
  return (await res.json()) as { ok: boolean };
}