#!/usr/bin/env bash
# =============================================================================
# Fluvio — Notion Connector Test
# =============================================================================
#
# Full flow:
#   1. Health checks
#   2. Get OAuth URL → open browser
#   3. Paste callback code → exchange for token
#   4. List your Notion pages and databases
#   5. Select pages/databases to sync
#   6. Trigger sync
#   7. Poll until sync complete
#   8. Search the synced knowledge
#   9. Chat over the synced pages
#
# Usage:
#   bash scripts/test_notion_connector.sh
#   bash scripts/test_notion_connector.sh --group-id GROUP_ID
#   bash scripts/test_notion_connector.sh --user-id USER_UUID
# =============================================================================

set -euo pipefail

CONNECTORS_URL="http://localhost:3006/graphql"
DATABASE_URL="http://localhost:3005/graphql"
COLLAB_URL="http://localhost:3003/graphql"
TWIN_URL="http://localhost:3002/graphql"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

pass()    { echo -e "  ${GREEN}✓${RESET} $*"; }
fail()    { echo -e "  ${RED}✗${RESET} $*"; exit 1; }
info()    { echo -e "  ${CYAN}→${RESET} $*"; }
ask()     { echo -e "  ${YELLOW}?${RESET} $*"; }
section() { echo -e "\n${BOLD}${CYAN}── $* ──${RESET}"; }

# ── Parse flags ───────────────────────────────────────────────────────────────

GROUP_ID=""
USER_ID=""
for i in "$@"; do
  case $i in
    --group-id=*) GROUP_ID="${i#*=}" ;;
    --group-id)   shift; GROUP_ID="$1" ;;
    --user-id=*)  USER_ID="${i#*=}" ;;
    --user-id)    shift; USER_ID="$1" ;;
  esac
done

# ── Health checks ─────────────────────────────────────────────────────────────

section "Health checks"
curl -sf "http://localhost:3006/health" &>/dev/null && pass "fluvio-connectors :3006" || fail "fluvio-connectors not running"
curl -sf "http://localhost:3005/health" &>/dev/null && pass "fluvio-database :3005"   || fail "fluvio-database not running"
curl -sf "http://localhost:3001/health" &>/dev/null && pass "fluvio-graph :3001"       || fail "fluvio-graph not running"

# ── Fluvio user (not Notion — browser opens in Step 1 for Notion OAuth) ────────

section "Setup"
if [[ -z "$USER_ID" ]]; then
  info "Creating a test Fluvio user (fluvio-database)..."
  USER_RESP=$(curl -s -X POST "$DATABASE_URL" \
    -H "Content-Type: application/json" \
    -d '{
      "query": "mutation { createUser(input: { firebaseUid: \"test-notion-001\", email: \"notion-test@fluvio.ai\", displayName: \"Notion Test User\" }) { id displayName } }"
    }')
  USER_ID=$(echo "$USER_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['createUser']['id'])" 2>/dev/null)
  [[ -z "$USER_ID" ]] && {
    echo "$USER_RESP" | python3 -m json.tool 2>/dev/null || echo "$USER_RESP"
    fail "failed to create test user — pass --user-id YOUR_UUID if you have one"
  }
  pass "Test user: $USER_ID"
else
  pass "Using user ID: $USER_ID"
fi

if [[ -n "$GROUP_ID" ]]; then
  info "Syncing into group: $GROUP_ID"
else
  info "Syncing into personal twin"
fi

# ── Step 1: Get OAuth URL ─────────────────────────────────────────────────────

section "Step 1: Notion OAuth"
info "Your browser will open to authorize Notion (not for Fluvio user ID)."
info "Before this works you need a Notion OAuth app:"
info "  → https://www.notion.so/my-integrations → New integration → OAuth"
info "  → Set redirect URI: http://localhost:3006/oauth/notion/callback"
info "  → Add NOTION_CLIENT_ID and NOTION_CLIENT_SECRET to services/fluvio-connectors/.env"
echo ""

OAUTH_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d '{"query":"mutation { getOauthUrl(kind: \"notion\") { url state } }"}')

OAUTH_URL=$(echo "$OAUTH_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['getOauthUrl']['url'])" 2>/dev/null)
STATE=$(echo "$OAUTH_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['getOauthUrl']['state'])" 2>/dev/null)

[[ -z "$OAUTH_URL" ]] && fail "failed to get OAuth URL — is NOTION_CLIENT_ID set in .env?"

