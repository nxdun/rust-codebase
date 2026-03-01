# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=lukemathwalker/cargo-chef:latest-rust-alpine
ARG BIN=nadzu
ARG BGUTIL_VERSION=0.7.2

FROM ${RUST_IMAGE} AS chef
RUN apk add --no-cache \
    build-base \
    musl-dev \
    zstd-dev \
    pkgconfig \
    ca-certificates
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM --platform=$BUILDPLATFORM chef AS builder
ARG TARGETARCH
ARG BIN=nadzu
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    if [ "$TARGETARCH" = "arm64" ]; then \
      RUST_TARGET="aarch64-unknown-linux-musl"; \
    else \
      RUST_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    rustup target add $RUST_TARGET && \
    cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    set -e; \
    if [ "$TARGETARCH" = "arm64" ]; then \
      RUST_TARGET="aarch64-unknown-linux-musl"; \
    else \
      RUST_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    cargo build --release --target $RUST_TARGET --bin "$BIN" && \
    strip "target/$RUST_TARGET/release/$BIN" && \
    install -D "target/$RUST_TARGET/release/$BIN" /out/app


# Fetch static ffmpeg
FROM mwader/static-ffmpeg:8.0.1 AS ffmpeg

FROM python:3.14-alpine AS runtime
ARG TARGETARCH
ARG BGUTIL_VERSION

# Copy static ffmpeg
COPY --from=ffmpeg /ffmpeg /usr/local/bin/
COPY --from=ffmpeg /ffprobe /usr/local/bin/

RUN apk add --no-cache \
    ca-certificates \
    curl \
    zstd-libs \
    tini \
    gcompat \
    && python3 -m venv /opt/yt \
    && /opt/yt/bin/pip install --no-cache-dir --upgrade pip yt-dlp \
    && set -e; \
    case "${TARGETARCH}" in \
    amd64) BGUTIL_ARCH="x86_64"; BGUTIL_SHA256="55c3710d25a1f2b35976f76f2a4c7baa5f6c15c20e83ba72700f1cad21cf03b7" ;; \
    arm64) BGUTIL_ARCH="aarch64"; BGUTIL_SHA256="7fa58d061dc01cf4cab44223b6cca51138673be335ec9ca57d1389c148528a96" ;; \
    *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && BGUTIL_BASE_URL="https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs/releases/download/v${BGUTIL_VERSION}" \
    && BGUTIL_BIN_ASSET="bgutil-pot-linux-${BGUTIL_ARCH}" \
    && BGUTIL_ZIP_ASSET="bgutil-ytdlp-pot-provider-rs.zip" \
    && BGUTIL_ZIP_SHA256="e5c729d59608d5d34ad7332b2905ff41d5f172a2db6e995563059d58ef11475a" \
    && curl -fL "${BGUTIL_BASE_URL}/${BGUTIL_BIN_ASSET}" -o /usr/local/bin/bgutil-pot \
    && echo "${BGUTIL_SHA256}  /usr/local/bin/bgutil-pot" | sha256sum -c - \
    && chmod +x /usr/local/bin/bgutil-pot \
    && curl -fL "${BGUTIL_BASE_URL}/${BGUTIL_ZIP_ASSET}" -o /tmp/bgutil-plugin.zip \
    && echo "${BGUTIL_ZIP_SHA256}  /tmp/bgutil-plugin.zip" | sha256sum -c - \
    && PLUGIN_SITE_DIR="$(/opt/yt/bin/python -c 'import site; print(site.getsitepackages()[0])')" \
    && /opt/yt/bin/python -c "import zipfile; zipfile.ZipFile('/tmp/bgutil-plugin.zip').extractall('${PLUGIN_SITE_DIR}')" \
    && rm -f /tmp/bgutil-plugin.zip \
    && addgroup -g 65532 app \
    && adduser -D -u 65532 -G app -h /home/app -s /sbin/nologin app \
    && mkdir -p /home/app/downloads \
    && chown -R 65532:65532 /home/app

WORKDIR /app
COPY --from=builder /out/app /usr/local/bin/app
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

ENTRYPOINT ["/sbin/tini", "-g", "--", "/usr/local/bin/docker-entrypoint.sh"]
