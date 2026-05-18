#!/usr/bin/env bash
# =============================================================================
# Fluvio Collab — Full End-to-End Test
# =============================================================================
#
# Tests the complete company brain flow:
#   1. Create owner user
#   2. Create contributor user
#   3. Create a group
#   4. Invite contributor
#   5. Accept invite
#   6. Owner contributes knowledge (auto-approved)
#   7. Contributor contributes knowledge (goes to pending queue)
#   8. Owner views pending queue
#   9. Owner approves contribution
#  10. Search group brain
#  11. Chat over group brain
#
# Usage:
#   bash scripts/test-collab.sh
#   bash scripts/test-collab.sh --collab-only   # skip gateway, test :3003 direct
# =============================================================================

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────

COLLAB_URL="http://localhost:3003/graphql"
DB_URL="http://localhost:3005/graphql"
GATEWAY_URL="http://localhost:4000"

DIRECT=0
for arg in "$@"; do
  [[ $arg == "--collab-only" ]] && DIRECT=1
done

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

pass()    { echo -e "  ${GREEN}✓${RESET} $*"; }
fail()    { echo -e "  ${RED}✗${RESET} $*"; exit 1; }
info()    { echo -e "  ${CYAN}→${RESET} $*"; }
section() { echo -e "\n${BOLD}${CYAN}── $* ──${RESET}"; }

gql() {
  local url=$1
  local user_id=$2
  local query=$3

  curl -s -X POST "$url" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $user_id" \
    -d "{\"query\": \"$query\"}"
}

extract() {
  # extract JSON field using python
  echo "$1" | python3 -c "
import sys, json
data = json.load(sys.stdin)
keys = '$2'.split('.')
v = data
for k in keys:
    if k.startswith('['):
        v = v[int(k[1:-1])]
    else:
        v = v[k]
print(v)
" 2>/dev/null || echo ""
}

check_errors() {
  local resp=$1
  local ctx=$2
  if echo "$resp" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if 'errors' in d:
    print(d['errors'][0]['message'])
    sys.exit(1)
" 2>/dev/null; then
    :
  else
    local err
    err=$(echo "$resp" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['errors'][0]['message'])" 2>/dev/null || echo "unknown error")
    fail "$ctx: $err"
  fi
}

# ── Health checks ─────────────────────────────────────────────────────────────

section "Health checks"

curl -sf "http://localhost:3003/health" &>/dev/null && pass "fluvio-collab :3003" || fail "fluvio-collab not running"
curl -sf "http://localhost:3005/health" &>/dev/null && pass "fluvio-database :3005" || fail "fluvio-database not running"
curl -sf "http://localhost:3001/health" &>/dev/null && pass "fluvio-graph :3001"    || fail "fluvio-graph not running"
curl -sf "http://localhost:3004/health" &>/dev/null && pass "fluvio-ingestion :3004" || fail "fluvio-ingestion not running"

# ── Step 1: Create owner user ─────────────────────────────────────────────────

section "Step 1: Create owner user"

OWNER_RESP=$(curl -s -X POST "$DB_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createUser(input: { firebaseUid: \"test-owner-001\", email: \"owner@fluvio.ai\", displayName: \"Alice Owner\" }) { id displayName } }"
  }')

check_errors "$OWNER_RESP" "createUser (owner)"
OWNER_ID=$(extract "$OWNER_RESP" "data.createUser.id")
[[ -z "$OWNER_ID" ]] && fail "owner id is empty"
pass "Owner created: $OWNER_ID"

# ── Step 2: Create contributor user ───────────────────────────────────────────

section "Step 2: Create contributor user"

CONTRIB_RESP=$(curl -s -X POST "$DB_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createUser(input: { firebaseUid: \"test-contrib-001\", email: \"bob@fluvio.ai\", displayName: \"Bob Contributor\" }) { id displayName } }"
  }')

check_errors "$CONTRIB_RESP" "createUser (contributor)"
CONTRIB_ID=$(extract "$CONTRIB_RESP" "data.createUser.id")
[[ -z "$CONTRIB_ID" ]] && fail "contributor id is empty"
pass "Contributor created: $CONTRIB_ID"

# ── Step 3: Create group ──────────────────────────────────────────────────────

section "Step 3: Create group"

GROUP_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "mutation { createGroup(name: \\\"Fluvio Team\\\", description: \\\"Company brain test\\\") { id name graphId } }")

