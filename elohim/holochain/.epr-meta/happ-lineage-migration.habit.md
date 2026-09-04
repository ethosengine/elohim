---
epr-habit-version: 1
id: happ-lineage-migration
invariant: >
  A hApp version crossing (an integrity change, a new DNA hash) is carried by the network
  itself: refused at verify unless the release names what it migrates from and the elohim's
  migration commitment for that path is already notarized (never a per-node or household consent); adopted beside v1 under the SAME agent key; every witnessed fact
  crosses with its original v1 action + signature, re-verified by v2's own validation; dual-cell
  peers bridge the window; revert is free by re-election until a separately ratified sunset
  closes the v1 chains. No wipe, no re-seed, no re-key, ever again.
status: red
active: false
checks:
  - "a2o @concern:happ-lineage-migration (genesis/a2o/features/delivery/happ-lineage-migration.feature — Stations 1-10, @wip; steps not yet written; rehearsal = node_registry v1 → v1+NotarizationWitness on the household mesh, 0.7)"
  - "spike verdict (§9 Lane B, conductor fork): must_get_record_from_lineage host fn measured in elohim/holochain-conductor before B2 is decided — recorded as a dated delta below"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "backlog home (the epic's backlog item): genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md"
  - "arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (ladder row 6)"
  - "upstream kernel: elohim/holochain-conductor/crates/holochain/tests/tests/migration.rs (chain-switch, #5842)"
retire-when: >
  when three consecutive integrity changes reach the alpha fleet through their release channel
  with every witness verified and zero records re-seeded from outside, each with its cycle-time
  row in the arc doc — the wipe is then not a delivery path for any class and the register
  describes a product, not a practice.
