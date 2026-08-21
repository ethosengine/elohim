#!/bin/bash
#
# hc-mesh-prologue.sh - Act I Prologue cast for an ALREADY-RUNNING hc-mesh
#
# Seeds the household-shaped substrate the Act I a2o scenarios (features/**
# tagged @act:i) actually need: named conductor identities (the cast fix —
# see genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md), the
# landing + lamad-spa bundles (browser AND the landing's SSR server bundle),
# the full CI-order substrate seed chain, and the household fixture manifest
# a2o's `src/framework/fixtures/household-mesh.ts` resolves against.
#
# This script NEVER starts, stops, or restarts a mesh component — it only
# seeds one that `just mesh start` (or `hc-mesh.sh start`) already brought
# up. Run it any number of times; every leg is idempotent by construction
# (the seeders self-skip on "already exists", stage-spa-blob.sh's PATCH is a
# content-addressed re-PATCH, and DECLARE_ONLY re-declares the same head).
#
# USAGE:
#   ./hc-mesh-prologue.sh
#   ./hc-mesh.sh prologue        # equivalent, via hc-mesh.sh's action switch
#
# Two blocking preconditions (checked up front, exit 2 if either is unmet):
#   - both doorways answer /health (the mesh must already be running)
#   - the browser/server dist directories exist (build the apps first)
#
# Beyond that, every leg runs and reports EXIT[<leg>]=<code> regardless of
# whether an earlier leg failed — a dev tool that stops at the first red
# hides every leg after it. Only a handful of legs are MUST-SUCCEED (the
# household cannot exist without them): the two seed.ts calls, the three
# bundle-staging PATCHes, seed-humans, seed-conductor-identities,
# seed-household-formation, and the fixture-manifest write. Everything else
# mirrors genesis/Jenkinsfile's own posture for these legs — UNSTABLE-grade,
# reported, non-blocking (the "partial-cluster principle: household is
# recoverable" comment on runProbedSeeder()). The script's own exit code is
# non-zero iff a MUST-SUCCEED leg failed; see the summary at the end.
#
# ENVIRONMENT: inherits hc-mesh.sh's MESH_PEERS / MESH_DIR / DOORWAY_PORT /
# MESH_API_KEY_ADMIN. DOORWAY_B_PORT (default 8889) matches hc-mesh.sh's own
# default (not otherwise exported by that script). MESH_DECLARE_MAX_ATTEMPTS
# bounds the canonical-head propagation retry ladder (default 6 — a loopback
# mesh's DHT publish lag is seconds, not the cross-conductor production
# window stage-spa-blob.sh's default 12 attempts is sized for).
#
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# shellcheck source=./hc-mesh.sh
source "$SCRIPT_DIR/hc-mesh.sh"   # guarded dispatch — sourcing never starts/stops the mesh
mesh_seed_env                     # CONDUCTOR_URLS, DOORWAY_URL(=A), STORAGE_URL, PEER_STORAGE_URLS,
                                   # SEEDER_TARGET_PEERS, API_KEY_ADMIN, DOORWAY_API_KEY, STORAGE_API_KEY_ADMIN

DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
DOORWAY_A_URL="$DOORWAY_URL"
DOORWAY_B_URL="http://localhost:$DOORWAY_B_PORT"
STAGE_BLOB_SCRIPT="$REPO_ROOT/scripts/ci/stage-spa-blob.sh"
SEEDER_DIR="$REPO_ROOT/genesis/seeder"
FIXTURE_PATH="$MESH_DIR/household-fixture.json"

banner() { printf '\n=== prologue: %s ===\n' "$1"; }
say()    { printf 'prologue[%s]: %s\n' "$(date -u +%H:%M:%SZ)" "$1"; }

MUST_FAILED=0
declare -A LEG_RC=()

# run_leg <name> <must: hard|soft> <command...>
# Runs the command as given (env-var prefix words in the argv are honored by
# bash as ordinary simple-command assignments). Never aborts the script on
# failure — every leg after a red one still runs, so a dev iterating on the
# mesh sees the WHOLE picture in one pass.
run_leg() {
  local name="$1" must="$2"; shift 2
  echo ""
  echo "── leg: $name ──"
  "$@"
  local rc=$?
  LEG_RC["$name"]=$rc
  echo "EXIT[$name]=$rc"
  if [ "$rc" -ne 0 ] && [ "$must" = "hard" ]; then
    MUST_FAILED=1
    echo "  ^ MUST-SUCCEED leg failed — the prologue's overall exit code will be non-zero." >&2
  fi
  return $rc
}

