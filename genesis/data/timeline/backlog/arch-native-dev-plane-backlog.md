---
id: "backlog-arch-native-dev-plane-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Native dev plane: the cross-cutting take-items of the one-derivation, earned-reach design pass"
slug: "arch-native-dev-plane-backlog"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
  - "native-dev-track-a-eprfs-core-slice4-derivation-plane"
  - "native-dev-track-b-gate-oracle-via-brit-build-affected"
  - "native-dev-track-c-brit-signs-derivation-record-exact-byte-verify"
  - "native-dev-track-d-honesty-claimed-exit-partly-measured-failclosed"
  - "native-dev-track-e-tier-blob-store-slice1-local-chunked-put"
  - "native-dev-d3-fork-preserving-space-head-and-ff-only-mirror"
tags: [native-dev-plane, derivation, reach, rea, retire, integrity, cluster]
---

# Native dev plane: cross-cutting take-items

Source spec: `native-dev-plane-one-derivation-earned-reach-design`
(`genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md`).
The spec's §11 is the sequencing hub. This cluster is the single re-surfacing point for the design
pass's take-items that are not one bounded task. **Fold new native-dev-plane concerns here.**

The parallel tracks stay standalone because each is one bounded task with its own write-set, so it
can be delegated and run beside the others:
[Track A](epr:native-dev-track-a-eprfs-core-slice4-derivation-plane) (eprfs-core slice 4),
[Track B](epr:native-dev-track-b-gate-oracle-via-brit-build-affected) (gate oracle),
[Track C](epr:native-dev-track-c-brit-signs-derivation-record-exact-byte-verify) (brit signs the record),
[Track D](epr:native-dev-track-d-honesty-claimed-exit-partly-measured-failclosed) (honest verdicts),
[Track E](epr:native-dev-track-e-tier-blob-store-slice1-local-chunked-put) (tier blob store) and
[D3](epr:native-dev-d3-fork-preserving-space-head-and-ff-only-mirror) (fork-preserving space head).

