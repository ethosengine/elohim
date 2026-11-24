---
title: "A verdict is an attestation on an artifact — what a lander reads to decide how much to re-run"
id: verdict-is-an-attestation-on-an-artifact-design
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session cacccb55 on shem, Fable 5.2, the integrator); evidence tables by session d6cfb7e1 on ethosengine (Opus 5.5); prose by a scribe seat"
serves: [push-gate-pays-once]
refines: native-dev-tooling-over-peers-plan
tags: [attestation, enrichment, verdict, artifact, closure-key, reuse, independence, graded-policy, per-crate, release-acceptance]
graduation-trigger: >-
  The steward has ruled on the property list of section 4a, the table of section 5 and the order of
  section 8, and the enrichment record and the grade reader of section 8 have each printed beside
  the real decision for ten landings on two devices.
cites: []
---

# A verdict is an attestation on an artifact

This document asks the steward of record, `human:matthew`, to rule on three things: the list of
properties a gate verdict should carry in a form a reader can read (section 4a), a graded policy
table that says, for each class of change, what lets a lander skip running a gate itself (section
5), and the order of the stations that would build it (section 8). Nothing in it is built. It takes
a direction he gave on 2026-11-24, that the integrity cost of a gate should move onto attestations
on build artifacts, and argues that two different things are needed for that: attestations that
carry enough and can accumulate, which let work be done once for everyone, and artifacts fine-grained
enough that a change invalidates little, which make the work smaller. This week's two-hour re-runs
were a failure of the second kind, not the first.

**Sources.** The steward's words are from the sidebar file kept beside the worktrees on shem,
`.worktrees/attest-sidebar.md`, untracked and not in the repository; its sections dated 2026-11-24
are quoted exactly and each is a direction, not a ruling. The evidence is four read-only files in
`genesis/docs/superpowers/specs/evidence/2026-11-24-gate-attestation/`, gathered on ethosengine:
`README.md` (the index), `attestation-properties-table.md` (Table 1, at dev `d74bc6707`),
`tree-scope-table.md` (the gates keyed on the whole tree), `split-units-table.md` (Table 2, at dev
`c4bea4ded`) and `v3-to-fold.tsv` (which fields moved each gate's key between two adjacent commits).
In this document "Table 1" and "Table 2" mean those two files. Their cells and line
references are used as they stand and were not re-derived. "The plan" is
`genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md`; its section 8
lists its stations, named units of work each with a Verify, and sections 9 and 10 hold its six tests
and the vocabulary this document uses. This document refines it, and builds on
`genesis/docs/superpowers/specs/2026-11-23-change-detection-one-reading-design.md` (the
change-detection design). Facts marked "integrator's record" were measured or decided on shem
during the week and are recorded by the integrator, session cacccb55; a proposal marked
"ethosengine's, unreviewed" is a note from session d6cfb7e1 that no other seat has reviewed.

**The steward's direction** (2026-11-24, about 15:30Z, his words, a direction and not a ruling):
"We should be able to shift that integrity cost to the epr-meta attestations of the build
artifacts, just because you didn't build all of it, shouldn't necessarily suggest much integrity
loss.. especially if our artifact stores carry enough information like compatibility and checksums
and stuff to make an informed decision about how a peer can be helpful...  Epr gives us a way to
enrich artifacts later, and bring new attestable and dht notorized properties to enable high
distributed integrity..."

He said it after the integrator reported that cycle times had not tightened: three full gate runs
of over two hours each on shem in one day, and no verdict reused between a gated head and the same
head plus five commits (the sidebar file).

**Terms.**

- **shem** and **ethosengine**: two of the steward's development devices; shem is the one that lands
  on the integration branch, `dev`.
- **gate**: a named check a device runs on a change before it may reach the integration branch; it
  has a recipe, a key and a verdict.
- **brit**: this repository's git tool; it also signs and stores notes on commits, which is where
  verdicts and validation attestations live.
- **CID**: content identifier, a name computed from the bytes it names.
- **DHT**: the shared distributed store on which peers notarize entries; in this project, Holochain's.
- **holon**: the steward's word, kept as his; this document does not define it.
- **roster**: the tracked list of the steward's admitted devices and their keys.
- **step fingerprint**: a hash over one recipe step's declared inputs; one part of a key.
- **artifact**: anything a step produces and someone else might use instead of producing it again:
  a binary, a library build, or a gate's result. It is named by its content, a CID.
- **closure**: everything a step's result depends on: its declared inputs, its recipe and runner,
  the files its code reads, the compiler, the platform and the flags.
- **key**: a hash over a closure. Two runs with the same key were asked the same question.
- **verdict**: the signed record of one gate's result, green or red, for one key. Today it is a brit
  note that crosses between devices as `gate-verdict@1`.
- **attestation**: any signed statement about an artifact by someone who can be named. A verdict is
  one attestation; a household's validation record and an artifact's production record are others.
- **enrichment**: a later attestation that points at an existing attestation or artifact by its CID
  and adds one property to it, without changing it. It does not exist yet (section 4a).
- **producer**: the device or service agent that ran the step and signed what it produced.
- **holder**: a device that holds an artifact's bytes and can serve them. A producer is usually a
  holder; a holder need not be a producer.
- **carrier**: a device or service agent that runs a gate for a lander who asked. Its result names
  the basis `under-grant` (plan section 9).
- **sponsored runner**: a service agent such as Jenkins that carries under a bounded grant from the
  steward. It is never a principal, and its result buys speed and never counts as an independent
  witness (ruled this week; integrator's record, shem, 2026-11-24).
- **admitted**: said of a device whose signed records this device accepts because the steward's
  roster names it. **sibling**: the steward's other admitted device.
- **independent execution**: one run of the same key on a different admitted device, signing with
  that device's own key. A sponsored runner's run counts as zero. Section 4a gives the counting rule.
- **standing**: what the signer is to the reader: this device, an admitted sibling, a sponsored
  runner under grant, or (not yet possible) a principal with standing in a shared space.
- **grade**: the reader's answer, computed from the attestations it can verify on one artifact, to
  how much re-execution the lander still owes for that artifact's class of change (section 5).
- **lander**: the device pushing to `dev` or `main`, which must hold a verdict for every selected gate.
- **shadow**: a mode that prints what a new rule would decide beside the real decision, and decides
  nothing.
- **integrity-class**: a change to selection, to a key, or to which work is skipped. Its shadow may
  be built; turning it `on` needs the steward's ruling and an adversarial review.

## 1. What a verdict is trusted for today

A lander reuses a verdict instead of running a gate when two things hold. First, the key says what
was tested: it is a hash over the gate's material (`genesis/orchestrator/gate-evidence.mjs:367-388`,
Table 1), so an equal key means the same inputs, recipe, runner, compiler and tree. Second, the
signer is trusted. For this device's own verdict the reader checks that the signing agent is itself
(`gate-evidence.mjs:601`) and checks no roster. For a sibling's verdict it checks the sending device
against the roster when it reads it (`elohim/eprfs/epr-cli/src/peer.rs:1790`) (Table 1).

Neither condition asks whether the lander did the work. Sibling reuse already acts on that. On
2026-11-22 a sibling ran `just gate eprfs` for the lander's commit in 802 s, the lander's log
printed `eprfs reused: green in 802 s on the sibling … independence: sibling`, and that gate cost
the lander 32 s (plan section 8, F4-probe). The direction above asks to take that reasoning further.

The rule as it stands is binary and shaped by devices. The reader takes the newest row or the first
sibling it finds (`gate-evidence.mjs:598,618`), so the answer is one of three: this device's green,
an admitted sibling's green, or run the gate. It counts nothing (Table 1, gap 2). The reuse fences
then refuse a held verdict outright on a red, dirty inputs, a signer from a device not admitted, or
a governance change (`genesis/orchestrator/gate-runner.mjs:189,223`, as the change-detection design
reads them). The governance fence is the bluntest: a push that touches any path on its list
(`SIBLING_REUSE_GOVERNANCE_PATHS` and `siblingGovernanceChange`, `gate-evidence.mjs:480-507`)
refuses every sibling verdict for every gate in the push. On the first F4-ask landing head,
ethosengine measured 16 selected and enrolled gates; one had its own judge changed, and the other 15
were refused only by that whole-push fence (plan section 8, Fence-scope).

The sibling path also trusts less than its log line suggests. A sibling's verdict reaches reuse
through an unsigned flow note (`gate-evidence.mjs:879-884`), so what the reader checks is the
roster's admission of the device that sent the note, not the brit signature on the verdict (Table 1,
gap 3).

