# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=rust:slim-bookworm
ARG MODE=release
ARG BIN=nadzu

FROM ${RUST_IMAGE} AS base
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        libzstd-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked cargo-chef
WORKDIR /app

FROM base AS planner
COPY Cargo.toml ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
ARG MODE=release
ARG BIN=nadzu
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    set -e; \
    if [ "$MODE" = "release" ]; then \
        cargo chef cook --release --recipe-path recipe.json; \
    else \
        cargo chef cook --recipe-path recipe.json; \
    fi
COPY Cargo.toml ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    set -e; \
    if [ "$MODE" = "release" ]; then \
        cargo build --release --bin "$BIN" && \
        install -D "target/release/$BIN" /out/app; \
    else \
        cargo build --bin "$BIN" && \
        install -D "target/debug/$BIN" /out/app; \
    fi

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libzstd1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 65532 app \
    && useradd --uid 65532 --gid 65532 --create-home --home-dir /home/app --shell /usr/sbin/nologin app
WORKDIR /app
COPY --from=builder /out/app /usr/local/bin/app
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/app"]
