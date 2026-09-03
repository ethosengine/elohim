---
epr-habit-version: 1
id: runtime-death-witnessed
invariant: >
  A supervised child's death is witnessed by the runtime that was its parent: its exit class,
  last words, resource snapshot, the supervisor's own preceding decisions, and the passport are
  written to the node's own disk before any restart, offered to the custodians named by the node's
  standing custody commitment, rendered at /epr/{cid} for whom the reach admits and refused to a
  stranger, and attested once per incident when a conductor is next available — so a household
  operator reads why a peer died from the peer itself, with no cluster tool in the path.
status: red
active: true
checks:
  - "a2o @concern:death-witness (genesis/a2o/features/resilience/death-witness.feature — four stations in five scenarios (3a custodian render, 3b stranger refused), all @wip until the household mesh launches its conductors under the envelope; runnable then via cd genesis/a2o && npx cucumber-js --tags '@concern:death-witness and not @wip')"
  - "cargo test --lib conductor::process_manager (elohim/elohim-storage — ring_buffer_keeps_the_last_n_and_drops_the_oldest, readiness_outcome_prefers_child_death_over_attempt_budget: the dead-vs-slow classifier and the ring, landed 264ce8ce4; a local leg for the classifier only — nothing yet writes, names, offers, or attests a witness)"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md (the primitive, the decision register §12, the corrections to the two atoms §1)"
  - "atoms: genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md · genesis/data/timeline/backlog/elohim-native-compute-envelope-the-pod-under-the-runtime.md"
  - "incident: genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md (CORRECTED DIAGNOSIS — the sentence the witness must be able to write)"
  - "supervisor today: elohim/elohim-storage/src/conductor/process_manager.rs (live only under EMBEDDED_CONDUCTOR=true — the alpha conductor pod; never on the mesh)"
  - "sibling habit: operator-runtime-surface (inspect/restart/reseed/reconcile as commitment-gated verbs — the witness makes 'inspect a runtime' true for a dead child)"
retire-when: >
  when the supervisor moves into the envelope crate (elohim/ark) and this atom moves with it,
  and a household operator has read a peer's death from the peer itself on a released steward
  build with no developer-shaped tool in the path — the habit then describes the runtime, not a
  storage feature.
---
DELTA 2026-09-02 (BORN red — design canonized, nothing built; the census refuses `unwired` when a
runnable check is declared, and two are: the a2o concern is declared with every station @wip, and
the cargo leg proves only the classifier — so the invariant is measured NOT held, which is red).
FIRST MOVE: S0 of the tevah spec (genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §11): the envelope as its own launchable unit — ark-core + ark-supervisor + the ark binary — and hc-mesh.sh launching one, then three, household conductors under it instead of `setsid nohup hc sandbox run`, so that a SIGKILLed conductor leaves a witness in the spool on the mesh. Until then every household-lane assertion is vacuous by construction (the only pipe-owning supervisor ships inside an alpha container behind a frozen image tag): lifecycle-as-fixture precedes lifecycle-as-feature. Station 1 green is the first measured step toward green; stations 2–4 carry it there.

GROUNDING: the tevah spec sealed the
primitive and the witness path after 21 adversarially verified grounding briefs. What the
grounding established for this habit: (1) liveness after readiness exists nowhere — a conductor
dying after boot is neither witnessed nor restarted by anything (`/health` stays 200 by design;
`/health/serving` is deliberately not a probe; the conductor pod has no bridge); (2) the ring
buffer + `try_wait` classifier landed (264ce8ce4) but ends as a logged error value — no disk
write, CID, offer, or attestation; (3) on the household mesh the conductor is not storage's child
(`hc sandbox run` or the `direct` launch mode, both deliberately parentless), so the lane that may
kill cannot see the supervisor and the lane with the supervisor may not kill; (4) `just mesh recovery cold <peer>` kills
STORAGE, never a conductor — the witness atom's done-when named the wrong drill; (5) custody
commitments are per exact blob hash, so a fresh witness has no custodian — the spec adds a
standing custody-spool commitment; (6) a new `runtime:*` content type moves the DNA hash — the
witness rides `issue-report` with a metadata kind. Status is red: the concern is declared and counted, and no station passes until S0's
launcher exists.

