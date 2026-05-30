#!/usr/bin/env bash
# sftp-test-target.sh — manage the Rift SFTP integration-test LXC (Proxmox 121
# `rift-sftp-test`). Mirrors scripts/cdp/c.sh ergonomics for the test target.
#
# The container IP is DHCP, so this resolves it LIVE via the Proxmox host on every
# call instead of trusting a hardcoded value — tests should `eval $(… env)` rather
# than pin RIFT_TEST_SFTP_HOST. Requires: `blazzer-labs` SSH alias (Proxmox root)
# + the rift key in .secrets.
#
# Usage:
#   bash scripts/sftp-test-target.sh health     # ssh ping as rift; prints IP + ok/down
#   bash scripts/sftp-test-target.sh ip          # current container IP (bare)
#   bash scripts/sftp-test-target.sh status      # pct status from host
#   bash scripts/sftp-test-target.sh reset       # rollback to pristine `baseline` snapshot
#   bash scripts/sftp-test-target.sh env          # print export lines for the test suite
#   bash scripts/sftp-test-target.sh ssh [cmd]   # ssh as rift (interactive or run cmd)
#   bash scripts/sftp-test-target.sh tree         # show the seeded fxserver tree
set -euo pipefail
VMID="${RIFT_TEST_VMID:-121}"
PVE="${RIFT_PVE_HOST:-blazzer-labs}"
KEY="${RIFT_TEST_SFTP_KEY:-/c/AI Workflow/.secrets/rift-sftp-test}"
USER_="${RIFT_TEST_SFTP_USER:-rift}"
cmd="${1:-health}"; shift || true

resolve_ip() { ssh "$PVE" "pct exec $VMID -- hostname -I" 2>/dev/null | tr -d ' \n'; }

case "$cmd" in
  ip)
    resolve_ip; echo ;;
  status)
    ssh "$PVE" "pct status $VMID; pct listsnapshot $VMID" ;;
  health)
    ip="$(resolve_ip)"
    if [ -z "$ip" ]; then echo "DOWN — container $VMID not running or no IP"; exit 1; fi
    if ssh -i "$KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=8 \
         "$USER_@$ip" 'echo ok' >/dev/null 2>&1; then
      echo "UP — $USER_@$ip (ssh+key ok)"
    else
      echo "REACHABLE but ssh/key FAILED — $USER_@$ip"; exit 1
    fi ;;
  reset)
    echo "rolling back $VMID to baseline…"
    ssh "$PVE" "pct rollback $VMID baseline && pct start $VMID" 2>&1 | tail -3
    echo "reset done — re-resolve IP with: $0 ip" ;;
  env)
    ip="$(resolve_ip)"
    [ -z "$ip" ] && { echo "# container down — cannot resolve IP" >&2; exit 1; }
    echo "export RIFT_TEST_SFTP_HOST=$ip"
    echo "export RIFT_TEST_SFTP_PORT=22"
    echo "export RIFT_TEST_SFTP_USER=$USER_"
    echo "export RIFT_TEST_SFTP_KEY=\"$KEY\"" ;;
  ssh)
    ip="$(resolve_ip)"
    ssh -i "$KEY" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$USER_@$ip" "$@" ;;
  tree)
    ssh "$PVE" "pct exec $VMID -- find /home/$USER_/fxserver -maxdepth 3" ;;
  *)
    echo "usage: $0 {health|ip|status|reset|env|ssh [cmd]|tree}" >&2; exit 2 ;;
esac