## 2. What each record carries today

Four kinds of record already make statements about what was built or tested. Table 1 reads each
against the properties a lander's policy would need. Its abbreviations are kept here as a citation
key: GE `genesis/orchestrator/gate-evidence.mjs`; AR epr-cli `artifact.rs`; DS `artifact/describe.rs`;
CU `artifact/custody.rs`; PR `elohim/eprfs/epr-cli/src/peer.rs`; HA the a2o
`household-attestation.ts`; SR `serving-receipt.mjs`; TR and OB the Jenkins bridge's `translate.rs`
and `observe.rs`; BA `scripts/brit-attest.mjs`. Read at dev `d74bc6707`; the cells are condensed from
Table 1, which also gives where each record is stored.

| Record | Closure and inputs | Toolchain | Platform and features | Signer and standing | Independent count | Method or judge version | Later enrichment |
|---|---|---|---|---|---|---|---|
| Gate verdict (brit note, crosses as `gate-verdict@1`) | hash only (GE:367-388); the note keeps `inputs_hash` (GE:812) | `rustc -vV` inside the hash (GE:274-283), not readable | rustflags in the hash (GE:372); host triple only inside the compiler hash; `hardware_profile` empty (GE:806-816); no features | brit workspace key; own reuse checks agentId is self (GE:601), no roster; sibling roster-checked at read (PR:1790) over an unsigned flow note (GE:879-884) | absent: newest row or first sibling (GE:598,618) | `manifestCid` signed (GE:810); recipe, runner and key version 3 only inside the hash | none: a re-put replaces the note (GE:801-804) |
| Artifact (`epr artifact put`) | key is method plus input CID (AR:977-1025); output CID (AR:1469) | `rustc -vV`, cargo, toolchain, readable and enforced at put (DS:68-91) | target, features, profile, rustflags, CFLAGS (DS:20-99), instruction level (AR:1577) | brit agentId and custodian DID (AR:1471-1480); siblings roster-checked at read (PR:1967-1976), own not | advisory: `holders` counts devices per output, `--min 2` (CU:349-370) | method CID and family (AR:1560-1564); no schema version | separate records by key CID (divergence recorded as a Cite, CU:436-458); no enrichment verb |
| Household ValidationAttestation | per component (`sut.ts:82-97`), artifactCid their hash (HA:167-240); no commit | absent | absent | workspace key; the reader admits only its own key (HA:1241-1245); no roster | absent (SR:105 counts scenarios) | validatorVersion recorded, never checked by the reader | none: a changed summary replaces the note |
| Jenkins (A: bridge claim; B: brit-attest CI note) | A: commit only with the orchestrator graph (TR:370-415); B: the commit tree (BA:254) | absent | absent for the machine that ran it; B names the harvesting host (BA:316) | A: unsigned, `service:jenkins`, offer approved by a distinct steward (OB:173-251); B: the device's brit key | absent | A: recipe id and version (TR:460); B: manifest CID | A: a new record breaks nothing; B: a changed verdict replaces the note |