check_errors "$GROUP_RESP" "createGroup"
GROUP_ID=$(extract "$GROUP_RESP" "data.createGroup.id")
[[ -z "$GROUP_ID" ]] && fail "group id is empty"
pass "Group created: $GROUP_ID"
info "Name: $(extract "$GROUP_RESP" "data.createGroup.name")"

# ── Step 4: Invite contributor ────────────────────────────────────────────────

section "Step 4: Invite contributor"

INVITE_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "mutation { invite(groupId: \\\"$GROUP_ID\\\", email: \\\"bob@fluvio.ai\\\", role: \\\"contributor\\\") { id token role } }")

check_errors "$INVITE_RESP" "invite"
INVITE_TOKEN=$(extract "$INVITE_RESP" "data.invite.token")
[[ -z "$INVITE_TOKEN" ]] && fail "invite token is empty"
pass "Invite created: token=$INVITE_TOKEN"

# ── Step 5: Accept invite ─────────────────────────────────────────────────────

section "Step 5: Accept invite"

ACCEPT_RESP=$(gql "$COLLAB_URL" "$CONTRIB_ID" \
  "mutation { acceptInvite(token: \\\"$INVITE_TOKEN\\\") { id role groupId } }")

check_errors "$ACCEPT_RESP" "acceptInvite"
MEMBER_ROLE=$(extract "$ACCEPT_RESP" "data.acceptInvite.role")
pass "Invite accepted — role: $MEMBER_ROLE"

# ── Step 6: Owner contributes (auto-approved) ─────────────────────────────────

section "Step 6: Owner contributes knowledge (auto-approved)"

OWNER_CONTRIB_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "mutation { contribute(groupId: \\\"$GROUP_ID\\\", input: { kind: \\\"text\\\", text: \\\"Fluvio is a knowledge graph platform that helps teams organize collective intelligence using AI-powered semantic search and graph traversal\\\", sourceUri: \\\"test://owner-doc1\\\" }) { surrealNodeId status queueId } }")

check_errors "$OWNER_CONTRIB_RESP" "contribute (owner)"
OWNER_NODE_ID=$(extract "$OWNER_CONTRIB_RESP" "data.contribute.surrealNodeId")
OWNER_STATUS=$(extract "$OWNER_CONTRIB_RESP" "data.contribute.status")
pass "Owner contribution: node=$OWNER_NODE_ID status=$OWNER_STATUS"
[[ "$OWNER_STATUS" == "approved" ]] && pass "Auto-approved correctly" || fail "Expected auto-approved for owner"

# ── Step 7: Contributor contributes (goes to pending) ─────────────────────────

section "Step 7: Contributor contributes (goes to pending queue)"

CONTRIB_CONTRIB_RESP=$(gql "$COLLAB_URL" "$CONTRIB_ID" \
  "mutation { contribute(groupId: \\\"$GROUP_ID\\\", input: { kind: \\\"text\\\", text: \\\"Knowledge graphs enable semantic search and reasoning over connected data enabling AI agents to answer complex questions with grounded evidence\\\", sourceUri: \\\"test://contrib-doc1\\\" }) { surrealNodeId status queueId } }")

check_errors "$CONTRIB_CONTRIB_RESP" "contribute (contributor)"
CONTRIB_NODE_ID=$(extract "$CONTRIB_CONTRIB_RESP" "data.contribute.surrealNodeId")
CONTRIB_STATUS=$(extract "$CONTRIB_CONTRIB_RESP" "data.contribute.status")
QUEUE_ID=$(extract "$CONTRIB_CONTRIB_RESP" "data.contribute.queueId")
pass "Contributor contribution: node=$CONTRIB_NODE_ID status=$CONTRIB_STATUS"
[[ "$CONTRIB_STATUS" == "pending" ]] && pass "Correctly queued for approval" || fail "Expected pending for contributor"

