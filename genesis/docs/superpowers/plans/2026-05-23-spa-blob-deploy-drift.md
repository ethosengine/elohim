# SPA Blob Deploy Drift — `stageSpaBlob` writes a blob that nothing references

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the silent CI regression introduced by the landing-page-EPR work (`2026-05-23-landing-page-epr-dual-doorway.md`). The App pipeline's `stageSpaBlob` helper successfully uploads the elohim-app browser bundle as a blob, but the follow-up call that links the blob to the two ContentNode rows (`lamad-spa`, `elohim-host-landing`) returns 405 Method Not Allowed and is silently swallowed. Result on alpha: bytes on disk, nothing pointing at them, `rootApp.ready: false`, gateway shell never advances past "Connecting…", `/apps/elohim-host-landing/index.html` 404.

**Architecture:** elohim-storage exposes HTTP routes for content. Today `/db/content/{id}` has `GET` (cached) and `DELETE` (auth-required) registered. A `PATCH` handler exists in code but **was never registered** in the route table, so the curl in `Jenkinsfile:252–256` (which uses `PUT`, not `PATCH`) falls through to `method_not_allowed()`. Compounding this, `UpdateContentInputView` is missing the `blob_hash` field that `CreateContentInputView` has — so even if a partial-update route were callable, the field would be discarded by serde. The shell `|| echo "WARNING"` in `stageSpaBlob` swallows curl's non-zero exit, Jenkins records success, and the warning scrolls past unnoticed.

**Tech Stack:** Rust (elohim-storage HTTP route + service + Diesel; elohim-views InputView; ts-rs codegen), TypeScript (regenerated bindings — no hand edits), Groovy/shell (root Jenkinsfile `stageSpaBlob` helper at line 223).

