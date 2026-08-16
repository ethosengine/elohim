#!/bin/bash
# run-mesh-quiesce-stage.sh — the hc-mesh-in-CI quiesce measure.
#
# THE POINT (minutes-quiesce plan §3 W4.2, task Q9:
# genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md):
# a definitive, minutes-scale, PER-COMMIT measure of the protocol's convergence
# machinery that OWNS ITS OWN MESH. Because the mesh is stood up by this stage
# and torn down by it, the DID-NOT-MEASURE class cannot exist here: there is no
# shared fleet whose churn can eat the window. Either the fleet-quiesce gate
# reaches sustained PASS inside its deadline (the stage passes and reports a
# rate) or it does not (the stage FAILS — churning IS the regression signal).
#
# Contrast with the alpha `Dataplane Validation` stage: that one measures a
# SHARED, deployed fleet and legitimately no-measures when the fleet is in
# post-deploy churn. This stage measures the same machinery on a mesh it
# controls, so it is allowed to be strict.
#
# ── TWO PHASES, TWO SEMANTICS (operator, 2026-08-16) ────────────────────────
#   PHASE 1 · QUIESCE (bootstrap)   fresh mesh -> seed under the declared grant
#     -> stage the landing blob -> fleet-quiesce gate to sustained PASS.
#     Its writes are the SETUP. Verdict is HARD: `MESH-QUIESCE: PASS rate=…`
#     / a failed build. Churn IS the regression signal.
#   PHASE 2 · E2E (runtime validation)   runs ONLY after phase 1 PASSes, on the
#     quiesced substrate: `cucumber-js --profile saga` (26 scenarios). Its
#     writes ARE the subject (agent actions flowing through the system), so it
#     must NOT re-assert global quiesce metrics. Verdict is UNSTABLE-grade:
#     `MESH-E2E: n/26 scenarios`, reported, non-blocking (see the step-debt
#     note at the phase-2 block).
#
# CALIBRATION (definitive local run, 2026-08-16, 3-peer hc-mesh, dev-tier
# pacing profile — reconcile 30s / sustain 33s):
#   mesh bring-up ~3min · seed 11s · quiesce-to-sustained-PASS 3m24s
#   => 1007 rows/min. Sprint target: >= 344 rows/min (§5).
# Stage hard budget: 15 min AFTER the binaries exist (MESH_STAGE_BUDGET_SECS).
#
# ── REUSE, NEVER REBUILD ────────────────────────────────────────────────────
# The edge pipeline already builds every binary this mesh needs, as container
# images, earlier in the same run:
#   elohim-storage:<GIT_COMMIT_HASH>  -> /usr/local/bin/elohim-storage
#                                     -> /usr/local/bin/holochain (the SHIPPING
#                                        conductor, embedded-conductor mode)
#   elohim-doorway:<GIT_COMMIT_HASH>  -> /usr/local/bin/doorway
# and `Build Edge Node Image` has already fetched elohim.happ from the DNA
# pipeline's Harbor artifact. This script EXTRACTS those binaries rather than
# compiling anything — a second cargo build of elohim-storage would cost ~5min
# of the 15min budget and would measure a DIFFERENT binary than the one the
# pipeline is about to ship. That is why the stage is placed after
# `Build hApp Installer` and before `Push to Harbor`/`Deploy`.
#
# ── ENVIRONMENT HONESTY (capability gate) ───────────────────────────────────
# The edge pipeline's `builder` container is ci-builder-nix (node:24-bookworm +
# nix + sccache). It has NO `hc` and NO `holochain` on PATH, and the edge pod
# does NOT mount the nix-store PVC nor run `nix develop` (only the DNA pipeline
# does). So the conductor toolchain has to be resolved at stage time:
#   1. `hc` / `holochain` already on PATH (a future ci-builder that bakes them
#      in activates this stage with no edit here);
#   2. `holochain` extracted from the storage image we just built (preferred —
#      it is literally the conductor this commit ships);
#   3. `hc` (the sandbox CLI; no image publishes it) fetched from the in-cluster
#      Nexus github-releases-proxy, the same pinned source the devspace images
#      use (che-devworkspaces containers/udi-plus-mem-rust-nix/Dockerfile).
# If the toolchain still cannot be assembled, the stage prints ONE loud
# SKIPPED-NO-CONDUCTOR line naming exactly what was missing and exits 0. That
# is a declared no-measure, not a silent one — and flipping the image (or the
# Nexus route) activates the stage with no Jenkinsfile change.
#
# Usage: run-mesh-quiesce-stage.sh            (no args; env-driven)
#
# Env knobs (all optional):
#   MESH_STAGE_BUDGET_SECS  overall wall budget after binaries exist (900)
#   MESH_QUIESCE_DEADLINE   gate deadline handed to hc-mesh-quiesce.sh (600)
#   MESH_RATE_TARGET        rows/min target printed beside the measure (344)
#   MESH_DIR                mesh data root (/tmp/elohim-ci-mesh)
#   HC_VERSION              stock `hc` CLI version to fetch (0.6.0)
#   HC_RELEASE_BASE         Nexus github-releases-proxy base URL
#   MESH_STAGE_ALLOW_SKIP   "0" turns the capability skip into a hard failure
#                           (use once the image ships the toolchain)
#   MESH_E2E_BLOCKING       "1" makes the phase-2 saga suite fail the build
#                           (flip once the global-quiesce step-debt is paid)
#   MESH_STAGE_APT          "0" skips the best-effort host-package leg
set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MESH_SCRIPT="$WORKSPACE_ROOT/app/elohim-app/scripts/hc-mesh.sh"
QUIESCE_SCRIPT="$WORKSPACE_ROOT/app/elohim-app/scripts/hc-mesh-quiesce.sh"
STAGE_BLOB_SCRIPT="$WORKSPACE_ROOT/scripts/ci/stage-spa-blob.sh"
HAPP_WORKDIR="$WORKSPACE_ROOT/elohim/holochain/dna/elohim/workdir"