Three absences matter for this design.

**No record kind can be enriched.** When a verdict changes, the new note replaces the old one
(GE:801-804). Side records that point back by CID do so by convention, and nothing reads them as
belonging to the original (Table 1, gap 6). So a second device that re-runs a gate and agrees has
nowhere to say so that adds to the first result; it can only write its own verdict, which the
reader treats as an alternative, not a confirmation.

**Nothing counts independent executions where a decision is made.** The gate reader takes the
newest row or the first sibling and stops. The only count anywhere is `epr artifact holders`, which
counts devices holding an output and is advice (CU:349-370; Table 1, gap 2).

**In the gate verdict, the properties are present only as hashes.** The verdict's key covers the
toolchain, the flags and the host triple, but one-way: the signed note has no readable compiler
version, target or flags, and its hardware profile is empty (Table 1, gap 1). A reader can tell that
two verdicts were asked the same question; it cannot tell what platform answered, so it cannot
judge whether an answer from another machine fits its own. The artifact record, by contrast,
already carries most of these readable (row 2).

### Found on the way

Two findings from Table 1, listed and not designed around here:

- The device notes publisher's refspec is unfiltered (`device-notes-publish`, refspec at line 65), so
  the harvested `ci:*` notes reach the forge although `scripts/brit-attest.mjs` says they stay on the
  device (Table 1, gap 7, checked by hand). Publishing them deliberately or filtering them is the
  steward's decision (section 9).
- A manifest that declares no rustflags inherits the ambient `RUSTFLAGS`, and that value is not in
  the gate's key (Table 1, gap 8).

## 3. Where the time goes

**One landing push, step by step.** The first per-step readings were recorded on shem for one
landing push that changed one gate-runner script and documents: 37 rows, 1,350 s of wall clock. The
largest steps were `orchestrator-tree` 512 s, `eprfs-live-corpus` 489 s, the Rust style leg 205 s,
`orchestrator` 90 s and `address-profile-lint` 14 s; every other step took under 7 s (integrator's
record, shem, 2026-11-24). These are one push on one device. For comparison, ethosengine's ledger
gives `orchestrator-tree` a mean of 150 s (p90 304 s) over its last green runs (`tree-scope-table.md`).

