#!/usr/bin/env bash
# verify-served-shell.sh — the boot-through-doorway gate (spec 2026-09-08
# epr-app-deliverability-through-doorway, D4b/D4c). HARD FAILURE, never UNSTABLE.
#
# Asks a doorway for the page a visitor gets at MOUNT and proves, through that same
# doorway, that the page can boot:
#   1. every script/stylesheet the page names answers 200 through this doorway;
#   2. the entry script the page names is the one the declared browser head holds
#      (/apps/<head>/index.html), so the shell and the assets are the SAME build;
#   3. when the head's bundle carries version.json, the served /version.json is
#      byte-identical to the head's copy (the 2026-09-06 "stale but 200" shape).
# Polls up to DEADLINE_SECS (default 90 = 2 doorway reconcile ticks + slack) so a
# doorway that is converging passes; a doorway that never converges fails naming
# the doorway, the head, the asset and the x-elohim-bundle marker it served.
#
# usage: verify-served-shell.sh <doorway-url> <mount-path> <slug> <expected-browser-head>
#   e.g. verify-served-shell.sh https://elohim.host / elohim-host-landing sha256-e0e2f7…
set -u
DOORWAY="${1:?doorway url}"; MOUNT="${2:?mount path}"; SLUG="${3:?slug}"; HEAD_HASH="${4:?expected browser head}"
DEADLINE_SECS="${DEADLINE_SECS:-90}"
CURL="curl -sS -m 15 -L"
# `auto` = read the declared browser head from the doorway's own content row (the edge
# pipeline authors no head; it verifies whatever storage declares is what the doorway serves).
if [ "$HEAD_HASH" = "auto" ]; then
  HEAD_HASH=$($CURL "$DOORWAY/db/content/$SLUG" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("blobHash") or "")' 2>/dev/null || true)
  [ -n "$HEAD_HASH" ] || { echo "✗ [$SLUG] $DOORWAY declares no browser head (/db/content/$SLUG has no blobHash)" >&2; exit 2; }
fi
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

assets_of() { # <html file> -> asset paths (script src + stylesheet href), one per line, root-relative
  grep -oE '(src|href)="[^"]+\.(js|css)(\?[^"]*)?"' "$1" | sed -E 's/^[a-z]+="//; s/"$//' \
    | grep -vE '^(https?:)?//' | sed -E 's#^\./##' | sort -u
}
entry_of() { grep -oE 'main-[A-Za-z0-9]+\.js' "$1" | head -1; }
mount_prefix() { case "$MOUNT" in /) echo "";; *) echo "${MOUNT%/}";; esac; }

# The head's own index — what the shell MUST match. Fetched once; a miss here is a
# storage-side fault (the authored head is not servable at all) and fails immediately.
if ! $CURL -o "$tmp/head.html" -w '%{http_code}' "$DOORWAY/apps/$HEAD_HASH/index.html" | grep -q '^200$'; then
  echo "✗ [$SLUG] $DOORWAY cannot serve the declared head's own index: /apps/$HEAD_HASH/index.html" >&2; exit 2
fi
HEAD_ENTRY=$(entry_of "$tmp/head.html")
[ -n "$HEAD_ENTRY" ] || { echo "✗ [$SLUG] declared head $HEAD_HASH names no main-*.js entry script" >&2; exit 2; }
head_has_version=0
$CURL -o "$tmp/head.version.json" -w '%{http_code}' "$DOORWAY/apps/$HEAD_HASH/version.json" | grep -q '^200$' && head_has_version=1

deadline=$(( $(date +%s) + DEADLINE_SECS )); attempt=0; last=""
while :; do
  attempt=$((attempt+1)); fail=""
  code=$($CURL -D "$tmp/hdr" -o "$tmp/page.html" -w '%{http_code}' "$DOORWAY$MOUNT" || echo 000)
  marker=$(grep -i '^x-elohim-bundle:' "$tmp/hdr" | tr -d '\r' | cut -d' ' -f2- | head -1)
  if [ "$code" != "200" ]; then fail="mount $MOUNT answered $code"
  else
    entry=$(entry_of "$tmp/page.html")
    if [ "$entry" != "$HEAD_ENTRY" ]; then
      fail="page names entry ${entry:-<none>} but the declared head $HEAD_HASH holds $HEAD_ENTRY"
    else
      prefix=$(mount_prefix)
      while IFS= read -r a; do
        [ -n "$a" ] || continue
        case "$a" in /*) url="$DOORWAY$a";; *) url="$DOORWAY$prefix/$a";; esac
        ac=$($CURL -o /dev/null -w '%{http_code}' "$url" || echo 000)
        [ "$ac" = "200" ] || { fail="asset $a answered $ac through this doorway"; break; }
      done < <(assets_of "$tmp/page.html")
      if [ -z "$fail" ] && [ "$head_has_version" = 1 ]; then
        $CURL -o "$tmp/served.version.json" -w '%{http_code}' "$DOORWAY$prefix/version.json" | grep -q '^200$' \
          && cmp -s "$tmp/head.version.json" "$tmp/served.version.json" \
          || fail="served ${prefix}/version.json is not the declared head's stamp"
      fi
    fi
  fi
  if [ -z "$fail" ]; then
    echo "✓ [$SLUG] $DOORWAY$MOUNT boots at head $HEAD_HASH (entry $HEAD_ENTRY, x-elohim-bundle: ${marker:-confirmed}) after $attempt attempt(s)"
    exit 0
  fi
  last="$fail (x-elohim-bundle: ${marker:-<none>})"
  if [ "$(date +%s)" -ge "$deadline" ]; then break; fi
  sleep 10
done
echo "✗ [$SLUG] $DOORWAY$MOUNT does NOT boot after ${DEADLINE_SECS}s: $last" >&2
echo "   declared browser head: $HEAD_HASH (entry $HEAD_ENTRY). A visitor to this doorway gets a page that cannot run." >&2
exit 1