| # | Item | Detail | Size | Owner |
|---|---|---|---|---|
| 1 | <a id="row-1"></a>**Retire duplicate mechanisms: one derivation plane, one offload path, one lease fold, one release plane** | Spec §3 "Duplicates to retire". No legacy arm: replace, then delete. Owner hints are proposals for the steward to confirm. Checklist: (a) `just measure`'s DHT `feedback_signal` offload → eprfs-peer carry (D9); owner `genesis/agentic/compute`; retire-when F4 lands and `just measure` uses the carry, with `rakia-executor` (the peer-stage worker) folded into it. (b) Berth's Python ledger and `genesis/agentic/bin/berth` → the eprfs-peer lease fold and `ark::Berth`; owner ark + eprfs-peer; retire-when the fold serves every `berth` verb the `just` lanes use. (c) The Python `.epr-meta` evaluator (`.claude/hooks/epr-meta-resolver.py`) and `habits-project.py` → Rust (`eprfs-meta`); retire-when parity holds (the hook already records host disputes; three Python-scoped validators live only there). (d) `bin_store`'s GitHub-download arm (`elohim/eprfs/device-runtime/src/bin_store/lifecycle.rs` ~531, `gh release download … --repo ethosengine/brit`) → `release-manifest` channel heads; owner device-runtime; retire-when the alone-path receipt is green (D7 rules 5 and 7). (e) `node_registry::CustodianAssignment` struck from plan row 5 (D6); `arch-authority-in-integrity-backlog` still carries a D4 on it, so reconcile there; retire-when row 5 uses only `CustodianCommitment` links and local B pledges. (f) Four ladder vocabularies (rung H/A, S0–S4, A-ids, "spaces") → R0–R4 (spec §4); retire-when no live doc uses the old names. (g) Three attestation shapes → `DerivationRecord`; three closure keys → one `DerivationKey` (D1, D2; Tracks A and C). (h) Hand-written CHANGELOG entries, typed trailers and habit DELTAs → projections (row 3). Retire-when: every item is done or struck with a reason. | weeks, spread over the joins | per item |
| 2 | <a id="row-2"></a>**[integrity] Evidence-laundering gaps that gate graduation (F6)** | Spec §5 D5, §7 rule 8, §11 F6. Each is ruled by a Claude model. Verified locations (adversarial review, 2026-11-04): gap 1, reanchor backfill admits attestations (`elohim/elohim-storage/src/services/reanchor_backfill.rs:51`; start from `dataplane-reanchor-dead-remaining-rekeyed-peer`); gap 2, the release threshold counts distinct agent strings, not principals (`elohim/elohim-storage/src/services/release_attestation.rs:859-861`; diversity was deferred as "additive later" in `task-release-soak-attestation-rail`); gap 3, DNA issuer authorization accepts every issuer (`content_store_integrity/src/attestation_validator.rs:67`; related `security-attestation-issuer-relaundering`, `dna-health-attestation-ci-authz`); gap 4, release attestations ride `attestation:device-health`, whose subject kinds are unenforced, a carrier D5 rejects. **Graduation is an enforced verb, not `to_attestation`.** It takes the cited policy, the accepted record set, one standing proof per counted principal, the acting steward and the target reach, and only then mints one notarized attestation per converged (derivation, output CID). `to_attestation` (`elohim/eprfs/eprfs-core/src/derive.rs:175`) is only the draft constructor the verb uses. A `derivation` attestation kind is minted in the next batched constitutional DNA move (F6); no stopgap on `device-health`. **Gating:** gaps 1–3 are enforced, each with an adversarial test where one human with several devices and agents yields k = 1, before F6. Until the `derivation` kind lands, nothing graduates past `intimate`. Retire-when: the four gaps close with tests and the kind lands. | days | elohim-storage + DNA owners |
| 3 | <a id="row-3"></a>**Spend as REA events; deliverables as projections** | Spec §6, joins F1r/F2/F5. (a) `ReaVerb` (`elohim/epr/src/witness.rs:315`) has Use, Consume, Produce, Cite, Affirm and Dismiss, with no `Work` or `deliver-service`; add them (`Work` for carrying under `delegates-compute`). (b) Gate cost rides as `--env` slots on a `Cite` note; move it to `Use` quantities (cpu-s, wall-s, bytes) against the producing `Produce` event. (c) Changelogs, commit trailers, steward-of-record lines, habit DELTAs and valueflow contributions become unsigned, recomputable projections over fulfilled commitments (seed: the contribution projection under `epr flow memory`); each hand-written record gets an owner and a retire-when and is removed once its projection reproduces it. (d) `.claude/data/*.jsonl` and berth ledgers, ad-hoc meters, become events. **Scope split:** minimal `Use`/`Produce` emission with quantities belongs to F1r and F2, whose receipts need it; F5 keeps `Work`/`deliver-service`, the meter retirement and deliverables-as-projections, so (b) and (d) start in F1r/F2 and finish in F5. Receipt: a changelog reproduced from fulfilled commitments. Retire-when: no hand-written CHANGELOG or meter ledger remains beside its projection. | weeks (F5) | epr + epr flow owners |
| 4 | <a id="row-4"></a>**Doc drift: rakia, eprfs and epr statements that no longer match code** | Spec §3 item 9, §11 closing paragraph. Fix in passing, by whichever track touches the file. (a) `elohim/rakia/CLAUDE.md` ~18 lists `rakia-executor` among planned, absent members; it is live as the measure peer-stage worker but untracked in the submodule pin (`rakia-executor-untracked-in-submodule-pin`), so check that before rewording. It is not a build executor (D8). (b) The rakia and eprfs README verb lists are stale, including the rakia plan README's `rakia build` (not individually re-verified). (c) `elohim/epr/src/kind.rs:1` says "the nine EPR kinds"; the enum has 12 variants. (d) The steward-space landing-plane spec's "Rakia serves a session the root manifest" wording (§5.1, near lines 573–578; re-locate with grep) is superseded: the tier serves, rakia names. Retire-when: the four statements match code. | an hour | the touching track |
