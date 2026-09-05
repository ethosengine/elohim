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
  - "a2o @concern:happ-lineage-migration (genesis/a2o/features/delivery/happ-lineage-migration.feature — Stations 1-10, @wip; steps in genesis/a2o/steps/delivery/happ-lineage-migration.steps.ts with the fixture helpers lineage-candidate.ts + lineage-commitments.ts; a live run needs A2O_RUN_WIP=1 or the @wip gate holds every scenario; rehearsal = node_registry v1 → v1+NotarizationWitness on the household mesh, 0.7)"
  - "spike verdict (§9 Lane B, conductor fork): must_get_record_from_lineage host fn measured in elohim/holochain-conductor before B2 is decided — recorded as a dated delta below"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "backlog home (the epic's backlog item): genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md"
  - "arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (ladder row 6)"
  - "upstream kernel: elohim/holochain-conductor/crates/holochain/tests/tests/migration.rs (chain-switch, #5842)"
guard: >
  Risk R1–R4 (scale; measured from CODE on 2026-09-04, not yet from a run — rows 1–4 of
  genesis/data/timeline/backlog/arch-scale-risk-backlog.md). Every station can be green on
  node_registry (tens of records, three peers) while all four are live, because none has a
  trigger below ~3k entries: R1 the export re-walks and re-digests the WHOLE chain per 64-record
  page (≈N²/64); R2 witness validation walks the carrier's whole activity per witness (O(W²) per
  validator, worse after Tasks 21–23 add couriers); R3 chains ≈×1.06 per migration, never pruned,
  and the held-carry fans out ∝ peers × records (≈110 min per courier for 3.5k records at
  16/30 s); R4 dual cells ≈×2 conductor RAM for the whole window on a tier that already hit
  ~14 GB overnight. Retire-when names three ALPHA crossings and lamad is the first role that
  exercises R1/R2 — so a green here on the rehearsal DNA is necessary and NOT sufficient, and
  the Station 3/5/8 receipts must carry elapsed, catch-up minutes and RSS before the first lamad
  crossing is scheduled. R1 and R5 retire coordinator-side (hash-neutral); R2 is an integrity
  change and rides the sunset-hardening crossing.
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
SUPERSEDED by Station 2's fact (1): the fixture's path pointer is now a decodable `fakeEntryHash`, so
the positive branch's live refusal is `path_not_notarized`, and the Station 1 step asserts that arm
BY NAME rather than merely "not dna_lineage_mismatch" (fix round 1, item 4).

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

DELTA 2026-09-04 (fix round 1 after review — one CRITICAL closed, and one OPEN TRUST BOUNDARY named;
re-run receipt `genesis/a2o/reports/release-ceremony/2026-09-04/cucumber-stations-mvp-r16.{log,json}`,
Stations 1 and 2, 2 scenarios / 22 steps, ALL PASSED, 4m01s, on the rebuilt mesh; gate: storage lib
tests + `cargo clippy --features "p2p p2p-iroh" --all-targets -- -D warnings` EXIT=0 + fmt):
CRITICAL — the lineage-class decision read `payload_json["action"]`, which is SELF-DECLARED. The
mishpat coordinator dispatches on the ENTRY's action and only some arms pin the body's second copy to
it (`validate_ratifies_limit_gradient` never does), so a commitment created as
`ratifies-limit-gradient` whose body claims `"action": "migrates-lineage"` passes the coordinator
carrying NO signatures — and would have read as an ACTIVE migration path on every peer without a
projection row, which is every peer but the author. Both sides now key on the ENTRY
(`GetCommitmentOutput.action` reading, `commitment.action` projecting, as `signals.rs` already did),
each pinned by its own forgery test. QUALIFIED: `validate_lineage_signatures` is AUTHOR-SIDE ONLY —
mishpat integrity's `commitment_action_requirements` has no lineage arm, so receiving peers never
re-verify a lineage quorum on gossip; "active" states the LIFECYCLE, never the quorum's soundness on
this peer. OPEN TRUST BOUNDARY, named and NOT closed this round: a `revokes-commitment` projects off
the author's post-commit signal too, so every other peer reads "no row" and therefore "not revoked" —
and `verify_path` checks `revoked_at` FIRST. An elohim's revocation of a migration path is invisible
to the peers it is meant to stop: the SAME hole as Station 10's missing roster check, pointed at the
revert instead of the quorum, and why Station 7 is not yet measurable. The seam that closes it is
`mishpat::get_commitment_state_links` through the peer's OWN conductor — the C5 rail the commitment
body already comes down — rather than a projection only the author holds (Task 13 / Station 7).
Also tightened: Station 1 asserts the PATH arm BY NAME (`path_not_notarized`, all three peers in
r16) instead of merely "not dna_lineage_mismatch", which would have passed on a stray
`manifest_schema_invalid` or `conductor_unavailable`; and `readV1Export` refuses a baseline that hit
its page cap, because a PREFIX of the chain would silently make every "untouched" and
`carried == v1Count` comparison compare the wrong set.

