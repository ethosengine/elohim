# Deploy Coherence — Atomic A+B Rollout + Genesis-Pair Partition Guard — Implementation Plan (F-DEPLOY)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed. Authored against the Federation Web2 Contract Ledger (`/projects/elohim/FEDERATION-WEB2-LEDGER-2026-06-14.md`) and the P2P-Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`). House style mirrors `2026-06-14-dataplane-diagnostic-plan.md`.

## 1. Context / why + the A/B-divergence facet it closes

**Goal:** make a divergent genesis pair UN-SHIPPABLE and a one-down edge VISIBLE at deploy time. Today the two doorways (doorway-alpha → MATTHEW; doorway-alpha-b/apex → ADAM) are deployed **serially with no joint barrier**, each rollout's failure settles **UNSTABLE not FAILURE**, and the matthew↔adam DNA-hash / coordinator-update flag coherence that keeps them on the SAME DHT lives **only in prose** (`adam-firstman.yaml:238` comment; `ALLOW_COORDINATOR_UPDATE` set NOWHERE as env). The result is the operator's confirmed symptom: two hostnames serving different deploy-version SHAs (`e0352a7`/`8a2c65e` — `DEPLOY_VERSION`, not content CIDs), one edge crash-looping, **no deploy verdict that says "the pair diverged."**

This plan closes the **deploy facet** of the A/B-divergence problem:
- **Version-skew window** — serial `kubectl apply` + per-deployment `rollout status` with no "both at same `DEPLOY_VERSION`" gate (CONFIRMED, `Jenkinsfile:776-803`, `:755-757`). → one **joint post-rollout barrier**.
- **One-down invisibility** — `catchError(buildResult:'UNSTABLE')` on each doorway case (`Jenkinsfile:781-800`) makes a never-Ready edge report green-enough. → divergence/partition is **FAILURE**, single-human flap stays UNSTABLE.
- **Genesis-pair flag drift** — `allowDnaReinstall` computed per-human independently (`Jenkinsfile:579`), `ALLOW_COORDINATOR_UPDATE` invisible (relies on binary default). → explicit env + a **render-time `gatePairCoherence`** assertion.

Detection of *served-head* divergence (the cross-edge EPR head compare) is **F-COHERENCE's** deliverable; this plan **CONSUMES** F-COHERENCE's `/api/v1/federation/coherence` endpoint in `verify-pair-coherence.sh` and **CONSUMES** P-DIAGNOSTIC's served-head exposure — it does not build a detector.

The facet this plan does NOT own: cross-edge head DETECTION (F-COHERENCE), bootstrap islanding ROOT FIX (F-BOOTSTRAP), `/p2p-peers` + CDN (F-EDGE), the doorway breaker/backoff (dataplane P-DEFENSE).

---

## 2. OWNED FILES (verbatim from federation ledger §2) + collision statement

**MUTATE (M):**
- `elohim/holochain/Jenkinsfile` — **SOLE owner of all doorway-deploy sequencing** (FS7). New `deployGenesisPairAtomic` + `gatePairCoherence` Groovy helpers; refactor `deployDoorwaysWithTestShape` (`:776`) into a joint Ready+coherence barrier. **Absorbs F-EDGE's `ATOMIC_DOORWAY_ROLLOUT` skew gate** as a sub-assertion (C-DEPLOY / CN4). **Heredoc-free** — bash bodies go in `scripts/ci/` per the Jenkinsfile size rule. Hand-off note: P-PROOFS owns `elohim/holochain/dna/Jenkinsfile:1375` (a DIFFERENT file).
- `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` — **SOLE owner** of the NEW `ALLOW_COORDINATOR_UPDATE` env lines (today only a COMMENT references it, `:255`; the env block is `:258`).
- `genesis/orchestrator/manifests/humans/adam-firstman.yaml` — **SOLE owner** of the NEW `ALLOW_COORDINATOR_UPDATE` env lines (after the hand-set `ALLOW_DNA_REINSTALL` at `:238`).
- `genesis/orchestrator/manifests/doorway/alpha-b.yaml` — **apex-failover / fail-loud posture** lines ONLY (Ingress `rules` / failover-backend region + affinity comment). Per C-MANIFEST: F-EDGE owns `annotations` (cache/LB), F-DEPLOY owns `rules`/failover-backend posture, F-BOOTSTRAP owns the `env` `BOOTSTRAP_MONGODB_DB`. **Additive-disjoint** — no shared YAML key.

