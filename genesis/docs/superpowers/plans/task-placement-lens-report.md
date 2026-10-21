---
id: placement-lens-report
title: Applying the three design-lens findings to the placement-contract spec
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#23
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-lens-brief | Apply the three design-lens findings to the placement-contract spec under rulings R1–R6 | sha256:fc3447c4b5d657a0 | path: genesis/docs/superpowers/plans/task-placement-lens-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:8b7eee252e2374c4 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
---

# Applying the three design-lens findings to the placement-contract spec

Item P0b, gap `#23`. All 67 lens findings (local-first F1–F22, gradient and commons F23–F45, p2p design
gate and concern canon F46–F67) were read in full from the workflow journal. The scratchpad copy was
truncated, so the journal is the source. Each finding was applied to
`genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md` under
rulings R1–R6, the brief's six reconciliations, and the orchestrator's mid-task correction on doorways
(R6a). Nothing was staged, committed or pushed.

**Gate evidence.** `epr flow cites verify genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md`
returned `✅ all cites content-addressed + resolvable` and `EXIT=0`. For this document, `epr flow context`
printed an ambiguous gate line (`just gate constants-sync | domain-types | memory-ceremony | reach-drift |
schema-codegen | schema-validate`). None of those projects owns a design doc, so no `just gate` was run.

**What held.**

- Section numbers did not change; only the heading text of §7.3 changed.
- The register set by the absolutes pass held: `never` or `always` occurs 12 times before and after these
  edits, and every new rule names its gate and the gate's state.
- Reconciliation 6 held: the depth-as-placement bullet in §7.2, "eligible is not enlisted", and its §16
  story were not touched.
- Each code identifier brought in from the lenses was checked against the tree:
  - `build_salvage_candidates`, `list_fresh`, `salvage_recheck_seconds`, `deterministic_custody_id`,
    `mvp_ignores_household_and_capacity_fields`, `reclaims_self_held_custody_under_a_new_agent_key`,
    `SalvageOutcome`, `spool_custody_author.rs` and `transport_resolve.rs` exist.
  - `REA_ACTIONS` exists in `content_store_integrity` and no validation path uses it.
  - `Intent` and `ServiceOffer` exist in `content_store_integrity`.
  - Two lens claims were wrong and are corrected below (F48, F56).

**Edits by section.**

| Section | What changed |
|---|---|
| §1 ruling 3 | The record of a carrying is kept at the need's reach |
| §2 → §3 | The holons §9.1 spaces bullet moved to §3 as a working hypothesis under product review (reconciliation 5) |
| §7.1 | Posture and berth-offer homes; what stays on the device; the pool's offer |
| §7.2 | Reworked across its whole length |
| §7.3 | Rewritten, with the heading renamed |
| §7.4 | Rewritten |
| §8 | Grant scopes are separated from commitments |
| §11 | Lead rewritten; three carrier rows and a Today column added |
| §12.2–12.7 | Edited throughout |
| §14 | Opening rewritten; nine rows added or replaced; head-plane cost, concern canon and network stakes added; SDO/RWA and identity-namespaces paragraphs extended |
| §15 | Thirteen nodes added or reworded |
| §16 | Three stories reworded; one story added |
| §17 | Slices 1 and 5 edited |
| §20 | Who sets the numbers |

## Findings

