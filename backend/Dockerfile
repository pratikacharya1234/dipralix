# syntax=docker/dockerfile:1.6

# ── Builder ──────────────────────────────────────────────────────────
FROM rust:1.86-slim-bookworm AS builder

WORKDIR /build

# Cache deps independently from source for faster rebuilds.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin dipralix-stats && \
    rm -rf src target/release/deps/dipralix_stats*

COPY src ./src
RUN cargo build --release --bin dipralix-stats && \
    strip target/release/dipralix-stats

# ── Runtime ──────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app
COPY --from=builder /build/target/release/dipralix-stats /app/dipralix-stats

ENV PORT=8080
EXPOSE 8080

USER nonroot
ENTRYPOINT ["/app/dipralix-stats"]