MESH_DIR="${MESH_DIR:-/tmp/elohim-ci-mesh}"
BIN_DIR="$MESH_DIR/bin"
BUDGET_SECS="${MESH_STAGE_BUDGET_SECS:-900}"
GATE_DEADLINE="${MESH_QUIESCE_DEADLINE:-600}"
RATE_TARGET="${MESH_RATE_TARGET:-344}"
HC_VERSION="${HC_VERSION:-0.6.0}"
HC_RELEASE_BASE="${HC_RELEASE_BASE:-https://nexus.ethosengine.com/repository/github-releases-proxy/holochain/holochain/releases/download}"
ALLOW_SKIP="${MESH_STAGE_ALLOW_SKIP:-1}"

DOORWAY_PORT="${DOORWAY_PORT:-8888}"
DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
CONTENT_ID="${MESH_QUIESCE_CONTENT_ID:-elohim-host-landing}"

banner() { printf '\n=== mesh-quiesce: %s ===\n' "$1"; }
say()    { printf 'mesh-quiesce[%s]: %s\n' "$(date -u +%H:%M:%SZ)" "$1"; }

# Loud, single-line, grep-able no-measure. Never silent.
skip_no_conductor() {
    echo ""
    echo "MESH-QUIESCE SKIPPED-NO-CONDUCTOR: $1"
    echo ""
    if [ "$ALLOW_SKIP" = "0" ]; then
        echo "  (MESH_STAGE_ALLOW_SKIP=0 — treating the missing capability as a failure)" >&2
        exit 1
    fi
    exit 0
}

fail() {
    echo ""
    echo "MESH-QUIESCE FAILED: $1"
    echo ""
    exit 1
}

# ---------------------------------------------------------------------------
# Teardown. ALWAYS runs — a leaked conductor/storage/doorway on a shared CI
# agent poisons the next build's port scheme (and the next measure's numbers).
# ---------------------------------------------------------------------------
MESH_STARTED=0
teardown() {
    local rc=$?
    # NEVER tear down a mesh this script did not start. The capability skip and
    # every pre-bring-up failure exit through here too, and on a developer box
    # (or any agent that runs this script to inspect it) an unguarded stop would
    # kill a live local mesh that belongs to somebody else.
    if [ "$MESH_STARTED" != "1" ]; then
        exit "$rc"
    fi
    banner "teardown"
    if [ -x "$MESH_SCRIPT" ]; then
        MESH_DIR="$MESH_DIR" DOORWAY_PORT="$DOORWAY_PORT" DOORWAY_B_PORT="$DOORWAY_B_PORT" \
            bash "$MESH_SCRIPT" stop || true
    fi
    # Belt-and-braces: hc-mesh.sh stop kills by exact binary identity; anything
    # it could not name (a half-generated sandbox) dies here.
    pkill -f "[h]c sandbox" 2>/dev/null || true
    pkill -x holochain 2>/dev/null || true
    say "mesh logs kept at $MESH_DIR/logs (archived by the stage's post block if wired)"
    exit "$rc"
}
trap teardown EXIT

