#!/usr/bin/env bash
# =============================================================================
# Fluvio Development Stack
# =============================================================================
#
# Usage:
#   bash scripts/dev.sh                # start everything
#   bash scripts/dev.sh --clean        # kill existing, rebuild, start fresh
#   bash scripts/dev.sh --skip-build   # skip cargo build
#   bash scripts/dev.sh --no-gateway   # start subgraphs only
#   bash scripts/dev.sh --release      # release build
#   bash scripts/dev.sh --help         # show this help
#
# Prerequisites:
#   SurrealDB:   surreal start --user root --pass root surrealkv://./fluvio_surreal_data
#   PostgreSQL:  brew services start postgresql@16
#   Rover:       https://www.apollographql.com/docs/rover/getting-started
#
# Logs: .logs/<service>.log
# PIDs: .pids/<service>.pid
# =============================================================================

set -euo pipefail

# ── Paths ─────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GATEWAY_DIR="$ROOT/services/fluvio-gateway"
LOG_DIR="$ROOT/.logs"
PID_DIR="$ROOT/.pids"
PROFILE="${PROFILE:-debug}"

# ── Colors ────────────────────────────────────────────────────────────────────

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

log()     { echo -e "${BOLD}${BLUE}▶${RESET} $*"; }
ok()      { echo -e "  ${GREEN}✓${RESET} $*"; }
warn()    { echo -e "  ${YELLOW}⚠${RESET} $*"; }
fail()    { echo -e "  ${RED}✗${RESET} $*" >&2; }
section() { echo -e "\n${BOLD}${CYAN}── $* ──${RESET}"; }

# ── Flags ─────────────────────────────────────────────────────────────────────

CLEAN=0; SKIP_BUILD=0; NO_GATEWAY=0

for arg in "$@"; do
  case $arg in
    --clean)      CLEAN=1      ;;
    --skip-build) SKIP_BUILD=1 ;;
    --no-gateway) NO_GATEWAY=1 ;;
    --release)    PROFILE=release ;;
    --help|-h)
      grep "^#" "$0" | head -20 | sed 's/^# \{0,1\}//'
      exit 0
      ;;
  esac
done

# ── Service registry ──────────────────────────────────────────────────────────
# "display_name:binary:port_var:default_port"

declare -a SERVICES=(
  "fluvio-graph:fluvio-graph:FLUVIO_GRAPH_PORT:3001"
  "fluvio-ingestion:fluvio-ingestion:FLUVIO_INGESTION_PORT:3004"
  "fluvio-twin:fluvio-twin:FLUVIO_TWIN_PORT:3002"
  "fluvio-database:fluvio-database:FLUVIO_DATABASE_PORT:3005"
  "fluvio-collab:fluvio-collab:FLUVIO_COLLAB_PORT:3003"
)

get_port() {
  local port_var=$1
  local default=$2
  echo "${!port_var:-$default}"
}

# ── Setup ─────────────────────────────────────────────────────────────────────

mkdir -p "$LOG_DIR" "$PID_DIR"

if [[ -f "$ROOT/.env" ]]; then
  set -a; source "$ROOT/.env"; set +a
  ok "Loaded .env"
else
  warn ".env not found — using defaults"
fi

# ── Process management ─────────────────────────────────────────────────────────

stop_service() {
  local name=$1
  local pid_file="$PID_DIR/${name}.pid"

  if [[ -f "$pid_file" ]]; then
    local pid
    pid=$(cat "$pid_file")
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      local i=0
      while kill -0 "$pid" 2>/dev/null && ((i < 30)); do
        sleep 0.1; ((i++))
      done
      kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$pid_file"
  fi

  pkill -f "target/${PROFILE}/${name}$" 2>/dev/null || true
}

stop_all() {
  for entry in "${SERVICES[@]}"; do
    stop_service "$(cut -d: -f1 <<< "$entry")"
  done
  pkill -f "router --config" 2>/dev/null || true
  sleep 0.5
}

# ── Cleanup trap ───────────────────────────────────────────────────────────────

ROUTER_PID=""

cleanup() {
  trap - EXIT INT TERM
  echo ""
  section "Shutting down"
  stop_all
  [[ -n "$ROUTER_PID" ]] && kill "$ROUTER_PID" 2>/dev/null || true
  ok "All services stopped"
}

trap cleanup EXIT INT TERM

# ── Prerequisites ──────────────────────────────────────────────────────────────

section "Prerequisites"

require() {
  command -v "$1" &>/dev/null && ok "$1" || { fail "$1 not found — $2"; exit 1; }
}

require cargo "install Rust: https://rustup.rs"
require curl  "install curl"

SURREAL_PORT="${SURREAL_PORT:-8000}"
if curl -sf "http://127.0.0.1:${SURREAL_PORT}/health" &>/dev/null; then
  ok "SurrealDB :${SURREAL_PORT}"
else
  fail "SurrealDB not running on :${SURREAL_PORT}"
  fail "Run: surreal start --log info --user root --pass root surrealkv://./fluvio_surreal_data"
  exit 1
fi

if pg_isready -q 2>/dev/null; then
  ok "PostgreSQL"
else
  fail "PostgreSQL not running"
  fail "Run: brew services start postgresql@16"
  exit 1
