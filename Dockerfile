FROM node:22-bookworm-slim AS web
WORKDIR /app
ARG BUILD_SHA
ENV BUILD_SHA=$BUILD_SHA
COPY package.json package-lock.json vite.config.ts tsconfig.json svelte.config.js ./
COPY frontend ./frontend
RUN test -n "$BUILD_SHA" && test "$BUILD_SHA" != "unversioned-build"
RUN npm ci && npm run build

FROM rust:1.88-bookworm AS server
WORKDIR /app
ARG BUILD_SHA
ENV BUILD_SHA=$BUILD_SHA
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src ./src
RUN test -n "$BUILD_SHA" && test "$BUILD_SHA" != "unversioned-build"
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system app \
    && useradd --system --gid app --home-dir /app app \
    && mkdir -p /app/data \
    && chown -R app:app /app
WORKDIR /app
COPY --from=server /app/target/release/stream-access-cues /usr/local/bin/stream-access-cues
COPY --from=web /app/dist ./dist
USER app
ENV PORT=8080 DATA_DIR=/app/data DIST_DIR=/app/dist RUST_LOG=stream_access_cues=info DEPLOYMENT_MODE=hosted
EXPOSE 8080
VOLUME ["/app/data"]
ENTRYPOINT ["/usr/local/bin/stream-access-cues"]
