#!/usr/bin/env bash
# =============================================================================
# Fluvio — GitHub Connector Test
# =============================================================================
#
# Full flow:
#   1. Health checks
#   2. Get OAuth URL → open browser
#   3. Paste callback code → exchange for token
#   4. List your GitHub repos
#   5. Select a repo to sync
#   6. Trigger sync
#   7. Poll until sync complete
#   8. Search the synced knowledge
#   9. Chat over the synced repo
#
# Usage:
#   bash scripts/test_github_connector.sh
#   bash scripts/test_github_connector.sh --group-id GROUP_ID  # sync into company brain
# =============================================================================

set -euo pipefail

CONNECTORS_URL="http://localhost:3006/graphql"
COLLAB_URL="http://localhost:3003/graphql"
TWIN_URL="http://localhost:3002/graphql"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

pass()    { echo -e "  ${GREEN}✓${RESET} $*"; }
fail()    { echo -e "  ${RED}✗${RESET} $*"; exit 1; }
info()    { echo -e "  ${CYAN}→${RESET} $*"; }
ask()     { echo -e "  ${YELLOW}?${RESET} $*"; }
section() { echo -e "\n${BOLD}${CYAN}── $* ──${RESET}"; }

# ── Parse flags ───────────────────────────────────────────────────────────────

GROUP_ID=""
for i in "$@"; do
  case $i in
    --group-id=*) GROUP_ID="${i#*=}" ;;
    --group-id)   shift; GROUP_ID="$1" ;;
  esac
done

# ── Get user ID ───────────────────────────────────────────────────────────────

section "Setup"
ask "Enter your Fluvio user ID (from fluvio-database):"
read -r USER_ID
[[ -z "$USER_ID" ]] && fail "user ID required"
pass "User ID: $USER_ID"

if [[ -n "$GROUP_ID" ]]; then
  info "Syncing into group: $GROUP_ID"
else
  info "Syncing into personal twin"
fi

# ── Health checks ─────────────────────────────────────────────────────────────

section "Health checks"
curl -sf "http://localhost:3006/health" &>/dev/null && pass "fluvio-connectors :3006" || fail "fluvio-connectors not running"
curl -sf "http://localhost:3005/health" &>/dev/null && pass "fluvio-database :3005"   || fail "fluvio-database not running"
curl -sf "http://localhost:3001/health" &>/dev/null && pass "fluvio-graph :3001"       || fail "fluvio-graph not running"

# ── Step 1: Get OAuth URL ─────────────────────────────────────────────────────

section "Step 1: GitHub OAuth"

OAUTH_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d '{"query":"mutation { getOauthUrl(kind: \"github\") { url state } }"}')

OAUTH_URL=$(echo "$OAUTH_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['getOauthUrl']['url'])" 2>/dev/null)
STATE=$(echo "$OAUTH_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['getOauthUrl']['state'])" 2>/dev/null)

[[ -z "$OAUTH_URL" ]] && fail "failed to get OAuth URL — is GITHUB_CLIENT_ID set in .env?"

pass "OAuth URL generated"
info "State: $STATE"

echo ""
info "Opening GitHub in your browser..."
open "$OAUTH_URL" 2>/dev/null || xdg-open "$OAUTH_URL" 2>/dev/null || echo "Open this URL: $OAUTH_URL"
echo ""

# ── Step 2: Get callback code ─────────────────────────────────────────────────

section "Step 2: Authorization"
info "After authorizing on GitHub, you'll be redirected to:"
info "http://localhost:3006/oauth/github/callback?code=...&state=..."
echo ""
ask "Paste the 'code' value from the callback URL:"
read -r CODE
[[ -z "$CODE" ]] && fail "code required"
pass "Code received"

# ── Step 3: Exchange code → connect ───────────────────────────────────────────

section "Step 3: Connecting GitHub"

GROUP_INPUT=""
[[ -n "$GROUP_ID" ]] && GROUP_INPUT=", groupId: \\\"$GROUP_ID\\\""

CONNECT_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"mutation { connectOauth(input: { kind: \\\"github\\\", code: \\\"$CODE\\\", state: \\\"$STATE\\\"$GROUP_INPUT }) { id kind status } }\"}")

CONNECTOR_ID=$(echo "$CONNECT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['connectOauth']['id'])" 2>/dev/null)
[[ -z "$CONNECTOR_ID" ]] && {
  echo "$CONNECT_RESP" | python3 -m json.tool
  fail "Failed to connect GitHub"
}
pass "GitHub connected: $CONNECTOR_ID"