**Substrate references (read before changing):**
- `elohim/elohim-storage/src/http.rs:3554` — PATCH handler (`handle_db_content_by_id`) already parses `UpdateContentInputView` and calls `services.content.update()`.
- `elohim/elohim-storage/src/http.rs:9280-9291` — route registry: `GET` + `DELETE` only; no `PATCH`/`PUT`.
- `elohim/elohim-storage/src/http.rs:3810-3825` — PATCH branch does NOT check auth before calling the service (inconsistent with DELETE at 9289 which declares `.auth_required()`).
- `elohim/elohim-storage/src/http.rs:3834` — `PUT` falls through to `method_not_allowed()` → 405.
- `elohim/elohim-views/src/lamad.rs:145-163` — `UpdateContentInputView` fields: `title`, `description`, `contentBody`, `contentFormat`, `metadata`, `tags`, `reach`. **No `blobHash`.**
- `elohim/elohim-views/src/lamad.rs` (same file, search for `CreateContentInputView`) — has `blobHash` already; mirror the field declaration.
- `elohim/elohim-storage/src/db/content_diesel.rs` — confirm the diesel update covers `blob_hash` column (column already exists; just verify the update path doesn't filter it out).
- `elohim/elohim-storage/src/services/content.rs` — service-layer `update()` plumbs the InputView fields to diesel.
- `genesis/seeder/src/seed-sqlite.ts:566-601` — `transformContent` passes `blob_hash` through from JSON when present; absent → `undefined` → dropped from serde body. No default placeholder is inserted.
- `genesis/data/lamad/content/elohim-host-landing.json` — placeholder already removed (commit `a869897e`); seed-sqlite will now omit `blobHash` from create calls, but **the stale row created during the prior seed run still holds the placeholder string** — it survives re-seed.
- `doorway/doorway-service/src/routes/root_app.rs:27-377` — bootstrap-shell loop. Polls `/health/startup` waiting for `rootApp.ready` and `rootApp.extracted`. **No fallback path** for a `blobHash` that resolves to nothing; the shell retries forever with a 5s reload (line 326) after three consecutive failures (line 309). This is the symptom the user sees.
- `Jenkinsfile` (root, lines 223-270) — `stageSpaBlob` helper. The blob PUT to `${storageUrl}/blob/${SPA_HASH}` works (matthew's `/data/blobs` has the 7.08 MB `sha256-ee5301c995967ff1e0a7b07a7f5e2de4f5f547bb3718547d33248453b77cb30f`). The followup PUT to `/db/content/${slug}` is the broken hop.
- `genesis/Jenkinsfile` — `Seed Operator Bindings` stage (commit `f4a53b3e`) is the reference for "POST a body, assert acceptance, fail-fast on non-2xx". Mirror its posture in `stageSpaBlob`.

**Acceptance criteria:**

1. `curl -X PATCH -H "X-API-Key: <admin>" -d '{"blobHash":"sha256-…"}' http://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8090/db/content/lamad-spa` → **200** with the row reflected in the response body; `blobHash` matches the request.
2. The same curl against `elohim-host-landing` → **200**.
3. Issuing the request without `X-API-Key` → **401**.
4. App pipeline run on a fresh dev commit → `stageSpaBlob` Jenkins log shows both PATCH writes succeeded + a read-back assertion line per slug; build status remains green; **no `WARNING: Content node update failed` lines anywhere**.
5. Post-deploy on alpha: `curl -s https://alpha.elohim.host/health/startup | jq '.rootApp'` → `{ready: true, extracted: true, slug: "elohim-host-landing", blobHash: "sha256-…"}`.
6. `curl -sI https://alpha.elohim.host/apps/elohim-host-landing/index.html` → **HTTP/2 200**, `content-type: text/html`.
7. Browser load of `https://alpha.elohim.host/` → 302 chain resolves; SPA renders; no infinite spinner; no four-stage shell.
8. `genesis/a2o/features/protocol/landing-page-dogfood.feature` runs green.

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `elohim/elohim-views/src/lamad.rs` | MODIFY | Add `blob_hash: Option<String>` to `UpdateContentInputView` (mirror the field on `CreateContentInputView`). |
| `elohim/elohim-storage/src/services/content.rs` | MODIFY (verify) | Ensure the service-layer `update()` forwards `blob_hash` to diesel. Likely already accepts the full InputView; just confirm the field is wired. |
| `elohim/elohim-storage/src/db/content_diesel.rs` | MODIFY (verify) | Confirm the diesel update writes `blob_hash` when present. The column exists; we just need to confirm it isn't filtered out of `ContentUpdate` (or equivalent). |
| `elohim/elohim-storage/src/http.rs` | MODIFY | Register `PATCH /db/content/{id}` in the route table (lines 9280-9291). Add `.auth_required()` to match DELETE. Inside the handler (line 3554), add the same auth gate the route declares — defense in depth. |
| `elohim/elohim-storage/tests/schema_contract.rs` | MODIFY | Extend the contract test so `blobHash` round-trips through PATCH. If a partial-update contract test doesn't yet exist, add one. |
| `elohim/sdk/storage-client-ts/src/generated/` | REGENERATE | `cargo test export_bindings` from `elohim/elohim-views` (per repo CLAUDE.md). Do NOT hand-edit. |
| `Jenkinsfile` (root) | MODIFY | Lines 223-270, `stageSpaBlob` helper: change verb from `PUT` to `PATCH`, pass admin API key header, drop the `\|\| echo` swallow, add a read-back GET that asserts `blobHash` matches the SHA just written. Preserve the existing loop over both slugs (added in `9200881d`). |

---

## Pre-flight

### Task 0: Confirm working tree clean

- [ ] **Step 0.1: Pull latest and confirm starting commit**

```bash
cd /projects/elohim
git status
git log --oneline -3
```

Expected: clean tree on `dev`, HEAD ≥ `b34890598` (the landing-page-EPR plan commit). If you see uncommitted changes inherited from a prior session, decide whether to stash or discard before continuing.

- [ ] **Step 0.2: Confirm the upstream stage is still broken (sanity-check the diagnosis)**

```bash
curl -s https://alpha.elohim.host/health/startup | python3 -m json.tool
```

Expected: `rootApp.ready: false`, `rootApp.blobHash: "sha256-PLACEHOLDER_REPLACED_BY_SEED_SCRIPT"`. If this returns `ready: true`, someone already fixed it; coordinate before proceeding.

---

## Task 1: Add `blob_hash` to `UpdateContentInputView` (TDD red phase)

The InputView is the wire-shape contract. Adding the field is the cheapest gate that, by itself, will already fix half the bug (the field-discarded-by-serde half) — but only after the route lands in Task 2.

**Files:**
- Modify: `elohim/elohim-views/src/lamad.rs`

- [ ] **Step 1.1: Read the existing CreateContentInputView for the exact shape to mirror**

```bash
cd /projects/elohim
grep -n "blobHash\|blob_hash\|CreateContentInputView\|UpdateContentInputView" elohim/elohim-views/src/lamad.rs | head -40
```

Note: per repo CLAUDE.md, snake_case stays inside Rust; serde's `rename_all = "camelCase"` on the struct produces `blobHash` over the wire. The struct field is `blob_hash: Option<String>`.

- [ ] **Step 1.2: Write the schema-contract test first (red)**

`elohim/elohim-storage/tests/schema_contract.rs` already validates Views against their JSON Schemas under `elohim/sdk/schemas/v1/views/`. Find the existing `UpdateContentInputView` test (or the inputs.schema.json reference) and extend it to assert `blobHash` is an optional string. Run:

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract -p elohim-storage 2>&1 | tail -10
```

Expected (red): the new assertion fails because the View doesn't declare the field yet.

- [ ] **Step 1.3: Add the field on the struct**

Edit `elohim/elohim-views/src/lamad.rs` `UpdateContentInputView` to add (matching the order of existing fields):

```rust
    /// Content-addressed SHA256 of the SPA bundle / asset this row projects.
    /// Set at deploy-time by Jenkinsfile:stageSpaBlob — see
    /// genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
    /// Deliberately optional: PATCH callers MAY set this without touching
    /// any other field.
    pub blob_hash: Option<String>,
```

- [ ] **Step 1.4: Run the schema-contract test again (green for the View; still red for the route)**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract -p elohim-storage 2>&1 | tail -10
```

Expected: schema-contract assertion for the View passes. (Route-level integration test will follow in Task 2.)

- [ ] **Step 1.5: Regenerate TypeScript bindings**

```bash
cd /projects/elohim/elohim/elohim-views
cargo test export_bindings 2>&1 | tail -5
cd /projects/elohim
git diff --stat elohim/sdk/storage-client-ts/src/generated/
```

Expected: `UpdateContentInputView.ts` (or equivalent generated file) shows `blobHash?: string;` added. No hand edits.

- [ ] **Step 1.6: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-views/src/lamad.rs elohim/elohim-storage/tests/schema_contract.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "$(cat <<'EOF'
feat(views): UpdateContentInputView accepts blobHash for partial updates

The InputView shape used by PATCH /db/content/{id} previously omitted
blobHash, so any caller setting it would have the field silently
dropped by serde. Add `blob_hash: Option<String>` mirroring the field
on CreateContentInputView.

Sets up the deploy-time stageSpaBlob path (root Jenkinsfile) to link
the SPA bundle blob to both content rows after the route registration
lands.

Refs genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
EOF
)"
```

---

## Task 2: Register `PATCH /db/content/{id}` with auth, persist `blob_hash`

The handler exists; the route entry doesn't. After this task, the API can accept partial updates.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`
- Verify (modify if needed): `elohim/elohim-storage/src/services/content.rs`
- Verify (modify if needed): `elohim/elohim-storage/src/db/content_diesel.rs`

- [ ] **Step 2.1: Locate the existing handler + route registration**

```bash
cd /projects/elohim
grep -n "handle_db_content_by_id\|/db/content/{id}\|Method::PATCH\|Method::PUT" elohim/elohim-storage/src/http.rs | head -20
```

Confirm:
- `handle_db_content_by_id` at line 3554 (handler).
- A route-table entry around lines 9280-9291 with `.get(...)` and `.delete(...).auth_required()` for `/db/content/{id}` — but no `.patch(...)`.

- [ ] **Step 2.2: Add `.patch(...).auth_required()` to the route entry**

In the route builder around line 9280, mirror the DELETE pattern. Example (adapt to actual builder shape — could be `.method(Method::PATCH)` or `.patch(...)`):

```rust
.route("/db/content/{id}",
    routing::get(handle_db_content_by_id_get)
        .patch(handle_db_content_by_id_patch).auth_required()
        .delete(handle_db_content_by_id_delete).auth_required(),
)
```

(If `handle_db_content_by_id` is one fn with internal method dispatch, the .patch handler just routes to the existing PATCH branch at line 3810. Read the surrounding builder code for the exact idiom.)

- [ ] **Step 2.3: Move the auth check from "ambient" to the handler entry**

The current handler dispatches to a PATCH branch at line 3810 without checking auth. After registering `.auth_required()` on the route, the framework's middleware will gate it — but as defense in depth, add an explicit auth-required assertion inside the PATCH branch too, mirroring the DELETE branch (search for `.auth_required()` invocations in the handler at line 3554+ to find the existing pattern).

- [ ] **Step 2.4: Verify service + diesel propagate `blob_hash`**

```bash
cd /projects/elohim
grep -n "blob_hash" elohim/elohim-storage/src/services/content.rs elohim/elohim-storage/src/db/content_diesel.rs elohim/elohim-storage/src/db/models.rs 2>&1 | head -20
```

Confirm:
- `models.rs` (or equivalent) declares `blob_hash` on the `Content` Diesel model.
- The service-layer `update()` accepts an Update struct that includes `blob_hash`.
- The diesel update path writes `blob_hash` when `Some(_)`.

If any of these gaps exist, fill them. The column exists on the table (we see it on the live alpha row), so this is wiring, not migration.

- [ ] **Step 2.5: Add a route-level integration test**

In `elohim/elohim-storage/tests/` (likely a new file `content_patch.rs` or extending an existing route-level test fixture), assert:

1. `PATCH /db/content/{id}` with valid admin auth + `{"blobHash": "sha256-test"}` returns 200, and a subsequent GET reflects the new hash.
2. The same request without auth returns 401.
3. The same request with a body that only sets `blobHash` does NOT clobber other fields (`title`, `description`, etc.) — partial update semantics.

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test content_patch -p elohim-storage 2>&1 | tail -15
# Or, if appended to an existing file:
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test patch_db_content -p elohim-storage 2>&1 | tail -15
```

Expected: all three pass.

- [ ] **Step 2.6: Run the full storage test suite (no regressions)**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage 2>&1 | tail -10
```

Expected: full green. Clippy + fmt as usual.

- [ ] **Step 2.7: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/src/services/content.rs \
        elohim/elohim-storage/src/db/content_diesel.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/tests/
git commit -m "$(cat <<'EOF'
feat(storage): register PATCH /db/content/{id} with auth + blobHash plumbing

The PATCH handler at http.rs:3554 was authored but never wired into
the route registry; PUT/PATCH against /db/content/{id} returned 405.
Register .patch(...).auth_required() alongside the existing GET +
DELETE entries, and ensure the service + diesel update paths persist
blob_hash from the InputView.

Adds route-level tests that cover (1) auth-gated 401 path, (2) happy
PATCH with blobHash, (3) partial-update semantics (other fields not
clobbered). The schema-contract test from the previous commit
guarantees the wire shape.

Closes the route-registration half of
genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
EOF
)"
```

---

## Task 3: Patch `stageSpaBlob` to use PATCH, auth, verify, and fail-loud

After Tasks 1+2 ship, the storage API can accept the write. Now teach CI to actually use it correctly, and add a regression seatbelt so this can never recur silently.

**File:**
- Modify: `Jenkinsfile` (root, lines 223-270)

- [ ] **Step 3.1: Locate the helper and the call site**

```bash
cd /projects/elohim
grep -n "stageSpaBlob" Jenkinsfile
```

Expected: `def stageSpaBlob(...)` around line 223 and exactly one call site lower in the file.

- [ ] **Step 3.2: Check how other deploy-time stages access the admin API key**

```bash
cd /projects/elohim
grep -n "API_KEY_ADMIN\|STORAGE_API_KEY\|X-API-Key" Jenkinsfile genesis/Jenkinsfile 2>&1 | head -10
```

Find the existing pattern (likely a `withCredentials` block exposing a `STORAGE_API_KEY_ADMIN` env). Mirror it. If no admin key path exists yet for storage routes (only doorway routes), add one — the secret already lives in `elohim-doorway-alpha-secrets` (`api-key-admin` key); reuse, don't mint a second.

- [ ] **Step 3.3: Replace the stageSpaBlob body**

Replace lines ~223-270 of `Jenkinsfile` with the corrected helper. Concrete shape:

```groovy
def stageSpaBlob(String storageUrl, String distDir) {
    // Uploads the elohim-app browser bundle as a single blob and links
    // it to TWO content rows (lamad-spa + elohim-host-landing) via the
    // storage PATCH /db/content/{id} route. After PATCH, GET the row
    // and assert blobHash matches — the regression seatbelt.
    //
    // Auth: admin API key from elohim-doorway-alpha-secrets#api-key-admin.
    // No || swallow: any 4xx/5xx on either the PATCH or the read-back
    // assertion FAILS the build.
    //
    // See: genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md
    withCredentials([string(credentialsId: 'storage-api-key-admin',
                            variable: 'STORAGE_API_KEY_ADMIN')]) {
        sh '''#!/bin/bash
            set -euo pipefail
            cd "''' + distDir + '''"
            zip -r lamad-spa.zip .
            SPA_HASH="sha256-$(sha256sum lamad-spa.zip | awk '{print $1}')"
            SPA_SIZE="$(du -h lamad-spa.zip | cut -f1)"
            echo "SPA blob hash: ${SPA_HASH}"
            echo "SPA blob size: ${SPA_SIZE}"

            # 1. Upload ZIP as blob
            curl -fSs -X PUT \
                -H "Content-Type: application/zip" \
                --data-binary @lamad-spa.zip \
                "''' + storageUrl + '''/blob/${SPA_HASH}"
            echo "  ✓ blob uploaded"

            # 2. Link blob to both content rows
            for slug in lamad-spa elohim-host-landing; do
                # PATCH the row
                curl -fSs -X PATCH \
                    -H "Content-Type: application/json" \
                    -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
                    -d "{\\"blobHash\\":\\"${SPA_HASH}\\"}" \
                    "''' + storageUrl + '''/db/content/${slug}" \
                    >/dev/null
                echo "  ✓ patched ${slug}"

                # Read back, assert blobHash matches (regression seatbelt)
                ACTUAL=$(curl -fSs \
                    "''' + storageUrl + '''/db/content/${slug}" \
                    | python3 -c "import sys, json; print(json.load(sys.stdin).get('blobHash',''))")
                if [ "${ACTUAL}" != "${SPA_HASH}" ]; then
                    echo "ERROR: ${slug} blobHash drifted after PATCH" >&2
                    echo "  expected: ${SPA_HASH}" >&2
                    echo "  actual:   ${ACTUAL}" >&2
                    exit 1
                fi
                echo "  ✓ verified ${slug} blobHash = ${SPA_HASH}"
            done

            rm -f lamad-spa.zip
        '''
    }
}
```

Notes for the implementor:
- `set -euo pipefail` plus `curl -fSs` (no `||`) is the no-swallow posture.
- The credentialsId (`storage-api-key-admin`) may need to be created in Jenkins if not present — check first; if missing, use the same value that's mounted into the doorway pod via `elohim-doorway-alpha-secrets`.
- python3 should be in the builder image; if not, `jq` is the fallback (`jq -r '.blobHash // ""'`).

- [ ] **Step 3.3a: Decide on the storageUrl writer target**

The current call site uses `http://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8090` (line ~836 of Jenkinsfile). Matthew is the projection writer in this cluster (`PROJECTION_WRITER=true` per the doorway env we observed). Keep it pointed at matthew so the local write + projection update are co-located. If the cluster gains additional projection writers, this becomes a fan-out concern — out of scope here.