**CREATE (C):**
- `genesis/orchestrator/scripts/verify-pair-coherence.sh` — **SOLE owner** (FS6). `<doorwayA_base> <doorwayB_base>` → diff `/health` conductor DNA hash + the F-COHERENCE coherence endpoint (served head digest + `buildId`); exit≠0 = pair FAILURE.
- `scripts/ci/deploy-pair-barrier.sh` — **SOLE owner** (NEW). The bash body the Jenkinsfile barrier calls (`rollout status` polling + `verify-pair-coherence.sh` invocation), kept out of the CPS method to respect the 64KB/heredoc-free rule.

**Collision statement.** Every file above is SOLE-owned by F-DEPLOY OR is the additive-disjoint shared `alpha-b.yaml` whose F-DEPLOY region (Ingress `rules`/failover posture) shares NO YAML key with F-EDGE's `annotations` region or F-BOOTSTRAP's `env` region (C-MANIFEST). This plan touches:
- **No file owned by another FEDERATION plan** — it CONSUMES F-COHERENCE's `routes/coherence.rs` endpoint over HTTP (never edits it); does NOT touch `routes/federation.rs` (F-EDGE), `services/federation.rs` (F-COHERENCE), `bootstrap/*` (F-BOOTSTRAP), or `routes/coherence.rs`.
- **No file owned by any DATAPLANE plan** — verified against dataplane Ledger §2: F-DEPLOY's files (`elohim/holochain/Jenkinsfile`, the three manifests, the two scripts) appear NOWHERE in the dataplane file map (dataplane owns `elohim-storage/*`, `elohim-compute/*`, `steward/*`, DNA, `sdk/schemas/*`, and `dna/Jenkinsfile:1375` for P-PROOFS — a different Jenkinsfile). It CONSUMES P-DIAGNOSTIC's served-head field over HTTP only.

---

## 3. NEW PRIMITIVES owned + CONSUMED (skip-if-present)

**skip-if-present rule (verbatim):** *"Before landing this type, verify the named owner module already exposes it. If present, VERIFY-ONLY (import + use). If absent at your integration point, land the owner plan's verbatim definition only as a temporary local shim, flag it in your plan's hand-off notes, and delete the shim when the owner lands."*

### OWNED (this plan defines)

