#!/usr/bin/env bash
set -e

# Ensure logs dir exists
mkdir -p .logs

# Kill any existing surreal instances
pkill -f "surreal start" || true
sleep 0.5

echo "Starting primary SurrealDB on port 8000..."
nohup surreal start --user root --pass root --bind 127.0.0.1:8000 surrealkv://./fluvio_surreal_data > .logs/surreal_8000.log 2>&1 &

echo "Starting collab SurrealDB on port 8001..."
nohup surreal start --user root --pass root --bind 127.0.0.1:8001 surrealkv://./fluvio_surreal_collab_data > .logs/surreal_8001.log 2>&1 &

sleep 2
curl -sf http://127.0.0.1:8000/health && echo "Surreal 8000 is healthy!"
curl -sf http://127.0.0.1:8001/health && echo "Surreal 8001 is healthy!"