- [ ] **Step 3.4: Verify the helper still parses (no Groovy CLI; structural sanity)**

```bash
cd /projects/elohim
grep -n "stageSpaBlob\|X-API-Key\|PATCH.*db/content" Jenkinsfile
```

Expected: exactly one helper def, exactly one call site (signature unchanged), the PATCH curl + auth header + read-back loop all present.

- [ ] **Step 3.5: Commit**

```bash
cd /projects/elohim
git add Jenkinsfile
git commit -m "$(cat <<'EOF'
feat(blob): stageSpaBlob writes via PATCH with auth + read-back verification

The previous PUT swallowed a 405 with || echo, so blob bytes uploaded
but the content rows never linked. Switch to authenticated PATCH
(matches the new route from elohim-storage), drop the || swallow with
set -euo pipefail + curl -fSs, and add a read-back GET that asserts
blobHash matches the SHA just written. Failure on either the PATCH or
the assertion fails the build.

Closes the CI half of
genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
EOF
)"
```

---

## Task 4: Local end-to-end dry-run before pushing

The bug was a silent CI failure. Don't push until you've reproduced the green path locally.

- [ ] **Step 4.1: Start a local Holochain + storage stack**

```bash
cd /projects/elohim
pnpm install
pnpm run hc:start:seed 2>&1 | tail -20
```