**One roll, gated twice.** Roll 2a v3 is the third revision of a set of changes rolled to the test
fleet; gating it took 2 h 24 min on shem and recorded 12 verdicts. Gating the same head plus five
commits (two crates and one backlog document; the head is called "the fold" in the evidence, after
its fix to how a device reads a sibling's event channel) took 2 h 07 min and reused none of them.
Both durations are the integrator's record (shem, 2026-11-24). Ethosengine measured the cause from
the key material of the two commits (`v3-to-fold.tsv`). Keys moved for three reasons:

1. **A whole shared crate in the read set.** The large cargo gates declare a read of the whole
   `elohim/eprfs/device-runtime` crate. The tsv shows the keys of `cargo-bridge`, `elohim-ark`,
   `elohim-storage`, `jenkins-bridge` and `steward-node` moved on that read, and for each one step
   fingerprint moved as well (for example `steps[cargo-build-storage]`). [GAP: whether narrowing the
   device-runtime read alone would have held these keys, given that a step fingerprint also moved;
   this blocks the Verify of the station that narrows those reads, not its shadow.]
2. **Keys on the whole commit tree.** 17 gates are keyed on the whole commit tree. Two mechanisms
   cause it (`tree-scope-table.md`): the read audit scans only `.rs` files, so a gate that is not a
   cargo gate has no read set and falls back to the tree, and a cargo gate on the unaudited-recipe
   list or with a read the audit cannot resolve does the same. A non-Rust gate cannot declare a read
   with `// gate-reads:`; its only route today is a suite read list, which exists for the orchestrator
   alone (`genesis/orchestrator/suite-reads.json`).
3. **A gate rooted at the repository.** A gate whose directory is `.` keys the whole tree as its
   project tree whatever its reads say. Most such gates are already among the 17; in the tsv one
   more, `manifest-hygiene`, moved on its project tree alone while its reads are paths. [GAP: whether
   its directory is `.`, which would make it the supplied "one gate on the repository root". Blocks
   no station.]

For a decision, use 17: the tsv's rows marked `scope=tree` are the measured count, from key
material at two commits. The supplied 16 was the integrator's count from a note, and
`tree-scope-table.md`'s 18 adds `orchestrator-tree`, which the tsv records with no key at that
comparison. The tsv also records no key at all
for `eprfs`, `eprfs-live-corpus`, `orchestrator-tree` and `memory-ceremony` in that comparison, with
the reason "no toolchain declared". [GAP: whether that reflects the script's environment or the
gates' state at those commits; it bears on how much of the 2 h 07 min a narrower key would have
saved, and does not block a station.]

**Which gates cost the most.** On ethosengine's ledger the mean seconds are `elohim-storage` 1,359
(p90 1,669), `eprfs` 1,210 (p90 1,565), `steward-node` 376 (p90 502), `elohim-ark` 78,
`jenkins-bridge` 75 and `cargo-bridge` 55. Storage and eprfs are 79% of the cargo gates' time
(`split-units-table.md`).

**Why attestations alone would not have saved this week's re-runs.** When the five commits moved
every key, there was no attestation of any kind for the new closures: nobody had run them. A richer
verdict on the old closures, carrying its toolchain readable and three agreeing executions, would
have described questions nobody was asking any more. What would have saved time is a unit small
enough that two crates and one backlog document move few keys, so that most of the 12 verdicts
still answered the new head. That is the second half of the proposal (4b). The first half (4a) pays
when another device has already run the same key, as on 2026-11-22, and it lets a lander decide
from what it can read instead of from which device signed.

## 4. The proposal

The proposed reading: a verdict is one attestation among several on an artifact. A lander's policy
reads the artifact's properties (its closure, toolchain and platform, who ran it and with what
standing, how many independent executions agree, and later enrichments such as a household witness
or a DHT notarization) and decides how much re-execution it owes, on a scale and not as a yes or no.
This reading is the integrator's, recorded in the sidebar file; it is not a ruling.

### 4a. The attestation

**What a verdict should carry, readable.** Each property below is a field a reader can read
without recomputing a hash. Proposal, for ruling.

| Property | Why a lander's policy needs it readable | Readable today? (Table 1) |
|---|---|---|
| Closure and inputs: the key, plus what it covers (declared inputs, audited reads, and whether its scope is paths or the whole tree) | to tell a narrow key from a whole-tree key, and to let a peer see whether its own checkout covers the closure | no: hash only, `inputs_hash` (GE:812) |
| Toolchain: compiler version, cargo, the pinned toolchain | to judge whether a result from another machine fits this one | no in the verdict (GE:274-283); yes in the artifact record (DS:68-91) |
| Platform and features: target triple, features, rustflags, hardware profile | the same; and to close the ambient-`RUSTFLAGS` hole of section 2 | no in the verdict; yes in the artifact record (DS:20-99) |
| Method or judge version: manifest CID, recipe, runner, key version | to tell a result produced under an older judge from one produced under the destination's judge | partly: `manifestCid` is signed (GE:810), the rest is inside the hash |
| Signer and standing: the signing key, the device it resolves to, and what that device is to the reader | to count independent executions and to give a sponsored runner zero | partly: brit key; a sibling's arrives through an unsigned note (GE:879-884) |
| The execution it reports: computed here, reused or carried; the commit; whether the inputs were clean; when; its seconds | to separate an execution from a reuse of someone else's, so that a reuse is never counted as a run | [GAP: Table 1 has no column for it; whether `gate-verdict@1` records its basis and seconds today is not established. The plan's contract says a verdict names its basis (section 9). Does not block the enrichment station] |

Most of this already exists, readable, in the artifact record that `epr artifact put` writes (row 2
of Table 1). One reading of the proposal, the integrator's: the gate verdict carries what the
artifact record already describes, rather than a second description of a closure.

**The enrichment record.** An enrichment is a separate signed statement that names an existing
attestation or artifact by its CID and adds one property to it. Examples:

- another admitted device re-executed the same key and agrees (same result for a gate; the same
  output CID for a build);
- another device re-executed it and disagrees; the artifact record's divergence Cite (CU:436-458) is
  the precedent for this, and it is the one case that lowers a grade;
- a household story ran against the artifact and witnessed it (the household record already names
  an artifact CID, HA:167-240);
- a device accepted the artifact into its store (section 7);
- a notarization on the DHT (whether this is an enrichment or a separate record is section 9's
  first question).

An enrichment never rewrites the original. The original's bytes, and so its CID, stay as they were;
a reader gathers the original and every enrichment it can verify and reads them together. Today's
behaviour, where a re-put replaces the note (GE:801-804), is what this replaces for the cases above.
The steward ruled on 2026-10-22, as the change-detection design cites it (section 4, from
`.claude/scripts/.epr-meta`), that a record others interpret and a judge that gives one answer belong
in the native crates, and that a script may sense and call. Here that means the enrichment record
and the reader that judges it are written in Rust, and the gate runner and the push hook only call
them. [GAP: where the enrichment record lives (a
brit note, the flow store, the artifact store or a crossing kind) and the five answers of the p2p
design gate for it: its class, whether an existing entry type holds it, its head-plane cost over a
year, its identity and the function that creates it. Not answered here. It does not block the
shadow station, which only writes records; it blocks any reader acting on them.]

