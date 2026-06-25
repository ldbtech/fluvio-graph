# CSP × Knowledge Graph × MCP — Integration Plan

> **Goal:** Make the **Capability Synthesis Protocol (CSP)** the layer that builds and
> reuses tools/capabilities, **grounded by the knowledge graph** (`fluvio-graph`) and
> **executing through MCP** (`fluvio-tool-builder`).
>
> **Status:** Design — no code yet.
> **Decisions locked:** capabilities live in the KG as a **`Capability` node kind**;
> CSP runs **inside `agent-planner`** (Python); write this doc first.

---

## 0. The reality (why CSP isn't "inside" fluvio-graph)

`fluvio-graph` is **Rust**. CSP is a **Python** library (`import csp`). You cannot
`import csp` into a Rust binary — same lesson as the SDK and the engine. So CSP runs
in Python (the `agent-planner`), and `fluvio-graph` plays the role it's already built
for: a **vector-indexed registry**. Capabilities become first-class graph nodes.

This is not a workaround — it's the better design. `fluvio-graph` already has nodes
with a `kind`, 384-dim `embeddings`, `metadata`, plus `search` (cosine), `neighbors`,
`upsert_node`, `upsert_edge`. A capability registry is exactly that, so we reuse it.

---

## 1. Architecture

```
        agent-planner (Python)  ── embeds CSP Orchestrator ──┐
              │                                               │
        a goal / pipeline step                                │
              │                                               │
   ┌──────────▼─────────────┐  1. existing MCP tool?          │   MCP: fluvio-tool-builder /mcp
   │  Precedence resolver    │────────────────────────────────┼──►  tools/list  (M1, done)
   │  tool > capability > new│                                 │
   └──────────┬─────────────┘  2. existing capability?        │   fluvio-graph :4001 (GraphQL)
              │                ─────────────────────────────────►  search(goal, kind=Capability)
              │ 3. neither → synthesize                        │     → semantic reuse-first
   ┌──────────▼─────────────┐                                  │
   │  CSP Synthesizer +      │  writes run(args), sandboxed     │
   │  PythonSandbox          │                                  │
   └──────────┬─────────────┘                                  │
              │ persist capability ───────────────────────────►│   upsert_node(kind=Capability,
              │   (spec → source_text → 384-dim embed)          │     source_text=spec, metadata=code…)
              │   + upsert_edge(Capability —USES_TOOL→ tool)    │   + edges to MCP tools & data nodes
              ▼                                                 │
        executes its steps via ───────────────────────────────►┘   MCP tools/call  (M3)
```

**Three precedence tiers** when the planner needs an operation:
1. **An existing MCP tool covers it** → call it (no synthesis). `tools/list` from M1.
2. **An existing capability covers it** → reuse it. Semantic `search` over `Capability`
   nodes in the graph (CSP reuse-first, company-wide instead of a local folder).
3. **Neither** → CSP synthesizes a new general verb, sandboxes it, and persists it to
   the graph as a `Capability` node with edges to the MCP tools it calls.

---

## 2. The `Capability` node schema

Reuse the existing node machinery — no new storage layer. A capability is a node:

| Node field | Holds |
|---|---|
| `kind` | **`Capability`** (new variant) |
| `source_text` | The capability **spec** — name + docstring + signature summary. **This is what gets embedded** (BGE-small, 384-dim), so reuse-first search is semantic. |
| `source_uri` | `csp://capability/<name>` |
| `metadata` (HashMap<String,String>) | `name`, `signature`, `generated_code`, `synthesis_guidance`, `tags`, `status` (`synthesized`/`registered`), `mcp_tools` (csv), `synthesized_at` |
| edges | `Capability —USES_TOOL→ <mcp tool node>` · `Capability —READS→ <data/table node>` · `Capability —SERVES→ <goal/topic node>` |

**Reuse-first** = `search(goal_embedding, kindFilter=Capability, topK=k)`; if the top
score ≥ threshold, load `metadata.generated_code` and reuse — no LLM. The edges make
the brain answer "what does this capability depend on?" by graph traversal.

> Generated code in `metadata` is a string value (HashMap<String,String>) — fine for
> M-sized snippets. If code grows large, store a blob ref in `source_uri` instead.

---

## 3. fluvio-graph changes (Rust — small, additive)

Capabilities reuse `upsert_node` / `search`; we only add the node kind.

