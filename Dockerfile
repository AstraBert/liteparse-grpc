# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.91-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libclang-dev \
    libtesseract-dev \
    libleptonica-dev \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates/  crates/
COPY proto/ proto/

RUN cargo build --release --bin liteparse-server

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libtesseract5 \
    libleptonica6 \
    tesseract-ocr-eng \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/liteparse-server /usr/local/bin/liteparse-server
# pdfium shared library
COPY --from=builder /root/.cache/pdfium-rs/ /usr/local/lib/pdfium-rs/

# Ensure pdfium is discoverable at runtime
ARG TARGETARCH

RUN set -eux; \
    case "${TARGETARCH}" in \
      amd64) PDFIUM_ARCH="pdfium-linux-x64" ;; \
      arm64) PDFIUM_ARCH="pdfium-linux-arm64" ;; \
      *) echo "Unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    ln -s "/usr/local/lib/pdfium-rs/chromium_7897/${PDFIUM_ARCH}" \
          /usr/local/lib/pdfium-rs/chromium_7897/pdfium-current

ENV PDFIUM_LIB_PATH="/usr/local/lib/pdfium-rs/chromium_7897/pdfium-current/lib"

EXPOSE 50051

CMD ["liteparse-server"]
