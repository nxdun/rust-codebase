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

FROM rust:1.93-slim AS bgutil-builder
WORKDIR /src
RUN apt-get update && \
        apt-get install -y --no-install-recommends \
            curl \
            ca-certificates \
            build-essential \
            pkg-config \
            libssl-dev \
            python3 && \
        BGUTIL_TAG="$(curl -Ls -o /dev/null -w '%{url_effective}' https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs/releases/latest | sed 's#.*/tag/##')" && \
        curl -L "https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs/archive/refs/tags/${BGUTIL_TAG}.tar.gz" \
            | tar -xz --strip-components=1 && \
        cargo build --release

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
        ca-certificates \
        curl \
        ffmpeg \
        libzstd1 \
        python3 \
        python3-pip \
        python3-venv \
        tini \
    && python3 -m venv /opt/yt \
    && /opt/yt/bin/pip install --no-cache-dir --upgrade pip yt-dlp \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 65532 app \
    && useradd --uid 65532 --gid 65532 --create-home --home-dir /home/app --shell /usr/sbin/nologin app \
    && mkdir -p /home/app/downloads \
    && chown -R 65532:65532 /home/app
WORKDIR /app
COPY --from=builder /out/app /usr/local/bin/app
COPY --from=bgutil-builder /src/target/release/bgutil-pot /usr/local/bin/bgutil-pot
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh
ENV YTDLP_PATH=/opt/yt/bin/yt-dlp
ENV YTDLP_POT_PROVIDER_URL=http://127.0.0.1:4416
ENV DOWNLOAD_DIR=/home/app/downloads
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
ENV APP_ENV=development
ENV ALLOWED_ORIGINS=*
ENV RUST_LOG=info
ENV MAX_CONCURRENT_DOWNLOADS=3
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "-g", "--", "/usr/local/bin/docker-entrypoint.sh"]
