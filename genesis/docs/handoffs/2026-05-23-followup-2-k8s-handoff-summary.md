# Follow-up Prompt 2 — K8s handoff summary + operator action

**Created:** 2026-05-23 from the alpha-landing-page dual-doorway shift.
**For:** A fresh session that picks up after the operator has acted on the credential blocker.
**Owner role:** ci-investigator or general agent — read-only diagnosis plus optional cleanup commits.

---

## Context

The prior shift drove the `alpha.elohim.host` dual-doorway landing-page work as far as it could without operator action on Jenkins credentials. Storage code, CI pipeline code, and ingress-check script fixes are all merged to `origin/dev` (tip `c8b98ed90`). The cluster has the new storage image. **Only thing missing: a Jenkins Global-scope credential** so the CI pipeline can PATCH content rows on the storage layer.

---

## What landed in code (origin/dev = c8b98ed90)

| Commit | What |
|---|---|
| `282e17371` | `feat(views): UpdateContentInputView accepts blobHash for partial updates` |
| `f70474ab9` | `feat(storage): register PATCH /db/content/{id} + plumb blob_hash through` |
| `d6b45e10c` | `feat(blob): stageSpaBlob writes via PATCH with auth + read-back verification` |
| `0afe4c60a` | `fix(ci): rename SPA blob credential to storage-api-key-admin` |
| `6ce1d1272` | `chore(ci): force-rebuild Edge to deploy storage PATCH route [build:edge]` |
| `c8b98ed90` | `fix(ci): credential fallback + ingress-check exact-host match [build:app]` |

---

## What's verified on the cluster

- **elohim-storage `1.0.0-dev-6ce1d127`** deployed to all 14 alpha edge nodes (matthew, jessica, adam, james, pete, frank, terrance, gertrude, susan, caleb, daniel, emma, eve, nancy) per Edge #992 log (`statefulset rolling update complete` × 14).
- **doorway-alpha primary** rolled out successfully.
- **`elohim.host` apex confirmed FREE** by the operator (`kubectl get ingress -A | grep elohim.host` returns only subdomain matches). cert-manager will mint `elohim-host-apex-tls` the moment alpha-b's ingress applies.

---

## What's blocked (operator action required)

### Critical — Jenkins Global credential

In Jenkins UI: **Manage Jenkins → Credentials → System → Global credentials (unrestricted) → Add Credentials**.

| Field | Value |
|---|---|
| Kind | Secret text |
| Scope | **Global** (not folder-scoped) |
| Secret | `dev-elohim-admin-2024` |
| ID | `storage-api-key-admin` |
| Description | "Admin key for storage PATCH/POST endpoints (per spa-blob-deploy-drift plan)" |

**Then create a second identical one** (same value) with ID `doorway-admin-bootstrap-key` — this fixes a separate silently-degraded bug in elohim-genesis seed (see follow-up 3).

**Verify:** Both credential IDs appear in `Manage Jenkins → Credentials → System → Global credentials` after adding.

### Why both?

`storage-api-key-admin` is what the new App pipeline's `stageSpaBlob` tries first. `doorway-admin-bootstrap-key` is what `genesis/Jenkinsfile:997` and `:1035` reference. The fallback in `Jenkinsfile:870-893` accepts either, but the genesis seed expects specifically `doorway-admin-bootstrap-key`. Adding both with the same value covers both code paths.

### Why Global, not folder?

The pipelines that need it (`elohim/dev`, `elohim-genesis/dev`) live in separate folders. A folder-scoped credential at one folder isn't visible at the other. Global scope is the cleanest fit for an admin key used across multiple pipelines.

---

## What to do after the credential lands

1. **Trigger an App rebuild:**
   ```bash
   git commit --allow-empty -m "ci: retrigger App after credential add [build:app]"
   git push
   ```
   (Or open a PR / merge — webhook will fire.)

2. **Watch the build.** Specifically watch for the new `Upload SPA Blob` stage's echo:
   ```
   stageSpaBlob auth: using credential 'storage-api-key-admin'
   ```
   If you see this echo and no `ABORT`, the credential is wired and the PATCH happened.

3. **Smoke-test alpha.elohim.host:**
   ```bash
   curl -sf https://alpha.elohim.host/health/startup | jq '.rootApp'
   ```
   Expect `rootApp.ready: true` AND `rootApp.blobHash: sha256-<not-placeholder>`.

4. **If `rootApp.blobHash` is still PLACEHOLDER after a successful Upload SPA Blob:** see follow-up 1 (doorway warm-stream architecture). The CI patch only updates matthew; other peers stream stale data into the projection cache.

5. **Trigger an Edge rebuild** to land the second doorway:
   ```bash
   git commit --allow-empty -m "ci: deploy alpha-b after apex freed [build:edge]"
   git push
   ```
   The ingress-check exact-host fix is already on dev; alpha-b should pass precondition now.

---

## Open items (not blocking)

- **Doorway warm-stream architecture** — see `followup-1-doorway-warm-stream-architecture.md`.
- **Genesis seed admin-promotion degraded** — see `followup-3-genesis-seed-admin-promotion-degraded.md`. This may have been silently degraded for some time; check whether retroactive cleanup is needed once the credential is added.
- **Workspace-loss flake on App pipeline** — App #1454 lost workspace on `elohim-dev-1454-rzcv3-xgdfj-53mfm` mid-build. Recoverable via retrigger but worth a watch for recurrence patterns on the build pods.

---

## Constraints for this session

- This handoff is **read + verify + minimal cleanup commits**. No new feature work.
- Do not push without the operator's confirmation that the credential is in place. The fallback ABORT is well-tested — pushing again before the credential lands burns another 25-min App build for nothing.
- If alpha goes green, file a one-line summary and close the alpha-landing-page shift. Then graduate to follow-up 1 (architecture work).

---

## Related artifacts

- Prior shift journal: `.claude/shifts/2026-05-23T05-25-alpha-landing-page-dual-doorway.journal.md`
- Plan that drove the prior shift: `genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md`
- Plan addendum (and the dev's response disputing parts of it): same file, sections "ADDENDUM" and "DEV RESPONSE"
