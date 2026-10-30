# Shared by setup.sh, deploy.sh, release.sh and monitor.sh: load the
# configuration of a Rail-Across-Suite instance and fill the defaults.
# shellcheck shell=bash

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\n== %s\n' "$*"; }

# parse_common <args…>: take --config <file> out of the arguments (sets
# RA_CONFIG); the rest is left in REST for the script's own options.
parse_common() {
  REST=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --config) [ $# -ge 2 ] || die "--config needs a file"; RA_CONFIG=$2; shift 2 ;;
      --config=*) RA_CONFIG=${1#--config=}; shift ;;
      *) REST+=("$1"); shift ;;
    esac
  done
}

load_config() {
  RA_CONFIG=${RA_CONFIG:-/etc/rail-across/rail-across.env}
  [ -f "$RA_CONFIG" ] || die "no configuration at $RA_CONFIG (copy deploy/rail-across.env.example there)"
  RA_CONFIG=$(cd "$(dirname "$RA_CONFIG")" && pwd)/$(basename "$RA_CONFIG")
  set -a
  # shellcheck disable=SC1090
  . "$RA_CONFIG"
  set +a

  local here sfx
  # The scripts live in deploy/; the checkout is one up.
  here=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
  : "${RA_DOMAIN:?RA_DOMAIN must be set in $RA_CONFIG}"
  # Several instances on one machine (test and production): an instance name
  # puts its own prefix on the units, user and data directory. Empty is the
  # first instance with the plain names (rail-across-server, rail-across, /var/lib/rail-across).
  RA_INSTANCE=${RA_INSTANCE:-}
  case "$RA_INSTANCE" in *[!a-z0-9]*) die "RA_INSTANCE may only hold a-z and 0-9" ;; esac
  sfx=${RA_INSTANCE:+-$RA_INSTANCE}
  RA_UNIT_PREFIX=rail-across$sfx-
  RA_LABEL=${RA_INSTANCE:+ [$RA_INSTANCE]}
  RA_REPO=${RA_REPO:-$here}
  # The site: the suite's start page (apps/home) at the root, each module's
  # app under its path (MODULE_APPS) — one directory, one address, one sign-in.
  RA_SITE_DIR=${RA_SITE_DIR:-$RA_REPO/dist}
  RA_USER=${RA_USER:-rail-across$sfx}
  RA_DATA=${RA_DATA:-/var/lib/rail-across$sfx}
  RA_DB=${RA_DB:-$RA_DATA/rail-across.sqlite}
  RA_BACKUPS=${RA_BACKUPS:-$RA_DATA/backups}
  RA_BACKUP_DAYS=${RA_BACKUP_DAYS:-30}
  RA_VENV=${RA_VENV:-$RA_DATA/venv}
  RA_TERRAIN_CACHE=${RA_TERRAIN_CACHE:-$RA_DATA/terrain-cache}
  RA_CLOUDS=${RA_CLOUDS:-$RA_DATA/clouds}
  RA_BIND=${RA_BIND:-127.0.0.1}
  RA_SERVER_PORT=${RA_SERVER_PORT:-8787}
  RA_OPTIMIZER_PORT=${RA_OPTIMIZER_PORT:-8099}
  RA_SERVER_LOG=${RA_SERVER_LOG:-info}
  RA_CADDY=${RA_CADDY:-system}
  RA_UPSTREAM=${RA_UPSTREAM:-$RA_BIND}
  RA_SITE_ROOT=${RA_SITE_ROOT:-$RA_SITE_DIR}
  RA_TRUST_PROXY=${RA_TRUST_PROXY:-127.0.0.1,::1}
  if [ "$RA_CADDY" = system ]; then RA_CADDY_DIR=${RA_CADDY_DIR:-/etc/caddy/conf.d}
  else RA_CADDY_DIR=${RA_CADDY_DIR:-$RA_DATA}; fi
  RA_CADDY_SITE=$RA_CADDY_DIR/rail-across$sfx.caddy
  RA_CADDY_RELOAD=${RA_CADDY_RELOAD:-}
  RA_ADMIN_LOGIN=${RA_ADMIN_LOGIN:-admin}
  RA_OPTIMIZER_TIMEOUT=${RA_OPTIMIZER_TIMEOUT:-120}
  RA_OPTIMIZER_WORKERS=${RA_OPTIMIZER_WORKERS:-2}
  RA_CLOUDJOBS_PARALLEL=${RA_CLOUDJOBS_PARALLEL:-2}
  RA_CLOUDRUNS_PARALLEL=${RA_CLOUDRUNS_PARALLEL:-2}
  # Memory per service (systemd MemoryMax). The point cloud jobs: up to
  # RA_CLOUDJOBS_PARALLEL preparations of 768 MB heap and RA_CLOUDRUNS_PARALLEL
  # long runs of 320 MB, plus the queue process — 3G holds two of each.
  RA_CLOUDJOBS_MEMORY=${RA_CLOUDJOBS_MEMORY:-3G}
  RA_SERVER_MEMORY=${RA_SERVER_MEMORY:-infinity}
  RA_OPTIMIZER_MEMORY=${RA_OPTIMIZER_MEMORY:-infinity}
  # What deploy.sh may put on this instance: "main" (git pull, the test
  # server) or "tags" (only prod-* tags with a green CI, production).
  RA_TRACK=${RA_TRACK:-main}
  RA_GITHUB_REPO=${RA_GITHUB_REPO:-Serigalan/rail-across-suite}
  RA_NODE=${RA_NODE:-$(command -v node || echo /usr/bin/node)}
  RA_STATE=$RA_DATA/.deploy-state
  export RA_DOMAIN RA_INSTANCE RA_UNIT_PREFIX RA_LABEL RA_REPO RA_SITE_DIR RA_USER RA_DATA RA_DB \
    RA_BACKUPS RA_BACKUP_DAYS RA_VENV RA_TERRAIN_CACHE RA_CLOUDS RA_BIND RA_SERVER_PORT \
    RA_OPTIMIZER_PORT RA_SERVER_LOG RA_CADDY RA_UPSTREAM RA_SITE_ROOT RA_TRUST_PROXY \
    RA_CADDY_DIR RA_CADDY_SITE RA_CADDY_RELOAD RA_ADMIN_LOGIN RA_OPTIMIZER_TIMEOUT \
    RA_OPTIMIZER_WORKERS RA_CLOUDJOBS_PARALLEL RA_CLOUDRUNS_PARALLEL RA_CLOUDJOBS_MEMORY \
    RA_SERVER_MEMORY RA_OPTIMIZER_MEMORY RA_TRACK RA_GITHUB_REPO RA_NODE

  case "$RA_BIND" in 0.0.0.0|::|'') die "RA_BIND=$RA_BIND would put the services on every interface" ;; esac
  case "$RA_CADDY" in system|external) ;; *) die "RA_CADDY must be system or external" ;; esac
  case "$RA_TRACK" in main|tags) ;; *) die "RA_TRACK must be main or tags" ;; esac
}

