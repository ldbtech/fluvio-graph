FROM alpine:3.19

# Apollo Router binary
ARG ROUTER_VERSION=1.49.1
RUN apk add --no-cache curl ca-certificates && \
    curl -sSL "https://github.com/apollographql/router/releases/download/v${ROUTER_VERSION}/router-v${ROUTER_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
      | tar -xz --strip-components=1 -C /usr/local/bin router && \
    chmod +x /usr/local/bin/router

WORKDIR /gateway
COPY router.yaml ./
COPY supergraph.graphql ./

EXPOSE 4001

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD curl -sf http://localhost:4001/health || exit 1

ENTRYPOINT ["router", "--config", "router.yaml", "--supergraph", "supergraph.graphql"]