| Primitive | Home | Shape / signature |
|---|---|---|
| `def deployGenesisPairAtomic` | `elohim/holochain/Jenkinsfile` | `(String env, String envFile, List allHumans)` — deploy matthew+adam (reuse `deployHumansInParallel` primitive), then both doorways (reuse `deployDoorwayManifest`), THEN ONE joint barrier: both Ready (`rollout status`) AND `verify-pair-coherence.sh` equal DNA hash + equal served head + equal `DEPLOY_VERSION`. Divergence = **FAILURE** (not UNSTABLE). Non-alpha envs (single doorway) skip the barrier. |
| `def gatePairCoherence` | `elohim/holochain/Jenkinsfile` | `(Map humanMatthew, Map humanAdam)` — assert `allowDnaReinstall` (`Jenkinsfile:579` formula) AND the new `allowCoordinatorUpdate` resolve IDENTICALLY for both at RENDER time; `error` if not. ~15 lines, no live calls. |
| `verify-pair-coherence.sh` | `genesis/orchestrator/scripts/` | `<doorwayA_base> <doorwayB_base>` → curl `/health` (`.conductor.dnaHash` once exposed; today `.conductor.connected`/`.healthy`) + the F-COHERENCE coherence endpoint (`.digest`, `.buildId`); diff; exit≠0 = pair FAILURE. WARN-only on the head-leg until X-DEPLOY-DIAG/FS1 land. |
| `scripts/ci/deploy-pair-barrier.sh` | `scripts/ci/` | bash body called by `deployGenesisPairAtomic` (rollout-status poll for both doorways + invoke `verify-pair-coherence.sh`). |
| `ALLOW_COORDINATOR_UPDATE` env line | template + adam manifests | `- name: ALLOW_COORDINATOR_UPDATE` / `value: "ALLOW_COORDINATOR_UPDATE_PLACEHOLDER"` (template) and hand-set `"true"` (adam). Stops relying on the binary's `ALLOW_DNA_REINSTALL`-default; makes the coordinator-hotswap class manifest-visible. |
| `ALLOW_COORDINATOR_UPDATE_PLACEHOLDER` sed + `allowCoordinatorUpdate` Groovy | `elohim/holochain/Jenkinsfile` | `def allowCoordinatorUpdate = (humanConfig.env == 'prod') ? 'false' : 'true'` + `"s\|ALLOW_COORDINATOR_UPDATE_PLACEHOLDER\|${allowCoordinatorUpdate}\|g"` in `sedArgs` (`:614` region). |

### CONSUMED (skip-if-present)

| Consumed primitive | Owner plan | Edge | Skip-if-present behavior |
|---|---|---|---|
| `GET /api/v1/federation/coherence` → `CoherenceManifest` (`digest`, `build_id`/`buildId`) | **F-COHERENCE** (FS1) | HARD | `verify-pair-coherence.sh` curls this for the served-head equality leg. If the endpoint 404s (F-COHERENCE not landed), the script's head-leg WARNs (`echo "WARN: coherence endpoint absent — head-equality leg skipped"`) and exits 0 on that leg; the DNA-hash + `DEPLOY_VERSION` legs still gate. No shim possible (HTTP endpoint, not a type). |
| served EPR head + `dnaHash` on `/health` (`P2PHealth` / `main.rs:483-503`) | **dataplane P-DIAGNOSTIC** (RESOLUTION-G) | HARD (cross-layer) | If the head field is NOT yet on `/health`, the script reads it from the F-COHERENCE endpoint instead (FS1 supplies the same digest). F-DEPLOY CONSUMES, never ADDS the field to `/health`. Head-equality leg WARN-only until exposed (X-DEPLOY-DIAG). |
| `deployHumansInParallel` / `deployDoorwayManifest` / `Verify Target Health` /health-parse loop | already-shipped (ledger §6) | n/a | REUSE as primitives; `deployGenesisPairAtomic` composes them. VERIFY-ONLY; never re-author. |

**No new Rust crate. No DHT entry. No new HTTP route owned by this plan.**

---

## 4. DEPENDENCY EDGES (intra-federation + cross-layer)

Edges read **A → B** = "A depends on B". **HARD** = A cannot fully function/gate-green without B. **SOFT** = works standalone, correct once B lands.

### Intra-federation
| Edge | Type | Reason |
|---|---|---|
| F-DEPLOY → F-COHERENCE | **HARD** | `verify-pair-coherence.sh` (FS6) asserts equal *served head* by curling F-COHERENCE's `/api/v1/federation/coherence` (FS1). Until FS1 exists the head-equality leg is WARN-only; the DNA-hash + `DEPLOY_VERSION` legs and the whole `gatePairCoherence` render-time guard work standalone. |
| F-DEPLOY → F-BOOTSTRAP | **SOFT** | the partition guard is most valuable once islanding is fixed; the guard itself (DNA-hash + flag-coherence) works independent of bootstrap sharing. |