DELTA 2026-09-04 (risk discipline; no status move): four scale risks read off the landed code and filed as
rows 1–4 of `genesis/data/timeline/backlog/arch-scale-risk-backlog.md` (tag `risk`, CONVENTIONS §Risks), carried
here as `guard:` R1–R4 and as `inject` pointers at `elohim/holochain/dna/.epr-meta` and the elohim-storage
manifest. None is measured by r15/r16 (node_registry, tens of records); the first live measure is a lamad-scale
run, and the receipts need elapsed / catch-up minutes / RSS before that run means anything.

DELTA 2026-09-05 (Station 5 GREEN on the household mesh — the held carry works, and the 2026-09-04
"BLOCKED (zome)" DELTA above is SUPERSEDED; receipt
`genesis/a2o/reports/release-ceremony/2026-09-05/cucumber-stations-mvp-r20.{log,json}`, run stamp
20260905025626, 1 scenario / 11 steps passed, 1m12s; the red it was driven from is `-r18`, same
directory): jessica never adopted anything and never moved, and 16 of her 180 v1 node-registry
records are readable in james's v2 side cell at her own v1 entry hashes, each witnessed by a link
james authored (courier `uhCAkTlfAatal8yJHFgPfUTtAVZD68x8ORRHweLHuumG1QQ8RAwd3`) — and jessica's own
v1 chain digest and count are unchanged across the sweep, which is what makes it a HELD carry and not
a write into someone else's chain. What closed it since 2026-09-04: Task 12's `LineageBridge` ticker
(one held page of 16 per neighbour per 30 s, armed the moment a window opens, idle otherwise) and
Task 24/26's agent-scoped v1 export — the substrate the earlier DELTA correctly reported as absent.
THREE MEASURED FACTS. (1) `RoleLineageView.sweep` is `skip_serializing_if = "Vec::is_empty"`, so the
field is ABSENT from `/version` until the first tick has touched somebody — which is exactly the state
the poll starts in; the step's non-optional index into it was Station 5's first red (`TypeError:
Cannot read properties of undefined`). (2) The sweep budget cannot be a constant: it moves 16 records
per neighbour per tick, so a chain of N takes ⌈N/16⌉ ticks, and this mesh's fixture chains are already
180+ (eleven ticks) against a six-tick budget. Both Stations 5 and 6 now derive the bound from the
neighbour's OWN record count, `(⌈N/16⌉ + 2) × 30 s`. (3) RISK ROW R1 HAS ITS FIRST LIVE NUMBERS, now
appended per run to `.claude/data/lineage-carry-metrics.jsonl`: james's 186-record self-carry took 40 s
end to end and the whole three-page v1 export walk 0.2 s, with `ExportPage.scanned` peaking at 634 —
an UNPINNED page reports the whole chain's action count by construction (186 records ≈ 634 actions), so
this is the linear-cost baseline the pinned-resume path has to beat, not a regression.