pass "OAuth URL generated"
info "State: $STATE"
echo ""
info "Opening Notion in your browser..."
open "$OAUTH_URL" 2>/dev/null || xdg-open "$OAUTH_URL" 2>/dev/null || echo "Open this URL: $OAUTH_URL"
echo ""

# ── Step 2: Get callback code ─────────────────────────────────────────────────

section "Step 2: Authorization"
info "After authorizing on Notion, your browser redirects to:"
info "http://localhost:3006/oauth/notion/callback?code=...&state=..."
info "Copy the code= value from that URL (shown in the browser address bar or JSON response)."
echo ""
ask "Paste the OAuth 'code' from the callback URL:"
read -r CODE
[[ -z "$CODE" ]] && fail "code required"
pass "Code received"

# ── Step 3: Exchange code → connect ───────────────────────────────────────────

section "Step 3: Connecting Notion"

GROUP_INPUT=""
[[ -n "$GROUP_ID" ]] && GROUP_INPUT=", groupId: \\\"$GROUP_ID\\\""

CONNECT_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"mutation { connectOauth(input: { kind: \\\"notion\\\", code: \\\"$CODE\\\", state: \\\"$STATE\\\"$GROUP_INPUT }) { id kind status } }\"}")

CONNECTOR_ID=$(echo "$CONNECT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['connectOauth']['id'])" 2>/dev/null)
[[ -z "$CONNECTOR_ID" ]] && {
  echo "$CONNECT_RESP" | python3 -m json.tool
  fail "Failed to connect Notion"
}
pass "Notion connected: $CONNECTOR_ID"

# ── Step 4: List pages and databases ─────────────────────────────────────────

section "Step 4: Your Notion pages and databases"

PAGES_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"{ connectorResources(connectorId: \\\"$CONNECTOR_ID\\\") { externalId name description selected nodeCount } }\"}")

echo "$PAGES_RESP" | python3 -c "
import sys, json
d     = json.load(sys.stdin)
pages = d['data']['connectorResources']
print(f'\n  Found {len(pages)} pages/databases:\n')
for i, p in enumerate(pages):
    sel  = '✓' if p['selected'] else ' '
    desc = f\" — {p['description'][:40]}\" if p.get('description') else ''
    print(f'  [{i+1:2d}] {sel} {p[\"name\"]}{desc}')
    print(f'       ID: {p[\"externalId\"]}')
print()
" 2>/dev/null

PAGE_COUNT=$(echo "$PAGES_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['data']['connectorResources']))" 2>/dev/null)
[[ "$PAGE_COUNT" -eq 0 ]] && fail "no pages found — make sure you granted access to pages during OAuth"
pass "Found $PAGE_COUNT pages/databases"

# ── Step 5: Select pages to sync ──────────────────────────────────────────────

section "Step 5: Select pages to sync"
ask "Enter page numbers to sync (comma-separated, e.g. 1,3,5):"
read -r SELECTION

SELECTED_IDS=$(echo "$PAGES_RESP" | python3 -c "
import sys, json
d     = json.load(sys.stdin)
pages = d['data']['connectorResources']
sel   = '$SELECTION'.strip()

selected = []
parts = [p.strip() for p in sel.split(',')]
for part in parts:
    if part.isdigit():
        idx = int(part) - 1
        if 0 <= idx < len(pages):
            selected.append(pages[idx]['externalId'])
    else:
        for p in pages:
            if part.lower() in p['name'].lower():
                selected.append(p['externalId'])
                break

print(','.join(selected))
" 2>/dev/null)

[[ -z "$SELECTED_IDS" ]] && fail "no pages selected"

EXTERNAL_IDS_JSON=$(echo "$SELECTED_IDS" | python3 -c "
import sys
ids = sys.stdin.read().strip().split(',')
print('[' + ', '.join(f'\\\\\\\"' + i + '\\\\\\\"' for i in ids) + ']')
")

SELECT_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"mutation { selectResources(input: { connectorId: \\\"$CONNECTOR_ID\\\", externalIds: $EXTERNAL_IDS_JSON }) { externalId selected } }\"}")

pass "Selected pages: $SELECTED_IDS"

# ── Step 6: Trigger sync ──────────────────────────────────────────────────────

section "Step 6: Syncing pages into knowledge graph"
info "Fetching and embedding Notion content..."

SYNC_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"mutation { syncNow(connectorId: \\\"$CONNECTOR_ID\\\") { id status } }\"}")

