#!/usr/bin/env bash
set -e

echo "Stopping services..."
pkill -f "target/debug/fluvio-twin" || true
pkill -f "target/debug/fluvio-ingestion" || true
pkill -f "target/debug/fluvio-graph" || true
pkill -f "gateway/router" || true
sleep 1

echo "Starting fluvio-graph..."
PORT=3001 ./target/debug/fluvio-graph >> .logs/fluvio-graph.log 2>&1 &
echo $! > .pids/fluvio-graph.pid

echo "Starting fluvio-ingestion..."
PORT=3004 ./target/debug/fluvio-ingestion >> .logs/fluvio-ingestion.log 2>&1 &
echo $! > .pids/fluvio-ingestion.pid

echo "Starting fluvio-twin..."
PORT=3002 ./target/debug/fluvio-twin >> .logs/fluvio-twin.log 2>&1 &
echo $! > .pids/fluvio-twin.pid

sleep 2

echo "Composing supergraph..."
cd gateway
rover supergraph compose --config supergraph.yaml --elv2-license accept --output supergraph.graphql --skip-update-check

echo "Starting router..."
./router --config router.yaml --supergraph supergraph.graphql >> ../.logs/router.log 2>&1 &
echo $! > ../.pids/router.pid

echo "Done!"
