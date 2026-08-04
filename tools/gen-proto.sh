#!/usr/bin/env bash
# Regenerates protocol bindings on POSIX systems: runs protoc/gradle protobuf
# task checks and cargo build -p tandem_proto so both languages compile from
# /proto in one step.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROTO_DIR="$REPO_ROOT/proto"

if [ ! -d "$PROTO_DIR" ]; then
  echo "gen-proto: no proto directory at $PROTO_DIR" >&2
  exit 1
fi

echo "==> Schema files"
find "$PROTO_DIR" -name '*.proto' -print | sort

# The Rust build vendors its own protoc, so a system protoc is optional and used
# only for the standalone syntax check below.
if command -v protoc >/dev/null 2>&1; then
  echo "==> protoc syntax check ($(protoc --version))"
  protoc --proto_path="$PROTO_DIR" -o /dev/null "$PROTO_DIR"/tandem/v1/*.proto
else
  echo "==> protoc not found; skipping standalone check (Rust codegen vendors it)"
fi

echo "==> Rust bindings (tandem_proto)"
cargo build --manifest-path "$REPO_ROOT/desktop/Cargo.toml" -p tandem_proto

if [ -d "$REPO_ROOT/android" ] && [ -x "$REPO_ROOT/android/gradlew" ]; then
  echo "==> Kotlin bindings (:app:generateProto)"
  (cd "$REPO_ROOT/android" && ./gradlew --quiet :app:generateDebugProto)
else
  echo "==> android/gradlew not present; skipping Kotlin codegen"
fi

echo "==> Protocol bindings up to date"
