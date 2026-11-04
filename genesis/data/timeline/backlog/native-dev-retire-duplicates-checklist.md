---
id: "backlog-native-dev-retire-duplicates-checklist"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire duplicate mechanisms: one derivation plane, one offload path, one lease fold, one release plane (checklist with owners)"
slug: "native-dev-retire-duplicates-checklist"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, retire, duplicates, cleanup]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Checklist (spec §3 'Duplicates to retire'; no legacy arm, replace then delete)

Owner hints are proposals, to be confirmed by the steward.

- [ ] `just measure` DHT `feedback_signal` offload path -> eprfs-peer carry (D9). Owner: orchestrator/agentic compute (`genesis/agentic/compute`). retire-when: F4 lands and `just measure` uses the carry; `rakia-executor` (the peer-stage worker) folds into the carry.
- [ ] Berth Python ledger and `genesis/agentic/bin/berth` -> eprfs-peer lease fold and `ark::Berth` (verified: `genesis/agentic/bin/berth` exists). Owner: ark + eprfs-peer. retire-when: lease fold serves every `berth` verb used by `just` lanes.
- [ ] Python `.epr-meta` evaluator (`.claude/hooks/epr-meta-resolver.py`) and `habits-project.py` -> Rust (`eprfs-meta`). Owner: eprfs-meta. retire-when: parity holds (the hook already records dispute between hosts; three python-scoped validators still live only there).
- [ ] `bin_store` GitHub-download arm -> `release-manifest` channel heads. Verified: `elohim/eprfs/device-runtime/src/bin_store/lifecycle.rs:~531` (`gh release download ... --repo ethosengine/brit`). Owner: device-runtime. retire-when: alone-path receipt green (D7 rule 5, 7).
- [ ] `node_registry::CustodianAssignment` struck from row 5 (D6). Note `arch-authority-in-integrity-backlog` still carries a D4 on it; reconcile there. Owner: node-registry DNA. retire-when: row 5 uses only `CustodianCommitment` links and local B pledges.
- [ ] Four ladder vocabularies (rung H/A, S0-S4, A-ids, 'spaces') -> R0-R4 (spec §4). Owner: plan/spec authors. retire-when: no live doc uses the old names.
- [ ] Three attestation shapes -> `DerivationRecord`; three closure keys -> one `DerivationKey` (D1, D2). Owner: Tracks A and C.
- [ ] Hand-written CHANGELOG entries, typed trailers, habit DELTAs -> projections (see `native-dev-rea-spend-as-events`).

**retire-when (this file).** Every box is checked or struck with a reason.