- `crates/fluvio-types/src/graph/enums.rs`
  - Add `Capability` to `NodeKind` (after `Conversation`; keep `ExternalRef` last to
    preserve ordering). Add the matching `NodeKindFilter` arm.
- `services/fluvio-graph/src/graphql/types.rs`
  - Add `Capability` to `GqlNodeKind` + both `From` impls (Gql↔types).
- `services/fluvio-graph/src/storage/surreal.rs`
  - `kind` already serializes to a string — confirm `Capability` round-trips; no schema
    migration needed (SurrealDB is schemaless on `kind`).
- **No new GraphQL fields required** — `upsert_node(kind: Capability, …)` and
  `search(query, kindFilter: Capability)` already do the job. (Optional sugar later:
  `searchCapabilities(goal, topK)` / `upsertCapability(...)` thin wrappers.)

That's the whole Rust surface. The heavy lifting is in Python.

---

## 4. agent-planner changes (Python — the real work)

CSP's persistence seam is `PlannerStore` (`save_capability`, `load_capabilities`,
`delete_capability`) and `CapabilityRegistry.summary_for_planner(goal)`. We back these
with the graph.

- `app/capabilities/graph_store.py` — **NEW**: `GraphPlannerStore(PlannerStore)`
  - `save_capability(cap)` → `upsert_node(kind=Capability, source_text=cap.spec,
    metadata={code, signature, tags, mcp_tools, …})` via the gateway, then
    `upsert_edge` to each MCP tool the capability calls.
  - `load_capabilities()` / goal-aware load → `search(goal, kind=Capability, topK=k)`
    so the planner only sees **relevant** capabilities (solves tool-bloat via the KG).
  - `delete_capability(name)` → soft-delete / `delete_node`.