Expected: storage on `localhost:8090`, doorway on `localhost:8888`, content rows seeded from `genesis/data/lamad/content/`. The `elohim-host-landing` row exists with NO `blobHash` set (the source JSON no longer carries the placeholder).

- [ ] **Step 4.2: Probe the PATCH route directly**

```bash
# Without auth — should 401
curl -i -X PATCH -H "Content-Type: application/json" \
    -d '{"blobHash":"sha256-test"}' \
    http://localhost:8090/db/content/elohim-host-landing | head -3

# With admin auth (use whatever the local stack accepts — see hc:start:seed output)
curl -i -X PATCH -H "Content-Type: application/json" \
    -H "X-API-Key: ${LOCAL_ADMIN_KEY}" \
    -d '{"blobHash":"sha256-test"}' \
    http://localhost:8090/db/content/elohim-host-landing | head -3

# Read back
curl -s http://localhost:8090/db/content/elohim-host-landing | python3 -m json.tool | head -10
```

Expected: 401 then 200, then `blobHash: "sha256-test"` in the read-back.

- [ ] **Step 4.3: Confirm the doorway resolves it**

```bash
curl -s http://localhost:8888/health/startup | python3 -m json.tool
```

Expected: `rootApp.slug: "elohim-host-landing"`, `rootApp.blobHash: "sha256-test"`, `rootApp.ready` should be `false` because `sha256-test` doesn't resolve to a real blob — that's correct! The doorway tried to fetch the blob and failed; the row is fine but the blob is missing. Sanity test only — proves the resolver reads the field.

