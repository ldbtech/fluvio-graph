import { KG_URL } from "./constants";

const ARCH_DOMAIN = "architecture";
const POLL_MS = 1500;
const MAX_WAIT_MS = 600_000;

export type ToolDetectResponse = {
  action: string;
  tool_name: string | null;
  file_name: string | null;
  rel_score: number | null;
  similarity: number | null;
};

export type ToolSpawnResult = {
  action: string;
  file_name: string;
  file_path: string;
  spec_path: string | null;
  tool_name: string;
  is_new: boolean;
  job_id: string | null;
};

export type ToolJobStatus = {
  job_id: string;
  phase: string;
  percent: number;
  message: string;
  error: string | null;
  done: boolean;
  result: ToolSpawnResult | null;
};

export type EnsureArchitectureToolsOptions = {
  onJobStatus?: (status: ToolJobStatus) => void;
};

/** Waiting for POST /tools/approve — file is still under `generated/` only. */
export type PendingToolApproval = {
  file_name: string;
  job_id: string;
  tool_name: string;
};

export type EnsureArchitectureToolsOutcome =
  | {
      ok: true;
      skippedSpawn: boolean;
      /** Shown after extend / use_existing-from-spawn; not used when pendingApproval is set. */
      userNote: string | null;
      pendingApproval: PendingToolApproval | null;
    }
  | { ok: false; error: string };

async function parseJsonOrThrow<T>(res: Response): Promise<T> {
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return JSON.parse(text) as T;
}

/** POST /tools/detect — graph-based match only; no code generation. */
export async function detectArchitectureTool(request: string): Promise<ToolDetectResponse> {
  const res = await fetch(`${KG_URL}/tools/detect`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ request, domain: ARCH_DOMAIN }),
  });
  return parseJsonOrThrow(res);
}

/** POST /tools/approve — promote `fluvio-tools/src/tools/generated/<file>` → `tools/<file>`. */
export async function approveArchitectureTool(fileName: string, jobId: string): Promise<void> {
  const res = await fetch(`${KG_URL}/tools/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      file_name: fileName,
      domain: ARCH_DOMAIN,
      job_id: jobId,
    }),
  });
  await parseJsonOrThrow(res);
}

/** DELETE /tools/jobs/:id — rollback generated files and remove the job. */
export async function discardArchitectureToolJob(jobId: string): Promise<void> {
  const res = await fetch(`${KG_URL}/tools/jobs/${encodeURIComponent(jobId)}`, {
    method: "DELETE",
  });
  await parseJsonOrThrow(res);
}

/**
 * Syncs the architecture tool pipeline with the user message before chat.
 *
 * 1. POST /tools/detect — if `use_existing`, nothing else runs.
 * 2. If `generate` or `extend`: POST /tools/spawn, poll GET /tools/jobs/:id until done.
 * 3. Brand-new tools stay in `generated/` until the user approves via {@link approveArchitectureTool}.
 */
export async function ensureArchitectureToolsForMessage(
  userMessage: string,
  options?: EnsureArchitectureToolsOptions,
): Promise<EnsureArchitectureToolsOutcome> {
  const request = userMessage.trim();
  if (!request) {
    return { ok: true, skippedSpawn: true, userNote: null, pendingApproval: null };
  }

  let detect: ToolDetectResponse;
  try {
    detect = await detectArchitectureTool(request);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { ok: false, error: msg };
  }

  if (detect.action === "use_existing") {
    return { ok: true, skippedSpawn: true, userNote: null, pendingApproval: null };
  }

  if (detect.action !== "generate" && detect.action !== "extend") {
    return { ok: true, skippedSpawn: true, userNote: null, pendingApproval: null };
  }

  let jobId: string;
  try {
    const res = await fetch(`${KG_URL}/tools/spawn`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ request, domain: ARCH_DOMAIN }),
    });
    const body = await parseJsonOrThrow<{ job_id: string }>(res);
    jobId = body.job_id;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { ok: false, error: `Tool spawn failed: ${msg}` };
  }

  const deadline = Date.now() + MAX_WAIT_MS;
  let last: ToolJobStatus | null = null;

  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${KG_URL}/tools/jobs/${encodeURIComponent(jobId)}`);
      const status = await parseJsonOrThrow<ToolJobStatus>(res);
      last = status;
      options?.onJobStatus?.(status);

      if (status.done) {
        if (status.phase === "failed" || status.error) {
          return { ok: false, error: status.error || status.message || "Tool job failed" };
        }
        const result = status.result;
        if (!result) {
          return { ok: false, error: "Tool job finished without a result payload." };
        }

        if (result.action === "generated" && result.is_new) {
          return {
            ok: true,
            skippedSpawn: false,
            userNote: null,
            pendingApproval: {
              file_name: result.file_name,
              job_id: jobId,
              tool_name: result.tool_name,
            },
          };
        }

        if (result.action === "extended") {
          return {
            ok: true,
            skippedSpawn: false,
            userNote: `Architecture tool agent extended "${result.tool_name}" (${result.file_name}) for your request.`,
            pendingApproval: null,
          };
        }

        return { ok: true, skippedSpawn: false, userNote: null, pendingApproval: null };
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return { ok: false, error: `Tool job poll failed: ${msg}` };
    }

    await new Promise((r) => setTimeout(r, POLL_MS));
  }

  const hint = last ? ` Last: ${last.phase} ${last.percent}%.` : "";
  return { ok: false, error: `Timed out waiting for tool job ${jobId}.${hint}` };
}
