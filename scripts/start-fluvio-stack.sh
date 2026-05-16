#!/usr/bin/env bash
# Start local subgraphs (fluvio-graph → fluvio-ingestion → fluvio-twin), compose the
# federated supergraph from running subgraph SDL (Apollo Rover), then run Apollo Router.
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - curl
#   - Apollo Rover CLI (`rover`) on PATH — https://www.apollographql.com/docs/rover/getting-started
#   - Repo-root `.env` with whatever each service needs (e.g. ANTHROPIC_API_KEY for fluvio-twin,
#     Surreal/graph env for fluvio-graph, etc.)
#
# Env overrides:
#   PROFILE=release       Build release binaries instead of debug.
#   SKIP_COMPOSE=1        Skip Rover compose; reuse existing services/fluvio-gateway/supergraph.graphql.
#   ROUTER_BIN=path       Apollo Router executable (default: services/fluvio-gateway/router).
#   LOG_DIR=path          Where subgraph stdout/stderr logs go (default: .logs-stack under repo root).
#
# Shutdown:
#   Ctrl+C (SIGINT) or SIGTERM stops Apollo Router and all subgraphs (trap cleanup).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATEWAY_DIR="$ROOT/services/fluvio-gateway"
LOG_DIR="${LOG_DIR:-$ROOT/.logs-stack}"
PROFILE="${PROFILE:-debug}"
SKIP_COMPOSE="${SKIP_COMPOSE:-0}"

mkdir -p "$LOG_DIR"

if [[ -f "$ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi

declare -a PIDS=()

cleanup() {
  trap - EXIT INT TERM
  echo ""
  echo "Stopping all services (gateway + subgraphs)..."
  local pid
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
}

trap cleanup EXIT INT TERM

wait_health() {
  local url=$1
  local name=$2
  local max="${3:-120}"
  local i
  echo "Waiting for ${name} (${url})..."
  for ((i = 1; i <= max; i++)); do
    if curl -sf "$url" >/dev/null 2>&1; then
      echo "${name} is up."
      return 0
    fi
    sleep 1
  done
  echo "Timeout waiting for ${name}"
  exit 1
}

echo "Building subgraph binaries (${PROFILE})..."
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release -p fluvio-graph -p fluvio-ingestion -p fluvio-twin
  BINDIR="$ROOT/target/release"
else
  cargo build -p fluvio-graph -p fluvio-ingestion -p fluvio-twin
  BINDIR="$ROOT/target/debug"
fi

echo "Starting fluvio-graph → ${LOG_DIR}/fluvio-graph.log"
"$BINDIR/fluvio-graph" >>"$LOG_DIR/fluvio-graph.log" 2>&1 &
PIDS+=("$!")

wait_health "http://127.0.0.1:3001/health" "fluvio-graph"

echo "Starting fluvio-ingestion → ${LOG_DIR}/fluvio-ingestion.log"
"$BINDIR/fluvio-ingestion" >>"$LOG_DIR/fluvio-ingestion.log" 2>&1 &
PIDS+=("$!")

wait_health "http://127.0.0.1:3004/health" "fluvio-ingestion"

echo "Starting fluvio-twin → ${LOG_DIR}/fluvio-twin.log"
"$BINDIR/fluvio-twin" >>"$LOG_DIR/fluvio-twin.log" 2>&1 &
PIDS+=("$!")

wait_health "http://127.0.0.1:3002/health" "fluvio-twin"

if [[ "$SKIP_COMPOSE" != "1" ]]; then
  if ! command -v rover >/dev/null 2>&1; then
    echo "ERROR: rover not found on PATH."
    echo "Install Apollo Rover, or rerun with SKIP_COMPOSE=1 to use the existing supergraph.graphql."
    exit 1
  fi
  echo "Composing supergraph from subgraph URLs in supergraph.yaml (Rover downloads SDL via introspection)..."
  (
    cd "$GATEWAY_DIR"
    APOLLO_ELV2_LICENSE="${APOLLO_ELV2_LICENSE:-accept}" \
      rover supergraph compose \
        --config supergraph.yaml \
        --elv2-license accept \
        --output supergraph.graphql \
        --skip-update-check
  )
else
  echo "SKIP_COMPOSE=1 — using existing ${GATEWAY_DIR}/supergraph.graphql"
fi

ROUTER_BIN="${ROUTER_BIN:-$GATEWAY_DIR/router}"
if [[ ! -x "$ROUTER_BIN" ]]; then
  echo "ERROR: Apollo Router not executable at ${ROUTER_BIN}"
  exit 1
fi

echo ""
echo "Gateway: http://127.0.0.1:4000 (see ${GATEWAY_DIR}/router.yaml)"
echo "Logs: ${LOG_DIR}/ (router → router.log)"
echo "Press Ctrl+C to stop the gateway and all subgraphs."
echo ""

cd "$GATEWAY_DIR"
"$ROUTER_BIN" --config router.yaml --supergraph supergraph.graphql \
  >>"${LOG_DIR}/router.log" 2>&1 &
ROUTER_PID=$!
PIDS+=("$ROUTER_PID")

# Router runs in the background so this shell stays in the foreground process group; Ctrl+C hits bash,
# runs the INT trap above, and kills every PID in PIDS (including the router).
wait "$ROUTER_PID" || true
