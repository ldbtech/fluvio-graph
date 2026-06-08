const pptxgen = require("pptxgenjs");

const pres = new pptxgen();
pres.layout = "LAYOUT_16x9";
pres.title = "Agent Planner Service — Complete Architecture";
pres.author = "alidaho";

// ─── Palette ────────────────────────────────────────────────────────────────
const C = {
  bg:        "0D1117",   // near-black
  bgCard:    "161B22",   // card surface
  bgCardAlt: "1C2333",   // alt card
  border:    "30363D",   // subtle border
  cyan:      "22D3EE",   // primary accent
  teal:      "14B8A6",   // secondary accent
  amber:     "F59E0B",   // warning / highlight
  rose:      "F43F5E",   // error / delete
  violet:    "8B5CF6",   // LLM / AI
  green:     "22C55E",   // success
  blue:      "3B82F6",   // info / GraphQL
  white:     "F0F6FF",   // primary text
  muted:     "8B949E",   // secondary text
  dimmed:    "6E7681",   // tertiary
};

const makeShadow = () => ({ type: "outer", blur: 8, offset: 3, angle: 135, color: "000000", opacity: 0.35 });

// ─── Helpers ─────────────────────────────────────────────────────────────────
function addBg(slide) {
  slide.background = { color: C.bg };
}

function addCard(slide, x, y, w, h, accentColor) {
  slide.addShape(pres.shapes.RECTANGLE, {
    x, y, w, h,
    fill: { color: C.bgCard },
    line: { color: accentColor || C.border, width: 1 },
    shadow: makeShadow(),
  });
  if (accentColor) {
    slide.addShape(pres.shapes.RECTANGLE, {
      x, y, w: 0.06, h,
      fill: { color: accentColor },
      line: { color: accentColor, width: 0 },
    });
  }
}

function title(slide, text, y = 0.18) {
  slide.addText(text, {
    x: 0.4, y, w: 9.2, h: 0.5,
    fontSize: 22, bold: true, color: C.white, fontFace: "Calibri",
    margin: 0,
  });
}

function subtitle(slide, text, y = 0.62) {
  slide.addText(text, {
    x: 0.4, y, w: 9.2, h: 0.28,
    fontSize: 10, color: C.muted, fontFace: "Calibri",
    margin: 0,
  });
}

function badge(slide, text, x, y, color) {
  slide.addShape(pres.shapes.RECTANGLE, { x, y, w: text.length * 0.085 + 0.2, h: 0.22, fill: { color }, line: { color, width: 0 } });
  slide.addText(text, { x: x + 0.04, y: y + 0.01, w: text.length * 0.085 + 0.12, h: 0.2, fontSize: 7.5, bold: true, color: C.bg, fontFace: "Calibri", margin: 0 });
}

// ─── SLIDE 1 — Title ─────────────────────────────────────────────────────────
{
  const s = pres.addSlide();
  s.background = { color: C.bg };

  // Large accent bar left
  s.addShape(pres.shapes.RECTANGLE, { x: 0, y: 0, w: 0.35, h: 5.625, fill: { color: C.cyan }, line: { color: C.cyan, width: 0 } });

  // Decorative grid dots
  for (let i = 0; i < 8; i++) {
    for (let j = 0; j < 5; j++) {
      s.addShape(pres.shapes.OVAL, { x: 0.8 + i * 1.1, y: 0.3 + j * 1.0, w: 0.06, h: 0.06, fill: { color: C.border }, line: { color: C.border, width: 0 } });
    }
  }

  s.addText("Agent Planner Service", {
    x: 0.7, y: 1.4, w: 8.9, h: 1.1,
    fontSize: 44, bold: true, color: C.white, fontFace: "Calibri", margin: 0,
  });
  s.addText("Complete Architecture", {
    x: 0.7, y: 2.45, w: 8.9, h: 0.8,
    fontSize: 36, bold: false, color: C.cyan, fontFace: "Calibri", margin: 0,
  });
  s.addText("Data Flow  ·  HTTP Contracts  ·  Component Interactions  ·  Internals", {
    x: 0.7, y: 3.2, w: 8.9, h: 0.4,
    fontSize: 13, color: C.muted, fontFace: "Calibri", margin: 0,
  });

  // Pill badges bottom
  const pills = [["FastAPI", C.teal], ["Anthropic Claude", C.violet], ["Apollo GraphQL", C.blue], ["Port 3007", C.amber]];
  pills.forEach(([t, c], i) => badge(s, t, 0.7 + i * 1.85, 4.9, c));
}