DELTA 2026-09-05 (Station 6 GREEN on the household mesh; receipt
`genesis/a2o/reports/release-ceremony/2026-09-05/cucumber-stations-mvp-r22.{log,json}`, run stamp
20260905031014, 1 scenario / 11 steps passed, 7m07s; the red it was driven from is `-r21`):
the window keeps both sides talking. jessica, who never crossed, authored a fresh v1 record
(`uhCEksNdZC-tU9iT1wM3-nxTRm2LCySbXvG-Ccm7-ArbIhG3cWJ83`) and james's bridge carried it into v2 with
her signature intact, courier james, in **366 s**; james's passport reports `backwardCarry:
unavailable` — a fact about v1 (its integrity zome declares no witness type, so a v2-authored record
has nowhere to land there), reported rather than inferred from silence; and the word "stale" appears
on no peer's `/version` or `/admin/adoption`, which is right, because the story defines stale as
"still on v1 AFTER the sunset" and no sunset exists in this world.
TWO MEASURED FACTS. (1) THE STORY'S "within one sweep interval" IS NOT THE MEASURE, and the run says
so in one number: 366 s ≈ 12 ticks, against jessica's 192-record chain. `next_sweep` rule 2 sends the
cursor back to the beginning at the end of every local view, so a record written mid-cycle is reached
when the walk NEXT PASSES IT — ⌈N/16⌉ ticks, not one. One interval is what a caught-up walk costs;
the Then's claim (james's bridge, not jessica, moves it, and her signature travels) is unchanged and
its bound is now chain-derived. The story line should be re-spelled when it is next touched.
(2) DEFECT in the harness, fixed: `connectRoleConductor` calls `authorizeSigningCredentials`, which
COMMITS a cap grant to the cell's own source chain. Re-connecting on every poll iteration against a
v2 cell the bridge is writing to every 30 s loses that race — `internal_error: … the source chain
head has moved since the bundle began`, Station 6's first red, on the READ path. A polling caller
now opens ONE rail and re-calls `get_witnesses_for` on it.
R1 ledger this run: james's 196-record self-carry 50 s; the v1 export walk 0.3 s at
`ExportPage.scanned` = 676.

DELTA 2026-09-05 (Station 7 GREEN on the household mesh — the revert is free, and it is REAL, not a
routing flip; receipt `genesis/a2o/reports/release-ceremony/2026-09-05/cucumber-stations-mvp-r25.{log,json}`,
run stamp 20260905032540, 1 scenario / 16 steps passed, 2m27s; the reds it was driven from are `-r23`
and `-r24`): matthew notarized a `revokes-commitment` against the migration inside its revert horizon,
and every one of the story's seven Thens held. jessica — who never applied, so her sweep never takes
`watch.rs`'s C6b idempotence exit — read `path_revoked` off her own conductor within one sweep
("path uhCEk7QNP… revoked at 2026-09-05T03:27:16Z"), which is Task 19's DHT-first evidence seam doing
exactly what the 2026-09-04 fix round said it would: an elohim's revocation is no longer invisible to
the peers it is meant to stop. james and matthew both flipped v1 back to AUTHORING with the side app
kept as the READING pointer, disabled and still installed. THE MEASURED CENTRE OF THIS STATION:
james's readopt receipt on `/admin/adoption` — **status `readopted`, readopted 1, alreadyPresent 206,
foreign 20, v2Total 227, pages 15** — 1+206+20 = 227, the successor's own count, so every record his
v2 cell held is ACCOUNTED FOR and the one he authored natively during the window is back on v1 at the
same entry hash. That arithmetic IS the story's "pending, never lost": the 20 foreign are v2's own
witnesses, which v1 declares no entry type for, and a walk that died partway would report its partial
under `status: failed` rather than reading as nothing.
THREE MEASURED FACTS, all of them corrections to what the steps said before this run. (1) The landed
revert semantics are v1-AUTHORS / v2-READS, not "authoring == reading == base" — the passport's
lineage view is still PRESENT after a revert with the two ids swapped, and the earlier ruling's
wording is withdrawn in code as well as in the ledger. (2) A single `export_records{cursor: null,
limit: 64}` reads the FIRST PAGE ONLY; on a v2 cell that has just carried a 200-record v1 chain a
freshly authored record is at the END, so the one-page read found nothing and reported "produced no
readable entry hash" — a page boundary wearing an authorship failure's costume (`-r23`). Every
"is this record on this chain" question now walks. (3) The v2 cell CANNOT be read after a revert: it
is disabled, and a disabled cell refuses even the capability grant a read needs
(`internal_error: … CellDisabled(CellId(…))`, `-r24`). A step that reached in there was asking the
wrong surface — the story says "reported by its author's PASSPORT", and Task 13c's
`RevertReceipt.readopt` is that report, with numbers.

DELTA 2026-09-05 (Station 8 GREEN on the household mesh — the sunset is the runtime's own act, and
the after-close fence fires exactly where HDI 0.8 lets it; receipt
`genesis/a2o/reports/release-ceremony/2026-09-05/cucumber-stations-mvp-r27.{log,json}`, run stamp
20260905033519, 1 scenario / 22 steps passed, 1m32s; the red it was driven from is `-r26`): with all
three peers dual-celled and carry-attested and NO sunset commitment, no peer closed anything. matthew
then notarized a `sunsets-lineage` naming the migration, and each peer's OWN runtime found it,
sealed, and reported the ceremony on its own `/admin/adoption` — matthew close
`uhCkkgynKaI8RNRVFLFT6V8vdvDc0h5kT0XmLN6_Txmwq9W5tLDqo`, jessica
`uhCkk5vlyA3EVnsQUhu_IEtXqr-tSG6QhwlXHUCGLNYIzVqyAfrSp`, james
`uhCkkUZ5e_lUTFHefpfO-qg-KT67WFCPrviEmmuvbCSbYRhuHig7u`, each with its own open naming that close and
its own seal witness carrying the close into v2, `get_closes_for` answering one `AuthorToClose` link
per peer. All three v1 chains stayed readable, all three passports read v2 authoring / v1 closed, and
the post-sunset revocation changed nothing.
THE FENCE, MEASURED, exactly as the Station was narrowed to claim: james self-carrying his own
post-close fact into his own v2 is REFUSED —
`NotarizationWitness proof 0: after close — uhCAkTlfAatal8yJHFgPfUTt…` — while matthew and jessica,
couriers whose own v2 chains hold their own close and never james's, ACCEPT it. That is filed gap G6
(Task 23), and this run is the first live evidence of the shape rather than an argument from the
source.
THREE MEASURED FACTS. (1) DEFECT in the harness, fixed by DELETION: the step used to call the
coordinator's `seal_close` itself, standing in for a trigger that did not exist when it was written.
Task 14b landed that trigger, so the harness and the runtime raced for the same source chain —
`-r26`'s `Source chain error: … the source chain head has moved since the bundle began`. `sealClose`
is gone from the steps file entirely and a comment holds its place, so a future edit cannot quietly
re-add it; the Thens read each peer's own `SunsetReceipt` instead, which is what the story's "each
peer's runtime seals" actually claims. (2) All three seals report `resumed: true` — `-r26`'s race HAD
closed the v1 chains before it died, and a CloseChain is permanent (no reset un-closes a chain), so
Task 14a's half-seal resume is what let the ceremony complete rather than closing v1 twice. That is
the fix round `27532918a` earning itself on a live mesh. (3) "Disabled" in this Station is the
storage-side `closed` flag, NOT `disable_app`: the admin seam disables a whole app and the base app
carries lamad, imagodei and infrastructure too, so the sunset disables nothing and uninstalls
nothing — what stops a peer writing to v1 again is that routing no longer points there.

DELTA 2026-09-05 (Station 10 GREEN on the household mesh — the roster and the root are REAL checks
now, and the 2026-09-04 "BLOCKED (no roster, no installed root)" DELTA above is SUPERSEDED; receipt
`genesis/a2o/reports/release-ceremony/2026-09-05/cucumber-stations-mvp-r28.{log,json}`, run stamp
20260905033528, 1 scenario / 14 steps passed, 2m46s; the red it was driven from is 2026-09-04's
`-r14`, 9 passed / 1 failed): both negative arms are refused by all three peers, each on its own
channel, each asserting the exact refusal DETAIL and not merely the reason code.
ARM 1 — a `migrates-lineage` notarized through JESSICA's own mishpat cell (a household peer, off the
declared roster; `create_lineage_commitment` signs with the CALLING agent's key and takes no other,
which is the only honest way to express "signed by a key that is not on the roster") is refused
`quorum_unmet` on matthew, jessica and james. On 2026-09-04 this same act was accepted by every peer
with `{"state":"ok"}` because THERE WAS NO ROSTER CHECK ANYWHERE; Task 16's `read_roster` closed it,
reading membership off the real `author-lens` commitment the run mints (roster
`uhCEk…`, one member: matthew's own agent key) rather than off a slug that always
terminated at "roster is not an address".
ARM 2 — the same crossing signed by the steward but under
`a2o-fixture-constitution-root-THE-V2-DNA-DOES-NOT-DECLARE` is refused `root_mismatch` on all three.
On 2026-09-04 this arm was unreachable in principle: `InstalledReality::from_happ_passport` set
`constitution_root: None` for every role, with its own comment saying the passport did not expose
one. Task 17 put the root on the passport and Task 14b made installed reality read the AUTHORING
cell on a crossed role, so `RootSource::Installed` is live and the check fires.
And the release itself stays earned and admissible through both refusals — only the PATH was refused,
which is the distinction the whole Station exists to hold.