- [ ] **Step 4.4: Verify the full chain with a real blob**

Build the elohim-app, upload the blob, repoint both rows, and watch the doorway flip to ready:

```bash
cd /projects/elohim
pnpm --filter elohim-app run build
cd app/elohim-app/dist/elohim-app/browser
zip -r /tmp/lamad-spa.zip .
HASH="sha256-$(sha256sum /tmp/lamad-spa.zip | awk '{print $1}')"
echo "HASH=${HASH}"

# Upload
curl -fSs -X PUT -H "Content-Type: application/zip" \
    --data-binary @/tmp/lamad-spa.zip \
    "http://localhost:8090/blob/${HASH}"

# Link both rows
for slug in lamad-spa elohim-host-landing; do
    curl -fSs -X PATCH -H "Content-Type: application/json" \
        -H "X-API-Key: ${LOCAL_ADMIN_KEY}" \
        -d "{\"blobHash\":\"${HASH}\"}" \
        "http://localhost:8090/db/content/${slug}" >/dev/null
done

# Doorway should flip to ready
sleep 5
curl -s http://localhost:8888/health/startup | python3 -m json.tool
curl -sI http://localhost:8888/apps/elohim-host-landing/index.html | head -3
```

Expected: `rootApp.ready: true`, `extracted: true`; index.html returns 200 + text/html.