### Cross-layer (federation → dataplane)
| Edge id | → Dataplane track | Type | Reason |
|---|---|---|---|
| **X-DEPLOY-DIAG** | **P-DIAGNOSTIC** (served-EPR-head on `/health`/`P2PHealth`, RESOLUTION-G) | **HARD** | `verify-pair-coherence.sh` asserts equal served head; the head must be EXPOSED on a read-model. P-DIAGNOSTIC owns `/health` / `main.rs:483-503` / `health.rs:77`. F-DEPLOY reads it (or the F-COHERENCE FS1 endpoint), never adds it. Head-leg WARN-only until exposed; flip to FAILURE in Wave F3. |
| **X-DEPLOY-DEF** | **P-DEFENSE** (`UpstreamBreakers`/shed, S11) | **SOFT** | the apex-failover ingress posture MUST NOT re-implement shedding/backoff — it complements the shipped doorway breaker. Posture is detection + fail-loud, not a parallel circuit. |
| **X-DEPLOY-ARC** | **P-RECONCILE / P-ARC** (bootstrap-island / DNA-hash convergence) | **SOFT** | genesis-pair DNA-hash coherence is the apply-time analog of the dataplane's bootstrap-island concern — no file overlap; complementary. |

**Cycle check:** F-DEPLOY is a terminal consumer (HARD on F-COHERENCE FS1 + P-DIAGNOSTIC head; nothing depends HARD on F-DEPLOY). No cycles.

---

## 5. Task-by-task TDD

Build/test discipline (memory): manifest + Jenkinsfile + shell tasks are **doc/lint-only** (no Rust build tonight, no `kubectl` ever — operator applies). For shell, validate with `bash -n` (parse) + `shellcheck` if present. For Groovy, the Jenkinsfile pre-commit hook is `genesis/orchestrator/scripts` adjacent — verify the method-size hook stays green (no heredocs). No Rust crate is touched by this plan, so no `cargo` commands; the CONSUMED contracts are verified by reading F-COHERENCE's/P-DIAGNOSTIC's plans, not by compiling.

> **Operator-apply note:** the edge Jenkinsfile is operator-owned per CLAUDE.md. Every task here describes a CHANGE and commits it; the integrator/operator applies. No build is RUN tonight.

### TASK 1 — Explicit `ALLOW_COORDINATOR_UPDATE` env (template + adam) [S · HIGH] — pure manifest

Files: `_edgenode-consolidated.template.yaml`, `adam-firstman.yaml`, `elohim/holochain/Jenkinsfile` (sed + Groovy).

- [ ] "Failing test" = a grep assertion that the env is ABSENT today (the gap): `grep -L "ALLOW_COORDINATOR_UPDATE" genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` returns the file (no env). Record this as the red baseline in the commit body.
- [ ] In `_edgenode-consolidated.template.yaml`, AFTER the `ALLOW_DNA_REINSTALL` block (`:258-259`), add:
```yaml
            # Coordinator-zome-only changes never move the DNA hash; they are
            # healed via the conductor's update_coordinators HOT-SWAP. Set
            # EXPLICITLY (was relying on the binary defaulting to
            # ALLOW_DNA_REINSTALL) so the coordinator-hotswap class is
            # manifest-visible and the genesis pair's value is auditable.
            - name: ALLOW_COORDINATOR_UPDATE
              value: "ALLOW_COORDINATOR_UPDATE_PLACEHOLDER"
```
- [ ] In `adam-firstman.yaml` (hand-rendered, no sed), AFTER `ALLOW_DNA_REINSTALL` (`:238-239`), add the same env hand-set to `"true"` with a comment ("genesis pair with matthew — both MUST resolve identically").
- [ ] In `elohim/holochain/Jenkinsfile`, after `def allowDnaReinstall = ...` (`:579`), add `def allowCoordinatorUpdate = (humanConfig.env == 'prod') ? 'false' : 'true'`; and in `sedArgs` (`:614` region) add `"s|ALLOW_COORDINATOR_UPDATE_PLACEHOLDER|${allowCoordinatorUpdate}|g",`.
- [ ] Verify (lint-only): `grep -n "ALLOW_COORDINATOR_UPDATE" genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml genesis/orchestrator/manifests/humans/adam-firstman.yaml elohim/holochain/Jenkinsfile` — confirm env present in both manifests AND a matching `_PLACEHOLDER` resolver in the Jenkinsfile (a placeholder with no resolver trips the existing `grep -c '_PLACEHOLDER'` post-sed check at `Jenkinsfile:742` → "Unresolved placeholders" error). **Both the env line and the sed must land together.**
- [ ] Commit (selective-stage the 3 files): `feat(deploy): explicit ALLOW_COORDINATOR_UPDATE env (manifest-visible coordinator-hotswap class)`.

