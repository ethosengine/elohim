---
id: placement-settle-report
title: Settling the placement-contract spec — report
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#1
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-settle-brief | Settle the placement-contract spec from the verified claim corrections and the coherence read | sha256:476173e9b116d7c8 | path: genesis/docs/superpowers/plans/task-placement-settle-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:d2a76864bcbbebd1 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:ddad5a389eb52537 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
---

# Settling the placement-contract spec — report

Seat: P0 of `2026-10-20-placement-contract-architecture-plan`. Target:
`genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md`
(941 → 1052 lines; section numbers unchanged; 65 exact-match replacements, each asserted to match once).
Nothing staged, committed or pushed. The pre-edit copy is kept as scratch at
`/tmp/claude-0/-projects-elohim/7e52adff-23e3-4a4c-86f3-735370908262/scratchpad/spec-before-settle.md`.

Gate evidence (the brief's verify step; the context gate line was ambiguous across doc gates, and the
brief names this check):

```
$ epr flow cites verify genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
epr flow cites verify …: ✅ all cites content-addressed + resolvable
EXIT=0
```

No cite was added, so `cites seal` was not needed. One cite description (storage-physics benchmark) was
edited in prose; its slug, fingerprint and path are unchanged.

## Evidence re-check

Every UPHELD item was re-checked against the source before it was applied. Highlights:
`storage-physics-benchmark.md` lines 79–101 (adam 6.56 / 2.22 GB, `wasm.db` 675+132 MB, DHT 60–90 MB,
gertrude 3.16 GB); live-session spec lines 355–381 (ranking, C1, household's own session);
`acquisition_pins.closure_rule_json` and `POST /api/v1/pins`; `client-resilience.feature:61-63`;
`household-at-rest.feature:60-64` (6 calls, 0 permits, 10 CPU s); qahal `message` `status: planned`;
`epr_atom_service.rs` `IntegrityNotify` and the unhandled-kind arm; `steward/node/src/p2p/nat.rs` TODO;
alpha.yaml's ClusterIP note on 9876; `RecoveryRequest` removed from EntryTypes; `compute_task.rs`
receipt binding; `elohim_service.rs` `MockBackend`, `wisdom.rs` `select_env_backend`;
`classify_reach`, `should_stock`, `can_serve_at_reach` (test-only caller); `affirm_membership`,
`issue_household_invite`, `sponsor_cid`; `salvage_pass` at custody.rs:458 and `SALVAGE_CAPACITY_ENABLED`;
schema `gates[].closure` (:264) and `graph.edges[].indexed` (:136); ark-core `ChildSpec`; trust contract
I2 and `c2-monotonic-authority@2` ("Not a placement rule"); `GRANT_SCOPES` and the agent-key recipient
check; `services/holds.rs`, `db/blob_arrivals.rs`; `ark describe`; `DeliverabilityVerdict`;
`reach-enforced-http.feature` (3 live, 3 `@wip`); eprfs adapters and `StorageEndpoint`; `TreeLimits`
fields; `seal_git_tree`; brit `snapshot restore`; `eprfs-core = 0.1.2` in brit; `handle_put_blob` and
`backfill-blake3` (no such binary); `apps-sw.ts` `scorePeerForSw` and `/api/v1/peers/delivery`;
measure habit rung H (jessica); feature tags of the five §16 rows; `device_enrollment.rs` self-approval
refusal; `heartbeat.rs:384-391`; fork patch 3c1e80525 under pin 96bcd2bee.

## Rulings touched (brief rule 3)

Facts under these rulings were wrong and were fixed; no ruling itself changed.

- **§7.2 Self-adopt**: the claim that live-session hosts are chosen by self-adoption was wrong (they
  are chosen by the participants; a host never chooses itself). The self-adopt rule stands.
- **§7.2 Event-driven, with hysteresis**: "heals fill, never move" was cited as an existing placement
  rule; it is a head rule that says it is not a placement rule. Hysteresis is now stated as the
  contract's own new rule. "Costs nothing" became "costs almost nothing" with the measured budget.
- **§7.3**: a definition of "the elohim" was added before the ceiling text (interpretability); the
  ceiling's powers are unchanged.
- **§12.3 rule 5**: a caveat names where the alias is written today. **§12.3 preamble**: "every tier
  implements" was present tense for a design rule; it now says only local adapters implement it.
- **§12.1 inversion 5**: the current sealer's work follows the tree, not the change; the inversion
  stands as design.
- **§2 Spaces**: the item sat under "already ruled" but holons §9.1 is a technical hypothesis under
  product review; the text now says so and states what D6 actually seals. See concern 3.

## Corrections (settle-corrections.md, 63 items; 58 UPHELD, 5 REJECTED)

| # | Item | Disposition | Why |
|---|---|---|---|
| 1 | §4 nearest existing needs (REJECTED) | skipped | Spec stands 2/2; the `closure` name collision it raised is covered by #26 |
| 2 | §1 fleet peer 6.0–6.6 GB "most of it compiled wasm" | applied | Benchmark: fleet split unread; household code store dominates |
| 3 | §2 live session "two household-diverse hosts" | changed-differently | Merged this BEST with #11's so §2 and §5.2 say one thing (circle hosts, ranked, household's own case, direct streams) |
| 4 | §2 Spaces are ruled | applied | Holons §9.1 is a hypothesis; D6 seals only the candidate status; "(ruling 18)" became "§10 item 18", its real home |
| 5 | §2 Addressing / BLAKE3 | applied | `/blob` routes accept `blake3-<hex>` today; dropped the record-proves cite name, which I could not resolve to a path |
| 6 | §5.1 "No `closure` anywhere" | applied | `closure_rule_json` exists on pins and in replicates-content; nothing resolves it |
| 7 | §5.1 "Conductor wasm dominates" | applied | Household and gertrude numbers; fleet split unread; also fixed the frontmatter cite description; dropped "5.27 GB" (benchmark says 5.3 GB live) |
| 8 | §5.1 "untested fork patch" | applied | 3c1e80525 is in pin 96bcd2bee; unit-tested, never measured, no VACUUM |
| 9 | §5.1 Experience depth probe | applied | `client-resilience.feature` scenario exists, red-first |
| 10 | §5.2 Live Proves `idle-is-free` | applied | Lane budget is 6 calls/min; dropped "the reading has not been taken", which I could not verify |
| 11 | §5.2 Live Supplies | applied | Live-session §7 ranking and household's-own rule |
| 12 | §5.2 Live Today "nothing built" | applied | `elohim/elohim-live-session` exists; story `@wip`; the MessagePack name (in the slice plan, not the spec) became "chosen only for slice 1's direct streams" |
| 13 | §5.2 Chat Declares | applied | qahal `message`/`thread` planned |
| 14 | §5.2 Chat Today | applied | platform-one row 5 and package-composition A2 fragments |
| 15 | §5.2 Attention notify receiver | applied | One kind; ingress is test-gated |
| 16 | §5.2 libp2p NAT stub | applied | Relay/DCUtR/AutoNAT wired; only `nat.rs` is a stub; 9876 ClusterIP |
| 17 | §5.3 Head Proves | applied | runtime-upgrade-propagation green; the other two are dataplane-convergence checks |
| 18 | §5.3 Head Today dataplane-convergence | applied | Harness defect and measured hub-carries-edit 0/1 |
| 19 | §5.3 Authority "no signed circle-admission record" (REJECTED) | skipped | Spec stands 2/2; left that cell as written even though #24 asked to reword it (concern 4) |
| 20 | §5.4 Recovery Records | applied | `RecoveryRequest` is now a plain struct; a Content entry carries the request |
| 21 | §5.5 Compute fulfilment / carrier records nothing | applied | Runner notarizes accept/reject/work; line numbers trimmed (spec quotes paths, not lines, in tables) |
| 22 | §5.5 Inference `LlmBackend` | applied | MockBackend only; in-process transport off everywhere; line numbers dropped |
| 23 | §5.6 Reach doorway gate | applied | Two gates: live `classify_reach` and unused `can_serve_at_reach` |
| 24 | §5.7 Membership admission | applied | Joiner-written `Membership`; coordinator-only check; Records cell took #34's wording |
| 25 | §5.7 Space shape change matcher | applied | `salvage_pass`, opt-in, never moves a commitment |
| 26 | §6 needs collide with schema names | applied | Paragraph added after §6's table; its truncated last clause completed |
| 27 | §6 tevah `ProcessSpec.closure` | applied | Spec-only field; built type `ChildSpec` has none; "renamed from" became "named apart from" (manifest.rs says the name avoids VF's ProcessSpec) |
| 28 | §7.2 self-adopt / live-session hosts | applied | Live-session §7, C1 |
| 29 | §7.2 "heals fill, never move" | applied | I2 is a head rule; the policy says it is not a placement rule |
| 30 | §7.1 Device metered link | applied | `network-class` route exists; no metered fact in archetypes |
| 31 | §8 scopes coordinator change | applied | `GRANT_SCOPES` in storage; zomes do not check the value; `runtime-offer` split out as a kind |
| 32 | §8 gap 1 fulfilment | applied | Also applied to §15 Grant → fulfilment (BEST truncated; completed in the same terms) |
| 33 | §8 gap 2 collective party | applied | Agent-key recipient check |
| 34 | §8 gap 3 circle admission | applied | Dropped "Any agent can commit a valid Membership naming any collective", which goes past what I verified |
| 35 | §9 Held bytes | applied | holds.rs and blob_arrivals.rs exist |
| 36 | §9 Compute carried | applied | No `ark status`; `ark describe` is the passport |
| 37 | §9 Runnable app | applied | Verdict built; failure layer specced |
| 38 | §9 Reach probe absent | applied | HTTP-only probe; named its real tag `reach-enforced-http` under the habit |
| 39 | §9 carrier at rest probe | applied | Also §7.2 and §16; the §5.2 cell took #10's wording |
| 40 | §9 materialize-what-is-read unwired (REJECTED) | skipped | Spec stands 2/2 |
| 41 | §12.3 "Every tier implements" | applied | Only local adapters |
| 42 | §12.3 `TreeLimits` fields | applied | Seven fields, with file path |
| 43 | §12.1/§12.3 every reader verifies (REJECTED) | skipped | Spec stands 2/2 |
| 44 | §12.1 transfer 1 any holder can serve | applied | No remote adapter yet |
| 45 | §12.2 depth 3 eprfs reads the pin | applied | Pin names a head ref; added the §15 node "Pin → closure" as the skeptic asked |
| 46 | §12.1 inversion 5 cost follows change | applied | `seal_git_tree` re-puts everything |
| 47 | §12.7 every commit has a root | applied | On-demand `brit snapshot seal` |
| 48 | §12.3 rule 5 / §15 sha256→iroh | applied | Caveat on rule 5; §15 line merged with #53 |
| 49 | §12.3 requester composes list | applied | SW scores its own list but fetches candidates; added §15 node "Requester → tier list" |
| 50 | §12.6 `just measure` | applied | Rung H on jessica; `--on adam` refuses |
| 51 | §13 bytes dedupe, runtimes do not (REJECTED) | skipped | Spec stands 2/2 |
| 52 | §14 grant scopes row | applied | Consumers named |
| 53 | §15 sha256→iroh partial | applied | Merged with #48's §15 text into one node |
| 54 | §15 brit F2 draft | applied | Also §12.2 "brit's sealed draft" |
| 55 | §16 peer-executed-stage concern | applied | File tag is `operator-runtime-surface` |
| 56 | §16 local-first-shared-sheet concern | applied | Split into two rows as BEST (truncated single-row alternative not used) |
| 57 | §16 recovery feature concern | applied | Tagged `@identity-lineage`, no `@concern` |
| 58 | §16 household-reciprocity concern | changed-differently | Took the skeptic's "more accurate row" (proposed new feature, habit kept) over the bare retag, because the existing file does not test the story |
| 59 | §16 declared-storage-policy proposed | applied | File exists, `@wip` |
| 60 | §17 slice 1 proof under the gate | applied | Two-step proof; brit pins registry 0.1.2 |
| 61 | §18 row 1 ceremony | applied | Location cell names the ceremony script; the truncated line-211 part is covered by #18 |
| 62 | §18 row 2 access_control.rs | applied | Unused function; live gate is `should_stock` |
| 63 | §18 row 4 heartbeat lines | applied | 384–391 verified |

## Coherence findings (settle-coherence.md, 17 items)

| # | Finding | Class | Disposition | Why |
|---|---|---|---|---|
| C1 | §7.1 App row lists 5 of 13 needs | interpretability | applied | Marked representative; §6 is the full list |
| C2 | `External` undefined | interpretability | applied | Defined at §12.3 first use from `TreeEntryKind::External` (kind + opaque reference) |
| C3 | "Signatures beyond the three inputs" | interpretability | applied | Walk inputs fixed; query signatures and return type settled while building |
| C4 | Ceiling slice unnumbered | interpretability | applied | "follows slice 5 as an unnumbered slice of its own" |
| C5 | Slice 1 and the §16 feature file | interpretability | applied | Feature files written beside slice 1, born `@wip`, mesh scenarios from slice 2 (matches plan lane T) |
| C6 | `closure` two definitions | interpretability | applied | §5.1 and §6 now say root + view naming the bytes |
| C7 | "floor" four senses | interpretability | changed-differently | One sentence in §7 reserves bare "floor" for placement and requires the qualifier elsewhere, rather than defining terms other specs own |
| C8 | §3 vocabulary column referent | interpretability | changed-differently | "In the table below" (the table follows the sentence; the finding said "above") |
| C9 | "the elohim" undefined | interpretability | applied | Defined at §7.3 as members' elohim and a pool's plural council; never overrides or ratifies |
| C10 | §12.8 heading level | correctness | applied | `##` → `###` |
| C11 | "first guess" | interpretability | applied | "baseline placement" in §7.2 (both uses) |
| C12 | ProjectionPath/TreeLimits unsourced | interpretability | applied | File paths added |
| C13 | "install ladder" undefined | interpretability | applied | "(register, then keep)" |
| C14 | eprfs/brit/rakia/lvi unexpanded | interpretability | applied | One paragraph after the two-words note in §1, from rakia's and lvi's own specs |
| C15 | §12.3 → §15 cross-reference | correctness | applied | Each rule mapped to its state or §15 node |
| C16 | §10 grouping | preference | skipped | Preference; costs more than one word |
| C17 | Today-column register | preference | skipped | Preference |

## Concerns

1. **The design-lens findings are not applied.** Plan item P0 reads "the verified claim corrections,
   the coherence findings and the design-lens findings"; the brief names only the first two. The lens
   file (`settle-lenses.md`, including a `blocker/design` finding on the walk's behaviour for a node it
   does not hold under `LocalOnly`, which binds slice 1's return type) changes design, so it needs a
   ruling or its own seat. P0's verify line is met for corrections; the lens part remains open.
2. **Downstream seals are now stale.** The plan and the brief cite the spec by fingerprint; both need
   `epr flow cites seal` once this settle is accepted. I did not write either file.
3. **§2 now carries a hypothesis under a heading that says "already ruled".** The fact is right; where
   the Spaces item sits may want the operator's eye.
4. **#24 asked for the §5.3 Authority cell to be reworded too, but #19 (REJECTED) says that cell
   stands.** I followed the rejection. §5.3 Authority still says "no signed circle-admission record";
   §5.7 and §8 now say more precisely "none that validators check".
5. Four BEST texts were cut off in the input (#26, #32, #56, #61). Each was completed in its own terms,
   as the table shows.
6. Line numbers in BEST texts were dropped wherever the spec quotes a path without one, to keep its
   register. §5.7 Space shape change and §18 keep theirs, as the spec already did.
