---
id: "backlog-recovery-m4-dual-anchor-sweettest-stubs"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Recovery M4 sweettest stubs premised on removed dual-anchor links — re-draw against the Content-entry read surface"
slug: "recovery-m4-dual-anchor-stubs"
written: "2026-06-11"
author: "dna-island-recompose-phase0"
status: "backlog"
priority: "low"
relatedNodeIds: []
tags: [holochain, sweettest, recovery-m4, imagodei, key-revocation, doc-debt, test-debt]
cites:
  - elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
---

## What is stale

`elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs` contains
TODO-stub scenarios whose doc-comments and assertion sketches are premised on
the `KeyRevocation` entry type and its `HumanToKeyRevocation` /
`RevokedKeyToRevocation` dual-anchor links — all REMOVED from
`imagodei_integrity` in Recovery M4 Task 15 (verified at source 2026-06-11):

- `imagodei_integrity/src/lib.rs` L744-745 ("KeyRevocation removed in Recovery
  M4 Task 15", "RevocationVote removed ..."), L903 (entry-types removal
  comment), L1021 ("KeyRevocation, RevocationVote, IdentityFreeze links removed
  T15"), L1114 (validation-arm removal), L1367-1370 (validators removed).
- Superseded by `governance-action:key-revocation` Content entries on the
  elohim DNA, read via TypeToContent traversal + `metadata_json` lifecycle
  flags — `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`
  ~L3275-3296 ("Recovery M4 — cross-DNA gate-reader query helpers"). A
  dual-anchor LINK pattern (`HumanToFreezeContent` /
  `RevokedKeyToRevocationContent`) is explicitly noted there (~L3290) as a
  FUTURE option only, "without changing this surface."

The stale stubs (all `#[ignore = "requires packed DNA artifact ..."]`, all
`TODO(M4-sweettest-bodies)` with empty passing bodies):

1. **`m4_self_revocation_happy_path`** (~L177; doc-comment ~L155-175): asserts
   "both dual-anchor links (HumanToKeyRevocation, RevokedKeyToRevocation) must
   be present" (L159) and sketches `get_links(.., HumanToKeyRevocation)` /
   `get_links(.., RevokedKeyToRevocation)` (L201, L206) — links that no longer
   exist.
2. **`m4_dual_anchor_link_invariants`** ("M4 Scenario 7", ~L447): the
   doc-comment was corrected 2026-06-11 (dna/ island-recompose session)
   (~L428-439 now states the links were removed in T15, supersession by
   Content entries, and that "this stub's assertion shape predates T15 and
   needs re-drawing against that Content-entry read surface"; it also corrects
   the historically miscited `LINK_ARCHITECTURE.md` §3 "dual-anchor primacy",
   which never existed in any version of that doc). The stub BODY's assertion
   sketch (~L451-467) still queries the removed links.
3. Scenarios 2-6 (`m4_emergency_contact_quorum_*` ~L243/L297,
   `m4_revocation_vote_idempotency` ~L336, `m4_rotation_blocked_by_*`
   ~L366/L402) reference the removed `KeyRevocation` ENTRY fetch-by-action-hash
   shape in their sketches to varying degrees — same supersession applies, audit
   each when re-drawing.

## Why this is doc-debt, not CI-red

Gospel rule (elohim/holochain/dna/CLAUDE.md ~L74): `#[ignore]` sweettests STILL
RUN in CI (`cargo nextest run --release --run-ignored all`). These stubs run
and PASS because their bodies are empty (conductor spawn + `Ok(())`). Nothing
is red; the debt is that the scenario intent on file describes assertions
against a substrate shape that no longer exists, so anyone filling the bodies
from the sketches would implement against removed types.

## What resolution requires

Either:
- **Re-draw** the stub doc-comments + sketches against the Content-entry read
  surface (`governance-action:key-revocation` on the elohim DNA via the
  cross-DNA gate-reader helpers, content_store/src/lib.rs ~L3275), turning
  Scenario 7's dual-anchor invariant into the equivalent invariant on the
  TypeToContent + metadata-flag surface (or deferring it until the FUTURE
  `RevokedKeyToRevocationContent` link pattern is actually adopted); or
- **Delete** the stubs that T15's supersession made meaningless, if the M4
  plan considers the T4/T13/T14/T18 scenarios (already written against the
  Content-entry surface, recovery_m4.rs ~L473+) to cover the same invariants.

No plan doc currently tracks the stub bodies: `grep -rn "sweettest-bodies"
genesis/` returns nothing (verified 2026-06-11); the `TODO(M4-sweettest-bodies)`
tag exists only in the test file itself.

OPEN QUESTION: do the landed T13/T14 scenarios (e.g.
`m4_t13_create_self_revocation_lands_governance_action_on_elohim_dna` ~L1356)
already subsume Scenarios 1-7's intent, making deletion the right call?

## Provenance

Surfaced during the holochain `dna/` island recompose Phase-0 verification
(2026-06-11); the Scenario-7 doc-comment correction landed the same day in the
same session. The HumanToMastery dual-home found in the same verification is
tracked separately
(genesis/data/timeline/backlog/lamad-imagodei-humantomastery-dual-home.md).
