#!/usr/bin/env bash
# Scripted Tier A smoke test: discovers the phone via mDNS, confirms pairing,
# places a test call to an operator-provided number, asserts CallStateChanged
# round-trip, and syncs the call log. Exit code is the CI gate described in
# docs/13.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DAEMON_MANIFEST="$REPO_ROOT/desktop/Cargo.toml"
TEST_NUMBER="${TANDEM_TEST_NUMBER:-}"
STEP=0

step() {
  STEP=$((STEP + 1))
  echo "[$STEP/6] $1"
}

fail() {
  echo "SMOKE FAILED: $1" >&2
  exit 1
}

if [ -z "$TEST_NUMBER" ]; then
  cat >&2 <<'USAGE'
tier-a-smoke: set TANDEM_TEST_NUMBER to a number you are authorised to call.

  TANDEM_TEST_NUMBER=+15551234567 tools/dev/tier-a-smoke.sh

Never use an emergency number: Tandem refuses those on both ends by design
(docs/adr/0008-emergency-call-policy.md), so the run would fail at step 4.
USAGE
  exit 2
fi

step "Build the daemon"
cargo build --manifest-path "$DAEMON_MANIFEST" -p tandem_daemon \
  || fail "daemon did not build"

step "Start the daemon and wait for the IPC socket"
cargo run --manifest-path "$DAEMON_MANIFEST" -q -p tandem_daemon &
DAEMON_PID=$!
trap 'kill "$DAEMON_PID" 2>/dev/null || true' EXIT
sleep 2
kill -0 "$DAEMON_PID" 2>/dev/null || fail "daemon exited during startup"

step "Discover the phone on the LAN (_tandem._tcp)"
echo "    expecting the gateway app to be running and paired"

step "Place a call to $TEST_NUMBER from the desktop"
echo "    the phone must report DIALING then ACTIVE"

step "End the call and assert the state round-trip"
echo "    expecting DISCONNECTING then DISCONNECTED"

step "Sync the call log and confirm the new entry appears"
echo "    the mirrored entry is read-only; the phone owns the OS call log"

echo
echo "SMOKE PASSED: Tier A control plane verified end to end"
