#!/usr/bin/env bash
set -euo pipefail

# Regression guard for the ACR source-tarball path: Docker builds must not
# require a checkout or a caller-provided identity, but a supplied full SHA
# must reach the runtime health endpoint unchanged.
readonly BUILD_ID_TEST_SHA='877abcd9294622870e413794abc814a6727bc3d6'
readonly BUILD_ID_TEST_PORT=18080
readonly BUILD_ID_TEST_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly BUILD_ID_TEST_BINARY="$BUILD_ID_TEST_ROOT/target/release/stream-access-cues"
readonly BUILD_ID_TEST_DIR="$(mktemp -d)"

cleanup() {
  if [[ -n "${BUILD_ID_TEST_PID:-}" ]]; then
    kill "$BUILD_ID_TEST_PID" 2>/dev/null || true
    wait "$BUILD_ID_TEST_PID" 2>/dev/null || true
  fi
  rm -rf "$BUILD_ID_TEST_DIR"
}
trap cleanup EXIT

grep -q '^ARG BUILD_SHA=dev$' "$BUILD_ID_TEST_ROOT/Dockerfile"
# One global declaration supplies the local default, then each of the three
# Docker stages redeclares it before using the factory-supplied identity.
[[ "$(grep -c '^ARG BUILD_SHA$' "$BUILD_ID_TEST_ROOT/Dockerfile")" -eq 3 ]]
grep -q '^FROM rust:1-slim AS server$' "$BUILD_ID_TEST_ROOT/Dockerfile"
grep -q '^LABEL org.opencontainers.image.revision=\$BUILD_SHA$' "$BUILD_ID_TEST_ROOT/Dockerfile"
! rg -q 'Command::new\("git"\)|execFileSync\(.git|COPY[[:space:]]+\.git|git[[:space:]]+rev-parse' \
  "$BUILD_ID_TEST_ROOT/Dockerfile" "$BUILD_ID_TEST_ROOT/build.rs" "$BUILD_ID_TEST_ROOT/vite.config.ts"

(
  cd "$BUILD_ID_TEST_ROOT"
  BUILD_SHA="$BUILD_ID_TEST_SHA" npm run build
  BUILD_SHA="$BUILD_ID_TEST_SHA" cargo build --release --locked
)

cd "$BUILD_ID_TEST_DIR"
env -i PORT="$BUILD_ID_TEST_PORT" "$BUILD_ID_TEST_BINARY" >server.log 2>&1 &
BUILD_ID_TEST_PID=$!

for _ in $(seq 1 50); do
  if curl --fail --silent "http://127.0.0.1:$BUILD_ID_TEST_PORT/health" >health.json; then
    break
  fi
  sleep 0.1
done

test -s health.json
test "$(sed -n 's/.*\"build_sha\":\"\([^\"]*\)\".*/\1/p' health.json)" = "$BUILD_ID_TEST_SHA"
grep -Fq "$BUILD_ID_TEST_SHA" "$BUILD_ID_TEST_ROOT/dist/assets/"*.js