fi

if [[ $NO_GATEWAY -eq 0 ]]; then
  if command -v rover &>/dev/null; then
    ok "rover"
  else
    warn "rover not found — skipping gateway"
    warn "Install: curl -sSL https://rover.apollo.dev/nix/latest | sh"
    NO_GATEWAY=1
  fi
fi

# ── Stop existing ─────────────────────────────────────────────────────────────

section "Stopping existing processes"
stop_all
ok "Clean"

# ── Build ─────────────────────────────────────────────────────────────────────

if [[ $SKIP_BUILD -eq 0 ]]; then
  section "Building (${PROFILE})"

  PKGS="-p fluvio-graph -p fluvio-ingestion -p fluvio-twin -p fluvio-database"

  if [[ "$PROFILE" == "release" ]]; then
    cargo build --release $PKGS
  else
    cargo build $PKGS
  fi

  ok "Build complete"
fi

BINDIR="$ROOT/target/${PROFILE}"

for entry in "${SERVICES[@]}"; do
  bin="$(cut -d: -f2 <<< "$entry")"
  if [[ ! -f "$BINDIR/$bin" ]]; then
    fail "Binary missing: $BINDIR/$bin — run without --skip-build"
    exit 1
  fi
done

# ── Start subgraphs ────────────────────────────────────────────────────────────

section "Starting subgraphs"

for entry in "${SERVICES[@]}"; do
  IFS=: read -r svc_name svc_bin port_var port_default <<< "$entry"
  svc_port=$(get_port "$port_var" "$port_default")

  log "${svc_name} → :${svc_port}"

  # Rotate log
  [[ -f "$LOG_DIR/${svc_name}.log" ]] && \
    mv "$LOG_DIR/${svc_name}.log" "$LOG_DIR/${svc_name}.log.prev"

  PORT="$svc_port" "$BINDIR/$svc_bin" \
    >> "$LOG_DIR/${svc_name}.log" 2>&1 &
  echo $! > "$PID_DIR/${svc_name}.pid"

  # Wait for health
  local_max=60
  local_i=0
  until curl -sf "http://127.0.0.1:${svc_port}/health" &>/dev/null; do
    sleep 1
    ((local_i++))
    if ((local_i >= local_max)); then
      fail "${svc_name} failed to start (timeout)"
      fail "Logs:"
      tail -30 "$LOG_DIR/${svc_name}.log" >&2
      exit 1
    fi
  done

  ok "${svc_name} healthy"
done

# ── Supergraph + Router ────────────────────────────────────────────────────────

if [[ $NO_GATEWAY -eq 0 ]]; then
  section "Composing supergraph"

  (
    cd "$GATEWAY_DIR"
    APOLLO_ELV2_LICENSE="${APOLLO_ELV2_LICENSE:-accept}" \
      rover supergraph compose \
        --config supergraph.yaml \
        --elv2-license accept \
        --output supergraph.graphql \
        --skip-update-check 2>&1 \
      | grep -v "newer version" || true
  )
  ok "supergraph.graphql composed"

  section "Starting Apollo Router"

  ROUTER_BIN="${ROUTER_BIN:-$GATEWAY_DIR/router}"
  if [[ ! -x "$ROUTER_BIN" ]]; then
    fail "Router not found: ${ROUTER_BIN}"
    fail "Run: curl -sSL https://router.apollo.dev/download/nix/latest | sh"
    fail "Then: mv router services/fluvio-gateway/router"
    exit 1
  fi

  [[ -f "$LOG_DIR/router.log" ]] && mv "$LOG_DIR/router.log" "$LOG_DIR/router.log.prev"

  cd "$GATEWAY_DIR"
  "$ROUTER_BIN" --config router.yaml --supergraph supergraph.graphql \
    >> "$LOG_DIR/router.log" 2>&1 &
  ROUTER_PID=$!

  local_i=0
  until curl -sf "http://127.0.0.1:8088/health" &>/dev/null; do
    sleep 0.5
    ((local_i++))
    if ((local_i > 30)); then
      fail "Router failed to start"
      tail -20 "$LOG_DIR/router.log" >&2
      exit 1
    fi
  done

  ok "Apollo Router healthy"
fi

# ── Ready ──────────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${BOLD}  🚀  Fluvio stack is running${RESET}"
echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""

if [[ $NO_GATEWAY -eq 0 ]]; then
  echo -e "  ${BOLD}Gateway     ${RESET}  http://127.0.0.1:4000"
  echo ""
fi

for entry in "${SERVICES[@]}"; do
  IFS=: read -r svc_name _ port_var port_default <<< "$entry"
  svc_port=$(get_port "$port_var" "$port_default")
  printf "  ${BOLD}%-20s${RESET} http://127.0.0.1:%s\n" "$svc_name" "$svc_port"
done

echo ""
echo -e "  ${CYAN}Logs  →  ${LOG_DIR}/${RESET}"
echo -e "  ${CYAN}Ctrl+C to stop all services${RESET}"
echo ""

# ── Block ─────────────────────────────────────────────────────────────────────

if [[ -n "$ROUTER_PID" ]]; then
  wait "$ROUTER_PID" || true
else
  while true; do sleep 10; done
fi