FROM node:24-bookworm-slim AS node_runtime
FROM rust:1.99.0-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    bubblewrap busybox-static util-linux coreutils curl \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy \
    && useradd --create-home --uid 1000 frontier \
    && mkdir -p /app /target /data /usr/local/cargo/registry \
    && chown frontier:frontier /app /target /data /usr/local/cargo/registry

COPY --from=node_runtime /usr/local/bin/node /usr/local/bin/node

ENV CARGO_TARGET_DIR=/target
ENV CARGO_BUILD_JOBS=2
WORKDIR /app
USER frontier
CMD ["cargo", "run", "--locked"]
