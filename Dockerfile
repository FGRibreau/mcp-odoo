# syntax=docker/dockerfile:1

# Static musl build: reqwest uses rustls, so no OpenSSL is needed at runtime.
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked \
    && cp target/release/mcp-server-odoo /usr/local/bin/mcp-server-odoo

# Alpine rather than distroless: busybox `wget` backs the HEALTHCHECK.
FROM alpine:3.22
RUN adduser -D -H -u 10001 mcp
COPY --from=build /usr/local/bin/mcp-server-odoo /usr/local/bin/mcp-server-odoo
USER 10001
ENV MCP_TRANSPORT=http \
    MCP_BIND=0.0.0.0:8000
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q -T 4 -O /dev/null http://127.0.0.1:8000/health || exit 1
ENTRYPOINT ["mcp-server-odoo"]
