# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=lukemathwalker/cargo-chef:latest-rust-alpine
ARG BIN=nadzu

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
RUN apk add --no-cache curl

WORKDIR /opt/yt
RUN python3 -m venv /opt/yt && \
  /opt/yt/bin/pip install --no-cache-dir --upgrade pip yt-dlp

# --- STAGE 6: Slim Runtime ---
FROM python:3.13-alpine AS runtime
ARG APP_PORT=8080

# Install runtime dependencies.
RUN apk add --no-cache \
  ca-certificates \
  curl \
  aria2 \
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
  YTDLP_EXTERNAL_DOWNLOADER=aria2c \
  YTDLP_EXTERNAL_DOWNLOADER_ARGS="aria2c:-x16 -j16 -s16 -k1M --file-allocation=none --summary-interval=0" \
  DOWNLOAD_DIR=/home/app/downloads \
  APP_HOST=0.0.0.0 \
  APP_PORT=${APP_PORT} \
  APP_ENV=production \
  RUST_LOG=info

USER 65532:65532
EXPOSE ${APP_PORT}
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:${APP_PORT}/health || exit 1

ENTRYPOINT ["/sbin/tini", "-g", "--", "/usr/local/bin/docker-entrypoint.sh"]
