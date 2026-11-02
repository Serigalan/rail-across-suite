#!/usr/bin/env bash
# Rail-Across-Suite — set up a server from scratch (Debian 12 / Ubuntu 22.04 or
# newer), or bring an existing one in line with the configuration. Safe to run
# again: every step checks what is there first. One server for the suite: its
# start page, every module's app, the project server with the one sign-in.
#
#   sudo deploy/setup.sh                     needs /etc/rail-across/rail-across.env (see rail-across.env.example)
#   sudo deploy/setup.sh --config <file>     another instance on the same machine (RA_INSTANCE)
#
# What it does: system packages (Node 22, Python venv, mdbtools, acl, Caddy),
# the service user and data directory (with the point clouds' directory), the
# NTv2 grids for PROJ, the systemd units (rail-across-server, rail-across-optimizer,
# rail-across-cloudjobs, the backup) and the Caddy site, then deploy.sh for the first build, and finally the
# first admin when the database has no user yet.
set -euo pipefail
# shellcheck source=deploy/lib.sh
. "$(dirname "$0")/lib.sh"

[ "$(id -u)" = 0 ] || die "run as root"
parse_common "$@"
[ ${#REST[@]} = 0 ] || die "unknown option ${REST[0]}"
load_config
cd "$RA_REPO"

step "Instance ${RA_INSTANCE:-(default)}: units ${RA_UNIT_PREFIX}*, user $RA_USER, data $RA_DATA"
# Never take over another instance's units, database or ports: a second
# configuration with the default names would silently rewire the first one.
for t in "${UNIT_TEMPLATES[@]}"; do
  u=/etc/systemd/system/$(unit_name "$t")
  case "$t" in *.timer|*backup*) continue ;; esac
  if [ -f "$u" ] && ! grep -q "^WorkingDirectory=$RA_REPO/" "$u"; then
    die "$u belongs to another checkout ($(grep -m1 '^WorkingDirectory=' "$u")) — set RA_INSTANCE"
  fi
done
for u in /etc/systemd/system/rail-across-*server.service; do
  if [ ! -f "$u" ] || [ "$(basename "$u")" = "$(unit_name rail-across-server.service)" ]; then continue; fi
  grep -q "^Environment=RA_SERVER_DB=$RA_DB\$" "$u" && die "$u already uses the database $RA_DB"
  grep -q "^Environment=RA_SERVER_PORT=$RA_SERVER_PORT\$" "$u" && die "$u already uses port $RA_SERVER_PORT"
done
for u in /etc/systemd/system/rail-across-*optimizer.service; do
  if [ ! -f "$u" ] || [ "$(basename "$u")" = "$(unit_name rail-across-optimizer.service)" ]; then continue; fi
  grep -q "^Environment=RA_OPTIMIZER_PORT=$RA_OPTIMIZER_PORT\$" "$u" && die "$u already uses port $RA_OPTIMIZER_PORT"
done
# The configuration names paths and users, nothing secret, but it is root's.
chmod go-w "$RA_CONFIG"

step "System packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q ca-certificates curl gnupg git rsync python3 python3-venv python3-dev \
  build-essential mdbtools acl debian-keyring debian-archive-keyring apt-transport-https

node_major() { node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0; }
if [ "$(node_major)" -lt 22 ]; then
  step "Node.js 22 (NodeSource)"
  install -d -m 0755 /etc/apt/keyrings
  curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor --yes -o /etc/apt/keyrings/nodesource.gpg
  echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" \
    > /etc/apt/sources.list.d/nodesource.list
  apt-get update -q
  apt-get install -y -q nodejs
fi
[ "$(node_major)" -ge 22 ] || die "Node.js 22 or newer is needed"
RA_NODE=$(command -v node); export RA_NODE

if [ "$RA_CADDY" = system ] && ! command -v caddy >/dev/null; then
  step "Caddy"
  curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/gpg.key \
    | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -fsSL https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt \
    > /etc/apt/sources.list.d/caddy-stable.list
  apt-get update -q
  apt-get install -y -q caddy
fi

step "Service user and data directory"
if ! id "$RA_USER" >/dev/null 2>&1; then
  useradd --system --home-dir "$RA_DATA" --shell /usr/sbin/nologin "$RA_USER"
fi
install -d -m 0750 -o "$RA_USER" -g "$(id -gn "$RA_USER")" "$RA_DATA"
for d in "$RA_BACKUPS" "$RA_TERRAIN_CACHE" "$RA_CLOUDS" "$RA_DATA/share/proj" "$(dirname "$RA_DB")"; do
  install -d -m 0750 -o "$RA_USER" -g "$(id -gn "$RA_USER")" "$d"
done
# The service user only reads the checkout; the build is done as root. A
# checkout under a closed directory (/root, mode 0700) gets the user through
# with an ACL that allows passing, not listing — for this user only.
dirs=() d=$RA_REPO
while [ "$d" != / ]; do d=$(dirname "$d"); dirs=("$d" "${dirs[@]}"); done
for d in "${dirs[@]}"; do   # from / inwards: an inner one may be closed only by an outer one
  if ! as_user test -x "$d"; then
    setfacl -m "u:$RA_USER:--x" "$d"
    echo "ACL: $RA_USER may pass through $d"
  fi
done
if ! as_user test -r "$RA_REPO/server/bin/rail-across-server.mjs"; then
  die "$RA_USER cannot read $RA_REPO — make the checkout readable (chmod -R a+rX)"
fi

step "NTv2 grids for PROJ"
(cd apps/layout-tool && node tools/data.mjs)
for g in apps/layout-tool/grids/*.tif; do
  install -m 0644 "$g" "$RA_DATA/share/proj/$(basename "$g")"
done

step "Python environment"
if [ ! -x "$RA_VENV/bin/python" ]; then
  python3 -m venv "$RA_VENV"
fi

step "First build and install"
"$RA_REPO/deploy/deploy.sh" --config "$RA_CONFIG" --no-restart

step "systemd units"
for t in "${UNIT_TEMPLATES[@]}"; do
  unit=$(unit_name "$t")
  # A unit linked in from elsewhere (an older hand-made setup) is replaced.
  [ -L "/etc/systemd/system/$unit" ] && rm "/etc/systemd/system/$unit"
  render "deploy/templates/$t" "/etc/systemd/system/$unit"
done
systemctl daemon-reload
mapfile -t services < <(service_units)
systemctl enable --now "${services[@]}" "$(unit_name rail-across-server-backup.timer)"
systemctl restart "${services[@]}"

step "Caddy site"
install -d "$RA_CADDY_DIR"
if [ "$RA_CADDY" = system ]; then
  render deploy/templates/Caddyfile "$RA_CADDY_SITE"
  # The package's own Caddyfile serves a placeholder on :80; replace it with
  # one that imports the sites. A Caddyfile somebody has written is kept and
  # only gains the import.
  if [ ! -f /etc/caddy/Caddyfile ] || grep -q 'The Caddyfile is an easy way to configure your Caddy web server' /etc/caddy/Caddyfile; then
    printf '# Sites live in %s/, one file each.\nimport %s/*.caddy\n' "$RA_CADDY_DIR" "$RA_CADDY_DIR" > /etc/caddy/Caddyfile
  elif ! grep -qF "import $RA_CADDY_DIR/*.caddy" /etc/caddy/Caddyfile; then
    printf '\nimport %s/*.caddy\n' "$RA_CADDY_DIR" >> /etc/caddy/Caddyfile
  fi
  caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
  systemctl enable caddy
  systemctl reload-or-restart caddy
else
  render deploy/templates/Caddyfile "$RA_CADDY_SITE"
  chmod 0644 "$RA_CADDY_SITE"
  if [ -n "$RA_CADDY_RELOAD" ]; then
    echo "site block in $RA_CADDY_SITE; reloading: $RA_CADDY_RELOAD"
    bash -c "$RA_CADDY_RELOAD"
  else
    echo "RA_CADDY=external: the site block is in $RA_CADDY_SITE —"
    echo "include it in the running Caddy's configuration and reload it there."
  fi
fi

step "First admin"
count=$(cd server && RA_SERVER_DB="$RA_DB" as_user "$RA_NODE" bin/rail-across-server.mjs user-count)
if [ "$count" = 0 ]; then
  pwfile="$RA_DATA/admin-start-password.txt"
  pw=$(python3 -c 'import secrets; print(secrets.token_urlsafe(15))')
  (cd server && RA_SERVER_DB="$RA_DB" RA_ADMIN_PASSWORD="$pw" as_user "$RA_NODE" bin/rail-across-server.mjs create-admin "$RA_ADMIN_LOGIN")
  umask 077
  printf 'login: %s\nstart password: %s\n(to be changed at the first sign-in; delete this file afterwards)\n' \
    "$RA_ADMIN_LOGIN" "$pw" > "$pwfile"
  chown "$RA_USER" "$pwfile"
  echo "start password for \"$RA_ADMIN_LOGIN\" in $pwfile"
else
  echo "$count user(s) exist — no admin created"
fi

"$RA_REPO/deploy/deploy.sh" --config "$RA_CONFIG" --check-only
echo
echo "Done: https://$RA_DOMAIN/"
