/** Types aligned with kg-engine `routes/rules.rs` and `security_agent.rs` (serde snake_case). */

import { kgBearerHeaders } from "@/shared/lib/kgBearer";

export type AgentPhase =
  | "idle"
  | "initializing"
  | "scanning"
  | "analyzing"
  | "writing_edges"
  | "done"
  | "failed";

export type AgentProgress = {
  agent_id: string;
  phase: AgentPhase;
  current_file: string | null;
  files_done: number;
  files_total: number;
  violations: number;
  error: string | null;
  running: boolean;
};

export type SecurityDeployResponse = {
  agent_id: string;
  status: string;
  poll: string;
  result: string;
};

export type SecurityDeployBody = {
  scope?: string | null;
  pdf_document_ids?: string[];
  similarity_threshold?: number;
  top_k_rules?: number;
  max_files?: number;
};

export type RulesLinkBody = {
  document_id?: string | null;
  code_path_filter?: string | null;
  similarity_threshold?: number;
  top_k?: number;
  use_llm?: boolean;
};

export type RuleMatch = {
  code_uri: string;
  rule_uri: string;
  confidence: number;
  edge_kind: string;
  explanation: string;
};

export type LinkGraphNode = {
  id: string;
  label: string;
  kind: string;
  source: string;
};

export type LinkGraphEdge = {
  from: string;
  to: string;
  label: string;
  confidence: number;
};

export type LinkResult = {
  document_id: string;
  filename: string;
  matches: RuleMatch[];
  violates_count: number;
  implements_count: number;
  related_count: number;
  graph_nodes: LinkGraphNode[];
  graph_edges: LinkGraphEdge[];
};

export type SecurityViolation = {
  file_path: string;
  symbol: string | null;
  code_uri: string;
  rule_uri: string;
  rule_text: string;
  rule_source: string;
  edge_kind: string;
  confidence: number;
  explanation: string;
  severity: string;
};

export type SecurityAgentResult = {
  agent_id: string;
  agent_node_id: string;
  files_analyzed: number;
  rules_checked: number;
  violations: SecurityViolation[];
  violates_count: number;
  implements_count: number;
  related_count: number;
  edges_written: number;
};

export async function postRulesLink(kgUrl: string, body: RulesLinkBody): Promise<LinkResult> {
  const res = await fetch(`${kgUrl}/rules/link`, {
    method: "POST",
    headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return JSON.parse(text) as LinkResult;
}

export async function postSecurityDeploy(
  kgUrl: string,
  body: SecurityDeployBody,
): Promise<SecurityDeployResponse> {
  const res = await fetch(`${kgUrl}/agents/security/deploy`, {
    method: "POST",
    headers: { ...kgBearerHeaders(), "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return JSON.parse(text) as SecurityDeployResponse;
}

export async function getSecurityStatus(kgUrl: string, agentId: string): Promise<AgentProgress> {
  const res = await fetch(`${kgUrl}/agents/security/${encodeURIComponent(agentId)}/status`, {
    headers: kgBearerHeaders(),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return JSON.parse(text) as AgentProgress;
}

/** Returns `null` while the agent is still running (HTTP 202). */
export async function getSecurityResult(
  kgUrl: string,
  agentId: string,
): Promise<SecurityAgentResult | null> {
  const res = await fetch(`${kgUrl}/agents/security/${encodeURIComponent(agentId)}/result`, {
    headers: kgBearerHeaders(),
  });
  const text = await res.text();
  if (res.status === 202) return null;
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return JSON.parse(text) as SecurityAgentResult;
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new DOMException("Aborted", "AbortError"));
      return;
    }
    const t = window.setTimeout(resolve, ms);
    const onAbort = () => {
      window.clearTimeout(t);
      reject(new DOMException("Aborted", "AbortError"));
    };
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

/**
 * Polls `/status` until phase is terminal, then polls `/result` until available or timeout.
 */
export async function runSecurityAgentToCompletion(
  kgUrl: string,
  deployBody: SecurityDeployBody,
  opts: {
    onProgress?: (p: AgentProgress) => void;
    pollMs?: number;
    signal?: AbortSignal;
  } = {},
): Promise<SecurityAgentResult> {
  const { onProgress, pollMs = 1500, signal } = opts;
  const { agent_id } = await postSecurityDeploy(kgUrl, deployBody);

  const maxStatusPolls = 2000;
  for (let n = 0; n < maxStatusPolls; n++) {
    await sleep(pollMs, signal);
    const p = await getSecurityStatus(kgUrl, agent_id);
    onProgress?.(p);
    if (p.phase === "failed") {
      throw new Error(p.error || "security agent failed");
    }
    if (p.phase === "done") break;
    if (n === maxStatusPolls - 1) {
      throw new Error("timed out waiting for security agent status");
    }
  }

  const resultPollMs = 400;
  const maxAttempts = 120;
  for (let i = 0; i < maxAttempts; i++) {
    const r = await getSecurityResult(kgUrl, agent_id);
    if (r) return r;
    await sleep(resultPollMs, signal);
  }
  throw new Error("timed out waiting for security agent result");
}
