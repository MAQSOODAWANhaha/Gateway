# syntax=docker/dockerfile:1

FROM node:20-bookworm-slim AS web_builder

WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1.92.0-bookworm AS rust_builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked --bin gateway-control-plane --bin gateway-data-plane

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl3 \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust_builder /app/target/release/gateway-control-plane /usr/local/bin/gateway-control-plane
COPY --from=rust_builder /app/target/release/gateway-data-plane /usr/local/bin/gateway-data-plane
COPY --from=web_builder /app/web/dist /app/web/dist
COPY deploy/entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod +x /usr/local/bin/entrypoint.sh

ENV RUN_CONTROL_PLANE=true \
  RUN_DATA_PLANE=true \
  CONTROL_PLANE_ADDR=0.0.0.0:9000

EXPOSE 9000 80 443

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
