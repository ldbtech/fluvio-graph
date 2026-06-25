FROM debian:bookworm-slim

# Apollo Router binary — multi-arch (works on amd64 and Apple Silicon arm64).
ARG ROUTER_VERSION=1.49.1
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && case "$TARGETARCH" in \
       amd64) RARCH=x86_64-unknown-linux-gnu ;; \
       arm64) RARCH=aarch64-unknown-linux-gnu ;; \
       *) echo "unsupported TARGETARCH=$TARGETARCH" && exit 1 ;; \
     esac \
  && curl -fsSL "https://github.com/apollographql/router/releases/download/v${ROUTER_VERSION}/router-v${ROUTER_VERSION}-${RARCH}.tar.gz" -o /tmp/router.tgz \
  && tar -xzf /tmp/router.tgz -C /tmp \
  && mv "$(find /tmp -type f -name router | head -1)" /usr/local/bin/router \
  && chmod +x /usr/local/bin/router \
  && rm -rf /tmp/router.tgz /tmp/dist

WORKDIR /gateway
COPY router.yaml ./
COPY supergraph.graphql ./

EXPOSE 4001

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD curl -sf http://localhost:4001/health || exit 1

ENTRYPOINT ["router", "--config", "router.yaml", "--supergraph", "supergraph.graphql"]