### TASK 2 — `gatePairCoherence` render-time flag-equality guard [S · HIGH] — Groovy

Files: `elohim/holochain/Jenkinsfile`.

- [ ] "Failing test" = there is no cross-check today that matthew & adam resolve the DNA/coordinator flags identically (the invariant lives in `adam-firstman.yaml:236` prose). Record as red baseline.
- [ ] Add helper (heredoc-free, ~15 lines):
```groovy
/**
 * Genesis-pair partition guard. matthew (on-prem) + adam (shem) MUST land the
 * SAME DNA hash and the SAME coordinator-update posture or they partition onto
 * different DHTs (CLAUDE.md "DNA changes don't redeploy" warning). adam is a
 * hand-rendered manifest while matthew is template-sed'd, so the two flag values
 * are computed on DIFFERENT code paths — assert they agree at render time.
 */
def gatePairCoherence(Map humanMatthew, Map humanAdam) {
    def reinstall = { h -> (h.env == 'prod') ? 'false' : 'true' }
    def coord = { h -> (h.env == 'prod') ? 'false' : 'true' }
    if (reinstall(humanMatthew) != reinstall(humanAdam)) {
        error "Genesis-pair DNA-reinstall flag DIVERGED: matthew=${reinstall(humanMatthew)} adam=${reinstall(humanAdam)} — would partition the DHT. Refusing to deploy."
    }
    if (coord(humanMatthew) != coord(humanAdam)) {
        error "Genesis-pair coordinator-update flag DIVERGED: matthew=${coord(humanMatthew)} adam=${coord(humanAdam)}. Refusing to deploy."
    }
    echo "✅ Genesis-pair flag coherence: reinstall=${reinstall(humanMatthew)} coord=${coord(humanMatthew)} (matthew == adam)"
}
```
  (NOTE: adam's value is hand-set in its manifest, NOT computed by this formula — the guard asserts the *intended* values agree; a future hardening reads adam's literal from the manifest. Document this limitation inline so no one over-trusts it. SEAM-DELTA below.)
- [ ] Call `gatePairCoherence` from `deployGenesisPairAtomic` (Task 3) BEFORE any apply.
- [ ] Verify (lint-only): method-size hook stays green (helper is heredoc-free, < 11000 bytes). `bash -n`-equivalent for Groovy is the hook; confirm no `sh """..."""` added.
- [ ] Commit: `feat(deploy): gatePairCoherence — render-time genesis-pair flag-equality guard`.

### TASK 3 — `verify-pair-coherence.sh` + `deploy-pair-barrier.sh` [M · HIGH] — bash

Files: `genesis/orchestrator/scripts/verify-pair-coherence.sh`, `scripts/ci/deploy-pair-barrier.sh`.

