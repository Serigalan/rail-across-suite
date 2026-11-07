#!/bin/sh
# Rail-Across-Suite, local package: starts the project server on 127.0.0.1 with a
# free port, without user management, serving the built suite in app/ beside the
# API — and opens the browser on it. The database lies in ./data/ beside this
# script. Uses the bundled Node.js (node/bin/node); only if that is missing,
# Node.js 22 or later on PATH.
cd "$(dirname "$0")"
mkdir -p data
: > data/server.log
if [ -x node/bin/node ]; then NODE=node/bin/node; else NODE=node; fi
RA_SERVER_DB=data/rail-across.sqlite RA_SERVER_CLOUDS=data/clouds \
  "$NODE" server/bin/rail-across-server.mjs serve --local --static app --host 127.0.0.1 --port 0 \
  > data/server.log 2>&1 &
SRV=$!
trap 'kill $SRV 2>/dev/null' EXIT INT TERM
URL=''
i=0
while [ $i -lt 150 ]; do
  URL=$(grep -o 'RA_DESKTOP_URL=.*' data/server.log 2>/dev/null | head -1 | cut -d= -f2)
  [ -n "$URL" ] && break
  kill -0 $SRV 2>/dev/null || { echo "the server did not start — see data/server.log" >&2; exit 1; }
  sleep 0.2; i=$((i + 1))
done
[ -n "$URL" ] || { echo "no address from the server — see data/server.log" >&2; exit 1; }
echo "Rail-Across-Suite (local package) on $URL"
( open "$URL" 2>/dev/null || xdg-open "$URL" 2>/dev/null ) &
wait $SRV