# run_seed_leg <name> <must: hard|soft> <shell-snippet>
# Same contract, but for genesis/seeder legs: runs the snippet in a subshell
# cd'd into genesis/seeder (eval, not exec — snippets are internally
# constructed VAR="$OTHER_VAR" prefixes, never external input).
run_seed_leg() {
  local name="$1" must="$2" snippet="$3"
  echo ""
  echo "── leg: $name ──"
  ( cd "$SEEDER_DIR" && eval "$snippet" )
  local rc=$?
  LEG_RC["$name"]=$rc
  echo "EXIT[$name]=$rc"
  if [ "$rc" -ne 0 ] && [ "$must" = "hard" ]; then
    MUST_FAILED=1
    echo "  ^ MUST-SUCCEED leg failed — the prologue's overall exit code will be non-zero." >&2
  fi
  return $rc
}

# ---------------------------------------------------------------------------
# Preflight — the two blocking preconditions. Fail loud and fast; nothing
# below can meaningfully run without them, and neither is this script's job
# to fix (it never starts a mesh component or builds an app).
# ---------------------------------------------------------------------------
banner "preflight"
for pair in "A|$DOORWAY_A_URL" "B|$DOORWAY_B_URL"; do
  label="${pair%%|*}"; url="${pair#*|}"
  if ! curl -s -m 3 "$url/health" >/dev/null; then
    echo "FATAL: doorway $label ($url) is not reachable — run 'just mesh start' first. This script never starts/stops/restarts mesh components." >&2
    exit 2
  fi
done
say "doorway A ($DOORWAY_A_URL) and doorway B ($DOORWAY_B_URL) both healthy"

LANDING_BROWSER_DIST="$REPO_ROOT/app/elohim-app/dist/elohim-app/browser"
LANDING_SERVER_DIST="$REPO_ROOT/app/elohim-app/dist/elohim-app/server"
LAMAD_BROWSER_DIST="$REPO_ROOT/app/lamad/dist/lamad/browser"
for d in "$LANDING_BROWSER_DIST" "$LANDING_SERVER_DIST" "$LAMAD_BROWSER_DIST"; do
  if [ ! -d "$d" ]; then
    echo "FATAL: missing dist dir: $d — build the apps first (cd app/elohim-app && pnpm build; cd app/lamad && pnpm build)." >&2
    exit 2
  fi
done
say "landing browser/server + lamad-spa browser dist dirs all present"
say "CONDUCTOR_URLS=$CONDUCTOR_URLS"

# ---------------------------------------------------------------------------
# 1. seed.ts — the base fixture corpus, via BOTH doorways. Via A because A is
#    the authoring doorway for everything else in this script; via B too
#    because jessica (doorway B's primary) has NO row on A's storage backend
#    otherwise — jessica's own scenarios need her doorway to have seeded her.
# ---------------------------------------------------------------------------
banner "1. seed.ts (base corpus) via A and B"
run_seed_leg "seed-base-corpus-via-A" hard \
  'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed.ts --ids=elohim-host-landing'
run_seed_leg "seed-base-corpus-via-B" hard \
  'DOORWAY_URL="$DOORWAY_B_URL" npx tsx src/seed.ts --ids=elohim-host-landing'

# ---------------------------------------------------------------------------
# 2-3. Operator bindings + projections — explicit, ahead of the substrate
#    chain: `seed.ts --ids=...` skips both (profiles.md §2 "the seed chain
#    never explicitly ran them"), and the doorway EPR router resolves ZERO
#    projections without seed-projections, which sheds `/`.
# ---------------------------------------------------------------------------
banner "2-3. operator bindings + projections"
run_seed_leg "seed-operator-bindings" soft 'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-operator-bindings.ts'
run_seed_leg "seed-projections"       soft 'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-projections.ts'