# The modules' apps, each built into its path of the site: "<app directory under apps/>:<path>".
# shellcheck disable=SC2034  # used by deploy.sh
MODULE_APPS=(layout-tool:layout signalling-tool:signalling)

# The units of this instance, as installed (template rail-across-<x> → $RA_UNIT_PREFIX<x>).
# shellcheck disable=SC2034  # used by setup.sh
UNIT_TEMPLATES=(rail-across-server.service rail-across-optimizer.service rail-across-cloudjobs.service rail-across-server-backup.service rail-across-server-backup.timer)
unit_name() { echo "$RA_UNIT_PREFIX${1#rail-across-}"; }
SERVICES=(server optimizer cloudjobs)
service_units() { local s; for s in "${SERVICES[@]}"; do echo "${RA_UNIT_PREFIX}$s.service"; done; }

# ci_green <commit>: true when GitHub's CI finished green for that commit.
ci_green() {
  curl -fsS "https://api.github.com/repos/$RA_GITHUB_REPO/actions/runs?head_sha=$1&per_page=20" 2>/dev/null \
    | python3 -c 'import json, sys
runs = json.load(sys.stdin).get("workflow_runs", [])
sys.exit(0 if any(r.get("conclusion") == "success" for r in runs) else 1)'
}

# render <template> <target>: replace every @RA_…@ in the template by the
# variable's value. Fails on a placeholder that has no value.
render() {
  python3 - "$1" "$2" <<'PY'
import os, re, sys
src, dst = sys.argv[1], sys.argv[2]
text = open(src, encoding="utf-8").read()
def sub(m):
    name = m.group(1)
    if name not in os.environ:
        sys.exit(f"{src}: no value for {name}")
    return os.environ[name]
out = re.sub(r"@(RA_[A-Z_]+)@", sub, text)
tmp = dst + ".tmp"
with open(tmp, "w", encoding="utf-8") as f:
    f.write(out)
os.replace(tmp, dst)
PY
}

# as_user <cmd…>: run a command as the service user (plain when that is us).
as_user() {
  if [ "$(id -un)" = "$RA_USER" ]; then "$@"; else runuser -u "$RA_USER" -- "$@"; fi
}

# changed <name> <file…>: true when the files differ from the last time this
# name was marked done (mark_done) — so deploy.sh reinstalls only what changed.
changed() {
  local name=$1; shift
  local sum
  sum=$(cat "$@" 2>/dev/null | sha256sum | cut -d' ' -f1)
  [ "$(cat "$RA_STATE/$name" 2>/dev/null)" != "$sum" ]
}
mark_done() {
  local name=$1; shift
  mkdir -p "$RA_STATE"
  cat "$@" 2>/dev/null | sha256sum | cut -d' ' -f1 > "$RA_STATE/$name"
}