# ── Step 8: Owner views pending queue ─────────────────────────────────────────

section "Step 8: Owner views pending queue"

QUEUE_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "{ pendingContributions(groupId: \\\"$GROUP_ID\\\") { id status kind surrealNodeId } }")

check_errors "$QUEUE_RESP" "pendingContributions"
PENDING_COUNT=$(echo "$QUEUE_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['data']['pendingContributions']))" 2>/dev/null || echo "0")
pass "Pending contributions: $PENDING_COUNT"
[[ "$PENDING_COUNT" -gt 0 ]] && pass "Queue has items" || fail "Expected pending items in queue"

# Get first pending item id
PENDING_ID=$(echo "$QUEUE_RESP" | python3 -c "
import sys,json
d=json.load(sys.stdin)
items=d['data']['pendingContributions']
print(items[0]['id']) if items else print('')
" 2>/dev/null || echo "")

# ── Step 9: Owner approves ────────────────────────────────────────────────────

section "Step 9: Owner approves contribution"

[[ -z "$PENDING_ID" ]] && fail "no pending item to approve"

APPROVE_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "mutation { approve(groupId: \\\"$GROUP_ID\\\", contributionId: \\\"$PENDING_ID\\\") { id status } }")

check_errors "$APPROVE_RESP" "approve"
APPROVED_STATUS=$(extract "$APPROVE_RESP" "data.approve.status")
pass "Contribution approved: status=$APPROVED_STATUS"
[[ "$APPROVED_STATUS" == "approved" ]] && pass "Status correctly set to approved" || fail "Expected approved status"

# ── Step 10: Search group brain ────────────────────────────────────────────────

section "Step 10: Search group knowledge brain"

# Give SurrealDB a moment to settle
sleep 1

SEARCH_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "{ searchGroup(groupId: \\\"$GROUP_ID\\\", query: \\\"knowledge graph AI semantic search\\\", topK: 5) { id text score } }")

check_errors "$SEARCH_RESP" "searchGroup"
RESULT_COUNT=$(echo "$SEARCH_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['data']['searchGroup']))" 2>/dev/null || echo "0")
pass "Search results: $RESULT_COUNT"

if [[ "$RESULT_COUNT" -gt 0 ]]; then
  echo "$SEARCH_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for r in d['data']['searchGroup']:
    score = r['score']
    text  = r['text'][:80]
    print(f'    score={score:.3f}  text={text}...')
" 2>/dev/null || true
  pass "Search returning results"
else
  fail "Search returned no results — node tagging may not be working"
fi

# ── Step 11: Chat over group brain ─────────────────────────────────────────────

section "Step 11: Chat over group knowledge brain"

CHAT_RESP=$(gql "$COLLAB_URL" "$OWNER_ID" \
  "{ groupChat(groupId: \\\"$GROUP_ID\\\", question: \\\"What is Fluvio and how does it help teams?\\\") { answer sources { id score } } }")

check_errors "$CHAT_RESP" "groupChat"
ANSWER=$(extract "$CHAT_RESP" "data.groupChat.answer")
SOURCE_COUNT=$(echo "$CHAT_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['data']['groupChat']['sources']))" 2>/dev/null || echo "0")

pass "Chat response received ($SOURCE_COUNT sources)"
echo ""
echo -e "  ${BOLD}Answer:${RESET}"
echo "$ANSWER" | head -10 | sed 's/^/    /'
echo ""

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${BOLD}  ✅  All tests passed${RESET}"
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "  Owner ID:       ${CYAN}$OWNER_ID${RESET}"
echo -e "  Contributor ID: ${CYAN}$CONTRIB_ID${RESET}"
echo -e "  Group ID:       ${CYAN}$GROUP_ID${RESET}"
echo -e "  Invite Token:   ${CYAN}$INVITE_TOKEN${RESET}"
echo ""