# ---------------------------------------------------------------------------
# 4. Stage bundles — the notarized heads. All three PATCH through A (the
#    single conductor-bridged authoring path — see stage-spa-blob.sh's own
#    "single notarized head" comment), then DECLARE_ONLY propagates each
#    resulting head to B so B doesn't wait on gossip alone (the backlog
#    finding: "gossip alone left a head un-adopted >8 min behind a 3,400-head
#    seed"). MESH_DECLARE_MAX_ATTEMPTS bounds the ladder for a loopback mesh.
# ---------------------------------------------------------------------------
banner "4. stage bundles (landing browser+server, lamad-spa browser) via A; DECLARE_ONLY propagate to B"
DECLARE_MAX_ATTEMPTS="${MESH_DECLARE_MAX_ATTEMPTS:-6}"
export DECLARE_MAX_ATTEMPTS

run_leg "stage-landing-browser-A" hard \
  env DO_PATCH=1 STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LANDING_BROWSER_DIST" elohim-host-landing "$DOORWAY_A_URL" browser
run_leg "propagate-landing-to-B-after-browser" soft \
  env DECLARE_ONLY=1 SOURCE_DOORWAY_URL="$DOORWAY_A_URL" STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LANDING_BROWSER_DIST" elohim-host-landing "$DOORWAY_B_URL" browser

run_leg "stage-lamad-spa-browser-A" hard \
  env DO_PATCH=1 STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LAMAD_BROWSER_DIST" lamad-spa "$DOORWAY_A_URL" browser
run_leg "propagate-lamad-spa-to-B" soft \
  env DECLARE_ONLY=1 SOURCE_DOORWAY_URL="$DOORWAY_A_URL" STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LAMAD_BROWSER_DIST" lamad-spa "$DOORWAY_B_URL" browser

run_leg "stage-landing-server-A" hard \
  env DO_PATCH=1 STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LANDING_SERVER_DIST" elohim-host-landing "$DOORWAY_A_URL" server
run_leg "propagate-landing-to-B-after-server" soft \
  env DECLARE_ONLY=1 SOURCE_DOORWAY_URL="$DOORWAY_A_URL" STORAGE_API_KEY_ADMIN="$API_KEY_ADMIN" \
  bash "$STAGE_BLOB_SCRIPT" "$LANDING_SERVER_DIST" elohim-host-landing "$DOORWAY_B_URL" server

# Read back the server bundle's hash/size — CI exports these from its own
# upload stage (Jenkinsfile env.CONTENT_BLOB_HASH/SIZE_BYTES); this chain
# never did (the backlog gap this Prologue exists to close). Read-back, not
# re-derivation: the landing row's serverBlobHash IS the value the PATCH
# above just wrote, and /blob/<hash> content-length is the authoritative
# byte count (curl -w size_download, never a local `du` estimate).
CONTENT_BLOB_HASH=""
CONTENT_BLOB_SIZE_BYTES=""
if [ "${LEG_RC[stage-landing-server-A]:-1}" -eq 0 ]; then
  CONTENT_BLOB_HASH="$(curl -fsS "$DOORWAY_A_URL/db/content/elohim-host-landing" \
    | python3 -c "import sys, json; print(json.load(sys.stdin).get('serverBlobHash') or '')" 2>/dev/null)"
  if [ -n "$CONTENT_BLOB_HASH" ]; then
    CONTENT_BLOB_SIZE_BYTES="$(curl -fsS -o /dev/null -w '%{size_download}' "$DOORWAY_A_URL/blob/$CONTENT_BLOB_HASH" 2>/dev/null)"
  fi
fi
if [ -n "$CONTENT_BLOB_HASH" ] && [ -n "$CONTENT_BLOB_SIZE_BYTES" ]; then
  export CONTENT_BLOB_HASH CONTENT_BLOB_SIZE_BYTES
  say "CONTENT_BLOB_HASH=$CONTENT_BLOB_HASH CONTENT_BLOB_SIZE_BYTES=$CONTENT_BLOB_SIZE_BYTES"
else
  say "WARN: could not resolve CONTENT_BLOB_HASH/SIZE from the landing row's serverBlobHash — seed-household-formation and seed-commitments will self-skip their custody legs (they self-skip cleanly on an absent CONTENT_BLOB_HASH; this is not a hard failure of THIS script)"
fi

