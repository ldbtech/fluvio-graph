"use client";

import { useId, useRef, useState, type ComponentPropsWithoutRef } from "react";

const focusRing =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/35 focus-visible:ring-offset-0";

const inputClass = `w-full min-h-[46px] rounded-2xl border border-white/[0.08] bg-black/35 px-4 text-[15px] font-normal tracking-[-0.01em] text-white shadow-[inset_0_1px_2px_rgba(0,0,0,.2)] outline-none placeholder:text-zinc-600 transition-[border-color,background-color] duration-150 focus:border-white/[0.16] focus:bg-black/45 focus:ring-0 disabled:opacity-38 ${focusRing}`;

const textareaClass = `min-h-[120px] w-full resize-y rounded-2xl border border-white/[0.08] bg-black/35 px-4 py-3 text-[15px] font-normal tracking-[-0.01em] text-white shadow-[inset_0_1px_2px_rgba(0,0,0,.2)] outline-none placeholder:text-zinc-600 transition-[border-color,background-color] duration-150 focus:border-white/[0.16] focus:bg-black/45 focus:ring-0 disabled:opacity-38 ${focusRing}`;

function BtnPrimary({ className = "", ...p }: ComponentPropsWithoutRef<"button">) {
  return (
    <button
      {...p}
      className={`inline-flex min-h-[44px] items-center justify-center rounded-full bg-zinc-100 px-5 py-2.5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-950 shadow-[inset_0_1px_0_rgba(255,255,255,.85)] transition-[transform,opacity,background-color] duration-150 enabled:hover:bg-white enabled:active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 ${focusRing} ${className}`.trim()}
    />
  );
}

function BtnSecondary({ className = "", ...p }: ComponentPropsWithoutRef<"button">) {
  return (
    <button
      {...p}
      className={`inline-flex min-h-[44px] items-center justify-center rounded-full border border-white/[0.12] bg-white/[0.04] px-5 py-2.5 text-[14px] font-semibold tracking-[-0.01em] text-zinc-100 transition-[transform,opacity,background-color,border-color] duration-150 enabled:hover:border-white/[0.18] enabled:hover:bg-white/[0.07] enabled:active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 ${focusRing} ${className}`.trim()}
    />
  );
}

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

/** Connectors an agent may call when synthesizing tools (expand as kg-engine adds OAuth targets). */
export type AgentToolConnector = "github_codebase" | "yahoo_finance" | "broker_account";

const CONNECTOR_LABEL: Record<AgentToolConnector, string> = {
  github_codebase: "GitHub · this project",
  yahoo_finance: "Yahoo Finance & business",
  broker_account: "Broker account",
};

type AgentDraft = { id: string; name: string; mission: string; connectors: AgentToolConnector[] };
type ToolDraft = { id: string; name: string; body: string };

export type SpecializedAgentToolStudioProps = {
  /** Mirrors dashboard `locked` — disables edits while profile unavailable. */
  disabled?: boolean;
  /** True when a codebase is already linked (Specialized → Software). Enables GitHub connector pick. */
  codebaseLinked?: boolean;
  /** Optional linked repo label for the GitHub connector row. */
  codebaseFileLabel?: string | null;
};