// ─── SLIDE 2 — System Context ────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "System Context — Service Boundaries");
  subtitle(s, "All actors, components, and external dependencies");

  // LEFT — Clients
  addCard(s, 0.25, 0.9, 2.2, 0.72, C.teal);
  s.addText("Frontend / PlannerPage", { x: 0.4, y: 0.96, w: 2.0, h: 0.28, fontSize: 9, bold: true, color: C.white, fontFace: "Calibri", margin: 0 });
  s.addText("React · Fluviome-web", { x: 0.4, y: 1.22, w: 2.0, h: 0.28, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

  addCard(s, 0.25, 1.75, 2.2, 0.72, C.teal);
  s.addText("Mobile / LAN Client", { x: 0.4, y: 1.81, w: 2.0, h: 0.28, fontSize: 9, bold: true, color: C.white, fontFace: "Calibri", margin: 0 });
  s.addText("Phone by IP · REST", { x: 0.4, y: 2.07, w: 2.0, h: 0.28, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

  // Arrow left → center
  s.addShape(pres.shapes.LINE, { x: 2.45, y: 1.6, w: 0.85, h: 0, line: { color: C.cyan, width: 2 } });
  s.addText("x-user-id\nHTTP", { x: 2.5, y: 1.35, w: 0.8, h: 0.4, fontSize: 6.5, color: C.cyan, align: "center", fontFace: "Calibri", margin: 0 });

  // CENTER — Agent Planner box
  s.addShape(pres.shapes.RECTANGLE, { x: 3.3, y: 0.85, w: 3.4, h: 3.8, fill: { color: C.bgCard }, line: { color: C.cyan, width: 2 }, shadow: makeShadow() });
  s.addText("Agent Planner :3007", { x: 3.35, y: 0.88, w: 3.3, h: 0.35, fontSize: 11, bold: true, color: C.cyan, fontFace: "Calibri", align: "center", margin: 0 });

  const routers = [["Chat Router", C.teal, "POST /chat · GET /history"], ["Compile Router", C.violet, "POST /plan/compile · /plan/context"], ["Execute Router", C.amber, "POST /deploy · GET /jobs/* · SSE"]];
  routers.forEach(([rName, rColor, rDesc], i) => {
    const ry = 1.32 + i * 1.05;
    addCard(s, 3.5, ry, 3.0, 0.82, rColor);
    s.addText(rName, { x: 3.68, y: ry + 0.08, w: 2.8, h: 0.28, fontSize: 9.5, bold: true, color: C.white, fontFace: "Calibri", margin: 0 });
    s.addText(rDesc, { x: 3.68, y: ry + 0.36, w: 2.8, h: 0.32, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });
  });

  // Internals label
  s.addText("worker_loop  ·  toolbox  ·  circuit_breakers", { x: 3.35, y: 4.48, w: 3.3, h: 0.22, fontSize: 7, color: C.dimmed, align: "center", fontFace: "Calibri", margin: 0 });

  // Arrow center → right
  s.addShape(pres.shapes.LINE, { x: 6.7, y: 2.0, w: 0.6, h: 0, line: { color: C.blue, width: 2 } });
  s.addShape(pres.shapes.LINE, { x: 6.7, y: 2.9, w: 0.6, h: 0, line: { color: C.violet, width: 2 } });
  s.addShape(pres.shapes.LINE, { x: 6.7, y: 3.8, w: 0.6, h: 0, line: { color: C.amber, width: 2 } });

  // RIGHT — External deps
  const deps = [
    ["Apollo Router :4001", "GraphQL Supergraph", C.blue],
    ["Anthropic API", "claude-sonnet-4  ·  LLM", C.violet],
    ["Tool Builder Service", "executeTool mutation", C.amber],
  ];
  deps.forEach(([dName, dDesc, dColor], i) => {
    const dy = 1.68 + i * 0.96;
    addCard(s, 7.3, dy, 2.45, 0.72, dColor);
    s.addText(dName, { x: 7.46, y: dy + 0.06, w: 2.25, h: 0.28, fontSize: 9, bold: true, color: C.white, fontFace: "Calibri", margin: 0 });
    s.addText(dDesc, { x: 7.46, y: dy + 0.34, w: 2.25, h: 0.28, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });
  });
}

// ─── SLIDE 3 — HTTP API Reference ────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "HTTP API Contract — All Endpoints");
  subtitle(s, "Every route, router, auth requirement, and purpose");

  const hdr = [
    { text: "Method", options: { bold: true, color: C.bg, fill: { color: C.cyan }, align: "center" } },
    { text: "Path", options: { bold: true, color: C.bg, fill: { color: C.cyan } } },
    { text: "Router", options: { bold: true, color: C.bg, fill: { color: C.cyan }, align: "center" } },
    { text: "Auth", options: { bold: true, color: C.bg, fill: { color: C.cyan }, align: "center" } },
    { text: "Description", options: { bold: true, color: C.bg, fill: { color: C.cyan } } },
  ];

  const rows = [
    [{ text: "GET", options: { color: C.green, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/health", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "main", options: { color: C.muted, align: "center", fill: { color: C.bgCard } } }, { text: "—", options: { color: C.dimmed, align: "center", fill: { color: C.bgCard } } }, { text: "Health check + circuit breaker states", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/chat", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "chat", options: { color: C.teal, align: "center", fill: { color: C.bgCardAlt } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Conversation loop — returns plan markdown + agent identity", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
    [{ text: "GET", options: { color: C.green, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/history/{workspace_id}", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "chat", options: { color: C.teal, align: "center", fill: { color: C.bgCard } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCard } } }, { text: "Return full chat history for a workspace", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/plan/compile", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "compile", options: { color: C.violet, align: "center", fill: { color: C.bgCardAlt } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Compile approved markdown → validated JSON steps array", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/plan/context", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "compile", options: { color: C.violet, align: "center", fill: { color: C.bgCard } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCard } } }, { text: "Raw markdown context assembly (workspace KG)", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/deploy", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCardAlt } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Enqueue deploy job → returns job_id immediately (fire-and-forget)", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
    [{ text: "GET", options: { color: C.green, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/jobs/{job_id}/status", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCard } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCard } } }, { text: "Poll job: queued / running / completed / failed", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "GET SSE", options: { color: C.cyan, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/jobs/{job_id}/stream", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCardAlt } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Server-Sent Events stream of real-time log lines", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
    [{ text: "GET", options: { color: C.green, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/deployments/{workspace_id}", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCard } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCard } } }, { text: "Deployment history / audit log", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/sandbox/resolve", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCardAlt } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Get port mapping for sandbox containers", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
    [{ text: "POST", options: { color: C.amber, bold: true, align: "center", fill: { color: C.bgCard } } }, { text: "/sandbox/provision", options: { color: C.white, fill: { color: C.bgCard } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCard } } }, { text: "x-user-id", options: { color: C.rose, align: "center", fill: { color: C.bgCard } } }, { text: "Create sandbox environment if not present", options: { color: C.muted, fill: { color: C.bgCard } } }],
    [{ text: "GET", options: { color: C.green, bold: true, align: "center", fill: { color: C.bgCardAlt } } }, { text: "/circuit-breakers", options: { color: C.white, fill: { color: C.bgCardAlt } } }, { text: "execute", options: { color: C.amber, align: "center", fill: { color: C.bgCardAlt } } }, { text: "—", options: { color: C.dimmed, align: "center", fill: { color: C.bgCardAlt } } }, { text: "Per-tool circuit breaker states (debug endpoint)", options: { color: C.muted, fill: { color: C.bgCardAlt } } }],
  ];

  s.addTable([hdr, ...rows], {
    x: 0.25, y: 0.92, w: 9.5, h: 4.5,
    colW: [0.75, 1.85, 0.8, 0.85, 5.25],
    border: { pt: 0.5, color: C.border },
    fontSize: 8.5, fontFace: "Calibri",
    rowH: 0.34,
  });
}

// ─── SLIDE 4 — Chat Flow ─────────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "POST /chat — Full Data Flow  ·  Phases 18 & 20");
  subtitle(s, "Intent detection → KG assembly → agent designation → plan writing → reflection");

  // Actors row
  const actors = [
    ["Client", C.teal],
    ["Auth", C.blue],
    ["Intent\nEngine", C.muted],
    ["Plan\nOrchestrator", C.cyan],
    ["Agent\nDesignator", C.amber],
    ["Plan\nWriter", C.violet],
    ["Reflection\nEngine", C.violet],
    ["Anthropic\nAPI", C.rose],
  ];
  const actorXs = [0.3, 1.45, 2.55, 3.65, 4.75, 5.85, 7.0, 8.55];
  const laneY1 = 1.32;
  const laneY2 = 5.2;

  actors.forEach(([name, color], i) => {
    const ax = actorXs[i];
    addCard(s, ax, 0.88, 0.9, 0.44, color);
    s.addText(name, { x: ax, y: 0.88, w: 0.9, h: 0.44, fontSize: 6.5, bold: true, color: C.white, fontFace: "Calibri", align: "center", valign: "middle", margin: 0 });
    // Lifeline
    s.addShape(pres.shapes.LINE, { x: ax + 0.45, y: laneY1, w: 0, h: laneY2 - laneY1, line: { color: C.border, width: 1, dashType: "dash" } });
  });

  // Arrow helper
  function arrow(sx, sy, ex, ey, label, color = C.white, dashed = false) {
    s.addShape(pres.shapes.LINE, { x: sx, y: sy, w: ex - sx, h: 0, line: { color, width: 1.2, dashType: dashed ? "dash" : "solid" } });
    // Arrowhead
    s.addShape(pres.shapes.LINE, { x: ex - 0.12, y: sy - 0.06, w: 0.12, h: 0.06, line: { color, width: 1.2 } });
    s.addShape(pres.shapes.LINE, { x: ex - 0.12, y: sy + 0.06, w: 0.12, h: 0.06, line: { color, width: 1.2 } });
    if (label) s.addText(label, { x: Math.min(sx, ex) + 0.05, y: sy - 0.2, w: Math.abs(ex - sx) - 0.1, h: 0.18, fontSize: 6, color, fontFace: "Calibri", margin: 0 });
  }
  function retArrow(sx, sy, ex, ey, label, color = C.dimmed) {
    arrow(ex, sy, sx, sy, label, color, true);
  }

  const steps = [
    // [fromIdx, toIdx, y, label, color, isReturn]
    [0, 1, 1.38, "POST /chat {workspace_id, message}", C.white, false],
    [1, 1, 1.62, "myWorkspaces(userId) → verify", C.blue, false],
    [1, 1, 1.82, "addChatMessage(sender='user')", C.blue, false],
    [0, 2, 2.08, "_is_deploy_intent(message)?", C.muted, false],
    [0, 1, 2.32, "PARALLEL: fetchChatHistory + fetchConnectors", C.cyan, false],
    [0, 2, 2.56, "needs_clarification() [Phase 20]", C.muted, false],
    [0, 3, 2.80, "PARALLEL: generate_plan_context + user_profile + nodes + docs", C.cyan, false],
    [0, 4, 3.04, "designate(first_msg, KG context)", C.amber, false],
    [0, 5, 3.28, "write_plan(session, message, context, history)", C.violet, false],
    [5, 7, 3.50, "POST /v1/messages (plan_writer.txt)", C.violet, false],
    [7, 5, 3.70, "plan markdown text", C.dimmed, true],
    [0, 6, 3.94, "reflect_on_plan(plan, tools_ctx) [Phase 18]", C.violet, false],
    [6, 7, 4.14, "POST /v1/messages (plan_reflection.txt)", C.violet, false],
    [7, 6, 4.34, "APPROVED | REVISED: ...", C.dimmed, true],
    [0, 1, 4.56, "addChatMessage(sender='ai', content=plan)", C.blue, false],
    [1, 0, 4.76, "ChatResponse {plan_md, intent, agent_name, agent_role}", C.teal, true],
  ];

  steps.forEach(([fi, ti, sy, label, color, isRet]) => {
    const fx = actorXs[fi] + 0.45;
    const tx = actorXs[ti] + 0.45;
    if (isRet) {
      arrow(tx, sy, fx, sy, label, color, true);
    } else {
      arrow(fx, sy, tx, sy, label, color, false);
    }
  });
}

// ─── SLIDE 5 — Compile Flow ───────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "POST /plan/compile — Validate-and-Retry Loop  ·  Phases 19, 21, 22, 23");
  subtitle(s, "Schema injection → tool graph → RAG → LLM call → 3-attempt validation loop");

  // Two-column layout
  // LEFT: inputs + prompt assembly
  addCard(s, 0.2, 0.88, 4.5, 2.12, C.blue);
  s.addText("① Gather (concurrent  asyncio.gather)", { x: 0.36, y: 0.92, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.cyan, fontFace: "Calibri", margin: 0 });
  const gathers = [
    "resolve_workspace_config(client, workspace_id)",
    "fetch_connectors_with_resources(client) → db_schema  [Phase 23]",
    "fetch_available_tools(client) → active_tool_ids",
    "fetch_similar_deployments(client, query) → RAG examples  [Phase 19]",
  ];
  gathers.forEach((g, i) => s.addText("▸ " + g, { x: 0.36, y: 1.22 + i * 0.34, w: 4.3, h: 0.3, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 }));

  addCard(s, 0.2, 3.04, 4.5, 1.86, C.violet);
  s.addText("② System Prompt Assembly", { x: 0.36, y: 3.08, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.violet, fontFace: "Calibri", margin: 0 });
  const prompt_parts = [
    "toolbox_section (registry.format_for_prompt()) — injected FIRST",
    "step_formulation.txt + environment_context JSON",
    "schema_section (exact column names)  [Phase 23]",
    "tool_graph_section (produces/consumes)  [Phase 22]",
    "User prompt: approved_markdown or chat history + RAG examples",
  ];
  prompt_parts.forEach((p, i) => s.addText("▸ " + p, { x: 0.36, y: 3.36 + i * 0.3, w: 4.3, h: 0.28, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 }));

  // RIGHT: retry loop
  addCard(s, 5.0, 0.88, 4.75, 4.02, C.amber);
  s.addText("③ Validate-and-Retry Loop  (max 3 attempts)", { x: 5.16, y: 0.92, w: 4.5, h: 0.26, fontSize: 9, bold: true, color: C.amber, fontFace: "Calibri", margin: 0 });

  const loop_steps = [
    [C.rose,   "POST api.anthropic.com/v1/messages\n{model: claude-sonnet-4, max_tokens: 4096, timeout: 90s}"],
    [C.white,  "_parse_claude_json() — strip ``` fences → JSON.parse\n   ✗ parse error → append to messages → retry"],
    [C.teal,   "registry.validate_steps(raw_steps)  [Phase 21]\n   ✗ tool_id/action/arg errors → build_retry_prompt() → retry"],
    [C.cyan,   "explain_sql(sql, db_url) — real EXPLAIN on Postgres  [Phase 23]\n   ✗ EXPLAIN fails → corrective prompt → retry"],
    [C.green,  "✓ ALL PASS → CompileResponse {steps: [validated]}\n   (non-blocking: validate_sql_columns() warns on bad refs)"],
  ];
  loop_steps.forEach(([c, txt], i) => {
    addCard(s, 5.15, 1.26 + i * 0.68, 4.45, 0.6, c);
    s.addText(txt, { x: 5.3, y: 1.3 + i * 0.68, w: 4.25, h: 0.52, fontSize: 7.5, color: C.white, fontFace: "Calibri", margin: 0 });
  });

  // Loop back arrow
  s.addShape(pres.shapes.LINE, { x: 9.45, y: 1.56, w: 0, h: 2.72, line: { color: C.amber, width: 1.5, dashType: "dash" } });
  s.addText("retry →", { x: 9.48, y: 2.7, w: 0.7, h: 0.22, fontSize: 6.5, color: C.amber, fontFace: "Calibri", margin: 0 });
}

// ─── SLIDE 6 — Deploy Flow ────────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "POST /deploy — Async Job Queue + Worker  ·  Phases 7, 9, 11, 19, 23");
  subtitle(s, "Fire-and-forget enqueue → background worker → SSE stream → audit → RAG write-back");

  // Two columns
  // LEFT: request path
  addCard(s, 0.2, 0.88, 4.5, 1.62, C.teal);
  s.addText("Request Path (synchronous — <1ms)", { x: 0.36, y: 0.92, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.teal, fontFace: "Calibri", margin: 0 });
  const req_steps = [
    "① POST /deploy {workspace_id, sandbox_id?, steps[]}",
    "② verify_workspace_access → myWorkspaces GraphQL",
    "③ create JobRecord(job_id=uuid, status=QUEUED)",
    "④ job_store.enqueue(record) → asyncio.Queue",
    "⑤ return {job_id, status: 'queued', total} immediately",
  ];
  req_steps.forEach((r, i) => s.addText(r, { x: 0.36, y: 1.22 + i * 0.24, w: 4.3, h: 0.22, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 }));

  addCard(s, 0.2, 2.6, 4.5, 1.0, C.cyan);
  s.addText("Client Observability (concurrent with worker)", { x: 0.36, y: 2.64, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.cyan, fontFace: "Calibri", margin: 0 });
  s.addText("GET /jobs/{job_id}/stream  → SSE text/event-stream\n   (each log line: data: <line>\\n\\n  ·  keepalive every 30s)\nGET /jobs/{job_id}/status  → {status, progress, total, error}\nGET /deployments/{workspace_id}  → audit history", { x: 0.36, y: 2.94, w: 4.3, h: 0.6, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

  addCard(s, 0.2, 3.7, 4.5, 1.56, C.amber);
  s.addText("Job Completion (success path)", { x: 0.36, y: 3.74, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.amber, fontFace: "Calibri", margin: 0 });
  s.addText("① audit_store.finish_run(job_id, status='completed')\n② store_deployment_summary(client, DeploymentRun) [Phase 19]\n   → KG write-back for future RAG retrieval\n③ addChatMessage(logs markdown)\n④ job.close_streams() → emit(None) to all SSE subscribers", { x: 0.36, y: 4.04, w: 4.3, h: 1.16, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

  // RIGHT: worker per-step
  addCard(s, 5.0, 0.88, 4.75, 4.38, C.violet);
  s.addText("Worker Loop — Per Step Execution", { x: 5.16, y: 0.92, w: 4.5, h: 0.26, fontSize: 9, bold: true, color: C.violet, fontFace: "Calibri", margin: 0 });

  const wsteps = [
    [C.blue,   "resolve credential_ref → inject db_url / tableau / powerbi into step.arguments.context"],
    [C.teal,   "IdempotencyStore: already_done(job_id, step_index)?  [Phase 9]\n→ IF yes: skip silently (idempotent re-runs)"],
    [C.cyan,   "explain_sql(sql, db_url) for spark/dbt steps  [Phase 23]\n→ PermanentToolError if EXPLAIN fails → abort"],
    [C.amber,  "CircuitBreaker.get_breaker(tool_id)  [Phase 7]\n→ CircuitOpenError: skip step, pipeline continues"],
    [C.violet, "with_retry(executeTool mutation, attempts=3, base_delay=1.0s)\n→ executeTool(toolId, inputs) via Apollo Router :4001"],
    [C.green,  "IdempotencyStore.mark_done(key)\nAuditStore.record_step(duration_ms, status)\nSSEStream.emit(log_line)"],
    [C.rose,   "PermanentToolError → run_rollback() [reversed]\nRollbackRegistry: mutable steps register compensating actions"],
  ];
  wsteps.forEach(([c, txt], i) => {
    addCard(s, 5.15, 1.26 + i * 0.52, 4.45, 0.46, c);
    s.addText(txt, { x: 5.3, y: 1.28 + i * 0.52, w: 4.25, h: 0.42, fontSize: 7, color: C.white, fontFace: "Calibri", margin: 0 });
  });
}

// ─── SLIDE 7 — Agent Designation ─────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Agent Designation System");
  subtitle(s, "First message → keyword scoring → persona selection → AgentSession + KG company context");

  // Five agent cards
  const agents = [
    { name: "Marcus", role: "ML Engineer", color: C.rose, triggers: "model, ml, predict, churn, train, classifier, forecast, embedding, fine-tune", personality: "Curious & experimental; explains trade-offs, flags data uncertainty before training" },
    { name: "Sage", role: "Platform Engineer", color: C.blue, triggers: "kafka, stream, airflow, dag, schedule, cron, realtime, ingest, queue, topic, event", personality: "Direct & infra-focused; thinks in topics, DAGs, schedules, and retries" },
    { name: "Zephyr", role: "Data Architect", color: C.violet, triggers: "architecture, schema design, migration, warehouse, lakehouse, data model, contract", personality: "Big-picture & strategic; designs schemas and data contracts before anything runs" },
    { name: "Nova", role: "Data Analyst", color: C.amber, triggers: "report, dashboard, pdf, tableau, powerbi, chart, trend, kpi, insight, analyze", personality: "Warm & narrative-driven; turns numbers into a story, leads with the insight" },
    { name: "Aria", role: "Data Engineer  ★ DEFAULT", color: C.teal, triggers: "pipeline, etl, elt, clean, transform, deduplicate, csv, load, extract, join, aggregate", personality: "Precise & methodical; cleans and validates data before trusting it" },
  ];

  agents.forEach((a, i) => {
    addCard(s, 0.2 + i * 1.93, 0.88, 1.8, 3.3, a.color);
    s.addText(a.name, { x: 0.22 + i * 1.93, y: 0.96, w: 1.76, h: 0.32, fontSize: 13, bold: true, color: a.color, fontFace: "Calibri", align: "center", margin: 0 });
    s.addText(a.role, { x: 0.22 + i * 1.93, y: 1.28, w: 1.76, h: 0.26, fontSize: 8, color: C.white, fontFace: "Calibri", align: "center", margin: 0 });
    s.addShape(pres.shapes.LINE, { x: 0.3 + i * 1.93, y: 1.56, w: 1.6, h: 0, line: { color: a.color, width: 0.5 } });
    s.addText("Triggers:", { x: 0.3 + i * 1.93, y: 1.62, w: 1.65, h: 0.2, fontSize: 7, bold: true, color: a.color, fontFace: "Calibri", margin: 0 });
    s.addText(a.triggers, { x: 0.3 + i * 1.93, y: 1.82, w: 1.65, h: 0.9, fontSize: 7, color: C.muted, fontFace: "Calibri", margin: 0 });
    s.addShape(pres.shapes.LINE, { x: 0.3 + i * 1.93, y: 2.74, w: 1.6, h: 0, line: { color: C.border, width: 0.5 } });
    s.addText(a.personality, { x: 0.3 + i * 1.93, y: 2.8, w: 1.65, h: 1.2, fontSize: 7, color: C.dimmed, fontFace: "Calibri", italic: true, margin: 0 });
  });

  // select_agent() description
  addCard(s, 0.2, 4.28, 4.5, 1.12, C.cyan);
  s.addText("select_agent(message: str) → AgentProfile", { x: 0.36, y: 4.32, w: 4.3, h: 0.26, fontSize: 9, bold: true, color: C.cyan, fontFace: "Calibri", margin: 0 });
  s.addText("• Word-boundary regex scores each profile by matching trigger keywords\n• Highest score wins; ties broken by pool order (specialized roles first)\n• Falls back to Aria (data engineer) when no keywords match", { x: 0.36, y: 4.6, w: 4.3, h: 0.72, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

  addCard(s, 5.0, 4.28, 4.75, 1.12, C.amber);
  s.addText("designate() → AgentSession", { x: 5.16, y: 4.32, w: 4.5, h: 0.26, fontSize: 9, bold: true, color: C.amber, fontFace: "Calibri", margin: 0 });
  s.addText("• Calls select_agent() on the FIRST user message (anchored for whole session)\n• summarize_company_context(): company name, industry, role, top-8 KG nodes, top-5 docs\n• Returns AgentSession {session_id, agent, workspace_id, twin_owner_id, company_context}", { x: 5.16, y: 4.6, w: 4.5, h: 0.72, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });
}

// ─── SLIDE 8 — Plan Orchestrator ──────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Plan Context Assembly — Knowledge Graph Integration");
  subtitle(s, "generate_plan_context() fetches 7+ data sources concurrently and assembles a token-budgeted markdown brief");

  // Layer 1 — parallel fetch
  addCard(s, 0.2, 0.88, 9.6, 0.26, C.cyan);
  s.addText("LAYER 1 — Parallel Fetch  (asyncio.gather)", { x: 0.36, y: 0.88, w: 9.3, h: 0.26, fontSize: 9, bold: true, color: C.cyan, fontFace: "Calibri", margin: 0 });
  const l1 = [
    ["fetch_connectors\n_with_resources()", C.blue],
    ["fetch_knowledge\n_documents(ws_id)", C.teal],
    ["fetch_semantic\n_nodes(ws_id)", C.teal],
    ["fetch_available\n_tools()", C.amber],
    ["fetch_workspace\n_shares(ws_id)", C.violet],
  ];
  l1.forEach(([t, c], i) => {
    addCard(s, 0.2 + i * 1.92, 1.18, 1.82, 0.64, c);
    s.addText(t, { x: 0.24 + i * 1.92, y: 1.22, w: 1.76, h: 0.56, fontSize: 7.5, bold: true, color: C.white, fontFace: "Calibri", align: "center", valign: "middle", margin: 0 });
  });

  // Layer 2 — user / company
  addCard(s, 0.2, 1.96, 9.6, 0.26, C.blue);
  s.addText("LAYER 2 — User & Company IAM  (sequential, only when user_id present)", { x: 0.36, y: 1.96, w: 9.3, h: 0.26, fontSize: 9, bold: true, color: C.blue, fontFace: "Calibri", margin: 0 });
  const l2 = [
    ["fetch_user_profile\n(user_id)", C.blue],
    ["fetch_company_users\n(companyId)", C.blue],
    ["fetch_company_teams\n(companyId)", C.blue],
    ["fetch_team_details\n(team_id) × N", C.blue],
  ];
  l2.forEach(([t, c], i) => {
    addCard(s, 0.2 + i * 2.42, 2.26, 2.3, 0.64, c);
    s.addText(t, { x: 0.24 + i * 2.42, y: 2.3, w: 2.24, h: 0.56, fontSize: 7.5, bold: true, color: C.white, fontFace: "Calibri", align: "center", valign: "middle", margin: 0 });
  });

  // Layer 3 — shared twins
  addCard(s, 0.2, 3.04, 9.6, 0.26, C.violet);
  s.addText("LAYER 3 — Shared Twin Knowledge  (for each share ≠ self, parallel)", { x: 0.36, y: 3.04, w: 9.3, h: 0.26, fontSize: 9, bold: true, color: C.violet, fontFace: "Calibri", margin: 0 });
  const l3 = [
    ["fetch_knowledge_docs\n(share_user_id)\n→ tagged [Shared Twin]", C.violet],
    ["fetch_semantic_nodes\n(share_user_id)\n→ tagged [Shared Twin Node]", C.violet],
  ];
  l3.forEach(([t, c], i) => {
    addCard(s, 0.2 + i * 3.0, 3.34, 2.85, 0.72, c);
    s.addText(t, { x: 0.24 + i * 3.0, y: 3.38, w: 2.78, h: 0.64, fontSize: 7.5, bold: true, color: C.white, fontFace: "Calibri", align: "center", valign: "middle", margin: 0 });
  });

  // Output assembly
  addCard(s, 0.2, 4.18, 9.6, 1.18, C.teal);
  s.addText("OUTPUT ASSEMBLY  →  ContextBudget().assemble(raw_sections)", { x: 0.36, y: 4.22, w: 9.3, h: 0.26, fontSize: 9, bold: true, color: C.teal, fontFace: "Calibri", margin: 0 });
  const sections = ["Workspace Connectors & Schemas", "Knowledge Documents", "Semantic Graph Nodes", "Available Execution Tools", "Company IAM Directory", "Team Squads & Workflows", "Current User Profile"];
  sections.forEach((sec, i) => {
    s.addText("## " + sec, { x: 0.36 + (i % 4) * 2.38, y: 4.52 + Math.floor(i / 4) * 0.34, w: 2.3, h: 0.3, fontSize: 7, color: C.white, fontFace: "Calibri", margin: 0 });
  });
}

// ─── SLIDE 9 — Reliability ────────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Reliability Stack  ·  Phases 7, 9, 11");
  subtitle(s, "Circuit breaker · Retry with backoff · Idempotency · Audit trail · Rollback compensation");

  const cols = [
    {
      title: "⚡ Circuit Breaker  [Phase 7]",
      color: C.amber,
      items: [
        "Per tool_id circuit breaker instance",
        "States: CLOSED → OPEN → HALF_OPEN",
        "CircuitOpenError → step skipped",
        "Pipeline continues (graceful degradation)",
        "GET /circuit-breakers exposes all states",
        "Prevents cascade failure on flaky tools",
      ],
    },
    {
      title: "🔄 Retry + Idempotency  [7, 9]",
      color: C.teal,
      items: [
        "with_retry(): 3 attempts, base_delay=1.0s",
        "Exponential backoff between attempts",
        "TransientToolError → retried",
        "PermanentToolError → abort immediately",
        "IdempotencyStore key = (job_id, step_index)",
        "Duplicate re-run → silently skipped",
        "Survives partial failures & restarts",
      ],
    },
    {
      title: "📋 Audit + Rollback  [Phase 11]",
      color: C.violet,
      items: [
        "AuditStore: in-memory per run",
        "start_run() / record_step(duration_ms) / finish_run()",
        "GET /deployments/{ws} → full history",
        "RollbackRegistry: mutable steps register",
        "  compensating actions (e.g. delete_report)",
        "On failure: actions run in reverse order",
        "Rollback failures logged (non-fatal)",
      ],
    },
  ];

  cols.forEach((col, i) => {
    addCard(s, 0.2 + i * 3.25, 0.88, 3.1, 4.5, col.color);
    s.addText(col.title, { x: 0.36 + i * 3.25, y: 0.94, w: 2.9, h: 0.3, fontSize: 9, bold: true, color: col.color, fontFace: "Calibri", margin: 0 });
    s.addShape(pres.shapes.LINE, { x: 0.36 + i * 3.25, y: 1.28, w: 2.8, h: 0, line: { color: col.color, width: 0.5 } });
    col.items.forEach((item, j) => {
      s.addText([{ text: item.startsWith("  ") ? "  " + item.trim() : "▸ " + item }], {
        x: 0.36 + i * 3.25, y: 1.36 + j * 0.44, w: 2.9, h: 0.4,
        fontSize: 8, color: j % 2 === 0 ? C.white : C.muted, fontFace: "Calibri", margin: 0,
      });
    });
  });
}

// ─── SLIDE 10 — Toolbox ───────────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Toolbox Registry & Step Validation  ·  Phases 21, 22, 23");
  subtitle(s, "Manifest loading → capability graph → schema injection → 3-stage LLM output validation");

  const flow = [
    { label: "① Startup: toolbox.load()", color: C.teal, desc: "Reads YAML manifests from TOOL_BUILDER_TOOLS_DIR\n→ Builds ToolRegistry keyed by tool_id" },
    { label: "② Scope to workspace", color: C.blue, desc: "fetch_available_tools() → active_tool_ids\ntoolbox.registry_for(ids) → scoped ToolRegistry\nregistry.format_for_prompt() → prepended FIRST in system prompt" },
    { label: "③ ToolCapabilityGraph  [Phase 22]", color: C.violet, desc: "ToolCapabilityGraph.from_active_tools(active_tools)\nTracks produces/consumes relationships\nformat_for_prompt() → tool_graph_section appended to prompt" },
    { label: "④ Schema Injection  [Phase 23]", color: C.cyan, desc: "extract_schema_from_resources(connectors_data) → db_schema\nformat_schema_for_prompt(db_schema) → exact column names\nInjected into system prompt so Claude writes correct SQL" },
    { label: "⑤ LLM Call", color: C.rose, desc: "POST api.anthropic.com/v1/messages\nModel: claude-sonnet-4-20250514  ·  max_tokens: 4096  ·  timeout: 90s" },
    { label: "⑥ 3-Stage Validation  (retry loop)", color: C.amber, desc: "A. _parse_claude_json() — strip code fences → JSON.parse\nB. registry.validate_steps() — tool_id, action, required args  [Phase 21]\nC. explain_sql() — real PostgreSQL EXPLAIN for spark/dbt steps  [Phase 23]" },
    { label: "⑦ Non-blocking Warnings", color: C.green, desc: "validate_sql_columns() checks column references against db_schema\nWarnings stored in step['_warnings'] — never block compilation\nLogged for debugging; surfaced to the caller via step metadata" },
  ];

  flow.forEach((f, i) => {
    const col = i < 4 ? 0 : 1;
    const row = i < 4 ? i : i - 4;
    const x = 0.2 + col * 5.0;
    const y = 0.88 + row * 1.12;
    addCard(s, x, y, 4.6, 1.0, f.color);
    s.addText(f.label, { x: x + 0.16, y: y + 0.06, w: 4.35, h: 0.26, fontSize: 9, bold: true, color: f.color, fontFace: "Calibri", margin: 0 });
    s.addText(f.desc, { x: x + 0.16, y: y + 0.34, w: 4.35, h: 0.6, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });

    // Connector arrow
    if (col === 0 && i < 3) {
      s.addShape(pres.shapes.LINE, { x: x + 2.3, y: y + 1.0, w: 0, h: 0.12, line: { color: f.color, width: 1 } });
    }
  });
}

// ─── SLIDE 11 — Data Models ───────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Core Data Models");
  subtitle(s, "Pydantic request/response contracts and async job state machine");

  const models = [
    {
      name: "ChatRequest / ChatResponse",
      color: C.teal,
      fields: [
        ["ChatRequest", ""],
        ["workspace_id", "str"],
        ["message", "str"],
        ["", ""],
        ["ChatResponse", ""],
        ["response", "str  (plan markdown)"],
        ["intent", "str | None  — 'deploy' | 'clarification' | 'plan'"],
        ["agent_name", "str | None"],
        ["agent_role", "str | None"],
      ],
    },
    {
      name: "CompileRequest / CompileResponse",
      color: C.violet,
      fields: [
        ["CompileRequest", ""],
        ["workspace_id", "str"],
        ["approved_markdown", "str | None"],
        ["message", "str | None"],
        ["", ""],
        ["CompileResponse", ""],
        ["steps", "list[dict]"],
        ["  tool_id", "str"],
        ["  action", "str"],
        ["  arguments", "dict"],
        ["  credential_ref?", "str"],
        ["  _warnings?", "list[str]"],
      ],
    },
    {
      name: "DeployRequest / JobRecord",
      color: C.amber,
      fields: [
        ["DeployRequest", ""],
        ["workspace_id", "str"],
        ["sandbox_id", "str | None"],
        ["steps", "list[dict]"],
        ["", ""],
        ["JobRecord", ""],
        ["job_id", "str  (uuid hex)"],
        ["status", "JobStatus enum"],
        ["progress", "int  (steps done)"],
        ["total", "int  (steps total)"],
        ["error", "str | None"],
        ["_subscribers", "list[asyncio.Queue]  (SSE)"],
      ],
    },
    {
      name: "JobStatus State Machine",
      color: C.rose,
      fields: [
        ["QUEUED", "→ enqueued in asyncio.Queue"],
        ["RUNNING", "→ worker picked up the job"],
        ["COMPLETED", "→ all steps succeeded"],
        ["FAILED", "→ PermanentToolError hit"],
        ["", ""],
        ["Transitions:", ""],
        ["QUEUED → RUNNING", "worker picks up"],
        ["RUNNING → COMPLETED", "all steps done"],
        ["RUNNING → FAILED", "step raises permanent error"],
        ["", ""],
        ["SSE sentinel:", "None → close_streams()"],
      ],
    },
  ];

  models.forEach((m, i) => {
    const x = 0.2 + i * 2.42;
    addCard(s, x, 0.88, 2.28, 4.52, m.color);
    s.addText(m.name, { x: x + 0.12, y: 0.92, w: 2.1, h: 0.28, fontSize: 8, bold: true, color: m.color, fontFace: "Calibri", margin: 0 });
    s.addShape(pres.shapes.LINE, { x: x + 0.12, y: 1.22, w: 2.0, h: 0, line: { color: m.color, width: 0.5 } });
    m.fields.forEach((f, j) => {
      if (!f[0] && !f[1]) return;
      const isSectionHeader = f[1] === "";
      s.addText(f[0], { x: x + 0.12, y: 1.28 + j * 0.28, w: 1.1, h: 0.26, fontSize: isSectionHeader ? 7.5 : 7, bold: isSectionHeader, color: isSectionHeader ? m.color : C.white, fontFace: "Calibri", margin: 0 });
      if (f[1]) s.addText(f[1], { x: x + 1.1, y: 1.28 + j * 0.28, w: 1.12, h: 0.26, fontSize: 6.5, color: C.muted, fontFace: "Calibri", margin: 0 });
    });
  });
}

// ─── SLIDE 12 — File Structure ────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Service File Structure");
  subtitle(s, "services/agent-planner/app/  —  annotated module map");

  const col1 = [
    ["app/", C.cyan, true],
    ["main.py", C.white, false, "FastAPI app, lifespan, 3 routers"],
    ["config.py", C.white, false, "Settings: port 3007, gateway URL, API key"],
    ["auth.py", C.white, false, "verify_workspace_access() via myWorkspaces GQL"],
    ["intent.py", C.amber, false, "Phase 20: needs_clarification() — no LLM"],
    ["reflection.py", C.violet, false, "Phase 18: reflect_on_plan() — 2nd Anthropic call"],
    ["toolbox.py", C.teal, false, "ToolManifest loader, ToolRegistry"],
    ["tool_graph.py", C.violet, false, "Phase 22: ToolCapabilityGraph"],
    ["schema_inspector.py", C.cyan, false, "Phase 23: extract_schema, explain_sql"],
    ["credential_vault.py", C.rose, false, "CredentialRef: postgres/tableau/powerbi"],
    ["idempotency.py", C.teal, false, "Phase 9: IdempotencyStore"],
    ["rollback.py", C.rose, false, "RollbackRegistry (compensating actions)"],
    ["routers/chat.py", C.teal, false, "POST /chat  GET /history/{id}"],
    ["routers/compile.py", C.violet, false, "POST /plan/compile  /plan/context"],
    ["routers/execute.py", C.amber, false, "POST /deploy  GET /jobs/*  /sandbox/*"],
  ];

  const col2 = [
    ["agent/", C.amber, true],
    ["designation.py", C.white, false, "designate() → AgentSession"],
    ["plan_writer.py", C.violet, false, "write_plan() → Anthropic call"],
    ["profiles.py", C.amber, false, "5 AgentProfiles (Marcus/Sage/Zephyr/Nova/Aria)"],
    ["fetch/", C.blue, true],
    ["chat.py", C.white, false, "fetch_chat_history, add_chat_message"],
    ["connectors.py, documents.py", C.white, false, "fetch_connectors, fetch_docs"],
    ["iam.py, teams.py, nodes.py", C.white, false, "fetch_user_profile, company_users, teams"],
    ["memory/", C.violet, true],
    ["rag.py", C.violet, false, "Phase 19: fetch_similar_deployments"],
    ["store.py", C.violet, false, "Phase 19: store_deployment_summary"],
    ["reliability/", C.rose, true],
    ["circuit_breaker.py", C.rose, false, "Phase 7: per-tool CircuitBreaker"],
    ["retry.py", C.rose, false, "Phase 7: with_retry() exponential backoff"],
    ["errors.py", C.rose, false, "PermanentToolError / TransientToolError"],
  ];

  function renderFileTree(items, startX) {
    items.forEach((item, i) => {
      const [name, color, isDir, desc] = item;
      const y = 0.88 + i * 0.29;
      if (isDir) {
        s.addText((isDir ? "📁 " : "  ") + name, { x: startX, y, w: 2.2, h: 0.26, fontSize: 7.5, bold: true, color, fontFace: "Calibri", margin: 0 });
      } else {
        s.addText("├─ " + name, { x: startX + 0.1, y, w: 2.1, h: 0.26, fontSize: 7, color: C.white, fontFace: "Calibri", margin: 0 });
        if (desc) s.addText(desc, { x: startX + 2.3, y, w: 2.3, h: 0.26, fontSize: 6.5, color: C.muted, fontFace: "Calibri", italic: true, margin: 0 });
      }
    });
  }

  // Render two columns with desc
  col1.forEach((item, i) => {
    const [name, color, isDir, desc] = item;
    const y = 0.88 + i * 0.29;
    s.addText((isDir ? "📁 " : "├─ ") + name, { x: 0.2 + (isDir ? 0 : 0.1), y, w: 2.3, h: 0.26, fontSize: isDir ? 7.5 : 7, bold: isDir, color, fontFace: "Calibri", margin: 0 });
    if (desc) s.addText("← " + desc, { x: 2.55, y, w: 2.35, h: 0.26, fontSize: 6.5, color: C.dimmed, fontFace: "Calibri", italic: true, margin: 0 });
  });

  col2.forEach((item, i) => {
    const [name, color, isDir, desc] = item;
    const y = 0.88 + i * 0.29;
    s.addText((isDir ? "📁 " : "├─ ") + name, { x: 5.05 + (isDir ? 0 : 0.1), y, w: 2.3, h: 0.26, fontSize: isDir ? 7.5 : 7, bold: isDir, color, fontFace: "Calibri", margin: 0 });
    if (desc) s.addText("← " + desc, { x: 7.38, y, w: 2.4, h: 0.26, fontSize: 6.5, color: C.dimmed, fontFace: "Calibri", italic: true, margin: 0 });
  });

  // Divider
  s.addShape(pres.shapes.LINE, { x: 4.9, y: 0.88, w: 0, h: 4.6, line: { color: C.border, width: 1, dashType: "dash" } });
}

// ─── SLIDE 13 — Phase Summary ─────────────────────────────────────────────────
{
  const s = pres.addSlide();
  addBg(s);
  title(s, "Implementation Phases — Feature Map");
  subtitle(s, "Where each phase lives, what it does, and how it connects");

  const phases = [
    { num: "7",  name: "Retry + Circuit Breaker", color: C.rose,   where: "reliability/, jobs/worker.py",         what: "3-attempt exponential retry · per-tool circuit breaker · PermanentToolError vs TransientToolError" },
    { num: "9",  name: "Idempotency",             color: C.teal,   where: "idempotency.py, jobs/worker.py",        what: "key = (job_id, step_index) · skip already-done steps · safe for re-runs and partial failures" },
    { num: "11", name: "Audit Trail + Rollback",  color: C.amber,  where: "audit/, rollback.py, execute.py",       what: "Per-step timing · GET /deployments history · compensating actions reversed on failure" },
    { num: "18", name: "Plan Reflection",         color: C.violet, where: "reflection.py, routers/chat.py",        what: "2nd Anthropic call reviews plan · returns APPROVED or REVISED: · only fires on plan-looking text" },
    { num: "19", name: "RAG Deployment Memory",   color: C.blue,   where: "memory/rag.py, memory/store.py",        what: "Successful deployments written to KG · retrieved as few-shot examples at compile time" },
    { num: "20", name: "Intent Disambiguation",   color: C.cyan,   where: "intent.py, routers/chat.py",            what: "Grounded clarifying question · only on vague verbs · only first turn · uses actual table names" },
    { num: "21", name: "Plan Critique",           color: C.teal,   where: "plan_critique.py, compile router",      what: "Validate step tool_ids against live workspace tool registry · auto-retry with corrective prompt" },
    { num: "22", name: "Tool Capability Graph",   color: C.violet, where: "tool_graph.py, compile router",         what: "produces/consumes graph injected into system prompt · guides Claude to chain tools correctly" },
    { num: "23", name: "Schema + EXPLAIN",        color: C.amber,  where: "schema_inspector.py, worker + compile", what: "Exact column names in prompt · real PostgreSQL EXPLAIN before execution · catch semantic SQL errors" },
  ];

  phases.forEach((p, i) => {
    const y = 0.88 + i * 0.51;
    // Phase badge
    s.addShape(pres.shapes.RECTANGLE, { x: 0.2, y, w: 0.5, h: 0.42, fill: { color: p.color }, line: { color: p.color, width: 0 } });
    s.addText(p.num, { x: 0.2, y: y + 0.02, w: 0.5, h: 0.38, fontSize: 13, bold: true, color: C.bg, fontFace: "Calibri", align: "center", valign: "middle", margin: 0 });
    // Name
    s.addText(p.name, { x: 0.82, y: y + 0.01, w: 2.0, h: 0.22, fontSize: 9, bold: true, color: p.color, fontFace: "Calibri", margin: 0 });
    s.addText(p.where, { x: 0.82, y: y + 0.22, w: 2.0, h: 0.18, fontSize: 6.5, color: C.dimmed, fontFace: "Calibri", italic: true, margin: 0 });
    // Desc
    s.addText(p.what, { x: 2.95, y: y + 0.04, w: 7.0, h: 0.34, fontSize: 7.5, color: C.muted, fontFace: "Calibri", margin: 0 });
    if (i < phases.length - 1) s.addShape(pres.shapes.LINE, { x: 0.2, y: y + 0.48, w: 9.6, h: 0, line: { color: C.border, width: 0.5 } });
  });
}

// ─── Write ────────────────────────────────────────────────────────────────────
pres.writeFile({ fileName: "/Users/alidaho/Developer/AWS/rust/kg-engine/docs/agent-planner-architecture.pptx" })
  .then(() => console.log("✅ Saved: docs/agent-planner-architecture.pptx"))
  .catch(err => { console.error("❌ Error:", err); process.exit(1); });