- [ ] Write `verify-pair-coherence.sh` (jq-based; mirrors `genesis/Jenkinsfile:1489-1496` /health-parse idiom). Behavior:
  1. `curl -sf "$A/health"` + `"$B/health"`; extract `.conductor.dnaHash // empty` (the field P-DIAGNOSTIC exposes; today absent → empty). If both non-empty AND differ → `echo "FAIL: DNA hash diverged A=$dnaA B=$dnaB"; exit 1`.
  2. `curl -sf "$A/api/v1/federation/coherence"` + `"$B/..."`; extract `.digest`, `.buildId`. If endpoint missing (curl non-2xx) → `echo "WARN: coherence endpoint absent (F-COHERENCE not landed) — head-equality leg skipped"` (do NOT fail). If both present AND `.digest` differ → `echo "FAIL: served EPR head diverged A=$digA B=$digB"; exit 1`.
  3. Report `.buildId` skew as a NAMED line ("buildId A=$bidA B=$bidB" — the operator's `e0352a7`/`8a2c65e` symptom) regardless of pass/fail.
  4. exit 0 only when no FAIL leg tripped.
- [ ] Write `scripts/ci/deploy-pair-barrier.sh <doorwayA_base> <doorwayB_base> <namespace>`: poll `kubectl rollout status` for BOTH doorway deployments... **STOP** — `kubectl` is operator-only and never runs from dev; the barrier's rollout-status calls stay in the Groovy `deployDoorwayManifest` reuse (Groovy `sh "kubectl rollout status..."`), and `deploy-pair-barrier.sh` ONLY runs the `verify-pair-coherence.sh` coherence diff (curl, no kubectl). Rewrite the bash body to: `bash genesis/orchestrator/scripts/verify-pair-coherence.sh "$1" "$2"` with the two doorway base URLs, capturing exit code → propagate.
- [ ] "Failing test": `bash -n genesis/orchestrator/scripts/verify-pair-coherence.sh` must parse; run against two LOCAL stub URLs is infeasible tonight (no stack) — instead add a self-test mode `verify-pair-coherence.sh --selftest` that asserts the jq extraction logic on two canned JSON fixtures embedded as heredocs IN THE SCRIPT (script-local, not Jenkinsfile), proving: equal digests → exit 0; divergent digests → exit 1; absent coherence endpoint → exit 0 with WARN. Run `bash genesis/orchestrator/scripts/verify-pair-coherence.sh --selftest` and confirm the three cases.
- [ ] Verify: `bash -n` both scripts; `shellcheck` if available (`command -v shellcheck && shellcheck genesis/orchestrator/scripts/verify-pair-coherence.sh scripts/ci/deploy-pair-barrier.sh || echo "shellcheck absent — bash -n only"`).
- [ ] Commit: `feat(deploy): verify-pair-coherence.sh — genesis-pair DNA-hash + served-head equality gate (head-leg WARN until F-COHERENCE/P-DIAGNOSTIC land)`.

### TASK 4 — `deployGenesisPairAtomic` joint barrier (refactor `deployDoorwaysWithTestShape`) [M · MED] — Groovy

Files: `elohim/holochain/Jenkinsfile`.

- [ ] "Failing test" = the current shape (`Jenkinsfile:776-803`) deploys A then B serially with two independent `catchError(UNSTABLE)` and NO joint barrier — record the red baseline (a one-down edge passes the gate).
- [ ] Add `deployGenesisPairAtomic(String env, String envFile, List allHumans)`:
  1. (alpha-only) call `gatePairCoherence(matthew, adam)` from `allHumans` BEFORE any apply.
  2. Deploy doorway-A via `deployDoorwayManifest(env, envFile, allHumans)` (reuse).
  3. (alpha-only) deploy doorway-B via `deployDoorwayManifest('alpha', envFile, allHumans, 'genesis/orchestrator/manifests/doorway/alpha-b.yaml', 'elohim-doorway-alpha-b')` (reuse).
  4. The `deployDoorwayManifest` calls already do per-deployment `rollout status --timeout=300s` (`:757`) — that is the "both Ready" leg. AFTER both return, run the JOINT coherence barrier:
```groovy
    if (env == 'alpha') {
        def doorwayA = 'https://doorway-alpha.elohim.host'
        def doorwayB = 'https://elohim.host'
        def rc = sh(
            script: "bash '${env.WORKSPACE}/scripts/ci/deploy-pair-barrier.sh' '${doorwayA}' '${doorwayB}'",
            returnStatus: true,
        )
        if (rc != 0) {
            error "Genesis-pair coherence barrier FAILED (DNA-hash or served-head diverged between doorway-alpha and apex). This is the partition the operator must not ship past."
        }
    }
```
     **Severity = FAILURE (`error`), NOT UNSTABLE** — divergence is the partition (Open-decision #2 recommendation). The per-doorway `deployDoorwayManifest` keep their `rollout status` semantics; only the JOINT coherence verdict is hard-FAILURE. **F-EDGE's `ATOMIC_DOORWAY_ROLLOUT` skew gate is FOLDED HERE** (the `deploy-pair-barrier.sh` also reports `buildId`/`DEPLOY_VERSION` skew — CN4/C-DEPLOY) rather than as a separate flag.
- [ ] Repoint the Deploy Edge stage's `deployDoorwaysWithTestShape(...)` call to `deployGenesisPairAtomic(...)` (or have `deployDoorwaysWithTestShape` delegate to it so the junit-emit shape is preserved). Keep `emitDeployJunit` so the per-doorway test-report tab still surfaces which deploy failed.
- [ ] Verify (lint-only): method-size hook green (the new helper is heredoc-free — the only `sh` is a single-line `bash '...'` invocation, no inline heredoc); `grep -c 'sh """' ` on the new helper region = 0.
- [ ] Commit: `feat(deploy): deployGenesisPairAtomic joint Ready+coherence barrier (FAILURE on pair divergence; folds F-EDGE skew gate)`.

### TASK 5 — Apex-failover detection posture in `alpha-b.yaml` [L · MED] — manifest doc/lint

Files: `genesis/orchestrator/manifests/doorway/alpha-b.yaml` (Ingress `rules`/failover region + affinity comment ONLY — C-MANIFEST).

- [ ] Per Open-decision #1 recommendation (keep shem-pinned fail-loud, but make a down apex VISIBLE), the manifest change is **detection-affirming, not fallback**: keep the `requiredDuringScheduling` shem affinity (`:106-114`) and single ingress rule (no on-prem fallback backend — that would break the "two real backends" federation test). Add a comment block to the Ingress region documenting that DETECTION lives in `verify-pair-coherence.sh` + the cross-edge coherence gate (a synthetic head-equality probe), NOT in an ingress fallback. This keeps F-EDGE's `annotations` region (cache/LB) untouched.
- [ ] DEFER the actual synthetic-probe alert wiring (the live cross-edge head-equality probe) to operator decision (alert vs ingress change) — it CONSUMES P-DIAGNOSTIC's head field and F-COHERENCE's endpoint, so it cannot gate-green until both land. Capture the decision as a FOLLOW-ON seam, not a manifest change tonight.
- [ ] Verify: `python -c "import yaml,sys; yaml.safe_load(open('genesis/orchestrator/manifests/doorway/alpha-b.yaml'))"` (or `yamllint` if present) — confirm the comment additions keep the manifest valid YAML; confirm NO `annotations` key touched (F-EDGE's region) and NO `env` key touched (F-BOOTSTRAP's region).
- [ ] Commit: `docs(deploy): affirm apex fail-loud + name cross-edge coherence as the detection seam (alpha-b ingress)`.

---

## 6. p2p-class of new entities

Per `p2p-design-gate` — every new entity classified:
- `deployGenesisPairAtomic`, `gatePairCoherence`, `deploy-pair-barrier.sh`, `verify-pair-coherence.sh` — **Cat-C node-local / operational** (CI deploy-gate infra). No DHT entry, no table, no coordinator fn, no signal.
- `ALLOW_COORDINATOR_UPDATE` env — **Cat-C operational config**; it is a manifest env CONSUMING the already-shipped `Mishpat::Commitment` / `update_coordinators` binary path (dataplane S12, DNA cad5fb67c), NOT a new Cat-A notarized type. The rollout gate is CI infra, not a `Mishpat::Commitment`.
- The served-head digest read by `verify-pair-coherence.sh` is a PROJECTION of F-COHERENCE's `CoherenceManifest` (Cat-C) and P-DIAGNOSTIC's `P2PStatusInfo`-derived head (Cat-C) — consumed, not defined.

**No DHT entry. No notarized actuation owned here.** (Cite the class; do not re-litigate — dataplane ledger §p2p-design-gate precedent.)

---

## 7. // FOLLOW-ON seams (for the integration pass / named siblings)

1. **Head-equality leg WARN → FAILURE flip (X-DEPLOY-DIAG, Wave F3).** Once P-DIAGNOSTIC exposes the served head on `/health` (or F-COHERENCE's FS1 endpoint is live), flip `verify-pair-coherence.sh`'s head-leg from `echo WARN; exit-0-on-that-leg` to a hard `exit 1` on digest divergence. One-line change behind the dataplane landing.
2. **`gatePairCoherence` reads adam's LITERAL flag (hardening).** Today the guard asserts the *intended* `(env=='prod')?...` formula agrees for both — but adam's value is hand-set in `adam-firstman.yaml`, computed on a different path. A true guard parses adam's manifest env literal (`yq '.spec.template.spec.containers[].env[]|select(.name=="ALLOW_DNA_REINSTALL").value'`) and compares to matthew's rendered value. SEAM-DELTA (below) — not in either ledger.
3. **Synthetic cross-edge head-equality probe (apex-failover detection, Open-decision #1).** A live periodic probe (alert vs ingress change — operator decision) consuming P-DIAGNOSTIC's head + F-COHERENCE's `/api/v1/federation/coherence`. Largest, most cross-layer; deferred to operator. The `verify-pair-coherence.sh` logic is the reusable core.
4. **`detect-stale-resources.sh` on the doorway path.** Today it runs only on the human path (`Jenkinsfile:709`), so doorway version drift has no stale-detector. Adding it to `deployDoorwayManifest` is a one-line follow-on (out of this plan's confirmed-gap scope — the joint barrier supersedes its need for the genesis pair).
5. **`DEPLOY_VERSION`/`buildId` skew surfaced to the operator dashboard.** `verify-pair-coherence.sh` already prints the skew line; surfacing it in the doorway-app federation view (NAMED Angular sibling) closes the loop on the operator's "two heads" symptom.

---

## 8. Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch (`feat`/dev). The integrator/operator pushes/merges and APPLIES the Jenkinsfile + manifests (edge Jenkinsfile + cluster are operator-owned per CLAUDE.md — this plan describes CHANGES and commits them; nothing is applied or `kubectl`'d tonight).
- **Wave placement (federation ledger §7):** Tasks 1–2 (manifest env + `gatePairCoherence`) are INDEPENDENT and begin in **Wave F1**. Tasks 3–5 land in **Wave F2** (`verify-pair-coherence.sh` + `deployGenesisPairAtomic` + apex posture). The DNA-hash + `DEPLOY_VERSION` legs and the render-time guard gate-green in F2; the head-equality leg's WARN→FAILURE flip is the **Wave F3** sequenced hand-off behind P-DIAGNOSTIC (X-DEPLOY-DIAG) and F-COHERENCE FS1.
- **Selective-stage** each commit (concurrent sessions share the worktree per memory) — per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **Heredoc-free Jenkinsfile discipline is load-bearing** (the root + edge Jenkinsfiles sit near the 64KB CPS limit). All bash bodies live in `scripts/ci/*.sh` / `genesis/orchestrator/scripts/*.sh`; the only `sh` in new Groovy helpers is single-line `bash '<path>' args`. Verify the method-size pre-commit hook stays green before commit.
- **No build RUN tonight** — manifest/Groovy/shell are doc+lint only; `bash -n` + yaml-load + the method-size hook are the verification surface. No `cargo`, no `kubectl`.
