import Anthropic from "@anthropic-ai/sdk";
import type { MessageParam } from "@anthropic-ai/sdk/resources/messages";
import { getKgEngineUrl } from "@/lib/constants";
import { TWIN_MODEL, TWIN_SYSTEM_PROMPT } from "@/lib/twinPrompt";

export const runtime = "nodejs";

type Body = {
  messages: Array<{ role: "user" | "assistant"; content: string }>;
  /** Optional graph selection context (same field name as kg-engine). */
  graph_context?: string;
};

function toParams(msgs: Body["messages"]): MessageParam[] {
  return msgs.map((m) => ({
    role: m.role,
    content: m.content,
  }));
}

function kgEngineBaseUrl(): string {
  return getKgEngineUrl();
}

export async function POST(req: Request) {
  let body: Body;
  try {
    body = (await req.json()) as Body;
  } catch {
    return new Response(JSON.stringify({ error: "Invalid JSON body." }), {
      status: 400,
      headers: { "Content-Type": "application/json" },
    });
  }

  if (!Array.isArray(body.messages) || body.messages.length === 0) {
    return new Response(JSON.stringify({ error: "messages must be a non-empty array." }), {
      status: 400,
      headers: { "Content-Type": "application/json" },
    });
  }

  for (const m of body.messages) {
    if (m.role !== "user" && m.role !== "assistant") {
      return new Response(JSON.stringify({ error: "Invalid message role." }), {
        status: 400,
        headers: { "Content-Type": "application/json" },
      });
    }
    if (typeof m.content !== "string" || !m.content.trim()) {
      return new Response(JSON.stringify({ error: "Each message must have non-empty string content." }), {
        status: 400,
        headers: { "Content-Type": "application/json" },
      });
    }
  }

  const graphBlock =
    typeof body.graph_context === "string" && body.graph_context.trim().length > 0
      ? `\n\nGRAPH VIEW (user selection in the connections UI):\n${body.graph_context.trim()}\n`
      : "";

  const key = process.env.ANTHROPIC_API_KEY;
  if (key) {
    const anthropic = new Anthropic({ apiKey: key });
    const stream = anthropic.messages.stream({
      model: TWIN_MODEL,
      max_tokens: 4096,
      system: `${TWIN_SYSTEM_PROMPT}${graphBlock}`,
      messages: toParams(body.messages),
    });

    const encoder = new TextEncoder();
    const readable = new ReadableStream({
      async start(controller) {
        try {
          stream.on("text", (textDelta) => {
            controller.enqueue(encoder.encode(textDelta));
          });
          await stream.finalMessage();
          controller.close();
        } catch (err) {
          controller.error(err instanceof Error ? err : new Error(String(err)));
        }
      },
    });

    return new Response(readable, {
      status: 200,
      headers: {
        "Content-Type": "text/plain; charset=utf-8",
        "Cache-Control": "no-store",
      },
    });
  }

  /** No key in Next.js: stream from kg-engine (same Anthropic key lives there). */
  const kgBase = kgEngineBaseUrl();
  const proxyPayload: Record<string, unknown> = {
    messages: body.messages.map((m) => ({ role: m.role, content: m.content })),
  };
  if (typeof body.graph_context === "string" && body.graph_context.trim()) {
    proxyPayload.graph_context = body.graph_context.trim();
  }

  try {
    const upstream = await fetch(`${kgBase.replace(/\/$/, "")}/twin/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(proxyPayload),
    });
    if (!upstream.ok) {
      const detail = await upstream.text().catch(() => upstream.statusText);
      return new Response(
        JSON.stringify({
          error: `Next.js has no ANTHROPIC_API_KEY; proxy to kg-engine failed (${upstream.status}). ${detail.slice(0, 200)}`,
        }),
        { status: 502, headers: { "Content-Type": "application/json" } },
      );
    }
    if (!upstream.body) {
      return new Response(JSON.stringify({ error: "kg-engine returned an empty body." }), {
        status: 502,
        headers: { "Content-Type": "application/json" },
      });
    }
    return new Response(upstream.body, {
      status: 200,
      headers: {
        "Content-Type": "text/plain; charset=utf-8",
        "Cache-Control": "no-store",
      },
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return new Response(
      JSON.stringify({
        error: `No ANTHROPIC_API_KEY in Next.js and could not reach kg-engine at ${kgBase}: ${msg}. Set ANTHROPIC_API_KEY in web/fluvio-ui/.env.local or point KG_ENGINE_URL / NEXT_PUBLIC_KG_URL at a running kg-engine with a key configured.`,
      }),
      { status: 503, headers: { "Content-Type": "application/json" } },
    );
  }
}