# ---------------------------------------------------------------------------
# 5. Substrate chain. Conductor Identities now runs BEFORE Seed Humans — the
#    reverse of genesis/Jenkinsfile's CI order (Seed Humans -> Presences ->
#    Collectives -> Accounts -> Conductor Identities -> Provide Rows -> Agent
#    Peer Bindings -> Household Formation -> Stewardship -> EPR Atoms ->
#    [blob authored above] -> Custody Commitments -> operator bindings/
#    projections [already run in step 2-3]). On this mesh doorway A's hosted
#    pool IS matthew's own conductor (4445): Seed Humans' hosted
#    `/auth/register` call for matthew (agencyPhase=doorway) passes no
#    human_id, so doorway mints a random UUID Human on that SAME conductor
#    (auth_routes.rs "minting a random UUID" warning) — if that runs first,
#    it embodies the conductor before Seed Conductor Identities gets a
#    chance to, and the founder cast reports `[C] Conflict` instead of
#    `[+]`/`[=]`. seed-conductor-identities.ts reads its human roster
#    directly from data/humans/humans.json (never from doorway or from Seed
#    Humans' output), so casting it first has no dependency to break; running
#    it first means each household conductor embodies its OWN canonical
#    `human-<name>` id before any hosted registration can mint a UUID onto
#    it. Seed Humans' own registration for matthew then hits the "Agent
#    already has a Human profile" recovery branch (auth_routes.rs) and
#    reports `exists` against the correct id instead of conflicting.
#    seed-nodes and seed-delegates-compute are both mesh-local levers, not
#    wired into CI (see their own file-header comments), included here
#    because the a2o Act I lane needs stewarded nodes and a bounded
#    delegates-compute grant that only exist on THIS mesh if something seeds
#    them.
# ---------------------------------------------------------------------------
banner "5. substrate chain (Conductor Identities cast BEFORE Seed Humans — see comment above)"
run_seed_leg "seed-conductor-identities" hard \
  'CONDUCTOR_URLS="$CONDUCTOR_URLS" npx tsx src/seed-conductor-identities.ts'
run_seed_leg "seed-humans" hard \
  'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-humans.ts'
run_seed_leg "seed-presences" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-presences.ts'
run_seed_leg "seed-collectives" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-collectives.ts'
run_seed_leg "seed-accounts" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" SEEDER_TARGET_PEERS="$SEEDER_TARGET_PEERS" npx tsx src/seed-accounts.ts'
run_seed_leg "seed-provide-rows" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" PEER_STORAGE_URLS="$PEER_STORAGE_URLS" npx tsx src/seed-provide-rows.ts'
run_seed_leg "seed-agent-bindings" soft \
  'CONDUCTOR_URLS="$CONDUCTOR_URLS" npx tsx src/seed-agent-bindings.ts'
run_seed_leg "seed-household-formation" hard \
  'CONDUCTOR_URLS="$CONDUCTOR_URLS" STORAGE_URL="$STORAGE_URL" DOORWAY_URL="$DOORWAY_A_URL" CONTENT_BLOB_HASH="${CONTENT_BLOB_HASH:-}" CONTENT_BLOB_SIZE_BYTES="${CONTENT_BLOB_SIZE_BYTES:-}" npx tsx src/seed-household-formation.ts'
run_seed_leg "seed-stewardship" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" npx tsx src/seed-stewardship.ts'
run_seed_leg "seed-epr-atom" soft \
  'STORAGE_URL="$STORAGE_URL" npx tsx src/seed-epr-atom.ts'
run_seed_leg "seed-commitments" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" CONTENT_BLOB_HASH="${CONTENT_BLOB_HASH:-}" CONTENT_BLOB_SIZE_BYTES="${CONTENT_BLOB_SIZE_BYTES:-}" npx tsx src/seed-commitments.ts'
# Act I's own cast, seeded AFTER the substrate chain because every leg below
# reads what that chain wrote (humans + household + the staged bundle blobs).
#
# 1. Drill fixtures: `heal-target` and `chaos-ladder` are named by
#    features/resilience/{app-blob-heal-on-read,chaos-peer-churn}.feature and
#    created by nothing else on this mesh; their steps say so in as many words
#    ("is not seeded on this mesh, so the heal-on-read drill has nothing to
#    break"). The leg also brings the existing commons EPR (`manifesto`) under
#    household custody when — and only when — its declared bytes can be
#    RECOVERED from a peer or proved from its own contentBody.
# 2. Their custody promises, authored through seed-commitments.ts so the
#    `custody-blob` wire contract lives in exactly one place. Skipped cleanly
#    when leg 1 wrote no pairs (an empty promise set is worse than none: the
#    chaos drill would kill peers to watch nothing).
# 3. The co-steward agreement: on alpha chapter 5 authors a `replicates-commons`
#    commitment at RUN time for its own e2e content, with adam as co-steward.
#    Nothing plays that part here, so Act I casts jessica co-stewarding the
#    landing EPR — the pledge saga ch09's `commonsCommitments` counts and the
#    distribution round its `stewardingCollectives` scope depends on.
# mkdir before use: step 6 creates MESH_DIR, and this leg runs before it.
mkdir -p "$MESH_DIR"
DRILL_PAIRS_PATH="$MESH_DIR/drill-custody-pairs.json"
rm -f "$DRILL_PAIRS_PATH"
export DRILL_PAIRS_PATH