---
DELTA 2026-09-03 (birth, RED): declared after the 0.6→0.7 cutover proved the class has no vehicle
(the fleet was wiped; rung 5 passed 5/5 on 0.7 but carries only coordinator bytes on a fixed line).
Grounded by four readers (Sonnet ×2, Opus, Codex): the v1 action signature carries no DNA hash and
verifies under v2; `must_get_*` is same-DNA only so the proof is embedded; upstream ships a
chain-switch test (#5842) validating a signed summary against DNA-properties signers — authorship
preserved, notarization not, which is the gap the NotarizationWitness closes. Rehearsal DNA:
node_registry (6 entry types, 1 storage call site, own bundle, not health-supervised). First
runnable red: Station 1 (verify's DnaLineageMismatch gains its positive branch). Not active: the
WIP fence is full (dataplane-convergence, runtime-death-witnessed); promotion is the operator's call.

DELTA 2026-09-04 (Probes A+B, sweettest on the 0.7 line, `sweettest/src/tests/happ_lineage_migration.rs`, EXIT=0, 2 passed;
re-run independently by the chief, 164 s): the KERNEL is green — v1 record re-created on v2 keeps its entry hash; the
NotarizationWitness carrying the v1 action + signature is accepted by v2's own validation; a flipped signature and a
foreign lineage hash are refused with typed messages; two cells under one agent key; late open_chain accepted.
FINDING: close_chain is not a fence (author and single-conductor authority both accept post-close writes) — Station 8
rewritten (the fence is ours: v1 cell disabled, v2 refuses carried facts after the close); Probe B2 (two conductors)
opened. Status stays RED: the habit measures the STORY on the household mesh (0 of 10 stations), not the kernel.
Landing rule: the witness type rides cargo feature `lineage-witness` so the default pack stays on `dna-hashes.baseline`.

DELTA 2026-09-04 (Probe B2, two conductors, `happ_lineage_migration.rs::probe_b2_remote_authority_after_close`, 1 passed,
EXIT=0, re-run by the chief): the remote agent-activity authority applies ActionAfterChainClose — rejects the FIRST
post-close action (seq close+1) and issues a warrant — but the tail after it validates again and the post-close record
stays fetchable locally and over the network. The sunset fence is ours (v1 cell disabled + v2 refuses carried facts after
the close, Station 8); the warrant is read as evidence. De-risk pass complete; the MVP plan is the execution surface.

DELTA 2026-09-04 (Station 1 GREEN on the household mesh — the FIRST station measured live; receipt
`genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r3.{log,json}`, run stamp
20260904123730, 1 scenario / 9 steps passed, 2m18s, three peers matthew:8090 jessica:8091 james:8092
on the 0.7 mesh): a `happ-lineage` release naming `migrateFrom = uhC0kNpWca3k…` (the INSTALLED v1
node_registry hash, read from each peer's own `/version` passport) and installing v2
(`uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1`) is admitted by every peer's own verify —
none of the three names `dna_lineage_mismatch`, all three reach the PATH arm — while the same v2 with
no parent named is refused by all three with `dna_lineage_mismatch` → "lineage mismatch", the story's
own words. TWO MEASURED FACTS: (1) the story's literal channel name
`runtime:lineage:node_registry:commons` is NOT a legal channel id — the manifest schema's `channelId`
pattern has no underscore, so packaging refuses before anything reaches the mesh; the run-scoped id
spells the role with a hyphen. (2) the positive branch's live refusal today is `conductor_unavailable`,
not `path_not_notarized`: Station 1's placeholder path-commitment cid is not a decodable EntryHash, so
`fetch_path_evidence`'s `get_commitment` errors and answers `Unreachable` — C4 working exactly as
designed (a read failure is never fabricated into an absence). `verify_envelope` runs before
`verify_path` (verify.rs:1046 then :1059), so reaching that arm at all IS the admissibility proof.

DELTA 2026-09-04 (Station 2 GREEN on the household mesh; receipt
`genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r7.{log,json}`, run stamp
20260904130252, 13/13 steps, 2m01s; the reds it was driven from are r5 and r6, same directory):
the path is refused before it is notarized and admitted after, on ALL THREE peers, with no one
asked anything. The elohim's signature is the zome's: `create_lineage_commitment` on matthew's own
mishpat cell took a payload with `signatures: []` and returned commitment
`uhCEkj-m21-rsHWL945MJavhpbhHdoyAxHBFdIHmeFqZWDygIqnJw` signed by matthew's agent
`uhCAkcTbutPk5V2…` — the harness supplied no key and constructed no signature, which is the whole
point of Task 11 part 2a. FOUR MEASURED FACTS, two of them defects this DELTA also fixes:
(1) `path_not_notarized` needs a DECODABLE commitment cid — `HoloHash`'s base64 decoder verifies
the four DHT-location bytes, so a hand-written pointer makes `get_commitment` ERROR and the peer
correctly answers `conductor_unavailable` (C4), never the absence the story names; the fixture now
uses `@holochain/client`'s `fakeEntryHash`.
(2) DEFECT, fixed: a `migrates-lineage` commitment fell to `mishpat_projection`'s unknown-action
fallback and projected `state: "proposed"`, while `verify_path` establishes a path only on
`"active"` — and NOTHING in the tree ever wrote `"active"` (`set_state` had one caller, a unit
test). Every correctly notarized path was refused "is proposed, not active". A lineage commitment's
notarization IS its activation (its quorum was verified in-wasm before the entry could exist; its
only other state is revoked, which `verify_path` checks first), so the two lineage actions now
project active and every other class keeps its acceptance step.
(3) DEFECT, fixed: `CommitmentCommitted` is a post-commit signal on the AUTHOR's conductor, so only
the notarizing peer ever holds a projection row. jessica and james read the same commitment off
their own DHT view, found no row, and `path_evidence::lifecycle` read that absence as `proposed` —
our own index gap asserted as a fact about the elohim's governance. An absent row is now an ANSWER
of no-row, and for a lineage action the notarized entry itself carries `active`. After the fix all
three peers report `state: ok`. NOT closed by this: a REVOCATION is still only visible to the peer
that projected it (Station 7's).
(4) The pointer between a release and its path runs ONE way — the manifest names the commitment,
`verify_path` refuses any other, and the commitment's own `release_cid` is never read back. So the
notarized path is minted first and the release names it; and it must go on a FRESH channel, because
`assertAdmissibleOverEarnedHead` refuses a publish over an earned head the acting peer has not
adopted — which, for the release Station 2's first half exists to have refused, it never will.

DELTA 2026-09-04 (Stations 3 and 4 GREEN on the household mesh, and Stations 1-4 green TOGETHER in
one run; receipts `genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r8` (S3,
11/11, 50s), `-r10` (S4, 11/11, 31s) and `-r12` (Stations 1-4, 4 scenarios / 44 steps, 6m11s), all
`.{log,json}` in that directory): james's runtime installed v2 BESIDE v1 and carried his whole v1
chain across, and every one of those records is in v2 at the same entry hash with a witness v2's own
validation accepted. Measured, from the receipts: one agent key `uhCAkTlfAatal8yJHFgPfUTtAVZD68x8ORRHweLHuumG1QQ8RAwd3`
on BOTH apps (base `elohim`, side `elohim@EKiIscIk5BDd`); passport role `node_registry` carries
`lineage { readingAppId: elohim, authoringAppId: elohim@EKiIscIk5BDd, closed: false }`; conductor pid
unchanged; james's v1 chain still at its pre-crossing digest and count; carry receipt
`carried == v1Count` (114/114 in r10, 119/119 in r11's world, 131 in r12's) with
`digest == v1Digest` and a witness per page — and v2 answers a witness at ALL of james's v1 entry
hashes. THE INSTALLED v2 HASH IS NOT THE PACKED ONE: james's authoring cell runs
`uhC0kN2gubu9MBAiyqmo3IbG0XfSp8aT2R4rx8-kIm-i7En2haNpV` while `hc dna hash node-registry-v2.dna`
says `uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1` — the install folds the hApp role
modifiers (inherited network seed + the lineage property `install_lineage` writes), exactly Task 1's
trap, now measured on the authoring side too. THREE MORE MEASURED FACTS. (1) The 0.7
`SignedActionHashed` wire shape splits the action into `content.header` + `content.data`, and the
entry hash lives on `data` — reading it at the pre-0.7 flat level yields NO hashes and reads as "the
chain is empty" (Station 4's first red). (2) `LineageCarryReceipt` reached a tracing line and nothing
else: it is now kept on `appliedRelease.carry`, because a completeness proof no operator and no check
can read is not a proof. (3) At any tier above staging the floor enforces the attestation threshold
for EVERY mode, canary included — so an earned-first ordering is circular (the canary cannot adopt
what it must first attest) and the canary necessarily acts at `staging`, which is rung 5's ceremony
unchanged.

DELTA 2026-09-04 (Station 9 GREEN on the household mesh; receipt
`genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r13.{log,json}`, 11/11, 44s):
all three peers dual-celled first (carried 261/261 matthew, 135/135 jessica, 141/141 james), then the
forgery committed through EACH peer's own v2 cell under that peer's own key — a harder test than the
story's fourth peer, because the refusal has to come from v2's validation and not from the network
declining a stranger. Both arms refused on all three, in v2's own words: one byte of a REAL signature
over a REAL action flipped gives "NotarizationWitness proof 0: carried signature does not verify
against the action's signer uhCAk…" (the story's "signature invalid"), and a REAL, VALID proof under
a parent DNA hash this v2 never declared gives "NotarizationWitness lineage_dna_hash uhC0k0dHR… is
not declared in this DNA's lineage property" (the story's "lineage unrecognized"). All 141
honestly-carried records still answer a witness afterwards.

DELTA 2026-09-04 (Stations 5 and 10 RED, BLOCKED, with the gaps named; Station 10's receipt is
`cucumber-stations-mvp-r14.{log,json}`, 9 passed / 1 failed, 2m02s):
STATION 5 (held-carry) is not implementable against this substrate. `export_records` is
`query(ChainQueryFilter…)` — the CALLING agent's own chain, local by construction — so james's v1
cell can only ever export james's records, and `carry_from`'s held-carry branch is unreachable from
the only caller that exists. There is no bridge sweep in elohim-storage (grepped). It needs a v1
extern that exports ANOTHER agent's records off the DHT plus a sweep caller: zome-side, so a BLOCKED
report rather than an edit.
STATION 10 reds on both arms for two named absences. Arm 1, MEASURED LIVE: a `migrates-lineage`
commitment notarized through JESSICA's mishpat cell — a household peer, not the bootstrap steward, so
off the declared roster — was accepted by every peer (`{"state":"ok","ok":true}`). THERE IS NO ROSTER
CHECK ANYWHERE: `verify_path` compares only the COUNT of `signatures` against `required_signatures`,
and `validate_lineage_signatures` verifies each signature against its own claimed key and never reads
`roster_cid`. `quorum_unmet` is additionally unreachable from any commitment mishpat will create,
because that validator refuses an under-quorum payload at commit. Arm 2, measured from the code (the
run never reaches it): `verify_path` compares roots only when `inst.constitution_root` is `Some`, and
`InstalledReality::from_happ_passport` sets it `None` for every role with its own comment saying the
passport does not expose one yet (verify.rs:136-138) — so `root_mismatch` cannot fire on a live peer.
Station 10 needs a machine-readable council roster (the story declares one in prose; nothing on the
substrate holds it) and a per-role constitution root on the passport.

DELTA 2026-09-04 (the deliverable's ONE-RUN receipt:
`genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r15.{log,json}` — Stations
1, 2, 3, 4 and 9 in a single cucumber process, 5 scenarios / 55 steps, ALL PASSED, 6m44s, on the
three-peer 0.7 household mesh, conductors untouched). Five of the seven MVP stations are measured
green; Stations 5 and 10 are red and BLOCKED with their gaps named in the DELTA above. Status stays
RED — it is the controller's to flip, and two stations of the seven are still unmeasurable on this
substrate.