DELTA 2026-09-02 (operator decisions, spec §12 items 20–24): ACTIVE — takes the WIP-fence slot from
operator-runtime-surface (green, wired). Branding split: tevah in prose, `ark` in code (`elohim/ark/`,
`ark-core`, `ark-supervisor`, the `ark` binary). The constitutional DNA batch waits for S1's measured
shape; the epr-pvc bridge is the unminted guide-star at the end of this chain. S0 is built through the
update-propagation loop's shape: seed artifact refs are closure-CID/channel-head from the first commit,
resolved pinned-local in S0, channel-head in S1. Next measured step: station 1 on the household mesh.

DELTA 2026-09-02 (clarification thread): units renamed `RuntimeManifest` / `Berth`; the passport this
habit's witness carries is the BERTH's (blade-scoped), and the household footprint is a tier above it —
station 1's "Jessica's passport" reads as Jessica's berth passport. Spec §3.1, register 25–29.

DELTA 2026-09-02 (S0 landed — station 1 GREEN on the household mesh): the ark crate family
(elohim/ark: ark-core pure, ark-supervisor I/O, ark binary — commits 6eb479480..8c522e336, gate
`just gate elohim-ark` green: 60 + 37 + 7 tests) launches the three household conductors
(`MESH_CONDUCTOR_LAUNCH=ark just mesh start`, commit 39299469e + da6968f44). `just test mesh
'@concern:death-witness and @station-1'` → 1 scenario, 8/8 steps PASSED; receipt
genesis/a2o/reports/sprint-report-household-20260902T205500Z-da6968f4.{json,md}. By hand: a SIGKILLed
conductor's witness was listable from the peer's own spool 520 ms after the kill (signal 9, uptime,
40 stderr lines, artifact sha == the file on disk, verdict restart; conductor back and ready with the
ark's incarnation unchanged; flows.jsonl carries the VF event/process projections). One defect found
and fixed on the mesh: the witness's passport snapshot lacked the pid (captured after the Died
transition) — 8c522e336. Status stays RED: stations 2–4 (custody, atom-home render, stranger refused,
anchored incident) remain @wip and are S1. NEXT: S1 — storage consumes ark-supervisor, custody-spool
commitment, amber-offered → green, passport atom; the `epr-rea` torn-line reader (M6) and the S1 gaps
the reviews named (orphan reaper, SpawnFailed class, shutdown-during-backoff) are in the plan.

DELTA 2026-09-03 (S1 station 2 GREEN on the household mesh): `just test mesh '@concern:death-witness and
@station-2'` → 1 scenario, 6/6 steps PASSED; receipt genesis/a2o/reports/sprint-report-household-20260903T035920Z-6f621068.{json,md}.
Chain proven live: ark spool → Jessica's storage ingests the witness (row before blob, one digest; c25728639 +
537186247) → the shard replication plane carries the private row to the custodians (dial REPLICATION_INTERVAL_SECONDS,
01d4ea9ef) → each custodian's SpoolCustodyAuthor expands its own custody-spool pledge (seeded by the prologue's
seed-spool-custody leg, a6e26bf87; authored on the custodian's own conductor = the counter-signature) into a
custody-blob for the witness, joining by DIGEST across the bafkrei/sha256 renderings (5dd815193 + 8dbc7d815) → bytes
pulled + serve-blob receipt from Jessica's peer. Measured 03:55–03:57Z: custody rows on both custodians ~75 s after
the kill; the story's budget is now the measured two minutes (b2834b34b). ZERO DNA change. Defects found only on the
mesh and fixed: hash-rendering join; commitments view drops a bare-string classification (M11, view fix in flight,
a2o reads the row's own metadata meanwhile); cucumber's 90 s step timeout. Status stays RED: stations 3a/3b/4 are
@wip (render, stranger refused, anchored incident). NEXT: station 3b first — the shard replication plane replicates
PRIVATE rows and their blobs to every household peer with no reach gate (M9, now with a named site), so "a stranger
is refused" is the substrate's next honest move; then 3a render; then 4 anchoring.

