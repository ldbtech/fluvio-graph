import { getKgEngineUrl } from "@/shared/lib/constants";
import { authHeaders } from "@/shared/lib/fluvioDashboardApi";

export type TwinChatApiMessage = { role: "user" | "assistant"; content: string };

async function readUtf8Stream(
  res:     Response,
  onDelta: (t: string) => void,
  signal:  AbortSignal,
) {
  if (!res.body) throw new Error("No response body");
  const reader = res.body.getReader();
  const dec    = new TextDecoder();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    if (signal.aborted) throw new DOMException("Aborted", "AbortError");
    onDelta(dec.decode(value, { stream: true }));
  }
}

/**
 * Stream the twin chat from kg-engine /twin/chat.
 * Falls back to Next.js /api/chat if kg-engine is unreachable.
 */
export async function streamTwinAssistant(
  messages: TwinChatApiMessage[],
  onDelta:  (t: string) => void,
  signal:   AbortSignal,
  opts?:    { graphContext?: string; graphOwnerId?: string },
): Promise<void> {
  const body: Record<string, unknown> = {
    messages: messages.map((m) => ({ role: m.role, content: m.content })),
  };

  const gc = opts?.graphContext?.trim();
  if (gc) body.graph_context = gc;

  const go = opts?.graphOwnerId?.trim();
  if (go) body.graph_owner_id = go;

  try {
    const kg = await fetch(`${getKgEngineUrl()}/twin/chat`, {
      method:  "POST",
      headers: authHeaders(),
      body:   JSON.stringify(body),
      signal,
    });
    if (kg.ok) {
      await readUtf8Stream(kg, onDelta, signal);
      return;
    }
  } catch {
    /* fall back to Next route if kg-engine is down */
  }

  // Fallback — Next.js API route (send Bearer so proxy can reach kg-engine with Surreal context)
  const res = await fetch("/api/chat", {
    method:  "POST",
    headers: authHeaders(),
    body:    JSON.stringify(body),
    signal,
  });

  if (!res.ok) {
    let detail = res.statusText;
    try {
      const j = (await res.json()) as { error?: string };
      if (j.error) detail = j.error;
    } catch { /* ignore */ }
    throw new Error(detail);
  }

  await readUtf8Stream(res, onDelta, signal);
}