FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/bidmart-core-be /usr/local/bin/app
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate
COPY --from=builder /app/migrations ./migrations
EXPOSE 8080
ENV RUST_LOG=info
CMD ["/usr/local/bin/app"]
