# LiteParse gRPC

A gRPC service and CLI client for document parsing, screenshotting, and complexity analysis, built on top of the [LiteParse](https://crates.io/crates/liteparse) Rust library.

## Overview

This project exposes LiteParse's document-processing capabilities over gRPC, making it easy to integrate PDF/image parsing into distributed systems. It includes:

- **`liteparse-server`** — A gRPC server that handles parse, screenshot, and complexity-analysis requests
- **`liteparse-client`** — A CLI client for interacting with the server

## Architecture

```
┌─────────────────┐     gRPC      ┌─────────────────┐
│  liteparse-     │  ──────────►  │ liteparse-server│
│   client        │               │   (tonic/gRPC)  │
└─────────────────┘               └────────┬────────┘
                                           │
                                    ┌──────┴──────┐
                                    │  LiteParse  │
                                    │   (Rust)    │
                                    └─────────────┘
```

## Crates

| Crate | Description |
|-------|-------------|
| `crates/grpc` | gRPC server (`liteparse-server`) and CLI client (`liteparse-client`) |
| `crates/observability` | Tracing subscriber setup (with optional OpenTelemetry support) |

## Protocol

The gRPC service is defined in [`proto/parser.proto`](proto/parser.proto) and provides three RPCs:

| RPC | Description |
|-----|-------------|
| `Parse` | Extract text from a document (PDF, image, Office file, etc.) with per-page bounding boxes |
| `Screenshot` | Render document pages as PNG images |
| `IsComplex` | Analyze document complexity and estimate whether OCR is needed |

### Key Message Types

- **`LiteParseConfig`** — Parsing options: OCR language, DPI, output format (JSON/Text/Markdown), image mode, page ranges, password, workers, and more
- **`ParseResponse`** — Extracted text plus per-page `TextItem` arrays with bounding boxes
- **`ScreenshotResponse`** — Page screenshots as PNG bytes with dimensions
- **`IsComplexResponse`** — Per-page complexity stats (text coverage, image coverage, garbled-text detection, OCR recommendations)

## Building

```bash
# Build everything
cargo build --release

# Build just the server
cargo build --release --bin liteparse-server

# Build just the client
cargo build --release --bin liteparse-client
```

## Running

### Start the server

```bash
cargo run --release --bin liteparse-server
```

The server listens on `0.0.0.0:50051` by default.

### Use the CLI client

```bash
# Parse a file to Markdown (default)
cargo run --release --bin liteparse-client -- parse -f document.pdf

# Parse to plain text
cargo run --release --bin liteparse-client -- parse -f document.pdf --no-markdown

# Parse and output JSON pages with bounding boxes
cargo run --release --bin liteparse-client -- parse -f document.pdf --json

# Screenshot all pages
cargo run --release --bin liteparse-client -- screenshot -f document.pdf

# Screenshot to a custom directory
cargo run --release --bin liteparse-client -- screenshot -f document.pdf -d ./output/

# Analyze complexity / OCR needs
cargo run --release --bin liteparse-client -- is-complex -f document.pdf
```

### Using a config file

Both `parse` and `screenshot` accept an optional `--config-file` (JSON) with a [`LiteParseConfig`](proto/parser.proto):

```bash
cargo run --release --bin liteparse-client -- parse -f document.pdf -c config.json
```

Example `config.json`:

```json
{
  "ocr_enabled": true,
  "ocr_language": "eng",
  "dpi": 300,
  "output_format": 3,
  "max_pages": 10,
  "target_pages": "1-5,10"
}
```

## Configuration Options

| Option | Description |
|--------|-------------|
| `ocr_enabled` | Enable OCR fallback for text-sparse pages and embedded images |
| `ocr_language` | Tesseract language code (e.g. `"eng"`, `"deu"`, `"fra"`) |
| `ocr_server_url` | Optional HTTP OCR server URL (uses local Tesseract if unset) |
| `tessdata_path` | Path to tessdata directory |
| `dpi` | Rendering DPI for OCR and screenshots (default: 150) |
| `output_format` | `1` = JSON, `2` = Text, `3` = Markdown |
| `image_mode` | `1` = Off, `2` = Placeholder, `3` = Embed |
| `max_pages` | Maximum pages to process |
| `target_pages` | Page range string (e.g. `"1-5,10,15-20"`) |
| `num_workers` | Concurrent OCR workers (defaults to CPU count − 1) |
| `password` | Password for encrypted documents |
| `extract_links` | Extract hyperlinks as Markdown `[text](url)` |
| `emit_word_boxes` | Include per-word bounding boxes in output |
| `ocr_failure_fatal` | Abort parse if OCR fails systemically |
| `ocr_hedge_delays_ms` | Request-hedging schedule for HTTP OCR (ms) |
| `preserve_very_small_text` | Keep tiny text normally filtered out |
| `quiet` | Suppress progress output |

## Observability

The `observability` crate sets up a `tracing_subscriber` with pretty formatting and a `DEBUG` level filter. OpenTelemetry (OTLP) support is stubbed out and can be enabled by uncommenting the relevant code in [`crates/observability/src/lib.rs`](crates/observability/src/lib.rs).

## Dependencies

- [tonic](https://github.com/hyperium/tonic) — gRPC server/client framework
- [LiteParse](https://crates.io/crates/liteparse) — Document parsing engine
- [tokio](https://tokio.rs/) — Async runtime
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing
- [tracing](https://github.com/tokio-rs/tracing) — Structured logging

## Docker

A multi-stage `Dockerfile` is included for containerized deployment.

### Pull from the GitHub Container registry

```bash
docker pull ghcr.io/astrabert/liteparse-grpc:main
```

### Build

```bash
docker build -t liteparse-grpc:latest .
```

### Run

```bash
docker run -p 50051:50051 liteparse-grpc:latest
```

### Dockerfile overview

| Stage | Base image | Purpose |
|-------|-----------|---------|
| `builder` | `rust:1.91-slim` | Compiles the `liteparse-server` binary. Installs build deps: `pkg-config`, `libssl-dev`, `protobuf-compiler`, `libclang-dev`, `libtesseract-dev`, `libleptonica-dev`, `cmake`, `g++` |
| Runtime | `debian:trixie-slim` | Runs the compiled server. Installs runtime deps: `ca-certificates`, `libssl3`, `libtesseract5`, `libleptonica6`, `tesseract-ocr-eng` |

The runtime stage also copies the **pdfium** shared library from the builder's cache and symlinks the correct architecture-specific build (`x64` or `arm64`) so it is discoverable at runtime via `PDFIUM_LIB_PATH`.
