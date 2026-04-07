# Stage 1: Chef planner -- compute dependency recipe
FROM rust:1.94.1-bookworm AS planner
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build -- cache deps, run quality gates, compile
FROM rust:1.94.1-bookworm AS builder
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app

# Cook dependencies first (cached layer -- only invalidated when deps change)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy full source
COPY . .

# Quality gates (ordered fast-to-slow, each RUN fails the build independently)
RUN cargo fmt --check
RUN cargo clippy --workspace -- -D warnings
RUN cargo test --workspace
RUN cargo build --release --workspace

# Stage 3: Runtime -- minimal image with just the binary
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/runtime /usr/local/bin/runtime
EXPOSE 8080
ENTRYPOINT ["runtime"]