- `app/capabilities/orchestrator.py` — **NEW**: build the CSP `Orchestrator` with
  `AnthropicLLM`, `planner_dir`-equivalent = `GraphPlannerStore`, and
  `synthesis_guidance` describing fluvioMe conventions (e.g. "tools are called via MCP
  `tools/call`; outputs that are plots → base64 PNG; SQL via the `database__*` tools").
- `app/routers/compile.py` — wire the **precedence resolver**: before asking Claude to
  emit a step, check (1) MCP `tools/list`, (2) graph capability `search`; only fall to
  (3) CSP synthesis when neither covers the goal.
- Synthesized capabilities **call MCP tools** (M3 client) rather than arbitrary I/O —
  so "build and check tools" stays inside the governed MCP substrate, and every call is
  recorded as a `USES_TOOL` edge.

---

## 5. The reuse-first decision, end to end

```
planner needs: "rank customers by 90-day churn risk"
  → MCP tools/list           → no single tool does this          (tier 1 miss)
  → graph search(Capability) → "score_churn_risk" exists? 
        ├─ yes (score ≥ τ)   → reuse: load code, run via MCP      ✅ no LLM
        └─ no                → CSP synthesizes "score_churn_risk"
                               (a GENERAL verb, not a one-off),
                               sandbox-tests it, upserts to graph
                               + USES_TOOL→ database__execute_query
                               next time → tier-2 hit, reused forever
```

---

## 6. Phases

| Phase | What | Status |
|---|---|---|
| **C1** | Rust: `Capability` node kind + `upsertCapability` / `searchCapabilities` (embed server-side). | ✅ **DONE & VERIFIED LIVE** |
| **C2** | Python: `GraphPlannerStore(PlannerStore)` — mirrors capabilities to the graph. | ✅ DONE (imports clean) |
| **C3** | CSP `Orchestrator` factory in agent-planner with the graph store. | ✅ DONE |
| **C4** | Reuse-first resolver + compile hint + `/capabilities/*` router. | ✅ DONE |
| **C5** | `USES_TOOL` edges + MCP execution substrate + auto tool-detection. | ✅ **DONE & VERIFIED LIVE** |

### C5 live verification
- `mirror_capability_to_graph` auto-detects MCP tools a capability calls (live catalog
  via `mcp_client.list_tool_names()`), upserts a Tool node per tool, and wires
  `Capability —USES_TOOL→ Tool` edges.
- **Graph traversal both directions** (against embedded SurrealDB):
  - `churn_report` → `database__execute_query`, `dashboard_syncer__generate_pdf_report`
  - reverse: `database__execute_query` → `churn_report`
- agent-planner MCP client connected to the live tool-builder `/mcp` (25 tools);
  `detect_mcp_tools` correctly isolated the two tools used by sample capability code.
- **Bonus fix:** the `neighbors`/`bfs` resolver was generating invalid SurrealQL
  (`->{0,depth}->`, unquoted UUID record ids, and a missing `array::flatten` level) —
  *all* graph traversal was broken in this SurrealDB version. Now fixed: valid
  bidirectional wildcard traversal with `type::record(...)` + flattened `INSIDE`.
- Deferred (true M3): synthesized code calling MCP *inside CSP's sandbox subprocess* —
  the substrate (`mcp_client.call_tool`) is built and tested; wiring it into the sandbox
  runtime is the remaining M3 step.

### C1 live verification (embedded SurrealDB + real BGE-small)
- `upsertCapability` → stored as `kind: CAPABILITY`, content-addressed id, metadata intact.
- Re-upsert by name → **same node id**, count unchanged (synthesized at most once, updates in place).
- `searchCapabilities` semantic reuse-first, calibrated cosine scores:

  | Goal vs capability | Score |
  |---|---|
  | near-exact ("rank customers by 90-day churn probability") | 0.964 |
  | same-intent rewording | 0.912 |
  | loose paraphrase ("which customers likely to cancel") | 0.675 |
  | unrelated goal | 0.415 |

  → **Reuse threshold set to 0.70** (was a guessed 0.85, which the data shows is too
  strict). Resolves open question #1.

C1–C2 are independent of the MCP migration. C5 depends on **M3** (worker calls tools via
MCP). C1 (Rust) and C2 (Python) can proceed in parallel.

---

## 7. What stays / what we do NOT change

- The plan→compile→deploy flow, job queue, retry/circuit-breaker/idempotency/rollback —
  **unchanged**. CSP adds a synthesis option to compile; it does not replace orchestration.
- The ingestion → graph pipeline (BGE-small, auto edge-wiring) — unchanged; capabilities
  ride the **same** embedding + search machinery as knowledge nodes.
- MCP (M1 done; M2/M3 pending) — CSP composes with it; it does not duplicate it.

---

## 8. File-by-file

**fluvio-graph (Rust)**
- `crates/fluvio-types/src/graph/enums.rs` — `NodeKind::Capability`, `NodeKindFilter`
- `services/fluvio-graph/src/graphql/types.rs` — `GqlNodeKind::Capability` + `From` impls
- `services/fluvio-graph/src/storage/surreal.rs` — confirm `kind` string round-trip

**agent-planner (Python)**
- `app/capabilities/__init__.py` — NEW
- `app/capabilities/graph_store.py` — NEW `GraphPlannerStore(PlannerStore)`
- `app/capabilities/orchestrator.py` — NEW CSP `Orchestrator` factory + synthesis_guidance
- `app/routers/compile.py` — precedence resolver (MCP tool → graph cap → synth)
- `requirements.txt` — add `csp-sdk @ git+https://github.com/ldbtech/capability-synthesis-protocol`

**docs**
- `docs/page.tsx` — the CSP section already explains synthesis; add a note that fluvioMe
  backs CSP's registry with the knowledge graph.

---

## 9. Open questions

1. ~~**Reuse threshold τ**~~ — **RESOLVED: 0.70**, calibrated live against BGE-small
   (0.96 exact / 0.91 same-intent / 0.67 paraphrase / 0.41 unrelated). See C1 verification.
2. **Per-workspace vs global capabilities** — are synthesized capabilities scoped to a
   workspace/owner (like other nodes) or shared company-wide? Likely owner-scoped with an
   opt-in "promote to shared."
3. **Code storage** — `metadata.generated_code` string vs blob ref in `source_uri` for
   large capabilities.
4. **Capability ↔ MCP tool boundary** — when a synthesized capability proves broadly
   useful, do we "graduate" it into a real MCP tool in `fluvio-tool-builder`? (A nice
   promotion path: synthesized verb → audited MCP tool.)
5. **Trust/audit** — synthesized code runs in CSP's sandbox; do we also gate it behind the
   enterprise token or a review step before it persists to a shared graph?

---

## Related
- `docs/MCP_MIGRATION_PLAN.md` — the MCP server CSP executes through (M1 done)
- `FLUVIOME_PLAN.md` — engine data flow + headless pivot
- CSP source: `~/Developer/AWS/capability_synthesis_protocol/csp`
  (`orchestrator/registry.py`, `planner_store.py`, `capability.py`)
