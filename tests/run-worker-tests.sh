#!/usr/bin/env bash
# Run ESP32 worker tests one by one (WASM engine only, ESP32 — no C3).
#
# Usage:
#   ./tests/run-worker-tests.sh                 # run ALL worker tests
#   ./tests/run-worker-tests.sh i2c             # run only test-worker-i2c.mjs
#   ./tests/run-worker-tests.sh --list         # just list the tests
#   ./tests/run-worker-tests.sh --skip-bt      # skip BT test
#   ./tests/run-worker-tests.sh --wifi        # include wifi/web/webserver (need gateway; flaky)
#   ./tests/run-worker-tests.sh --timeout 600  # per-test timeout in seconds (default 900)
#
# The WiFi-gateway tests (wifi/web/webserver) are SKIPPED by default: they need
# the openhw-studio-gateway server AND their wifi-association time is
# environmentally variable (7s in one session, 200s+ in another). Pass --wifi to
# run them. They get a longer per-test timeout when enabled.
#
# Requires the compile server on http://localhost:5525 for most tests.
set -u

cd "$(dirname "$0")/.." || exit 1
repo_root="$(pwd)"
tests_dir="$repo_root/tests"

SKIP_BT=0
RUN_WIFI=0
TIMEOUT=900
FILTER=""

for arg in "$@"; do
  case "$arg" in
    --skip-bt) SKIP_BT=1 ;;
    --wifi) RUN_WIFI=1 ;;
    --skip-wifi) RUN_WIFI=0 ;;   # accepted for back-compat (gateway tests skip by default anyway)
    --list) LIST=1 ;;
    --timeout=*) TIMEOUT="${arg#--timeout=}" ;;
    --*)
      echo "Unknown option: $arg" >&2
      echo "Usage: $0 [--skip-bt] [--wifi] [--timeout=N] [test-name]" >&2
      exit 2
      ;;
    *) FILTER="$arg" ;;
  esac
done

# Collect tests (worker family only, ESP32 — C3 tests are gone)
shopt -s nullglob
TESTS=( "$tests_dir"/test-worker-*.mjs )
shopt -u nullglob

if [ "$FILTER" != "" ]; then
  matched=()
  for t in "${TESTS[@]}"; do
    name="$(basename "$t")"
    if [[ "$name" == *"$FILTER"* ]]; then matched+=("$t"); fi
  done
  TESTS=("${matched[@]}")
  if [ ${#TESTS[@]} -eq 0 ]; then
    echo "No test matches '$FILTER'." >&2
    exit 2
  fi
fi

if [ "${LIST:-0}" = "1" ]; then
  for t in "${TESTS[@]}"; do echo "$(basename "$t")"; done
  exit 0
fi

# Preflight: compile server reachable?
if ! curl -s -m 3 -o /dev/null "http://localhost:5525/api/compile/status/0" 2>/dev/null; then
  echo "WARNING: compile server not reachable at http://localhost:5525 — most tests will fail." >&2
fi

LOG_DIR="${LOG_DIR:-/tmp/esp32-worker-test-logs}"
mkdir -p "$LOG_DIR"

pass=0; fail=0; skipped=0
failures=()
declare -A SKIP_REASONS=(
  [test-worker-bt.mjs]="BT controller-init passes; skipped on request (--skip-bt)"
  [test-worker-wifi.mjs]="needs WiFi gateway server (flaky); run with --wifi"
  [test-worker-web.mjs]="needs WiFi gateway server (flaky); run with --wifi"
  [test-worker-webserver.mjs]="needs WiFi gateway server (flaky); run with --wifi"
  [test-worker-net-protocols.mjs]="needs WiFi gateway server (flaky); run with --wifi"
  [test-worker-ipv6.mjs]="needs WiFi gateway server (flaky); run with --wifi"
  [test-worker-espnow.mjs]="needs WiFi gateway server (two nodes share a room); run with --wifi"
  [test-worker-coap-server.mjs]="needs WiFi gateway server (UDP forward); run with --wifi"
)

echo "=================================================================="
echo " ESP32 worker tests — WASM engine (ENGINE=wasm)"
echo " Tests: ${#TESTS[@]}  Timeout: ${TIMEOUT}s/test"
echo "=================================================================="

declare -A RESULTS
for t in "${TESTS[@]}"; do
  name="$(basename "$t")"
  reason="${SKIP_REASONS[$name]:-}"
  if [ -n "$reason" ]; then
    skip=0
    case "$name" in
      test-worker-bt.mjs)
        [ "$SKIP_BT" = "1" ] && skip=1 ;;
      test-worker-wifi.mjs|test-worker-web.mjs|test-worker-webserver.mjs|test-worker-net-protocols.mjs|test-worker-espnow.mjs|test-worker-coap-server.mjs|test-worker-ipv6.mjs)
        [ "$RUN_WIFI" = "0" ] && skip=1 ;;
    esac
    if [ "$skip" = "1" ]; then
      echo "[SKIP ] $name — ${reason}"; skipped=$((skipped+1)); continue;
    fi
  fi

  # Gateway tests have variable wifi-association time; give them a longer
  # timeout than the default so an environmental slow-association isn't a FAIL.
  test_timeout="$TIMEOUT"
  case "$name" in
    test-worker-wifi.mjs|test-worker-web.mjs|test-worker-webserver.mjs|test-worker-net-protocols.mjs|test-worker-espnow.mjs|test-worker-coap-server.mjs|test-worker-ipv6.mjs)
      if [ "$test_timeout" -lt 600 ]; then test_timeout=600; fi ;;
  esac

  log="$LOG_DIR/$name.log"
  echo -n "[RUN  ] $name ... "
  start=$(date +%s)
  if ENGINE=wasm timeout "$test_timeout" node "$t" >"$log" 2>&1; then
    el=$(( $(date +%s) - start ))
    echo "PASS (${el}s)"
    RESULTS[$name]="PASS"
    pass=$((pass+1))
  else
    rc=$?
    el=$(( $(date +%s) - start ))
    if [ "$rc" = "124" ]; then
      echo "TIMEOUT (>${TIMEOUT}s) — log: $log"
      RESULTS[$name]="TIMEOUT"
    else
      echo "FAIL (${el}s, exit=$rc) — log: $log"
      RESULTS[$name]="FAIL"
    fi
    fail=$((fail+1))
    failures+=("$name")
  fi
done

echo "=================================================================="
echo " SUMMARY"
echo "=================================================================="
for t in "${TESTS[@]}"; do
  name="$(basename "$t")"
  printf "  %-4s %s\n" "${RESULTS[$name]:-SKIP}" "$name"
done
echo "------------------------------------------------------------------"
echo " PASS=$pass  FAIL=$fail  SKIP=$skipped"
[ "$fail" = "0" ] && exit 0 || exit 1