| # | Sev | Disposition | Where and why |
|---|---|---|---|
| F1 | blocker | applied | §12.3 walk: named absences, local first, continues past them; §17 slice 1 proof and fixed return property. The working tree's `walk.rs` already yields `WalkItem::Absent` |
| F2 | major | applied | §7.2 new bullet "Reads only what it holds" (R4), with its gate state: no generalized floor yet |
| F3 | major | applied | §7.2 new bullet "Alone" |
| F4 | major | applied | §7.3 "The floor does not wait for the ceiling": newest held set, else the built-in set |
| F5 | major | applied | §7.2 new bullet "Leases are long-lived" (reconciliation 3, R1); §15 lapsed-lease node |
| F6 | major | applied | §7.1 "What stays on the device"; §7.2 membrane bullet ("not offered") |
| F7 | major | applied | §7.3 "At device scope" bullet |
| F8 | major | applied-differently | §7.1 and §14 show posture as a signed record, household-local inside a household and notarized on `Intent` only for public or commons reach (reconciliation 1), not "held by those in reach" alone; hysteresis on posture changes |
| F9 | major | applied | §7.2 "For a hosted person" bullet. It keeps the doorway as a participant that holds the key, not as an apology |
| F10 | major | refused | Operator stance on doorways (R6a). The doorway-fetched delivery-peer list is not called a defect, and no "may add but never source" rule was added. "A tier reads its own store first" already stands in §12.3 |
| F11 | major | applied | §12.2 overlay lives on the writing device; depth 0 write brings the path to depth 1 (R6) |
| F12 | major | applied | §12.3 rule 2: a sealed overlay's root has its author's reach; `has_blob` answers absent |
| F13 | major | applied | §12.4: signs with its own key, provisional root before first contact, enrollment re-signs nothing (merged with F41) |
| F14 | major | applied | §12.4: depth 1 opens offline; doorway-held key and visitor cases |
| F15 | major | applied | §12.6: device's own envelope first; task kept and named when no berth |
| F16 | minor | applied | §12.4: what each arrival learns and what survives its loss |
| F17 | minor | applied | §12.5: recompute locally when no holder of the output is reachable |
| F18 | minor | applied-differently | §12.7: a person's partial clone is depth 1 or 2, not depth 1 alone, so the existing on-read mode stays; depth 0 for consumers with no working copy; offline integrate |
| F19 | minor | applied | §12.2: depth-2 let-go keeps overlay bytes and sole copies |
| F20 | minor | applied | §7.2 eligibility and §12.3 rule 2: unknown reach refuses and does not fetch; own store skips reach |
| F21 | minor | applied | §7.2 scarcity: with no ratified order, the built-in set's order, and it says no priority was chosen (merged with F33, F66) |
| F22 | minor | applied-differently | §7.1 "serve as a wake ingress"; §7.2 battery default per reconciliation 2 (carrying for others off by default and liftable; own-household holding deferred, not refused) |
| F23 | blocker | applied | §7.3: every parameter set in force is ratified, whether or not a commitment moves; a ratified tuning carries its record; the floor reads none without it (R5) |
| F24 | major | applied | §7.3 heading "each member's elohim drafts, the participants tune"; no space-elohim, no pool-elohim; reads only its own person's pressures |
| F25 | major | applied | §7.3 "For a pool" bullet: members ratify the facility's parameters; council holds the commons share; humans in infancy; sortition councils govern and may overturn |
| F26 | major | applied | §7.3 band: no weight reaches zero; record-weighted parameters are consequences with a notice first rung |
| F27 | major | applied | §7.2 closing line: absence costs sensing and drafting, not the participants' power to tune |
| F28 | major | applied | §7.2 opening: honest about divergent views; converges on the honoured count; probe measures convergence |
| F29 | major | applied | §7.2 eligibility: unread input is ineligible; today only the opt-in and the household key are read |
| F30 | major | applied | §7.2 membrane: household reconcile is a public baseline until admission; "intimate reach" claim removed; unknown household is its own domain |
| F31 | major | applied | §7.2 observe and converge: honoured is claimed until a holding probe; credit by key, not binding; §15 nodes |
| F32 | major | applied | §7.4: why-answer bounded by reach; others' refusals counted, not listed |
| F33 | major | applied | §7.2 scarcity: members' ratified order; shown at the need's reach and no wider; §15 charter-update node |
| F34 | major | applied | §15 node "Elohim draft → ratified parameter set"; §16 story "An elohim's draft tuning, unratified, changes no placement on any peer" (`placement-ceiling`) |
| F35 | major | applied | §11 rows for commitment record, query tier (depth 0), inference berth; closing paragraph names the commitment record |
| F36 | major | applied | §11 Today column for every carrier; tier row bound reworded |
| F37 | major | applied | §11 lead: a carrier that can sign as the person is chosen by the person; others on demand (reconciliation 4) |
| F38 | major | applied | §12.3 rule 2 "on both sides", pending §19 question 10; read-pattern paragraph rests on the requester side |
| F39 | major | applied | §12.3 rule 3: shared-tier budget is the members' appropriation rule; §20 who sets the numbers |
| F40 | minor | refused | Operator stance on doorways (R6a); same ground as F10 |
| F41 | minor | applied | §12.4, merged with F13 |
| F42 | minor | applied-differently | Per reconciliation 2, as F22; §15 battery node reworded |
| F43 | minor | applied | §7.2 names the salvage ticker; §15 "Salvage ticker → event-driven floor" |
| F44 | minor | applied | §7.1 Space row: what members pledged through the pool, and no more |
| F45 | minor | applied | §7.3 "Lineage on every draft"; §12.5 independence observed, not a count of keys |
| F46 | blocker | applied | §7.2 new bullet "A commitment is kept at the reach of the need it answers" (R2); §14 Placement commitment row; §11 row; §1 ruling 3; §15 node |
| F47 | major | applied | §14 Head-plane cost paragraph (R1); §15 per-blob → per-root node |
| F48 | major | applied-differently | §14 rows for parameter set, head and ratification; §7.3 C5 re-derivation. The lens named `Proposal`/`ProposalVote` as an existing elohim-DNA entry type; no entry type of that name is defined there (only a mishpat comment says votes moved), so the row states the class and leaves the carrying entry to be read before the ceiling slice |
| F49 | major | applied | §7.3 three kinds, each with its own author (R5); band bullet lists what a set may not touch |
| F50 | major | applied | §7.2 make-before-break for a ratified set; §15 and §16 tuning probe and story reworded |
| F51 | major | applied | §7.2 guarded self-candidacy, deterministic id, release lowest-ranked first (R4); §15 reworded plus "Own offer → self-candidacy"; §16 "converges on the same holders" |
| F52 | major | applied | §7.2 per reconciliation 2; §16 story renamed to "A phone on battery is not asked to carry for another household unless its person offers, is told why, and still holds its own household's bytes when it charges" |
| F53 | major | applied-differently | §14: berth offer with posture on `Intent` for public or commons reach only; a separate household-local row (reconciliation 1); a private-facts row |
| F54 | major | applied-differently | §14 SDO/RWA "The floor and the ceiling" paragraph. The lens's "counts as one domain" became "unproven, and the design says so", because counting every self-declared household as one domain was not defined anywhere |
| F55 | major | applied | §8 and §14: grant is consent and eligibility, commitment is placement (R3); integrity checks bounds keys only |
| F56 | major | applied | §14 opening, corrected: the `_ => None` fallthrough is in mishpat integrity's `commitment_action_requirements`, not `validate_commitment` |
| F57 | major | applied | §14 Concern canon C0–C14, seam-registry row at birth; §17 slice 5 |
| F58 | major | applied | §7.2 Rank: method named by CID (R4); §7.4 commitment carries set CID, method CID and eligible-set digest |
| F59 | major | applied | §12.3 `fetch_blobs(root, cids, policy)` (R6); §14 C4 note: `Refused` and `Unverifiable` arms; one refusal shape outside reach |
| F60 | major | applied | §14 derivation row: corpus-digest bundling, counted; §12.5 record travels only at its root's reach |
| F61 | minor | applied | §14 View is Private (B) when discovered |
| F62 | minor | applied | §14 rows for tree node and blob, overlay, kept set |
| F63 | minor | applied | §14 Identity namespaces: enrolled device key; bindings evidence-only until cross-signed; §15 node |
| F64 | minor | applied | §14 Network stakes paragraph |
| F65 | minor | applied | §7.3: published-at versus in-force-for; the space of the need's membrane; revert is a new declaration |
| F66 | minor | applied | §7.2 scarcity: each unserved party derives "not carried"; nobody sends it; §15 node |
| F67 | minor | applied | §7.3: no set in force without ratification; drafts name the layered model identity |