# ---------------------------------------------------------------------------
# Budget. One deadline for the whole measure; every phase gets `timeout` with
# whatever remains, so an early overrun can never silently eat the gate window.
# ---------------------------------------------------------------------------
STAGE_START=$(date +%s)
remaining() {
    local now elapsed
    now=$(date +%s)
    elapsed=$((now - STAGE_START))
    echo $((BUDGET_SECS - elapsed))
}
require_budget() {
    local need="$1" label="$2" left
    left="$(remaining)"
    if [ "$left" -lt "$need" ]; then
        fail "stage budget exhausted before ${label} (${left}s left, ${need}s needed, budget ${BUDGET_SECS}s)"
    fi
}

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 0 — host packages the mesh assumes (all present in the devspace image,
# not all present in ci-builder). Best-effort: a failure here is not fatal by
# itself; the capability probe below is the authority.
#   iproute2     `ss` — hc-mesh.sh's conductor/port readiness probes
#   python3-yaml hc-mesh.sh's k2Gossip config patch (`import yaml`)
#   libsodium23  runtime dep of the extracted elohim-storage/doorway binaries
#   libssl3      ditto
#   procps       pgrep/pkill used by hc-mesh.sh stop and this teardown
#   zip          stage-spa-blob.sh bundle zip (ci-builder already has it)
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 0: host packages"
if [ "${MESH_STAGE_APT:-1}" = "1" ] && command -v apt-get >/dev/null 2>&1; then
    DEBIAN_FRONTEND=noninteractive apt-get update -qq >/dev/null 2>&1 || true
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
        iproute2 python3-yaml libsodium23 libssl3 procps zip >/dev/null 2>&1 \
        || say "WARN: apt-get install best-effort leg failed — capability probe decides"
fi

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 1 — assemble the binaries from artifacts THIS pipeline already built.
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 1: binaries (reuse — nothing is compiled here)"
mkdir -p "$BIN_DIR" "$MESH_DIR"

# GIT_COMMIT_HASH is the image tag both Build Doorway and Build Storage apply
# alongside IMAGE_TAG (`nerdctl tag <img>:${IMAGE_TAG} <img>:${GIT_COMMIT_HASH}`).
# Read it from the pipeline's own build.env so this script needs no Groovy
# plumbing; fall back to deriving it the same way Setup Version does.
BUILD_ENV="${WORKSPACE:-$WORKSPACE_ROOT}/build.env"
if [ -f "$BUILD_ENV" ]; then
    # shellcheck disable=SC1090
    GIT_COMMIT_HASH="$(grep -E '^GIT_COMMIT_HASH=' "$BUILD_ENV" | head -1 | cut -d= -f2-)"
fi
if [ -z "${GIT_COMMIT_HASH:-}" ]; then
    GIT_COMMIT_HASH="$(git -C "$WORKSPACE_ROOT" rev-parse HEAD 2>/dev/null | cut -c1-8)"
fi
say "image tag: ${GIT_COMMIT_HASH:-<unresolved>}"

STORAGE_IMAGE="${MESH_STORAGE_IMAGE:-elohim-storage:${GIT_COMMIT_HASH}}"
DOORWAY_IMAGE="${MESH_DOORWAY_IMAGE:-elohim-doorway:${GIT_COMMIT_HASH}}"

# Extract one file out of a container image. Two mechanisms because nerdctl
# versions differ in which one they support: `run --entrypoint cat` (works
# whenever the image has a shell-less cat, which these debian-slim images do)
# then `create` + `cp`.
extract_bin() {
    local image="$1" src="$2" dest="$3" cid
    if nerdctl -n k8s.io run --rm --entrypoint /bin/cat "$image" "$src" > "$dest" 2>/dev/null \
       && [ -s "$dest" ]; then
        chmod +x "$dest"; return 0
    fi
    cid="$(nerdctl -n k8s.io create "$image" true 2>/dev/null)" || return 1
    if nerdctl -n k8s.io cp "${cid}:${src}" "$dest" 2>/dev/null && [ -s "$dest" ]; then
        nerdctl -n k8s.io rm -f "$cid" >/dev/null 2>&1 || true
        chmod +x "$dest"; return 0
    fi
    nerdctl -n k8s.io rm -f "$cid" >/dev/null 2>&1 || true
    rm -f "$dest"
    return 1
}

MISSING=""
if ! command -v nerdctl >/dev/null 2>&1; then
    MISSING="${MISSING}nerdctl-absent "
else
    extract_bin "$STORAGE_IMAGE" /usr/local/bin/elohim-storage "$BIN_DIR/elohim-storage" \
        || MISSING="${MISSING}storage-binary(${STORAGE_IMAGE}) "
    extract_bin "$DOORWAY_IMAGE" /usr/local/bin/doorway "$BIN_DIR/doorway" \
        || MISSING="${MISSING}doorway-binary(${DOORWAY_IMAGE}) "
fi

# ── conductor toolchain ────────────────────────────────────────────────────
# `holochain`: prefer PATH, then the storage image's embedded conductor (that
# is the conductor this commit SHIPS — measuring anything else would measure a
# fleet we do not deploy).
HOLOCHAIN_SOURCE="none"
if command -v holochain >/dev/null 2>&1; then
    ln -sf "$(command -v holochain)" "$BIN_DIR/holochain"
    HOLOCHAIN_SOURCE="PATH"
elif [ -n "${GIT_COMMIT_HASH:-}" ] && command -v nerdctl >/dev/null 2>&1 \
     && extract_bin "$STORAGE_IMAGE" /usr/local/bin/holochain "$BIN_DIR/holochain"; then
    HOLOCHAIN_SOURCE="storage-image(${STORAGE_IMAGE})"
elif curl -fsSL --max-time 120 -o "$BIN_DIR/holochain" \
        "${HC_RELEASE_BASE}/holochain-${HC_VERSION}/holochain-x86_64-unknown-linux-gnu" 2>/dev/null \
     && [ -s "$BIN_DIR/holochain" ]; then
    chmod +x "$BIN_DIR/holochain"
    HOLOCHAIN_SOURCE="nexus-stock-${HC_VERSION}"
else
    MISSING="${MISSING}holochain-binary "
fi

# `hc`: the sandbox CLI. No image we build publishes it (the conductor image
# ships only /usr/bin/holochain), so PATH-or-Nexus is the whole ladder. Version
# skew note: a stock `hc` drives the forked `holochain` fine within 0.6.x —
# `hc sandbox` only writes conductor-config.yaml and spawns the binary — but
# the pairing IS printed below so a future 0.7 skew is diagnosable, not silent.
HC_SOURCE="none"
if command -v hc >/dev/null 2>&1; then
    ln -sf "$(command -v hc)" "$BIN_DIR/hc"
    HC_SOURCE="PATH"
elif curl -fsSL --max-time 120 -o "$BIN_DIR/hc" \
        "${HC_RELEASE_BASE}/holochain-${HC_VERSION}/hc-x86_64-unknown-linux-gnu" 2>/dev/null \
     && [ -s "$BIN_DIR/hc" ]; then
    chmod +x "$BIN_DIR/hc"
    HC_SOURCE="nexus-stock-${HC_VERSION}"
else
    MISSING="${MISSING}hc-sandbox-cli "
fi

# ── happ bundle ────────────────────────────────────────────────────────────
# Already fetched from Harbor by Build Edge Node Image / Build hApp Installer
# (oras pull of elohim-happ:<floating tag>). Copy, never re-fetch: a second
# pull could resolve a DIFFERENT floating tag mid-build.
mkdir -p "$HAPP_WORKDIR"
if [ ! -f "$HAPP_WORKDIR/elohim.happ" ]; then
    for candidate in \
        "$WORKSPACE_ROOT/elohim/holochain/edgenode/elohim.happ" \
        "$WORKSPACE_ROOT/elohim/holochain/edgenode/scripts/elohim.happ"; do
        if [ -f "$candidate" ]; then
            cp "$candidate" "$HAPP_WORKDIR/elohim.happ"
            say "happ: reused $candidate"
            break
        fi
    done
fi
[ -f "$HAPP_WORKDIR/elohim.happ" ] || MISSING="${MISSING}elohim.happ "

# ── runtime linkage probe ──────────────────────────────────────────────────
for b in "$BIN_DIR/elohim-storage" "$BIN_DIR/doorway"; do
    if [ -x "$b" ] && command -v ldd >/dev/null 2>&1; then
        if ldd "$b" 2>/dev/null | grep -q 'not found'; then
            MISSING="${MISSING}sharedlibs($(basename "$b"):$(ldd "$b" 2>/dev/null | awk '/not found/{printf "%s,", $1}')) "
        fi
    fi
done
command -v python3 >/dev/null 2>&1 || MISSING="${MISSING}python3 "
python3 -c 'import yaml' >/dev/null 2>&1 || MISSING="${MISSING}python3-yaml "
command -v ss >/dev/null 2>&1 || MISSING="${MISSING}iproute2(ss) "
command -v pnpm >/dev/null 2>&1 || MISSING="${MISSING}pnpm "

if [ -n "$MISSING" ]; then
    skip_no_conductor "this container cannot host a 3-peer mesh — missing: ${MISSING% }. \
Toolchain sources tried: holochain=${HOLOCHAIN_SOURCE} hc=${HC_SOURCE}. \
Bake hc+holochain into ci-builder (or restore the Nexus github-releases-proxy route) and this stage \
activates with no Jenkinsfile change."
fi

export PATH="$BIN_DIR:$PATH"
say "toolchain: holochain=${HOLOCHAIN_SOURCE} hc=${HC_SOURCE}"
say "storage=$BIN_DIR/elohim-storage doorway=$BIN_DIR/doorway happ=$HAPP_WORKDIR/elohim.happ"

# Everything below this line is inside the 15-minute budget.
STAGE_START=$(date +%s)

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 2 — bring the mesh up.
#
# CONDUCTOR-GENERATE FLAKE MITIGATION (chosen: settle + one retry).
# Observed 2026-08-16: `hc sandbox generate -n 3` failed 4/4 IN SCRIPT CONTEXT
# on the THIRD conductor when it ran seconds after the doorways started; the
# same sequence typed by hand (i.e. with the doorways long since up and idle)
# succeeded. Of the three candidate mitigations —
#   (a) generate before starting the doorways,
#   (b) `-n 1` three times,
#   (c) settle + retry
# — (a) and (b) both require restructuring hc-mesh.sh's start_all, which is the
# LOCAL dev-loop rail (Q8's surface, and concurrently in use by a live saga
# run); Q9 is the CI seam and must not fork that script. (c) reproduces the
# manual sequence's timing exactly and lives entirely here: attempt 1 may lose
# the race, `stop` clears the half-built sandboxes, the settle window ages the
# doorways, and attempt 2's generate no longer runs seconds after doorway
# start — hc-mesh.sh skips already-healthy doorways, so the retry only
# regenerates conductors. If the flake ever proves to be load- rather than
# proximity-driven, (a) is the durable cure and belongs in hc-mesh.sh.
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 2: mesh bring-up"
BRINGUP_ATTEMPTS="${MESH_BRINGUP_ATTEMPTS:-2}"
SETTLE_SECS="${MESH_GENERATE_SETTLE_SECS:-30}"
mesh_up=0
attempt=1
while [ "$attempt" -le "$BRINGUP_ATTEMPTS" ]; do
    require_budget 240 "mesh bring-up attempt ${attempt}"
    left="$(remaining)"
    bringup_timeout=$(( left < 420 ? left : 420 ))
    say "bring-up attempt ${attempt}/${BRINGUP_ATTEMPTS} (timeout ${bringup_timeout}s)"
    # From here on the mesh is OURS — teardown is armed even if bring-up dies
    # halfway (half-generated sandboxes must not leak onto a shared agent).
    MESH_STARTED=1
    if MESH_DIR="$MESH_DIR" \
       STORAGE_BIN="$BIN_DIR/elohim-storage" \
       DOORWAY_BIN="$BIN_DIR/doorway" \
       DOORWAY_PORT="$DOORWAY_PORT" \
       DOORWAY_B_PORT="$DOORWAY_B_PORT" \
       timeout "$bringup_timeout" bash "$MESH_SCRIPT" start; then
        mesh_up=1
        break
    fi
    say "bring-up attempt ${attempt} failed — stopping, settling ${SETTLE_SECS}s, retrying"
    MESH_DIR="$MESH_DIR" bash "$MESH_SCRIPT" stop || true
    sleep "$SETTLE_SECS"
    attempt=$((attempt + 1))
done
[ "$mesh_up" = "1" ] || fail "mesh did not come up in ${BRINGUP_ATTEMPTS} attempt(s) — see $MESH_DIR/logs and elohim/holochain/local-dev/.sandbox_log"

DOORWAY_A_URL="http://localhost:${DOORWAY_PORT}"
DOORWAY_B_URL="http://localhost:${DOORWAY_B_PORT}"
STORAGE_A_URL="http://localhost:8090"
STORAGE_B_URL="http://localhost:8091"

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 3 — seed the fixture corpus under the declared grant.
#
# The stakes-declaration line is GREP-ASSERTED: it is the proof that the corpus
# was seeded under an explicit preproduction Simulacra grant (§3 W2) rather
# than by some ceremony-skipping shortcut. A local rate won by bypassing the
# ceremony over-predicts and invalidates the transfer coefficient (§5) — so a
# seed with no declaration is a failed measure, not a fast one.
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 3: seed"
require_budget 180 "seed"
SEED_LOG="$MESH_DIR/seed.log"
(
    cd "$WORKSPACE_ROOT" || exit 1
    pnpm install --frozen-lockfile --filter "holochain-seeder..." >/dev/null 2>&1
) || say "WARN: seeder workspace install returned non-zero — attempting the seed anyway"

left="$(remaining)"
seed_timeout=$(( left < 300 ? left : 300 ))
(
    cd "$WORKSPACE_ROOT/genesis/seeder" || exit 1
    DOORWAY_URL="$DOORWAY_A_URL" \
    STORAGE_URL="$STORAGE_A_URL" \
    SKIP_VERIFICATION=true \
    timeout "$seed_timeout" pnpm exec tsx src/seed.ts
) > "$SEED_LOG" 2>&1
seed_rc=$?
tail -40 "$SEED_LOG"
[ "$seed_rc" -eq 0 ] || fail "seeder exited ${seed_rc} — see $SEED_LOG"

if ! grep -q '^stakes declaration: ' "$SEED_LOG"; then
    fail "seed produced NO stakes declaration line — the corpus was not seeded under a declared grant, so the measure would not be transferable (plan §5). See $SEED_LOG"
fi
grep '^stakes declaration: ' "$SEED_LOG" | tail -1

# Rows the measure is denominated in: the corpus the grant covers.
# Parsed with grep/sed, not a python one-liner — scripts/ci/.epr-meta
# `deploy-scripts-bash-coreutils-only` (edge #1183). This stage is a
# build-path measure and never runs on the deploy path, but the rule is
# cheap to honour and keeps the dependency surface honest.
STAKES_JSON="$WORKSPACE_ROOT/genesis/seeder/stakes-declaration.json"
ROWS="$(grep -o '"itemsDeclared"[[:space:]]*:[[:space:]]*[0-9]*' "$STAKES_JSON" 2>/dev/null \
        | head -1 | sed 's/.*:[[:space:]]*//')"
[ -n "${ROWS:-}" ] || ROWS=0
say "corpus rows under grant: ${ROWS}"

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 4 — stage the landing blob (authors the ONE notarized head).
#
# The quiesce gate requires GET /db/content/<id> == 200 on BOTH doorways and a
# converged reconcile sweep on storage-A; without a notarized head on the
# landing row there is nothing for the convergence machinery to converge, and
# the gate would measure an empty question. The dist directory is synthetic on
# purpose: the edge pipeline does not build the Angular SPA (that is the app
# pipeline), and the bundle's BYTES are irrelevant to this measure — what
# matters is that a real content-addressed blob is PUT and a real DNA-notarized
# blobHash PATCH is authored through the conductor bridge, which a one-file
# bundle exercises identically.
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 4: stage landing blob (notarized head)"
require_budget 120 "blob staging"
SYNTH_DIST="$MESH_DIR/synthetic-landing-dist"
rm -rf "$SYNTH_DIST"
mkdir -p "$SYNTH_DIST"
printf '<!doctype html><title>mesh-quiesce fixture</title><p>%s</p>\n' "${GIT_COMMIT_HASH}" \
    > "$SYNTH_DIST/index.html"

DO_PATCH=1 \
STORAGE_API_KEY_ADMIN="${STORAGE_API_KEY_ADMIN:-}" \
STAGE_BLOB_ATTEMPTS="${STAGE_BLOB_ATTEMPTS:-3}" \
    bash "$STAGE_BLOB_SCRIPT" "$SYNTH_DIST" "$CONTENT_ID" "$DOORWAY_A_URL" browser
blob_rc=$?
[ "$blob_rc" -eq 0 ] || fail "landing blob staging exited ${blob_rc} — no notarized head to converge"

# ---------------------------------------------------------------------------
# PHASE 1 (QUIESCE) step 5 — the measure.
# ---------------------------------------------------------------------------
banner "PHASE 1 QUIESCE · step 5: the gate"
require_budget 120 "quiesce gate"
left="$(remaining)"
gate_deadline=$(( left < GATE_DEADLINE ? left : GATE_DEADLINE ))
say "gate deadline ${gate_deadline}s (of the ${GATE_DEADLINE}s nominal window; ${left}s budget left)"

quiesce_start=$(date +%s)
MESH_DIR="$MESH_DIR" \
QUIESCE_DEADLINE_SECS="$gate_deadline" \
MESH_QUIESCE_DOORWAY_A="$DOORWAY_A_URL" \
MESH_QUIESCE_DOORWAY_B="$DOORWAY_B_URL" \
MESH_QUIESCE_STORAGE_A="$STORAGE_A_URL" \
MESH_QUIESCE_STORAGE_B="$STORAGE_B_URL" \
MESH_QUIESCE_CONTENT_ID="$CONTENT_ID" \
    bash "$QUIESCE_SCRIPT"
gate_rc=$?
quiesce_secs=$(( $(date +%s) - quiesce_start ))

# ---------------------------------------------------------------------------
# The reported number. Printed on EVERY run, pass or fail — a measure that only
# speaks when it is happy is not a measure.
# ---------------------------------------------------------------------------
if [ "$gate_rc" -eq 0 ] && [ "$quiesce_secs" -gt 0 ]; then
    RATE=$(( ROWS * 60 / quiesce_secs ))
    VERDICT="PASS"
else
    RATE=0
    VERDICT="FAIL"
fi

echo ""
echo "MESH-QUIESCE: ${VERDICT} rate=${RATE} rows/min (target >=${RATE_TARGET})"
echo "MESH-QUIESCE RATE: ${RATE} rows/min (target >=${RATE_TARGET})"
echo "MESH-QUIESCE DETAIL: verdict=${VERDICT} rows=${ROWS} quiesce=${quiesce_secs}s gate_exit=${gate_rc} deadline=${gate_deadline}s conductor=${HOLOCHAIN_SOURCE} hc=${HC_SOURCE} commit=${GIT_COMMIT_HASH}"
echo ""

if [ "$gate_rc" -ne 0 ]; then
    # Deliberately a FAILURE, not a no-measure. This stage owns its mesh: there
    # is no shared fleet whose churn could excuse the window. Not reaching
    # sustained PASS in ${gate_deadline}s IS the regression signal (plan §W4.2).
    # PHASE 2 never runs off an unquiesced substrate — an e2e verdict taken
    # mid-churn measures the churn, not the code.
    fail "fleet-quiesce gate did not reach sustained PASS within ${gate_deadline}s on a mesh this stage owns (gate exit ${gate_rc}). Convergence regression — or the dev-tier pacing profile no longer holds. PHASE 2 (e2e) skipped: no quiesced substrate to validate on."
fi

if [ "$RATE" -lt "$RATE_TARGET" ]; then
    echo "MESH-QUIESCE WARN: ${RATE} rows/min is BELOW the ${RATE_TARGET} rows/min sprint target (plan §5) — converged, but slower than the declared floor." >&2
fi

# ===========================================================================
# PHASE 2 — E2E (runtime validation) on the quiesced substrate.
#
# Distinct semantics from PHASE 1, deliberately:
#   PHASE 1 is a BOOTSTRAP measure — it asks "did the convergence machinery
#   reach and hold quiescence?", and its writes (seed, blob, head) are the
#   SETUP. Its verdict is hard.
#   PHASE 2 is RUNTIME VALIDATION — the saga's agent actions flowing through
#   an already-quiesced system. Its writes ARE the subject under test, so it
#   must NOT re-assert global quiesce metrics: a scenario that writes and then
#   demands fleet-wide `divergent_actionable == 0` is asserting that its own
#   subject never happened.
#
# KNOWN STEP-DEBT (why phase 2 is UNSTABLE-grade, not blocking, for this first
# landing): some saga steps still assert global-quiesce predicates inside e2e —
# exactly the confusion above. Until those steps are re-pointed at per-scenario
# subjects, a phase-2 red can be step-debt rather than a code regression, so it
# reports and does not fail the build. Flip MESH_E2E_BLOCKING=1 once the debt
# is paid.
#
# PROFILE DISCIPLINE (load-bearing): `--profile saga` and NEVER bare CLI paths.
# cucumber-js MERGES a profile's `paths` with CLI positionals rather than
# replacing them, so pointing the default profile at the saga directory runs
# ~800 scenarios (content ingests included) — a heavy write event that
# re-churns the very mesh phase 1 just measured (genesis/a2o/cucumber.mjs,
# 2026-08-16 Q10 finding). The saga profile is exactly 26 scenarios.
#
# DENOMINATOR: of those 26, three are @wip (two of them carrying undefined
# steps, which would make cucumber exit non-zero on EVERY run regardless of
# substrate health — a permanently-red signal is not a signal). They are
# filtered out, so the measured denominator is 23 today; the printed n/N reads
# cucumber's own summary line, so it self-corrects the moment the @wip debt
# closes and the filter is dropped. No saga scenario is @browser-only today —
# the exclusion is a guard against the documented playwright hard-fail class
# (backlog a2o-playwright-device-hardfail-topology), not a live subtraction.
# ===========================================================================
banner "PHASE 2 E2E · saga profile on the quiesced substrate"
E2E_BLOCKING="${MESH_E2E_BLOCKING:-0}"
E2E_LOG="$MESH_DIR/e2e-saga.log"
A2O_DIR="$WORKSPACE_ROOT/genesis/a2o"

# name=host:port CSV, same shape hc-mesh.sh status prints.
PEER_CSV="matthew=localhost:8090,jessica=localhost:8091,james=localhost:8092"

(
    cd "$WORKSPACE_ROOT" || exit 1
    pnpm install --frozen-lockfile --filter "@elohim/a2o..." >/dev/null 2>&1
) || say "WARN: a2o workspace install returned non-zero — attempting the suite anyway"

left="$(remaining)"
e2e_timeout=$(( left > 60 ? left : 600 ))
(
    cd "$A2O_DIR" || exit 1
    mkdir -p reports
    PEER_STORAGE_URLS="$PEER_CSV" \
    INTERNAL_DOORWAY_URL="localhost:${DOORWAY_PORT}" \
    E2E_DOORWAY_ALPHA="$DOORWAY_A_URL" \
    E2E_DOORWAY_B="$DOORWAY_B_URL" \
    E2E_STORAGE_URL="$STORAGE_A_URL" \
    BUILD_TAG="mesh-quiesce-${GIT_COMMIT_HASH}" \
    timeout "$e2e_timeout" pnpm exec cucumber-js \
        --profile saga \
        --format "json:reports/cucumber-report-mesh-saga.json" \
        --tags 'not @wip and not @browser-only'
) 2>&1 | tee "$E2E_LOG"
e2e_rc="${PIPESTATUS[0]}"

# Scenario tally straight off cucumber's own summary line
# ("26 scenarios (2 failed, 24 passed)") — grep/sed, no JSON parser
# (scripts/ci/.epr-meta: bash + coreutils only).
e2e_line="$(grep -oE '^[0-9]+ scenarios \(.*\)$' "$E2E_LOG" | tail -1)"
e2e_total="$(printf '%s' "$e2e_line" | sed -n 's/^\([0-9]*\) scenarios.*/\1/p')"
e2e_passed="$(printf '%s' "$e2e_line" | grep -oE '[0-9]+ passed' | sed 's/ passed//')"
[ -n "${e2e_total:-}" ]  || e2e_total=0
[ -n "${e2e_passed:-}" ] || e2e_passed=0

echo ""
if [ "$e2e_rc" -eq 0 ]; then
    echo "MESH-E2E: ${e2e_passed}/${e2e_total} scenarios (PASS, saga profile)"
else
    echo "MESH-E2E: ${e2e_passed}/${e2e_total} scenarios (cucumber exit ${e2e_rc}, saga profile) — UNSTABLE-grade; see ${E2E_LOG}. Some saga steps still assert global-quiesce predicates inside e2e (known step-debt), so a red here is not automatically a code regression."
fi
echo ""

if [ "$e2e_rc" -ne 0 ] && [ "$E2E_BLOCKING" = "1" ]; then
    fail "saga e2e suite exited ${e2e_rc} with MESH_E2E_BLOCKING=1"
fi

say "measure complete (phase 1 verdict=${VERDICT} rate=${RATE} rows/min; phase 2 ${e2e_passed}/${e2e_total})"
exit 0