export function SpecializedAgentToolStudio({
  disabled = false,
  codebaseLinked = false,
  codebaseFileLabel = null,
}: SpecializedAgentToolStudioProps) {
  const pdfRef = useRef<HTMLInputElement>(null);
  const agentNameId = useId();
  const agentMissionId = useId();
  const toolNameId = useId();
  const toolBodyId = useId();
  const connectorGroupId = useId();

  const [agentName, setAgentName] = useState("");
  const [agentMission, setAgentMission] = useState("");
  const [selectedConnectors, setSelectedConnectors] = useState<Set<AgentToolConnector>>(() => new Set());
  const [connectorHint, setConnectorHint] = useState<string | null>(null);
  const [toolName, setToolName] = useState("");
  const [toolBody, setToolBody] = useState("");
  const [pdfFileName, setPdfFileName] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentDraft[]>([]);
  const [tools, setTools] = useState<ToolDraft[]>([]);

  const toggleConnector = (id: AgentToolConnector, checked: boolean, allowed: boolean) => {
    if (!allowed || disabled) return;
    setSelectedConnectors((prev) => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
    setConnectorHint(null);
  };

  const canPickGithub = codebaseLinked && !disabled;
  const canPickYahoo = false;
  const canPickBroker = false;
  const anyConnectorSelectable = canPickGithub || canPickYahoo || canPickBroker;

  const saveAgent = () => {
    const name = agentName.trim();
    const mission = agentMission.trim();
    if (!name || !mission) return;
    if (anyConnectorSelectable && selectedConnectors.size === 0) {
      setConnectorHint("Pick at least one connector this agent may use when building tools.");
      return;
    }
    setConnectorHint(null);
    const connectors = Array.from(selectedConnectors);
    setAgents((prev) => [{ id: newId(), name, mission, connectors }, ...prev]);
    setAgentName("");
    setAgentMission("");
    setSelectedConnectors(new Set());
  };

  const saveTool = () => {
    const name = toolName.trim();
    const body = toolBody.trim();
    if (!name || !body) return;
    setTools((prev) => [{ id: newId(), name, body }, ...prev]);
    setToolName("");
    setToolBody("");
  };

  function ConnectorRow(props: {
    id: AgentToolConnector;
    title: string;
    description: string;
    checked: boolean;
    canPick: boolean;
    badge?: string;
  }) {
    const inputId = `${connectorGroupId}-${props.id}`;
    const rowDisabled = disabled || !props.canPick;
    return (
      <label
        htmlFor={inputId}
        className={`flex cursor-pointer gap-3 rounded-2xl border border-white/[0.08] bg-black/25 px-4 py-3 ring-1 ring-white/[0.04] transition-colors hover:border-white/[0.12] ${rowDisabled ? "cursor-not-allowed opacity-50 hover:border-white/[0.08]" : ""}`}
      >
        <input
          id={inputId}
          type="checkbox"
          checked={props.checked}
          disabled={rowDisabled}
          onChange={(e) => toggleConnector(props.id, e.target.checked, props.canPick)}
          className={`mt-1 size-4 shrink-0 rounded border-white/20 bg-zinc-900 text-violet-500 ${focusRing}`}
        />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[14px] font-semibold tracking-[-0.02em] text-white">{props.title}</span>
            {props.badge ? (
              <span className="rounded-full bg-amber-500/12 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-amber-200/90 ring-1 ring-amber-400/20">
                {props.badge}
              </span>
            ) : null}
          </div>
          <p className="mt-1 text-[12px] leading-relaxed text-zinc-500">{props.description}</p>
        </div>
      </label>
    );
  }

  return (
    <div className="flex max-w-3xl flex-col gap-8">
      <p className="rounded-2xl border border-sky-500/15 bg-sky-500/[0.06] px-4 py-3 text-[13px] leading-relaxed text-sky-100/90 ring-1 ring-sky-400/12">
        <span className="font-semibold text-sky-200/95">UI only.</span> FluvioMe stays small: we do not ship endless
        first-party agents. <span className="text-sky-100/95">Your agents</span> compose{" "}
        <span className="text-sky-100/95">tools</span> that combine live connector data (e.g.{" "}
        <span className="whitespace-nowrap">{"Yahoo Finance & business"}</span>) with PDFs ingested into the knowledge
        graph—so a strategy memo for specific futures plus market data becomes a runnable tool, not another hand-built
        Fluvio agent. A <span className="text-sky-100/95">broker account</span> connector will follow for guarded
        automation. Drafts stay in this session until kg-engine persists the loop.
      </p>

      <section className="space-y-3" aria-label="Create an agent">
        <div>
          <h4 className="text-[15px] font-semibold tracking-[-0.02em] text-white">Create an agent</h4>
          <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
            Choose which connectors this agent may call when it generates tools, then describe what you are trying to
            achieve. PDFs below are what the agent should learn as structured graph knowledge—not only live feeds.
          </p>
        </div>
        <div className="space-y-3">
          <div>
            <label htmlFor={agentNameId} className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
              Agent name
            </label>
            <input
              id={agentNameId}
              value={agentName}
              onChange={(e) => setAgentName(e.target.value)}
              disabled={disabled}
              placeholder="e.g. Futures tools from my PDFs + Yahoo"
              className={inputClass}
            />
          </div>

          <fieldset className="space-y-2">
            <legend className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
              Connectors for tools
            </legend>
            <p id={`${connectorGroupId}-help`} className="mb-2 text-[12px] leading-relaxed text-zinc-500">
              Today we are sure about <span className="text-zinc-400">GitHub (this project)</span> once a repo is linked,
              and <span className="text-zinc-400">{"Yahoo Finance & business"}</span> next. More connectors (e.g. broker
              for investing automation) will appear here as OAuth ships.
            </p>
            <div className="flex flex-col gap-2" role="group" aria-describedby={`${connectorGroupId}-help`}>
              <ConnectorRow
                id="github_codebase"
                title="GitHub · this project"
                description={
                  codebaseLinked && codebaseFileLabel?.trim()
                    ? `Uses your linked repo: ${codebaseFileLabel.trim()}`
                    : "Link a public repository under Specialized → Software to enable this connector."
                }
                checked={selectedConnectors.has("github_codebase")}
                canPick={canPickGithub}
              />
              <ConnectorRow
                id="yahoo_finance"
                title="Yahoo Finance & business"
                description="Live quotes, watchlists, and business headlines for tools that need market context."
                checked={selectedConnectors.has("yahoo_finance")}
                canPick={canPickYahoo}
                badge="Later"
              />
              <ConnectorRow
                id="broker_account"
                title="Broker account"
                description="For order routing and investing automation with explicit guardrails—after compliance and APIs."
                checked={selectedConnectors.has("broker_account")}
                canPick={canPickBroker}
                badge="Later"
              />
            </div>
            {connectorHint ? (
              <p className="text-[12px] font-medium text-amber-200/90" role="status">
                {connectorHint}
              </p>
            ) : null}
          </fieldset>

          <div>
            <label htmlFor={agentMissionId} className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
              {"Goal & instructions"}
            </label>
            <textarea
              id={agentMissionId}
              value={agentMission}
              onChange={(e) => setAgentMission(e.target.value)}
              disabled={disabled}
              placeholder="What you want the agent to achieve with the connectors you picked, risk rules, when to act vs only notify…"
              className={textareaClass}
              rows={4}
            />
          </div>
          <BtnPrimary type="button" disabled={disabled} onClick={saveAgent} className="w-full sm:w-auto">
            Save agent draft
          </BtnPrimary>
        </div>
      </section>

      <section className="space-y-3" aria-label="PDF to knowledge graph">
        <div>
          <h4 className="text-[15px] font-semibold tracking-[-0.02em] text-white">PDF → knowledge graph</h4>
          <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
            Upload what the agent must learn—policies, strategies, memos—so it becomes nodes and edges the tools can
            rely on, alongside Yahoo or GitHub-backed data.
          </p>
        </div>
        <input
          ref={pdfRef}
          type="file"
          accept="application/pdf,.pdf"
          className="sr-only"
          aria-hidden
          tabIndex={-1}
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            setPdfFileName(f?.name ?? null);
          }}
        />
        <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
          <BtnSecondary type="button" disabled={disabled} onClick={() => pdfRef.current?.click()} className="w-full sm:w-auto">
            Choose PDF
          </BtnSecondary>
          <BtnPrimary type="button" disabled className="opacity-45" title="Graph ingest not wired yet">
            Queue for graph extract
          </BtnPrimary>
        </div>
        {pdfFileName ? (
          <p className="rounded-xl border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-[13px] text-zinc-300 ring-1 ring-white/[0.04]">
            Selected <span className="font-medium text-zinc-200">{pdfFileName}</span> — will become graph-backed
            knowledge for tools your agent generates (ingest not wired yet).
          </p>
        ) : (
          <p className="text-[12px] text-zinc-600">No PDF staged.</p>
        )}
      </section>

      <section className="space-y-3" aria-label="Describe a tool for the graph">
        <div>
          <h4 className="text-[15px] font-semibold tracking-[-0.02em] text-white">Describe a tool for the graph</h4>
          <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">
            A concrete tool definition: what it reads from the graph, what it may call on connectors, outputs, and human
            checkpoints. Agents promote these from drafts into registered tools once the backend exists.
          </p>
        </div>
        <div className="space-y-3">
          <div>
            <label htmlFor={toolNameId} className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
              Tool name
            </label>
            <input
              id={toolNameId}
              value={toolName}
              onChange={(e) => setToolName(e.target.value)}
              disabled={disabled}
              placeholder="e.g. Futures sleeve allocator"
              className={inputClass}
            />
          </div>
          <div>
            <label htmlFor={toolBodyId} className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.12em] text-zinc-600">
              Graph-linked description
            </label>
            <textarea
              id={toolBodyId}
              value={toolBody}
              onChange={(e) => setToolBody(e.target.value)}
              disabled={disabled}
              placeholder="Graph entities (from PDF ingest), Yahoo or GitHub fields to read, broker actions if allowed, when to execute vs ask, risk caps…"
              className={textareaClass}
              rows={5}
            />
          </div>
          <BtnPrimary type="button" disabled={disabled} onClick={saveTool} className="w-full sm:w-auto">
            Save tool description
          </BtnPrimary>
        </div>
      </section>

      {(agents.length > 0 || tools.length > 0) && (
        <section className="rounded-[20px] border border-white/[0.06] bg-black/25 p-4 ring-1 ring-white/[0.04] sm:p-5">
          <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500">Session drafts</p>
          <div className="mt-3 grid gap-4 sm:grid-cols-2">
            <div>
              <p className="text-[12px] font-medium text-zinc-400">Agents · {agents.length}</p>
              {agents.length === 0 ? (
                <p className="mt-1 text-[12px] text-zinc-600">None yet.</p>
              ) : (
                <ul className="mt-2 max-h-48 space-y-2 overflow-y-auto pr-1 text-[13px] text-zinc-300">
                  {agents.map((a) => (
                    <li key={a.id} className="rounded-lg border border-white/[0.05] bg-white/[0.02] px-2.5 py-2">
                      <span className="font-medium text-white/95">{a.name}</span>
                      {a.connectors.length > 0 ? (
                        <p className="mt-1 flex flex-wrap gap-1">
                          {a.connectors.map((c) => (
                            <span
                              key={c}
                              className="rounded-md bg-white/[0.06] px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-zinc-400"
                            >
                              {CONNECTOR_LABEL[c]}
                            </span>
                          ))}
                        </p>
                      ) : (
                        <p className="mt-1 text-[10px] uppercase tracking-wide text-zinc-600">No connectors (draft)</p>
                      )}
                      <p className="mt-0.5 line-clamp-2 text-[12px] leading-snug text-zinc-500">{a.mission}</p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div>
              <p className="text-[12px] font-medium text-zinc-400">Tools · {tools.length}</p>
              {tools.length === 0 ? (
                <p className="mt-1 text-[12px] text-zinc-600">None yet.</p>
              ) : (
                <ul className="mt-2 max-h-48 space-y-2 overflow-y-auto pr-1 text-[13px] text-zinc-300">
                  {tools.map((t) => (
                    <li key={t.id} className="rounded-lg border border-white/[0.05] bg-white/[0.02] px-2.5 py-2">
                      <span className="font-medium text-white/95">{t.name}</span>
                      <p className="mt-0.5 line-clamp-2 text-[12px] leading-snug text-zinc-500">{t.body}</p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
