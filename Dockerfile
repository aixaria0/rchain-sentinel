FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
COPY backend ./backend
WORKDIR /app/backend
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/backend/target/release/rchain-sentinel /usr/local/bin/rchain-sentinel
ENV RCHAIN_RNODE_URL=http://localhost:40403
ENV RCHAIN_RNODE_URLS=
ENV PORT=8080
EXPOSE 8080
CMD ["/usr/local/bin/rchain-sentinel"]
