# Rust API image for container hosts (Cloud Run, Fly, Koyeb, Render).
#
# The API applies pending migrations itself on start-up, so a deploy needs no
# separate migration step. The `ecoquest` CLI ships alongside it anyway for
# operational work against the deployed database (seeding, health checks),
# built from the same commit as the server it talks to.

FROM rust:1.97-slim-bookworm AS builder
WORKDIR /build

# pkg-config and libssl-dev are needed by the TLS stack; managed Postgres
# providers require TLS, so this cannot be dropped.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Manifests first, so dependency compilation is cached until they change.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/application/Cargo.toml crates/application/
COPY crates/infrastructure/Cargo.toml crates/infrastructure/
COPY crates/api/Cargo.toml crates/api/
COPY crates/cli/Cargo.toml crates/cli/
COPY crates/game-wasm/Cargo.toml crates/game-wasm/
RUN for c in domain application infrastructure game-wasm; do \
        mkdir -p crates/$c/src && echo '' > crates/$c/src/lib.rs; \
    done \
    && mkdir -p crates/api/src crates/api/src/bin crates/cli/src \
    && echo 'fn main() {}' > crates/api/src/main.rs \
    && echo '' > crates/api/src/lib.rs \
    && echo 'fn main() {}' > crates/api/src/bin/seed.rs \
    && echo 'fn main() {}' > crates/cli/src/main.rs \
    && cargo build --release --bin ecoquest-api --bin ecoquest \
    && rm -rf crates

COPY crates crates
COPY migrations migrations
# Cargo skips rebuilding when only mtimes look stale, so force the real sources.
RUN touch crates/*/src/lib.rs crates/*/src/main.rs 2>/dev/null; \
    cargo build --release --bin ecoquest-api --bin ecoquest

FROM debian:bookworm-slim AS runtime
# ca-certificates is required to verify the managed database's TLS certificate.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home ecoquest
WORKDIR /app
COPY --from=builder /build/target/release/ecoquest-api /usr/local/bin/ecoquest-api
COPY --from=builder /build/target/release/ecoquest /usr/local/bin/ecoquest
# Migrations are embedded at compile time, but keeping them here makes the
# image self-describing for anyone inspecting a running container.
COPY migrations /app/migrations
USER ecoquest
# Overridden by the platform's PORT; declared for local `docker run`.
ENV PORT=8080
EXPOSE 8080
CMD ["ecoquest-api"]