---

## Task 5: Push, deploy, and verify on alpha

The pipeline does the work. Watch.

- [ ] **Step 5.1: Pause and confirm with operator**

This is a real-world deploy. Pause and ask: "Three commits ready (View, route, Jenkinsfile). Push to dev so the orchestrator dispatches App + Edge pipelines?" Only continue on explicit yes.

- [ ] **Step 5.2: Push**

```bash
cd /projects/elohim
git push origin dev
```

- [ ] **Step 5.3: Watch the orchestrator + App pipeline**

The orchestrator should fire the App pipeline because `Jenkinsfile` (root) changed. The App pipeline runs `stageSpaBlob`. With the new helper:

- The Jenkins log should show `✓ blob uploaded`, `✓ patched lamad-spa`, `✓ verified lamad-spa blobHash = sha256-…`, then the same pair for `elohim-host-landing`.
- A failure on PATCH or read-back FAILS the build (no more silent green).

If `ci-observer` reports a failure, escalate to `ci-investigator`.

- [ ] **Step 5.4: Post-deploy verification**

```bash
# 1. Row reflects new hash
curl -s https://alpha.elohim.host/db/content/elohim-host-landing | python3 -c "
import sys, json
data = json.load(sys.stdin)
h = data.get('blobHash', '')
assert h.startswith('sha256-') and 'PLACEHOLDER' not in h, f'unexpected: {h}'
print('OK: blobHash =', h)
"
curl -s https://alpha.elohim.host/db/content/lamad-spa | python3 -c "
import sys, json
data = json.load(sys.stdin)
h = data.get('blobHash', '')
assert h.startswith('sha256-'), f'unexpected: {h}'
print('OK: blobHash =', h)
"

# 2. Doorway flips to ready
curl -s https://alpha.elohim.host/health/startup | python3 -c "
import sys, json
ra = json.load(sys.stdin)['rootApp']
assert ra['ready'] and ra['extracted'], f'not ready: {ra}'
assert ra['slug'] == 'elohim-host-landing'
print('OK: rootApp ready, blobHash =', ra['blobHash'])
"

# 3. /apps/elohim-host-landing/index.html serves the SPA
curl -sI https://alpha.elohim.host/apps/elohim-host-landing/index.html | head -3

# 4. /lamad gateway shell now bootstraps the SPA (visual; browser check)
echo "Open https://alpha.elohim.host/lamad in a browser; the gateway 'Connecting…' shell should advance to the lamad SPA within ~5 seconds."

# 5. / redirects and lands
curl -sIL https://alpha.elohim.host/ | grep -E "HTTP|location:"
```

