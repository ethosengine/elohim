---
title: Doorway Auth Refusal — Operating Runbook
id: doorway-auth-refusal-runbook
tier: architecture
status: accepted — companion to the auth-posture canon; decision trees written from two real mis-attributions, probe technique proven 2026-08-24 on the local mesh
created: 2026-08-25
pillar coupling: doorway (web2 projection), elohim (substrate boundary)
informed-by:
  - The apex seed 403 (app #1672/#1673) and its June predecessor (401, ~2 weeks stale) — the same class, mis-attributed both times
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md — the sibling runbook whose shape this follows
informs:
  - Any operator or agent reading a doorway refusal, a stale-but-200 host, or a red deploy seed leg
cites:
  - "doorway-auth-posture-declared-stage | the canon half of this pair — the rule this runbook operationalizes; read it for WHY a refusal is shaped this way, come back here for what to do about one | sha256:31cda806d3207fd9 | path: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md"
  - "substrate-trust-contract-runbook | the sibling runbook whose shape this follows, and the doc you are probably in by mistake if your symptom is a doorway refusal — that one owns dataplane divergence, this one owns write-path authorization; section 3d is the boundary between them | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "doorway-access-tier-patterns | the read-side tier catalog — consult it when the refusal is a reader being denied content rather than a writer being denied a seed, which is a different gate with a different cure | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md"
---

# Doorway Auth Refusal — Operating Runbook

**Read this when:** a deploy seed leg is red, a host serves an old bundle while
returning 200, a local seed suddenly demands a credential, or you are about to
file a doorway refusal as a dataplane divergence.

The **why** lives in `doorway-auth-posture-declared-stage`. This is the **what
to do**. The one thing to carry over from it: authority derives from the
declared stage, never from `DEV_MODE`.

---

## 1. The invariants (what you may now assume)

| # | Invariant |
|---|---|
| **A1** | Seed authority is decided by `AppState::network_stage` (`ELOHIM_NETWORK_STAKES`), resolved ONCE at boot, fail-closed to `Bootstrap`. `DEV_MODE` decides nothing on this path. |
| **A2** | The two refusals are DISTINCT answers, not a severity gradient. **401** = nothing resolved above Public. **403** = a real identity resolved, below Admin. Reading 403 as "bad credential" is the single most common mis-step — it means a credential was *accepted and found insufficient*. |
| **A3** | An undeclared or blank `API_KEY_SEED` means the doorway has NO fleet seed authority — not an empty one that an empty header matches. |
| **A4** | `peer_is_loopback` comes from the accepted socket, never `X-Forwarded-For`. Behind an ingress the peer is the ingress pod, so a cluster doorway always authenticates. You cannot spoof your way to loopback with a header. |
| **A5** | The two pre-coordination affordances (loopback, fleet key) switch off when a doorway declares `coordinated`. If seeding stops working right after a stage declaration, that is A5 firing correctly, not a regression. |
| **A6** | **A doorway refusal is not a dataplane divergence.** The byte plane refusing a write and the head plane disagreeing about a head are different failures with different cures. See §3d — they have been confused twice. |
| **A7** | The fleet's `api-key-admin` / `api-key-authenticated` / `jwt-secret` values are committed plaintext in `genesis/orchestrator/manifests/doorway/*.yaml` and applied VERBATIM. The repo value IS the live value. There is no sealed-secret controller and no injection path. |

---

## 2. The probes (how each is watched)

| Probe | Surface | Watches |
|---|---|---|
| **Wrong-hash auth probe** (below) | on demand | whether the gate ADMITTED you, isolated from whether the write would succeed |
| `GET {doorway}/status.json` | on demand | the declared stage **and its provenance** — `OperatorConfig` vs `BootstrapDefault` distinguishes a declared stage from an applied default (A1) |
| `seed <host> <bundle> (browser\|server)` legs | every App deploy console | the deploy path's own verdict, per host, per bundle |
| `x-elohim-freshness` response header | on demand | `amber` = serving a last-reconciled bundle. A host can be 200-and-wrong (§3b) |
| `✓/⚠ canonical head propagated` | every App deploy console | the DECLARE leg — which is **not** seed-gated, so it can succeed while the byte seed is refused (§3d) |

### The wrong-hash auth probe

The gate runs on headers only and refuses **before** the body is read; content
addressing then refuses a mismatched write. So a `PUT /admin/seed/blob`
carrying a deliberately wrong `X-Blob-Hash` and an empty body separates the two
questions cleanly:

- **409 `Hash mismatch`** → **the gate ADMITTED you.** Auth is not your problem.
- **401 / 403** → the gate refused; go to §3a.

Nothing is written in either case — the 409 arm is refused by content
addressing, which is precisely why this probe is safe to run against a live
host. This is the technique that proved the pre-`62b658784` hole on the local
mesh (2026-08-24): an anonymous PUT answered 409, i.e. it had passed the gate
and only content addressing stopped it.

> Note for agents: sending a credential to a live host may be blocked by
> permission policy in some sessions. If you cannot run it, do not infer the
> result — read the manifests (A7) and the deploy console instead, and say the
> probe was not run.

---

## 3. The runbook (what to do when a probe reds)

### 3a. A deploy seed leg is red (401 or 403)

Run in order; stop at the first hit.

1. **Which refusal?** (A2) **401** → no credential arrived: check the CI step
   actually sends `X-API-Key` and that the credential resolved in Jenkins (the
   App pipeline falls back `storage-api-key-admin` → `doorway-admin-bootstrap-key`;
   a missing binding surfaces as an empty value, not an error). **403** → a
   credential arrived and was insufficient: continue.
2. **Right question, wrong key?** (A7) Compare what CI sends against **both**
   `api-key-admin` AND `api-key-seed` for *that host* in
   `genesis/orchestrator/manifests/doorway/`. A key that works on one doorway
   and 403s on another is the admin-vs-fleet conflation — the deploy path wants
   `API_KEY_SEED`, which is uniform across the fleet; `API_KEY_ADMIN` is
   deliberately per-doorway.
3. **Is the seed authority declared on that doorway at all?** (A3) No
   `API_KEY_SEED` env in its manifest → only that doorway's own Admin identity
   can seed. This is the intended posture for prod and staging-read.
4. **Has the doorway moved past pre-coordination?** (A5) `status.json` shows
   `coordinated` or `enforced` → the fleet key is refused BY DESIGN. The cure is
   not to re-open it; it is to give the pipeline a bounded authority (the
   successors are named in the posture doc).
5. **Is the running binary the one you think?** A doorway image predating
   2026-08-25 ignores `API_KEY_SEED` entirely (unknown env vars are simply not
   read). Half-deploy states are safe but silent — see §3c.
6. **Escalate** with: the exact status code, the host, that host's manifest
   env block, and the deploy console line.

### 3b. A host returns 200 but serves an old bundle

This is the shape a refused seed takes **downstream** — the host is healthy and
serving its last-reconciled bundle, so nothing about the response says
"refused".

1. Check `x-elohim-freshness`. `amber` = last-reconciled, not current.
2. Go read that host's `seed …` legs in the **App** deploy console. A stale
   apex with green legs is a different problem; a stale apex with red legs is
   §3a and everything else is downstream noise.
3. Do NOT open a dataplane investigation before doing (2). The apex spent
   ~2 weeks stale in June and two builds stale in August with the answer
   sitting in the seed legs the whole time.

### 3c. Half-deployed states (both are safe; neither announces itself)

| State | Behaviour |
|---|---|
| New manifest, old image | `API_KEY_SEED` is set in the pod env and never read. Seeding fails exactly as before. |
| New image, old manifest | No seed key declared → falls back to Admin identity (A3). Seeding fails exactly as before. |

Neither corrupts anything, and neither is distinguishable from the original
failure without checking the image tag and the pod env. If a fix "didn't take",
check both before re-diagnosing.

### 3d. Auth is fine but the head is wrong — the ungated declare

**Known gap, filed not fixed.** `POST /db/content/{slug}/canonical-head` does
**not** call `require_seed_authority`. So the declare fan-out can succeed
against a doorway whose byte seed was just refused, leaving it declaring a head
whose bytes it does not hold.

If a doorway declares a head it cannot serve: check whether its **byte** seed
leg was red in the same build (§3a). This presents as a dataplane divergence and
is not one (A6) — the head plane did its job; the byte plane was refused.

---

## 4. Change discipline (what a maintaining agent may touch)

- **Never** re-open an affordance to make a deploy pass. That is what
  `DEV_MODE: "true"` on alpha-b was, and it hid the real gap for months while
  carrying its own TODO to remove it.
- **Never** widen a gate to make a probe green. If the fleet key is refused,
  A5 may be firing correctly.
- A new affordance must say in the predicate what makes it stop applying, and
  must be narrower than Admin unless it genuinely needs an identity.
- Changing an auth predicate means updating its `seam-registry.yaml` row —
  `require_seed_authority` is registered, with C12 `partial` and C8 `unbound`
  stated honestly. Do not upgrade those statuses without the binding work.

---

## 5. Why this doc exists

The same failure class has now been mis-attributed twice, both times as a
content/dataplane problem when it was an authorization refusal on the write
path:

- **2026-06-27** — doorway-B 401'd every byte seed; the apex served a stale
  bundle for ~2 weeks (`scripts/ci/stage-spa-blob.sh` records it). Papered over
  by setting `DEV_MODE: "true"` on doorway-B.
- **2026-08-25** — that bypass was correctly removed, the same class returned as
  403 across app #1672/#1673, and it was first filed as *"an operator credential
  decision for the second premises"*. It was not: the values were plaintext
  fixtures in the repo the whole time (A7).

Both cost far more than the cure. What was missing was not a fix but a
**decision tree between a refusal and a divergence** — §3b and §3d are that
tree. Where this doc and live behaviour disagree, the probes in §2 are the
authority.
