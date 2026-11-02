#!/usr/bin/env bash
# Rail-Across-Suite — watch every instance of this machine: services running,
# project server and optimizer answering, the site answering from outside
# through Caddy, its certificate not about to run out, space left for the data.
# Exits non-zero when something is wrong, so the timer's unit shows up in
# `systemctl --failed` and the journal (journalctl -u rail-across-monitor); with
# RA_MONITOR_NOTIFY set, that command is run with the findings on stdin.
#
#   deploy/monitor.sh <rail-across.env> [<rail-across.env> …]             check once
#   deploy/monitor.sh --install <rail-across.env> [<rail-across.env> …]   copy to /usr/local/lib/rail-across
#                                                         and run every 5 minutes (rail-across-monitor.timer)
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=deploy/lib.sh
. "$here/lib.sh"

CERT_DAYS=${RA_MONITOR_CERT_DAYS:-14}
FREE_GB=${RA_MONITOR_FREE_GB:-20}

if [ "${1:-}" = --install ]; then
  shift
  [ "$(id -u)" = 0 ] || die "run as root"
  [ $# -gt 0 ] || die "name the configurations to watch"
  for c in "$@"; do [ -f "$c" ] || die "no configuration $c"; done
  dest=/usr/local/lib/rail-across
  install -d "$dest"
  install -m 0755 "$here/monitor.sh" "$dest/monitor.sh"
  install -m 0644 "$here/lib.sh" "$dest/lib.sh"
  RA_MONITOR_EXEC="$dest/monitor.sh $(printf '%q ' "$@")"
  export RA_MONITOR_EXEC
  render "$here/templates/rail-across-monitor.service" /etc/systemd/system/rail-across-monitor.service
  render "$here/templates/rail-across-monitor.timer" /etc/systemd/system/rail-across-monitor.timer
  systemctl daemon-reload
  systemctl enable --now rail-across-monitor.timer
  systemctl start rail-across-monitor.service || true
  systemctl --no-pager --lines=20 status rail-across-monitor.service | tail -n +1
  exit 0
fi

[ $# -gt 0 ] || die "usage: monitor.sh <rail-across.env> [<rail-across.env> …]"
problems=()
problem() { echo "FAIL $1"; }
ok() { echo "ok   $1"; }

check_instance() {
  RA_CONFIG=$1
  load_config
  local name=${RA_INSTANCE:-default} u days avail
  for u in $(service_units) "$(unit_name rail-across-server-backup.timer)"; do
    if systemctl is-active -q "$u"; then ok "$name: $u active"; else problem "$name: $u is $(systemctl is-active "$u")"; fi
  done
  if curl -fs -m 10 -o /dev/null "http://$RA_BIND:$RA_SERVER_PORT/api/health"; then ok "$name: project server answers"
  else problem "$name: project server does not answer on $RA_BIND:$RA_SERVER_PORT"; fi
  if curl -fs -m 10 -o /dev/null "http://$RA_BIND:$RA_OPTIMIZER_PORT/health"; then ok "$name: optimizer answers"
  else problem "$name: optimizer does not answer on $RA_BIND:$RA_OPTIMIZER_PORT"; fi
  # From outside, through Caddy and TLS (the optimizer is behind the sign-in there).
  if curl -fs -m 15 -o /dev/null "https://$RA_DOMAIN/api/health"; then ok "$name: https://$RA_DOMAIN answers"
  else problem "$name: https://$RA_DOMAIN/api/health does not answer"; fi
  days=$(echo | timeout 15 openssl s_client -connect "$RA_DOMAIN:443" -servername "$RA_DOMAIN" 2>/dev/null \
    | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)
  if [ -z "$days" ]; then problem "$name: no certificate read from $RA_DOMAIN"
  else
    days=$(( ($(date -d "$days" +%s) - $(date +%s)) / 86400 ))
    if [ "$days" -lt "$CERT_DAYS" ]; then problem "$name: certificate of $RA_DOMAIN runs out in $days days"
    else ok "$name: certificate valid $days more days"; fi
  fi
  avail=$(df -BG --output=avail "$RA_DATA" 2>/dev/null | tail -1 | tr -dc 0-9)
  if [ -z "$avail" ]; then problem "$name: no data directory $RA_DATA"
  elif [ "$avail" -lt "$FREE_GB" ]; then problem "$name: only $avail GB free for $RA_DATA"
  else ok "$name: $avail GB free for $RA_DATA"; fi
}

for c in "$@"; do
  # Each in its own subshell: load_config sets the instance's variables.
  out=$( (check_instance "$c") 2>&1 )
  rc=$?
  echo "$out"
  [ "$rc" = 0 ] || problems+=("$c: ${out##*$'\n'}")
  while IFS= read -r line; do
    case "$line" in "FAIL "*) problems+=("${line#FAIL }") ;; esac
  done <<<"$out"
done

if [ ${#problems[@]} -gt 0 ]; then
  if [ -n "${RA_MONITOR_NOTIFY:-}" ]; then
    printf '%s\n' "Rail-Across-Suite on $(hostname):" "${problems[@]}" | bash -c "$RA_MONITOR_NOTIFY" || true
  fi
  echo "${#problems[@]} problem(s)" >&2
  exit 1
fi
echo "all well"