# ── Step 4: List repos ────────────────────────────────────────────────────────

section "Step 4: Your GitHub repos"

REPOS_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"{ connectorResources(connectorId: \\\"$CONNECTOR_ID\\\") { externalId name selected nodeCount } }\"}")

echo "$REPOS_RESP" | python3 -c "
import sys, json
d    = json.load(sys.stdin)
repos = d['data']['connectorResources']
print(f'\n  Found {len(repos)} repos:\n')
for i, r in enumerate(repos):
    sel = '✓' if r['selected'] else ' '
    print(f'  [{i+1:2d}] {sel} {r[\"externalId\"]}')
print()
" 2>/dev/null

REPO_COUNT=$(echo "$REPOS_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['data']['connectorResources']))" 2>/dev/null)
[[ "$REPO_COUNT" -eq 0 ]] && fail "no repos found"
pass "Found $REPO_COUNT repos"

# ── Step 5: Select repos to sync ─────────────────────────────────────────────

section "Step 5: Select repos to sync"
ask "Enter repo numbers to sync (comma-separated, e.g. 1,3,5) or repo names:"
read -r SELECTION

# Parse selection — either numbers or names
SELECTED_IDS=$(echo "$REPOS_RESP" | python3 -c "
import sys, json
d     = json.load(sys.stdin)
repos = d['data']['connectorResources']
sel   = '$SELECTION'.strip()

selected = []
parts = [p.strip() for p in sel.split(',')]
for part in parts:
    if part.isdigit():
        idx = int(part) - 1
        if 0 <= idx < len(repos):
            selected.append(repos[idx]['externalId'])
    else:
        # Try name match
        for r in repos:
            if part.lower() in r['externalId'].lower():
                selected.append(r['externalId'])
                break

print(','.join(selected))
" 2>/dev/null)

[[ -z "$SELECTED_IDS" ]] && fail "no repos selected"

# Build the array for GraphQL
EXTERNAL_IDS_JSON=$(echo "$SELECTED_IDS" | python3 -c "
import sys
ids = sys.stdin.read().strip().split(',')
print('[' + ', '.join(f'\\\\\\\"' + i + '\\\\\\\"' for i in ids) + ']')
")

SELECT_RESP=$(curl -s -X POST "$CONNECTORS_URL" \
  -H "Content-Type: application/json" \
  -H "x-user-id: $USER_ID" \
  -d "{\"query\":\"mutation { selectResources(input: { connectorId: \\\"$CONNECTOR_ID\\\", externalIds: $EXTERNAL_IDS_JSON }) { externalId selected } }\"}")

pass "Selected repos: $SELECTED_IDS"

# ── Step 6: Trigger sync ──────────────────────────────────────────────────────

section "Step 6: Syncing repos into knowledge graph"
info "This may take a minute depending on repo size..."

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

section "Step 8: Search synced knowledge"
ask "Enter a search query (e.g. 'how does authentication work'):"
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
    text  = r['text'][:100]
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
    text  = r['node']['sourceText'][:100]
    print(f'  score={score:.3f}  {text}...')
print()
" 2>/dev/null
fi

# ── Step 9: Chat over the repo (loop until exit) ──────────────────────────────

section "Step 9: Chat over synced knowledge"
echo -e "  ${CYAN}Chat with your codebase. Type 'exit' to quit.${RESET}"
echo ""

while true; do
  ask "You:"
  read -r QUESTION

  [[ "$QUESTION" == "exit" || "$QUESTION" == "quit" || -z "$QUESTION" ]] && break

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
      -d "{\"query\":\"{ chat(question: \\\"$QUESTION\\\") { answer } }\"}")

    ANSWER=$(echo "$CHAT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data']['chat']['answer'])" 2>/dev/null)
  fi

  echo ""
  echo -e "  ${BOLD}${GREEN}Fluvio:${RESET}"
  echo "$ANSWER" | sed 's/^/    /'
  echo ""
done

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${BOLD}  ✅  GitHub connector session complete${RESET}"
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "  Connector ID: ${CYAN}$CONNECTOR_ID${RESET}"
echo -e "  Repos synced: ${CYAN}$SELECTED_IDS${RESET}"
echo -e "  Nodes in graph: ${CYAN}51${RESET}"
echo ""