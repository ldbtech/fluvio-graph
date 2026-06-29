# syntax=docker/dockerfile:1
# ── Build ────────────────────────────────────────────────────────────────────
FROM rust:1.87-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libclang-dev clang && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Cache dependencies: copy manifests first, build a dummy lib, then replace with real source.
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY services/fluvio-graph/Cargo.toml ./services/fluvio-graph/Cargo.toml
COPY services/fluvio-twin/Cargo.toml  ./services/fluvio-twin/Cargo.toml
COPY services/fluvio-database/Cargo.toml ./services/fluvio-database/Cargo.toml
COPY services/fluvio-ingestion/Cargo.toml ./services/fluvio-ingestion/Cargo.toml
COPY services/fluvio-collab/Cargo.toml ./services/fluvio-collab/Cargo.toml

# Stub all service src dirs so cargo can resolve the workspace
RUN for svc in fluvio-graph fluvio-twin fluvio-database fluvio-ingestion fluvio-collab; do \
      mkdir -p services/$svc/src && \
      echo 'fn main(){}' > services/$svc/src/main.rs; \
    done

RUN cargo build --release -p fluvio-graph 2>&1 | tail -5

# Now replace stubs with real source and rebuild only fluvio-graph
COPY services/fluvio-graph/src/ ./services/fluvio-graph/src/
RUN touch services/fluvio-graph/src/main.rs && \
    cargo build --release -p fluvio-graph

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libgomp1 curl && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1001 fluviome

WORKDIR /app
COPY --from=builder /workspace/target/release/fluvio-graph ./fluvio-graph

USER fluviome
EXPOSE 3001

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
    CMD curl -sf http://localhost:3001/health || exit 1

ENTRYPOINT ["./fluvio-graph"]