run_seed_leg "seed-drill-fixtures" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" DOORWAY_B_URL="$DOORWAY_B_URL" STORAGE_URL="$STORAGE_URL" PEER_STORAGE_URLS="$PEER_STORAGE_URLS" DRILL_CUSTODY_PAIRS_OUT="$DRILL_PAIRS_PATH" npx tsx src/seed-drill-fixtures.ts'
run_seed_leg "seed-drill-custody" soft \
  'if [ -s "$DRILL_PAIRS_PATH" ]; then DOORWAY_URL="$DOORWAY_A_URL" CUSTODY_PAIRS_JSON="$DRILL_PAIRS_PATH" npx tsx src/seed-commitments.ts; else echo "no drill custody pairs written (leg 1 found no seedable fixture) — nothing to promise"; fi'
run_seed_leg "seed-household-costeward" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" DOORWAY_B_URL="$DOORWAY_B_URL" STORAGE_URL="$STORAGE_URL" PEER_STORAGE_URLS="$PEER_STORAGE_URLS" npx tsx src/seed-household-costeward.ts'

run_seed_leg "seed-nodes" soft \
  'STORAGE_URL="$STORAGE_URL" npx tsx src/seed-nodes.ts'
run_seed_leg "seed-delegates-compute" soft \
  'STORAGE_URL="$STORAGE_URL" npx tsx src/seed-delegates-compute.ts'

# ---------------------------------------------------------------------------
# 6. Household fixture manifest — the a2o framework's single authority for
#    live household-mesh acceptance fixtures
#    (genesis/a2o/src/framework/fixtures/household-mesh.ts). processControl
#    is the whole point: without it, requireFixturePeerPid/
#    requireFixtureDoorwayLogPath fail closed and the kill-a-peer / tail-the-
#    log families can never be honestly green.
# ---------------------------------------------------------------------------
banner "6. household fixture manifest"
mkdir -p "$MESH_DIR"
PEERS_TSV=""
i=0
for name in "${PEERS[@]}"; do
  port="$(http_port $i)"
  pid="$(pgrep -f "elohim-storag[e].*--http-port $port" | head -1)"
  PEERS_TSV+="${name}|${port}|${pid}|${LOGDIR}/${name}.log"$'\n'
  i=$((i+1))
done
export PROLOGUE_PEERS_TSV="$PEERS_TSV"
export PROLOGUE_MESH_DIR="$MESH_DIR"
export PROLOGUE_DOORWAY_A_PORT="$DOORWAY_PORT"
export PROLOGUE_DOORWAY_B_PORT="$DOORWAY_B_PORT"
export PROLOGUE_LOGDIR="$LOGDIR"

python3 - > "$FIXTURE_PATH" <<'PYEOF'
import json, os, sys

peers_raw = os.environ.get("PROLOGUE_PEERS_TSV", "")
doorway_a_port = os.environ["PROLOGUE_DOORWAY_A_PORT"]
doorway_b_port = os.environ["PROLOGUE_DOORWAY_B_PORT"]
logdir = os.environ["PROLOGUE_LOGDIR"]

storage_peers = {}
pool_urls = []
for line in peers_raw.splitlines():
    if not line:
        continue
    name, port, pid, log = line.split("|")
    url = f"http://localhost:{port}"
    entry = {"url": url, "logPath": log}
    if pid.strip():
        entry["pid"] = int(pid.strip())
    storage_peers[name] = entry
    pool_urls.append(url)

primary_a = pool_urls[0] if pool_urls else None
primary_b = pool_urls[1] if len(pool_urls) > 1 else primary_a