Totals: 58 applied; 7 applied differently (F8, F18, F22, F42, F48, F53, F54); 2 refused (F10 and F40, both
under R6a).

## Concerns

1. **Two places still read the doorway ask as a gap.** Under R6a they may need reconciling. The seat that
   rewrote §2 and §12.3 rule 6 left two related passages as they were, and I did not touch them because
   they are that seat's:
   - The §12.3 paragraph beginning "A tier reads its own store first" still says the service worker
     "gets them by asking" and calls "the other half" a missing node.
   - The §15 node "Requester → tier list" is still marked "Partial: … fetches them from the doorway".

   The orchestrator should rule whether these now contradict §2.
2. **The ratification record's carrying entry is unverified (F48).** The ceiling slice must read which
   elohim-DNA record carries proposals and votes before it is built.
3. **Today the spec states rules nothing enforces.** These are marked absent and owned by slices 5 and 9:
   the floor reading only what it holds, guarded self-candidacy, per-root commitments, household-local
   records below public reach, the battery default, method and parameter-set CIDs, and the ratification
   gate.
4. **The gate line is ambiguous for a design doc.** The only check run is `epr flow cites verify` (EXIT=0).
   No `just gate` project owns this file.
5. **No covenant floor is needed.** These edits are inside the declared purpose ("settle the
   placement-contract spec"), and no blind-reader variance was used.