All steps expected to pass. If step (1) shows the placeholder still on `elohim-host-landing` after a green App pipeline, the read-back assertion in `stageSpaBlob` must be broken — that's the most likely failure mode and is the highest-priority debug target.

---

## Task 6: Cross-doorway parity (when `elohim-doorway-alpha-b` lands)

This task is a **follow-up**, dependent on the separate "apex ingress unblock" operational work tracked at the end of `2026-05-23-landing-page-epr-dual-doorway.md`. Out of scope for this PR; included here as a checklist for after `elohim-doorway-alpha-b` is running and `elohim.host` resolves:

- [ ] **Step 6.1: Confirm alpha-b reads the same content rows**

```bash
curl -s https://elohim.host/health/startup | python3 -m json.tool | grep -E "ready|extracted|slug|blobHash"
curl -sI https://elohim.host/apps/elohim-host-landing/index.html | head -3
```

Expected: identical to alpha. Both doorways share the storage pool, so the single blob update covers both surfaces.

- [ ] **Step 6.2: Federation peer discovery**

```bash
curl -s https://alpha.elohim.host/admin/federation/peers
curl -s https://elohim.host/admin/federation/peers
```

Expected: each surface lists the other.

- [ ] **Step 6.3: Run a2o feature**

```bash
cd /projects/elohim
# From CI on the next deploy, or locally with hc:start
ls genesis/a2o/features/protocol/landing-page-dogfood.feature
```