fixture = {
    "$comment": "Emitted by hc-mesh-prologue.sh (just mesh prologue). Act I mesh owns its substrate.",
    "commonsEprId": "elohim-host-landing",
    "convergenceWindowMs": 60000,
    "connectedPeersFloor": 2,
    "processControl": True,
    "processControlReason": "Act I mesh — peers and doorways run as processes on this host",
    "doorways": {
        "alpha": {
            "url": f"http://localhost:{doorway_a_port}",
            "primaryStorageUrl": primary_a,
            "poolStorageUrls": pool_urls,
            "logPath": f"{logdir}/doorway.log",
        },
        "beta": {
            "url": f"http://localhost:{doorway_b_port}",
            "primaryStorageUrl": primary_b,
            "poolStorageUrls": pool_urls,
            "logPath": f"{logdir}/doorway-b.log",
        },
        "apex": {
            "url": f"http://localhost:{doorway_b_port}",
            "primaryStorageUrl": primary_b,
        },
        "gamma": {
            "absentReason": "Act I mesh stages two doorways (A/alpha and B/beta=apex); a scenario naming a third must say which act it needs",
        },
    },
    "storagePeers": storage_peers,
}
json.dump(fixture, sys.stdout, indent=2)
print()
PYEOF
fixture_rc=$?
echo "EXIT[write-household-fixture]=$fixture_rc"
if [ "$fixture_rc" -ne 0 ]; then
  MUST_FAILED=1
  echo "  ^ MUST-SUCCEED leg failed — the prologue's overall exit code will be non-zero." >&2
fi
say "household fixture manifest written: $FIXTURE_PATH"

# ---------------------------------------------------------------------------
# a2o env block — the vars the mesh profile (genesis/a2o/layering/profiles.md
# §mesh profile) reads. E2E_DOORWAY_APEX is deliberately NOT printed here:
# the fixture file above already declares doorways.apex, and
# fixtureDoorwayUrl() resolves it from the manifest — a printed env var would
# just duplicate declared knowledge.
# ---------------------------------------------------------------------------
banner "a2o env"
POOL_CSV="$(peer_url_csv)"
echo "export E2E_DOORWAY_ALPHA=\"$DOORWAY_A_URL\""
echo "export E2E_DOORWAY_B=\"$DOORWAY_B_URL\""
echo "export E2E_DOORWAY_BETA=\"$DOORWAY_B_URL\""
echo "export E2E_STORAGE_URL=\"$STORAGE_URL\""
i=0
for name in "${PEERS[@]}"; do
  if [ "$i" -eq 1 ]; then
    echo "export E2E_STORAGE_B=\"http://localhost:$(http_port $i)\""
  fi
  echo "export E2E_STORAGE_$(echo "$name" | tr '[:lower:]' '[:upper:]')=\"http://localhost:$(http_port $i)\""
  i=$((i+1))
done
echo "export E2E_DOORWAY_POOL_STORAGE_URLS=\"$POOL_CSV\""
echo "export E2E_HOUSEHOLD_FIXTURE_PATH=\"$FIXTURE_PATH\""
echo "export ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=\"$REPO_ROOT/genesis/manifests/cluster-state.act1-household.yaml\""
echo "export ELOHIM_REMOTE_COMPUTE_STATUS=\"unavailable\""
# One JSON report per run: cucumber MERGES the config's format list with any --format flag, so a
# stage that names its own report otherwise gets a second byte-identical copy at the generic path
# (genesis/a2o/cucumber.mjs, jsonReportPath note). Same value `just test mesh` exports.
mkdir -p "$MESH_DIR/reports"
echo "export CUCUMBER_JSON_REPORT=\"$MESH_DIR/reports/mesh.json\""

# ---------------------------------------------------------------------------
# Summary.
# ---------------------------------------------------------------------------
banner "summary"
for name in "${!LEG_RC[@]}"; do
  printf '  %-38s EXIT=%s\n' "$name" "${LEG_RC[$name]}"
done | sort
if [ "$MUST_FAILED" -eq 1 ]; then
  echo ""
  echo "PROLOGUE: at least one MUST-SUCCEED leg failed — the household is not fully cast. See the leg output above."
  exit 1
fi
echo ""
echo "PROLOGUE: complete — all MUST-SUCCEED legs green (soft legs may still show non-zero above; that mirrors genesis/Jenkinsfile's own UNSTABLE-grade posture for those same seeders)."
exit 0
