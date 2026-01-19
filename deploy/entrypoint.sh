#!/bin/sh
set -eu

is_truthy() {
  case "${1:-}" in
    1|true|TRUE|yes|YES) return 0 ;;
    *) return 1 ;;
  esac
}

pids=""

if is_truthy "${RUN_CONTROL_PLANE:-true}"; then
  gateway-control-plane &
  pids="$pids $!"
fi

if is_truthy "${RUN_DATA_PLANE:-true}"; then
  gateway-data-plane &
  pids="$pids $!"
fi

if [ -z "$(echo "$pids" | tr -d ' ')" ]; then
  echo "RUN_CONTROL_PLANE 与 RUN_DATA_PLANE 都被禁用，容器将退出。" >&2
  exit 1
fi

terminate() {
  for pid in $pids; do
    kill -TERM "$pid" 2>/dev/null || true
  done
}

trap 'terminate; for pid in $pids; do wait "$pid" 2>/dev/null || true; done; exit 0' INT TERM

while :; do
  for pid in $pids; do
    if ! kill -0 "$pid" 2>/dev/null; then
      set +e
      wait "$pid"
      status=$?
      set -e
      terminate
      for pid2 in $pids; do
        wait "$pid2" 2>/dev/null || true
      done
      exit "$status"
    fi
  done
  sleep 1
done