If the feature runs green in the genesis pipeline post-deploy, this task is done.

---

## What's intentionally NOT in this plan

- **The PUT verb.** The route registers PATCH because PATCH is what the existing handler already implements (partial update semantics, not replace). The plan's curl uses PATCH to match.
- **Migrating the stale `elohim-host-landing.blobHash` placeholder via a one-shot script.** The first App pipeline run with the patched `stageSpaBlob` overwrites the row correctly; no separate migration is needed. If alpha is in a state where someone needs the row clean BEFORE the next App run, an operator can run the same curl from a jump pod — but that's a workaround, not a plan step.
- **The `elohim.host` 503 / `elohim-doorway-alpha-b` missing deploy.** Tracked separately. Hostname conflict with `elohim-prod/elohim-site-ingress` needs ops resolution before alpha-b's `kubectl apply` will succeed.
- **Auth model for the new PATCH route beyond "X-API-Key like DELETE".** If/when the project graduates to JWT-only admin operations, this route should follow whatever pattern the doorway admin routes adopt. Out of scope for this fix.
- **A separate landing-page bundle distinct from the elohim-app build.** Current architecture dogfoods the same bytes per the landing-page plan; not revisited here.

---

## Rollback

If the deploy step fails irrecoverably:

```bash
cd /projects/elohim
git log --oneline -5    # find the pre-plan HEAD
git revert --no-commit HEAD~2..HEAD
git commit -m "revert: SPA blob deploy drift fix (rollback)"
git push origin dev
```

The orchestrator re-dispatches with the reverts; the API loses the PATCH route; `stageSpaBlob` falls back to the silent-broken PUT. State on alpha is unchanged from pre-plan (rows still hold the placeholder; doorway still shows the gateway shell). No data is lost.

---

## Why this matters

The landing-page dual-doorway work is the protocol dogfooding itself — Matthew-stewarded EPR projected through the protocol's own content-addressing path. When the deploy silently fails to link the bytes to the row, every visitor to `alpha.elohim.host/` sees the four-stage bootstrap shell stall, and the protocol's first-impression surface communicates "this is broken." The fix is small (one route line, one InputView field, one Jenkinsfile rewrite) but the regression seatbelt — the read-back assertion in `stageSpaBlob` — is the load-bearing piece: it ensures any future drift between the CI write contract and the storage API surfaces as a red build, not as a silently-broken production surface.