JOB_ID=$(echo "$SYNC_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['syncNow']['id'])" 2>/dev/null)
[[ -z "$JOB_ID" ]] && fail "failed to start sync job"
pass "Sync job started: $JOB_ID"

# ── Step 7: Poll until complete ───────────────────────────────────────────────

section "Step 7: Waiting for sync to complete"

MAX_WAIT=120
i=0
while true; do
  JOB_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"query\":\"{ syncJob(jobId: \\\"$JOB_ID\\\") { status nodesAdded error } }\"}")

  STATUS=$(echo "$JOB_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['syncJob']['status'])" 2>/dev/null)
  NODES=$(echo "$JOB_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['syncJob']['nodesAdded'])" 2>/dev/null || echo "0")

  echo -ne "\r  ${CYAN}→${RESET} Status: $STATUS | Nodes: $NODES | Elapsed: ${i}s  "

  if [[ "$STATUS" == "complete" ]]; then
    echo ""
    pass "Sync complete — $NODES nodes added to knowledge graph"
    break
  elif [[ "$STATUS" == "failed" ]]; then
    echo ""
    ERROR=$(echo "$JOB_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['syncJob']['error'])" 2>/dev/null)
    fail "Sync failed: $ERROR"
  fi

  sleep 2
  ((i+=2))
  if ((i >= MAX_WAIT)); then
    echo ""
    fail "Sync timed out after ${MAX_WAIT}s"
  fi
done

# ── Step 8: Search the synced knowledge ──────────────────────────────────────

section "Step 8: Search synced Notion knowledge"
ask "Enter a search query (e.g. 'product roadmap Q3'):"
read -r SEARCH_QUERY

if [[ -n "$GROUP_ID" ]]; then
  SEARCH_RESP=$(curl -s -X POST "$COLLAB_URL" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"query\":\"{ searchGroup(groupId: \\\"$GROUP_ID\\\", query: \\\"$SEARCH_QUERY\\\", topK: 5) { id text score } }\"}")

  echo "$SEARCH_RESP" | python3 -c "
import sys, json
d       = json.load(sys.stdin)
results = d['data']['searchGroup']
print(f'\n  Found {len(results)} results:\n')
for r in results:
    score = r['score']
    text  = r['text'][:120]
    print(f'  score={score:.3f}  {text}...')
print()
" 2>/dev/null
else
  SEARCH_RESP=$(curl -s -X POST "http://localhost:3001/graphql" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"query\":\"{ search(query: \\\"$SEARCH_QUERY\\\", config: { similarityTopK: 5 }) { score node { id sourceText } } }\"}")

  echo "$SEARCH_RESP" | python3 -c "
import sys, json
d       = json.load(sys.stdin)
results = d['data']['search']
print(f'\n  Found {len(results)} results:\n')
for r in results:
    score = r['score']
    text  = r['node']['sourceText'][:120]
    print(f'  score={score:.3f}  {text}...')
print()
" 2>/dev/null
fi

# ── Step 9: Chat over the pages ───────────────────────────────────────────────

section "Step 9: Chat over synced Notion knowledge"
ask "Ask a question about your Notion pages:"
read -r QUESTION

if [[ -n "$GROUP_ID" ]]; then
  CHAT_RESP=$(curl -s -X POST "$COLLAB_URL" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"query\":\"{ groupChat(groupId: \\\"$GROUP_ID\\\", question: \\\"$QUESTION\\\") { answer sources { id score } } }\"}")

  ANSWER=$(echo "$CHAT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['groupChat']['answer'])" 2>/dev/null)
else
  CHAT_RESP=$(curl -s -X POST "$TWIN_URL" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"query\":\"{ chat(message: \\\"$QUESTION\\\") { answer } }\"}")

  ANSWER=$(echo "$CHAT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['chat']['answer'])" 2>/dev/null)
fi

echo ""
echo -e "  ${BOLD}Answer:${RESET}"
echo "$ANSWER" | sed 's/^/    /'
echo ""

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${BOLD}  ✅  Notion connector test complete${RESET}"
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "  Connector ID:  ${CYAN}$CONNECTOR_ID${RESET}"
echo -e "  Job ID:        ${CYAN}$JOB_ID${RESET}"
echo -e "  Pages synced:  ${CYAN}$SELECTED_IDS${RESET}"
echo ""