**What counts as an independent execution.** One run of the same key, with the same result, signed
by a different admitted device with its own key. The same device twice counts once. A sponsored
runner under grant counts as zero, as ruled. The count is computed by the reader, from the records
it can verify itself; a count that a producer writes into its own record is never read. Today that
needs one more thing first: a sibling's verdict reaches the reader through an unsigned note
(Table 1, gap 3), so the reader must be able to verify the brit signature on the verdict itself.
[GAP: whether the reading device can verify a sibling's brit signature on the verdict today; Table 1
records a roster check, not a signature check. Blocks the grade reader's count, not its shadow line.]

**What the count is not.** It bears on how much re-execution a lander owes. It is not a count of
independent witnesses toward reach. The plan keeps the two apart: a sibling's result buys speed and
never counts as an independent witness (section 9, test 5), and reach counts principals with
standing; k, the plan's count of independent principals, is 1 today, because two devices of one
steward are one principal (section 10). A grade
that says "two independent executions agree" means two machines got the same answer; it never means
two people vouched for it.

### 4b. The unit

Attestations let work be done once for everyone; a finer unit makes the work smaller. Table 2 ranks
the native gates by recorded seconds and names the finer unit each could stand on.

**eprfs first, per crate.** eprfs has 13 crates (Table 2) and already selects among them: `GATE_NARROW`
computes the reverse dependencies of the changed packages (`gate-runner.mjs:629-660`). What it
lacks is somewhere honest to record the narrowed result. A narrowed run would have been recorded as
a green for the whole project closure, so `GATE_NARROW=on` is now refused until a per-package verdict
key exists (landed `d74bc6707`; integrator's record, shem, 2026-11-24). A shadow per-crate key for
eprfs exists on an unlanded branch, `self/ethosengine-crate-shadow` at `aa5424180`, and is in
independent review; it decides nothing (integrator's record, shem, 2026-11-24). Table 2's integrity
note sets the rule: a per-crate verdict keys the crate's own sources, its path dependencies, its
features, flags and toolchain, and never the project's.

**The storage gate second, and only if the seconds say so.** `elohim-storage` is first by seconds
(1,359 s mean) but is one crate, so a per-crate key does nothing for it. Its candidate units are by
step and by feature set: two clippy compiles and one test compile of the same source under
different features, plus one unit per test binary for changes under `tests/` (Table 2). What blocks
consuming any of them is Table 2's first blocker: one recipe, one process, one verdict. Every one of
these gates is `gate-steps.sh` running its steps in one shell, its exit status is the first red,
and no step records anything of its own. Splitting storage pays only if the per-step reading shows
the feature-set compiles are a large share of its time; until that reading has a few pushes behind
it, the split is an expectation, not a measurement (Table 2).

## 5. The graded policy

**Proposal, integrator's, for ruling.** Each row is a class of change; each column after "Today" is
something that might suffice to skip running the gate locally. A cell gives the proposed value and
one clause of reason. "Suffices" means the lander reuses; "does not suffice" means it runs the gate
or looks for more. Where a cell is stricter or looser than today, it says so. Every loosening of a
present refusal is integrity-class: its shadow may be built, and its `on` waits for the steward's
ruling and an adversarial review. N is the number of independent executions agreeing, counted as in
section 4a; the values proposed for N are open (section 9).

| Class of change | Today | Own green for this key | One admitted sibling's green | A carrier's green under grant | N independent executions agreeing | Household witness required | Never skipped |
|---|---|---|---|---|---|---|---|
| Documentation only | own green or an admitted sibling's green for the same key, for enrolled gates; a whole-tree key moves on any document | suffices; unchanged | suffices; unchanged | suffices alone only after its shadow has agreed for ten landings; a loosening, integrity-class | not required (N = 1) | no | no |
| Test or tooling code (not a gate's own judge or recipe) | as above | suffices; unchanged | suffices; unchanged | does not suffice alone, since it counts zero executions; unchanged | not required (N = 1) | no | no |
| A library crate | as above, per project; a per-crate result cannot be recorded, so `GATE_NARROW=on` is refused | suffices at the crate's own key; a finer key, not a looser rule | suffices at the crate's own key | does not suffice alone; unchanged | not required (N = 1) | no | no |
| A serving path | as above for the gates; the serving receipt reads household attestations (SR:105) and refuses only at `dev` and `main` (ruled 2026-11-24, `7308da97c`) | suffices for the gate; unchanged | suffices for the gate; unchanged | does not suffice; unchanged | not required (N = 1) | yes at `dev` and `main`, as ruled; unchanged | no |
| A gate's own judge or recipe (a governance path) | every sibling verdict refused for every gate in the push (GE:480-507); [GAP: whether this device's own green is refused too; it decides whether the last column is stricter than today; blocks no station] | suffices for gates whose own judge and recipe did not change; for the changed gates, see the last column | suffices for gates whose own judge and recipe did not change; a loosening of the whole-push fence (station Fence-scope), integrity-class | does not suffice; unchanged | not required | no | yes, for each gate whose own judge or recipe changed; the rest of the push is no longer refused |
| A release closure taken from a sibling or the forge | two paths that disagree. The operative one for an `epr` or `brit` release is the store's ingest and land, which refused brit 0.3.4's verified closure (source mismatch, then dirty build) and then rebuilt it from source for about 30 minutes. The second, the sibling draw, keys an `epr` release by the commit the sibling claims, with no recomputed key: a known inconsistency the proposal retires. (Mesh binaries fetched with `epr artifact fetch` recompute the key, verify bytes per chunk and treat the holder count as advice.) | not applicable: own green means this device built it | does not suffice alone: one producer's word for a commit is the sibling draw's rule, which the proposal retires | does not suffice; counts zero | suffices when this device recomputes the key from the clean closure, N distinct admitted producers hold the same output for it and none holds another (divergence refuses), the bytes verify, and the device's own judges still pass; ethosengine's, unreviewed | not proposed; open | no |

What changes against today, in short. A library crate keeps today's rule at a finer key, and a
serving path is unchanged. A governance path narrows the fence from the whole push to the gates
whose own judge or recipe changed, which is Fence-scope and the largest loosening in the table. A
release gets one rule in place of two paths. Of today's two, the store's ingest and land is the
stricter: it refuses a closure whose source or build does not match, where the sibling draw accepts
the sibling's claimed commit. The proposal is stricter than the sibling draw, because it recomputes
the key, and refuses a mismatch as ingest and land does; what it changes against ingest and land is
that verified bytes from agreeing producers are accepted instead of rebuilt.

**Why documentation alone admits a carrier's green** (the integrator's reasoning). A
documentation-only change selects only gates whose result does not depend on the machine, such as
link and cite checks and formatting; their cost is small; and a wrong green is caught by the next
landing's own run. That is why a carrier's green may stand alone there, after its shadow agrees,
and nowhere else in the table.

**The release row in full** (ethosengine's, unreviewed; its note is not yet tracked). Accept a
release when this device recomputes the release's key from the clean closure; a number of distinct
admitted producers (the N of the table) hold the same output for that key and none holds a
different output, so a divergence refuses; the bytes verify to that output; the host's judges still
pass; and the device records its own acceptance, referencing the producers' records by CID. That
acceptance record is an enrichment in the sense of section 4a.

## 6. How this meets the plan's six tests, and what it does to the open stations

The tests are the plan's section 9 (tests 1 to 5) and section 10 (test 6).

1. **A request reserves nothing.** Reading attestations and writing an enrichment reserve no seat,
   lease or device; nothing here asks anyone for anything.
2. **A claim is bounded and the requester can always run alone.** A grade never forbids running a
   gate; a missing, unreadable or unverifiable attestation reads as "run it here".
3. **Capacity is sensed by the carrier.** Unchanged: this design changes what a lander reuses, not
   who carries or how a device decides it has room.
4. **The unit is a step keyed on exactly what its recipe reads.** Section 4b is this test, and the
   two narrowing stations of section 8 are its broken rows: a whole shared crate and the whole tree.
5. **Settlement is a signed record keyed on the closure; a sibling buys speed, never a witness.**
   Kept: the independent-execution count is not a witness count, a sponsored runner counts zero, and
   an enrichment is a signed record keyed by CID. Meeting it fully needs the reader to verify the
   signature on a sibling's verdict, not only the roster (section 4a).
6. **Would this behave the same on rented sandboxes with a remote cache?** For the unit, yes:
   per-crate keys, keys on exactly what is read, and hermetic inputs are among the practices the
   change-detection design's industry notes describe (its section 5), to be taken as given. For
   the attestation, no: signed records from named devices, a count the reader computes, enrichment
   by CID and a later notarization are the native part, at the rung of reach called `self` and
   `sibling` today.

**Effect on the open stations of plan section 8.**

- **CD-reading** stays first and gains a reader. The per-step reading has landed and its first rows
  exist (`README.md`: landed at `9f272d343`; one push's 37 rows on shem), but its Verify, ten
  landings on two devices, is not met. Its seconds decide whether the storage split of 4b pays.
- **CD-key-moves** is partly absorbed. Its sampled re-execution of one reused verdict per landing is
  exactly "another device re-executed and agrees, or disagrees": its result is written as an
  enrichment record instead of a separate finding.
- **CD-no-fingerprint** is unchanged and belongs to the unit half: a gate with no key can carry no
  attestation at all.
- **CD-read-selects, CD-rust-style-once, CD-always-run and CD-ci-stage-selection** are unchanged; they
  concern selection and repeated cost, not what a verdict carries.
- **Fence-scope** (and CD-fence-scope, the same work) becomes the governance row of section 5. Its
  integrity class, its ruling and its adversarial review stand as written.
- **CD-jenkins-shadow** becomes the carrier column. Its shadow line can print a Jenkins green as an
  attestation whose independent-execution count is zero; its three rulings are the cells of that
  column.
- **F4-ask-unattended** keeps its Verify. A reading, not a ruling: an unattended device deciding
  whether to take a posted miss could read the artifact's attestations to judge whether its run
  would add an independent execution the lander's class needs. That is what the steward's "how a
  peer can be helpful" asks for.

## 7. The steward's other two reminders of the day

**Presence over the p2p runtime.** His words (2026-11-24, about 16:10Z, a reminder for design, not a
ruling): "reminder we're going to need to to reuse our ark/berth to design a efficient way to watch
and track agent and human presence and live action over the p2p runtime." This design gives
presence one thing: an attestation is evidence that a device was present and working at a time, and
an enrichment ("this device re-ran it and agrees") is a durable trace of live action that a watcher
can read after the fact without having watched. It does not address presence itself: the watchers
that died without a word, a sleeping laptop that reads as a gone one, a mooring that read stale while
its device kept exchanging, and a held tool call that stalled two devices (the sidebar file's list).
Those need the presence design he asked for.

**Updates, releases, upgrade and rollback.** His words (2026-11-24, about 17:10Z, a direction, not a
ruling): "something to keep in the back of your mind is auto-updates, publishing releases,
upgrade/rollback, and maintaining comms through all those processes.. if we do all this \"right\"..
these all should be rather effortless and elegant, while still everything being being legible to
epr, bounded/gated by reach, and distributed over p2p, and notarizable by holons.. noting any
friction that we could solve so we catch and sense, plan, implement, and resolve...  those frictions
in flight is always a value add.." This design gives that path two things. Taking a release becomes
an acceptance record of the same kind as any enrichment, so brit 0.3.4's case (refused as a source
mismatch, then as a dirty build, then rebuilt for about 30 minutes though verified binaries existed)
would be decided by reading the release's attestations instead of by rebuilding. And a rollback is
accepting an earlier artifact whose attestations still stand: nothing is rebuilt and nothing is
rewritten. It does not address keeping communication through an update (a peer daemon restart that
drops a sibling mid-fold), sessions that keep a pinned `epr` across a landing, a device that reads
its roster from an old checkout, or whether rollback works across two devices at all, which was
never exercised (the sidebar file's list of frictions).

## 8. Stations

The order puts shadows first, and every `on` that is integrity-class waits for the steward's ruling
and an adversarial review. Records come before readers, because a reader needs something to read;
the per-crate key comes early because its shadow already exists; the two narrowing stations come
last because each changes a key from its first `on`. The steward may reorder (section 0).

- [ ] **Attest-enrichment — a native enrichment record exists: signed, naming an existing attestation or artifact by CID, adding one property, and read by nothing** (serves push-gate-pays-once)

State: not built. No record kind has an enrichment primitive; a changed verdict replaces the note
(GE:801-804; Table 1, gap 6). The artifact record's divergence Cite (CU:436-458) is the nearest
precedent. Where the record lives and its p2p design-gate answers are a GAP (section 4a). In shadow
it is not integrity-class: it is written and nothing decides on it. Its first writer is CD-key-moves'
sampled re-execution.

Verify: on one landing the sampled re-execution writes an enrichment that names the reused verdict
by CID; the reused verdict's bytes hash the same before and after; and the landing's decisions are
byte-identical with the writer on and off.

- [ ] **Attest-crate-eprfs — eprfs records a verdict per crate, keyed on the crate's own closure, so that `GATE_NARROW=on` has somewhere honest to record** (serves push-gate-pays-once)

State: a shadow per-crate key exists on `self/ethosengine-crate-shadow` (`aa5424180`), unlanded and in
independent review; it decides nothing. `GATE_NARROW` already computes the reverse dependencies
(`gate-runner.mjs:629-660`), and `on` is refused until a per-package key exists (`d74bc6707`).
Integrity-class from `on`.

Verify: in shadow, for a commit that changes one eprfs crate, the per-crate keys of crates outside
its reverse dependencies are unchanged and those inside move; a mutation of one crate's path
dependency moves that crate's key. After the ruling and review: a landing that changes one crate
reuses the per-crate greens of the rest, and the log names each.

- [ ] **Attest-grade — the reader computes a grade and an independent-execution count for each selected gate and prints them beside today's reuse decision** (serves push-gate-pays-once)

State: not built. Nothing counts independent executions where a decision is made (Table 1, gap 2);
a sibling's verdict arrives through an unsigned note (gap 3), and whether the reader can verify the
signature itself is a GAP (section 4a) that blocks the count but not the printed line. Native, as
ruled; the gate runner calls it. In shadow it decides nothing.

Verify: for ten landings on shem and on ethosengine, every reuse line has a grade line beside it,
and the decisions are byte-identical with the reader on and off; a fixture holding a sibling's green
and a sponsored runner's green for one key prints a count of one, and a fixture holding two greens
from the same device prints one.

- [ ] **Attest-release — taking a release from a sibling or the forge says whether the store would have accepted it without a rebuild** (serves [GAP: which habit; the plan names `runtime-upgrade-propagation` for F5 and `workspace-device-arrival` for tool distribution, and this station could serve either. Does not block the shadow])

State: brit 0.3.4's verified release closure was refused by the device's store, first as a source
mismatch until the checkout's pin matched and then as a dirty build because it had been ingested
from a dirty checkout; the path that worked rebuilt from source for about 30 minutes (integrator's
record, shem, 2026-11-24). The acceptance rule is ethosengine's, unreviewed (section 5, release row).
In shadow it accepts nothing; `on` is integrity-class.

Verify: on the next release drawn from a sibling, the shadow prints accept or refuse and the check
that decided (recomputed key, producers, divergence, bytes, judges) while the existing path runs as
today; a fixture in which two admitted producers hold different outputs for one key prints refuse.

- [ ] **Attest-narrow-cargo-reads — the large cargo gates declare what they read of `elohim/eprfs/device-runtime` instead of the whole crate** (serves push-gate-pays-once)

State: `cargo-bridge`, `elohim-ark`, `elohim-storage`, `jenkins-bridge` and `steward-node` declare a
read of the whole crate, and their keys moved on it between roll 2a v3 and the fold, each with one
step fingerprint as well (`v3-to-fold.tsv`). [GAP: what each of the five gates actually reads of
device-runtime; nothing in the sources lists it. Blocks this station's Verify.] Integrity-class: it
changes a key.

Verify: in shadow, replayed on the two commits of `v3-to-fold.tsv`, each gate's narrowed key either
holds or names a moved file it reads; and CD-key-moves' mutation test moves the narrowed key for a
file inside the narrowed read and holds it for one outside.

- [ ] **Attest-read-list — the gates that are not cargo gates declare a read list, and those rooted at `.` name their own directory, so that their keys leave the whole tree** (serves push-gate-pays-once)

State: the read audit scans only Rust sources, so a non-Rust gate has no read set; the existing
route is a suite read list, which exists only for the orchestrator (`genesis/orchestrator/suite-reads.json`);
a gate whose directory is `.` also needs its directory narrowed (`tree-scope-table.md`). That table
ranks the gates by seconds, with `orchestrator-tree` first (on the unaudited-recipe list; its recipe
runs non-Rust legs such as `lint-jenkinsfiles`) and `elohim-app` and `sweettest-check` next.
Integrity-class: it changes a key.

Verify: in shadow, for each gate taken, its row in a re-run of the tree-scope computation reads
paths, not tree, and a documentation-only commit outside its reads leaves its key unchanged.

## 9. Open questions for the steward

1. Is a DHT notarization a later enrichment of the same artifact record, or a separate, stronger
   record? (Put to him on 2026-11-24 in the sidebar file.) In practice an enrichment is a signed
   record a device stores and its peers verify, while a notarization is an entry other agents
   validate on the DHT, and every one adds to the head plane's growth.
2. What is N for each class of section 5? The table proposes 1 everywhere except the release row,
   where ethosengine's proposal leaves it open.
3. May a sponsored runner's green ever stand alone for anything? The table proposes it for
   documentation only, after its shadow agrees.
4. Should the harvested CI notes be published to the forge deliberately, or filtered out of the
   device notes publisher (section 2, found on the way)? Publishing makes them readable by anyone
   with read access to the forge repository; each holds a Jenkins verdict harvested by
   `scripts/brit-attest.mjs`, with an inputs hash over the commit tree, a manifest CID and the
   harvesting host's name, signed by the device's brit key (Table 1, row 4).
5. Do the six properties of section 4a cover what a peer needs to decide how it can help, or is
   one missing? A constructed example: ethosengine sees shem's posted miss for a library crate's
   gate, reads the closure's attestations, and finds one green whose signer is a sponsored runner,
   which counts zero; its own run would be the first execution by an admitted device, which the
   library row says suffices, so it takes the miss.
6. Is section 8's order the one to build in?

## 10. What was not established

**Readings and proposals in this document.**

- The proposed reading of section 4 (a verdict is one attestation among several) is the
  integrator's, from the sidebar file; the steward has given a direction, not a ruling.
- The property list (4a), the graded table (5) and the station order (8) are proposals for ruling.
- That the gate verdict should carry what the artifact record already describes is the integrator's
  reading (4a).
- The F4-ask-unattended reading in section 6 is a reading.

**Unreviewed notes.**

- The release acceptance rule (section 5) is ethosengine's proposal, unreviewed, and its note is not
  yet tracked. So is ethosengine's description of today's two acceptance paths.
- The per-crate shadow key (`aa5424180`) is in independent review and has not landed.

**Measurements and evidence.**

- The per-step readings are one push on one device; nothing here is a mean over pushes.
- Whether `manifest-hygiene` is the supplied "one gate on the repository root" (section 3; the
  whole-tree count to use is the tsv's 17).
- Why the tsv records no key for eprfs, eprfs-live-corpus, orchestrator-tree and memory-ceremony.
- Whether narrowing the device-runtime read alone would have held the five cargo keys, since a step
  fingerprint moved too, and what those gates actually read of the crate.
- Whether `gate-verdict@1` records its basis and seconds today, and whether a reading device can
  verify a sibling's brit signature on a verdict.
- Whether a governance-path push also refuses this device's own green, and so whether the
  governance row of section 5 is stricter than today for the gates whose judge changed.
- Table 2's account of the `GATE_NARROW` hole was read from the code, not run; the refusal of `on`
  has since landed (`d74bc6707`).
- Table 1's cells were gathered by a search agent; only the cells marked checked were re-read by hand.

**Not answered:** where the enrichment record lives and its p2p design-gate answers; which habit
the release station serves.
