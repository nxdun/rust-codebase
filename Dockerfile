# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=lukemathwalker/cargo-chef:latest-rust-alpine
ARG BIN=nadzu
ARG BGUTIL_VERSION=0.7.2

# --- STAGE 1: Rust Chef ---
FROM --platform=$BUILDPLATFORM ${RUST_IMAGE} AS chef
RUN apk add --no-cache build-base musl-dev zstd-dev pkgconfig ca-certificates
WORKDIR /app

# --- STAGE 2: Rust Planner ---
FROM --platform=$BUILDPLATFORM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# --- STAGE 3: Rust Builder ---
FROM --platform=$BUILDPLATFORM chef AS builder
ARG TARGETARCH
ARG BIN=nadzu
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/app/target \
  case "$TARGETARCH" in \
  arm64) RUST_TARGET="aarch64-unknown-linux-musl" ;; \
  *)     RUST_TARGET="x86_64-unknown-linux-musl" ;; \
  esac && \
  rustup target add $RUST_TARGET && \
  cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/app/target \
  set -e; \
  case "$TARGETARCH" in \
  arm64) RUST_TARGET="aarch64-unknown-linux-musl" ;; \
  *)     RUST_TARGET="x86_64-unknown-linux-musl" ;; \
  esac && \
  cargo build --release --target $RUST_TARGET --bin "$BIN" && \
  install -D "target/$RUST_TARGET/release/$BIN" /out/app && \
  strip /out/app

# --- STAGE 4: Static FFmpeg ---
FROM mwader/static-ffmpeg:8.0.1 AS ffmpeg

# --- STAGE 5: Python & Plugin Builder ---
FROM python:3.13-alpine AS python-builder
ARG TARGETARCH
ARG BGUTIL_VERSION
RUN apk add --no-cache curl unzip

WORKDIR /opt/yt
RUN python3 -m venv /opt/yt && \
  /opt/yt/bin/pip install --no-cache-dir --upgrade pip yt-dlp

RUN set -e; \
  case "${TARGETARCH}" in \
  amd64) BGUTIL_ARCH="x86_64"; BGUTIL_SHA256="55c3710d25a1f2b35976f76f2a4c7baa5f6c15c20e83ba72700f1cad21cf03b7" ;; \
  arm64) BGUTIL_ARCH="aarch64"; BGUTIL_SHA256="7fa58d061dc01cf4cab44223b6cca51138673be335ec9ca57d1389c148528a96" ;; \
  *) exit 1 ;; \
  esac \
  && BGUTIL_BASE_URL="https://github.com/jim60105/bgutil-ytdlp-pot-provider-rs/releases/download/v${BGUTIL_VERSION}" \
  && curl -fL "${BGUTIL_BASE_URL}/bgutil-pot-linux-${BGUTIL_ARCH}" -o /usr/local/bin/bgutil-pot \
  && echo "${BGUTIL_SHA256}  /usr/local/bin/bgutil-pot" | sha256sum -c - \
  && chmod +x /usr/local/bin/bgutil-pot \
  && curl -fL "${BGUTIL_BASE_URL}/bgutil-ytdlp-pot-provider-rs.zip" -o /tmp/plugin.zip \
  && PLUGIN_SITE_DIR="$(/opt/yt/bin/python -c 'import site; print(site.getsitepackages()[0])')" \
  && unzip /tmp/plugin.zip -d "${PLUGIN_SITE_DIR}"

# --- STAGE 6: Slim Runtime ---
FROM python:3.13-alpine AS runtime
ARG APP_PORT=8080

# Install runtime dependencies.
RUN apk add --no-cache \
  ca-certificates \
  curl \
  zstd-libs \
  tini \
  gcompat \
  python3 \
  libgcc \
  libstdc++
# ffmpeg: perform video/audio processing tasks like format conversion, thumbnail generation, or metadata extraction
COPY --from=ffmpeg /ffmpeg /usr/local/bin/
# ffprobe: validate uploads, compute duration, detect codecs, or inspect stream metadata
COPY --from=ffmpeg /ffprobe /usr/local/bin/ 
# yt-dlp: download videos from various platforms.
COPY --from=python-builder /opt/yt /opt/yt
# bgutil-pot: yt-dlp pot token provider
COPY --from=python-builder /usr/local/bin/bgutil-pot /usr/local/bin/bgutil-pot
# note: rust application binary
COPY --from=builder /out/app /usr/local/bin/app

# Set up non-root user
RUN addgroup -g 65532 app && \
  adduser -D -u 65532 -G app -h /home/app -s /sbin/nologin app && \
  mkdir -p /home/app/downloads && \
  chown -R 65532:65532 /home/app

WORKDIR /home/app
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Environment 
ENV YTDLP_PATH=/opt/yt/bin/yt-dlp \
  YTDLP_POT_PROVIDER_URL=http://127.0.0.1:4416 \
  DOWNLOAD_DIR=/home/app/downloads \
  APP_HOST=0.0.0.0 \
  APP_PORT=${APP_PORT} \
  APP_ENV=production \
  RUST_LOG=info \
  YTDLP_COOKIES_FILE=/run/secrets/ytdlp-cookies.txt

USER 65532:65532
EXPOSE ${APP_PORT}

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:${APP_PORT}/health || exit 1

ENTRYPOINT ["/sbin/tini", "-g", "--", "/usr/local/bin/docker-entrypoint.sh"]
