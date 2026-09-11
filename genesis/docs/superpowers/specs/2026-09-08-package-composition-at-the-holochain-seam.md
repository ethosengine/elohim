---
title: "Package composition at the Holochain seam — do's and don'ts, from what we measured"
id: package-composition-at-the-holochain-seam
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: every rule in §3/§4 is either cited by a landed package (bundle, candidate, or manifest addition) or retired with a measurement that overturned it, AND sprint 2026-09-08 T12 has chosen the §5 budgets with receipt paths, AND the D6 cell-qualifier contract (T13) is sealed so D3 is enforceable rather than declared
actor: agent:orchestrator@fable-5.1
written: 2026-09-08
habits: [dataplane-convergence, runtime-upgrade-propagation, happ-lineage-migration, operator-runtime-surface]
cites:
  - "ai-stewarded-commons-reimplementation-plan | §5.1 bounded witnessing below the application; §8 keep/deepen/replace decision experiment; §10.3 the six boundaries the default hApp obscures; §10.6 what a developer composes | sha256:0d5f1300b5d615dc | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "holons-are-spaces-how-we-use-holochain | §0b sealed decisions D0–D10; §6 hard constraints; §6½ arcs; §9.1 holon granularity | sha256:ac1de36d2423be82 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - "elohim-seam-map-concern-routing | what-do-you-ADD disambiguator: manifest → SDK seam, crate → bridge seam, native code → mod seam; participation tracks T1–T4 | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "runtime-artifacts-elected-content | runtime artifacts are elected content heads the storage plane replicates; rung 5 | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "compute-envelope-tevah | the ark/tevah envelope a hosted executable runs under; death witnessed | sha256:6c9f84bc831ac1bc | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "accountable-correction-contract | §4 notification carries the signed act; §8 submission binding | sha256:691e8b89f394214c | path: genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md"
  - "ratchet-to-delivery-dataplane-sdk-lanes | the SDK destination: an app developer persists, syncs, accounts, projects and each object carries its governance and economic surface | sha256:162f2cde07f0de8e | path: genesis/docs/superpowers/specs/2026-08-28-ratchet-to-delivery-dataplane-sdk-lanes-design.md"
  - "seamless-groupware-ux-addendum | the surface this spec's rules must carry; written alongside | sha256:a9698f7152f60430 | path: genesis/docs/superpowers/specs/2026-09-08-seamless-groupware-ux-addendum.md"
  - "sprint-velocity-quiescence-holochain-close | the sprint whose T4, T9, T9a, T10, T12, T13 this spec's rules bind to | sha256:1e2863ecb8af29e6 | path: genesis/docs/superpowers/plans/2026-09-08-sprint-velocity-quiescence-holochain-close.md"
  - genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md
  - genesis/data/timeline/backlog/object-open-with-runtime-facet-and-install-ladder.md
  - genesis/data/timeline/backlog/feedback-discovery-sweep-is-o-n-in-history.md
  - genesis/data/timeline/backlog/runtime-steward-adoption-of-fleet-cut-release.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
---

# Package composition at the Holochain seam — do's and don'ts

**Why this exists.** The groupware experience we want (open any object with a peer-native app, one
click to install it to a device or a hub, feel at least as seamless as Google Drive and at times
faster) meets Holochain at one seam: *which acts get currency-level integrity, and from which
validation context*. That seam is high risk for the performance story in both directions. Put too
much behind witnessing and every ordinary act pays a DHT round trip and a head election. Put each
tool or group in its own DNA and you mint tiny networks that cannot link to each other, each
paying its own gossip, its own arc, its own recovery. This spec states the rules we can defend from
measurements taken on the household mesh and the alpha fleet, so that packaging decisions stop
being re-derived per app.

It is written under the sealed decisions of 2026-09-06 (D0–D10). Where a rule rests on a
measurement, the measurement is quoted. Where a rule rests on a decision, the decision is named.
Nothing here grants a habit a status; it binds packaging work to the habits that will measure it.

## 1. The evidence the rules stand on

| # | Measured or sealed fact | Source | Rule it drives |
|---|---|---|---|
| E1 | A clone cell under gossip costs **+1.62 MiB RSS, +2.11 MiB disk (~8× the idle figure), +21 FDs, +10 threads**; create+enable 148 ms median | fixtures-clone report 2026-09-07 | §3 D1, D2 |
| E2 | The storage projection is keyed by role (`h_app_id`), not by cell; the sync plane carries clone rows; a receiving peer **re-authors clone content into its base cell** | D6 measured, `clone-content-escapes-via-shared-projection` | §3 D3, §4 A2 |
| E3 | DHT planes ARE isolated by identity (base-actions/ops/content/targets diff empty after a clone write) | same report | §3 D1 (isolation is real where it is needed) |
| E4 | Coordinator-only changes keep the DNA hash and hot-swap over one admin call; integrity or modifier changes move the hash and re-key on reinstall | CLAUDE.md "DNA changes don't redeploy by default"; rung-5 shift 2026-08-31 | §4 A3, A4 |
| E5 | Rung 5 propagates a **single-role** coordinator candidate by election across the fleet; the canary applies ~60 s, attestation +86 s; a release cut FOR the fleet binds `appliesTo` to the fleet's running coordinators | workspace-to-fleet crossing 2026-09-06; adoption atom | §4 A4, A5 |
| E6 | The DNA hash is path-dependent (per-crate metadata includes the package path); a CI RUSTFLAG change moved every hash once (2026-09-02) | run note 2026-09-07; `dna/elohim/justfile` | §4 A6 |
| E7 | Head-plane cost is linear in items: every notarized head is a conductor round trip, an election participant, a divergence surface (~4k at genesis; 24 MB corpus is count-bound, not byte-bound) | arch-dataplane row 16 | §5 P1 |
| E8 | Feedback discovery sweeps 8 members per 60 s with no cursor; 107→333 s across a day's rounds | discovery atom | §5 P3 |
| E9 | One rejected op blocks the author's cell forever on 0.7 (storageArc null on every peer, no authorities) | memory `household space partition blocks` | §3 D4 |
| E10 | `post_commit`/`emit_signal` fire only on the AUTHORING cell; a remote peer never receives a "…Committed" signal via the DHT | memory `post_commit signals are cell-local` | §5 P3 |
| E11 | Restart churn ≈ 20 min on the fleet; warm recovery 258 s homo-iroh; conductor RAM ∝ corpus at full arc | trust-contract runbook; recovery matrix 2026-08-28 | §3 D2, §5 P4 |
| E12 | Sealed: D0 classify before witnessing; D1 the connected graph across validation contexts is the product, aggregates are views never DNAs; D3 membrane = peer-side validation of the join proof; D4 cross-plane acts need named successor authority; D6 group clone spaces are a candidate behind a measured phone/hub budget | holons doc §0b | everything below |

## 2. Vocabulary (so the do's and don'ts are unambiguous)

- **Witness context** — one DNA hash + one network seed: the set of peers that validate each
  other's acts by the same rules. A *cell* is one agent's membership in one witness context.
- **Currency-level integrity** — an act whose validity other parties will *rely on* without
  re-checking: authorship, admission, a commitment, a transfer, an accepted correction. D0 Class A.
- **Product bundle** — UI assets, renderers, an optional executable, declared workflows and
  dependency versions. Content-addressed; an elected artifact head.
- **Witness binding** — the *reference* a bundle carries to the context(s) it needs: DNA hash,
  rule version, admission requirement. A reference, never a copy of the DNA.
- **Resolved composition** — the pinned record of which bundle bytes, domain/schema versions,
  runtime capabilities, and witness bindings a given install resolved to (§10.6 step 5).
- **The seam** — the line between what a peer *witnesses* (Class A in a context) and what it
  *projects, syncs, or computes* (everything else).

## 3. Don'ts (each with the measurement that forbids it)

**D1. Don't mint a witness context per app, per tool, or per small group.**
E1 says every cell costs ~2 MiB disk and ~10 threads *idle-plus-gossip*; a suite of ten tools ×
three groups is thirty cells on a phone before a single document exists. E12/D1 says the product is
the connected graph *across* contexts; a DNA per group is a partition, and links do not cross DNAs.
The clone-per-household mapping was retired (D6). A new context is justified only by a **new
integrity requirement** (different rules) or a **measured isolation need** (§10.6 step 4), and
never by a UI, a brand, or a team boundary.

**D2. Don't let the number of contexts on a device float with the number of relationships.**
Contexts must be bounded by the phone/hub budget chosen in the decision experiment (sprint T12).
An unchosen budget is an open acceptance prerequisite (reimplementation plan §8), so until it is
chosen, the answer to "can I add a context" is *no* by default.

**D3. Don't project or sync a row without its cell qualifier.**
E2 is the measured escape: a role-keyed projection let clone content cross the sync plane and be
re-authored in another peer's base cell. Every projected row and every sync document carries the
DNA hash and cell id of the act it came from, and the re-author path refuses to move content
across contexts. This is the D6 contract (sprint T13) and it is a **precondition** for any
multi-context packaging, not a follow-up.

**D4. Don't put a rule in integrity that can be a rule in a coordinator or in the app.**
E9: one rejected op partitions the author out of the context on 0.7 with no recovery short of a
new key. E4: integrity changes move the hash and re-key. The integrity zome holds only what the
whole network must agree on forever; everything else lives in coordinators (hot-swappable, E4) or
in projections (rebuildable). Validation of *facts about the world* (the hall closes at six, the
delivery happened) does not belong in integrity at all (§5.1).

**D5. Don't ship a bundle that carries a DNA.**
A bundle carries a witness *binding* (a reference). Shipping DNA bytes inside app bundles is how
two installs of "the same app" land on two networks (E6 makes this silent). The witness artifact
is packaged independently enough that a UI update never forces a network migration (§10.6 step 4).

**D6. Don't equate the four installer acts.**
Obtain a package · enable its runtime capabilities · join a community · provision-or-join a
witness context (§10.3). A one-click surface *composes* them; the system never treats "installed"
as "joined" or "capable" as "authorized". The UX addendum owns the surface; this spec owns the
refusal: any code path that grants one act as a side effect of another is a defect.

**D7. Don't make the ordinary path pay for first-contact assurance.**
The performance story (§6.1) is that trust makes the ordinary path light and depth makes repair
possible. Reading a document you already hold is a projection read (zero round trips); opening it
with a peer's runtime is a blob fetch from the nearest custodian over iroh; neither waits on a DHT
get. Witness verification is *verify-locally-then-serve* (trust-contract runbook), off the hot path.

**D8. Don't rely on DHT signals for cross-peer wake-up.**
E10: a remote peer never gets a post-commit signal. Cross-peer acceleration is a **notification
that carries the signed act** (correction contract §4), and discovery without notification must
still converge (station 1). Packages that need "tell the other peer" declare it as a notification
kind, not as a hope about gossip latency.

## 4. Do's

**A1. Do classify every record the package introduces before it gets a route.**
D0: authority · durable source · validation contract · rebuild path · partition behavior. The
`p2p-design-gate` skill is the gate; the expected answers for the groupware records are: the
runtime *offer* is Class A (a commitment); local install state is private (B) or ephemeral (C);
the object being opened must name its witness context (D3).

**A2. Do reuse an existing witness context when the rules and admission fit.**
Today's bundle creates five roles with cloning disabled; that is a deployment choice to inventory,
not the ontology of community software (§10.3). A meal board, a spreadsheet, and a chat thread that
are all "content authored by admitted members of this community under these rules" share one
context and link freely. New vocabulary is a **manifest** addition (SDK seam), never a DNA
addition (seam map: "what do you ADD?").

**A3. Do put per-app behavior in coordinators and projections, and version them by hash.**
E4: coordinator-only changes hot-swap with no re-key. The bundle's witness binding names the
coordinator hashes it was tested against; the adoption controller (`services/release_adoption/`)
judges whether the peer already runs them (sprint T4: "runs the target bytes" IS adopted).

**A4. Do ship bundles as elected artifact heads and install-to-hub as adoption by election.**
E5: the mechanism exists and has crossed to the fleet. A product bundle is one more artifact class
under the same election; "install to my hub" is the hub applying a candidate. No second
distribution channel, no push.

**A5. Do cut single-role candidates, and make `appliesTo` describe the *target's* running
coordinators, never the builder's.**
E5's binding surprise (the workspace peer refused for running the builder's mishpat) is the
general shape of every "install on a device that is not the fleet". A candidate for a device names
what that device must run; adoption is judged against what it runs.

**A6. Do keep the witness identity reproducible.**
E6: identical integrity source packed from two paths yields two DNA hashes. Until the remap is
folded into the next integrity crossing (D5, sprint T9a measures it), every packaging step that
*packs* a DNA is a fleet operation done from the canonical path, and app packaging never packs a
DNA at all (D5 above).

**A7. Do declare runtime placement and authority separately from the bundle.**
Which device executes, serves, stores ciphertext, or runs a workload is a *placement* decision
(§10.3 row "Runtime placement"); who may read or act is an *authority* decision. A hosted
executable runs under the ark/tevah envelope so its death is witnessed and its restart is governed;
placement never changes who authored an act or who can decrypt it.

**A8. Do record the resolved composition and make it recoverable.**
§10.6 step 5: pin bundle bytes, schema versions, runtime requirements, enabled capabilities,
witness bindings. Recovering on a new box restores the composition and the legitimate grants
(§10.4). This record is what Shefa's per-device view reads.

**A9. Do budget the head plane per meaningful act, not per row.**
E7: heads cost linearly. Immutable or bulk content rides composite roots (row 16); an
individually authored, governance-bearing act keeps its own head. One model refresh must not
create one notarized event (§8 decision experiment).

**A10. Do treat cross-context evidence as portable qualified evidence, not as shared membership.**
D4: a cross-plane act names its successor authority and its partition behavior *before* code.
Two communities coordinating one supper do not merge contexts; an accepted commitment in one is
evidence carried, with its origin DNA hash and action hash, into the other (correction contract
§4 `actRef` is the shape).

## 5. The performance story at the seam, stated as budgets

The claim "as seamless as Google Drive, at times faster" is only honest as measured budgets on the
hardware people actually own. These are the axes; sprint T12 chooses the numbers and cites the
receipts.

| Axis | What "faster than a hosted drive" means here | Measured today | Lever |
|---|---|---|---|
| P1 Local-save latency | Write to the local projection, return; witnessing is async | (T12) | Head-plane budget (A9); no per-row heads |
| P2 Open-with latency (LAN) | Blob from the nearest custodian over iroh, no central RTT | recovery 258 s warm is the *cold* bound (E11); LAN fetch unmeasured | Custody facet on the chip picks the near custodian |
| P3 Cross-peer visibility | Notification carries the act (D8); sweep converges without it | 8 members / 60 s, 107→333 s (E8) | Sprint T6 cold-retire + notification re-arm |
| P4 Recovery / restart | Composition restored from the resolved record; heads re-elected | ≈20 min fleet churn; 258 s warm | A8; fills-never-moves |
| P5 Idle cost per context | What a phone pays to *be* in a context | +2 MiB disk, +10 threads, +21 FDs per cell (E1) | D1, D2 |

The rule for adding anything near the seam: name which row it moves, in which direction, and how
that will be read (a receipt path under `genesis/a2o/reports/`).

## 6. Checklist for a package author (the whole spec in twelve lines)

1. Classify each new record (A1). No route before the class.
2. Which existing context carries it? (A2). New context only for new rules or measured isolation.
3. Vocabulary → manifest. Behavior → coordinator. Facts about the world → not integrity (D4).
4. Bundle = content-addressed head; witness binding = reference; never ship a DNA (D5, A4).
5. Every projected row and sync doc carries DNA hash + cell id (D3).
6. Candidate `appliesTo` names the target's coordinators (A5).
7. DNA packing only from the canonical path, only as a fleet operation (A6).
8. Placement and authority declared separately from the bundle (A7).
9. Resolved composition recorded, recoverable, readable by Shefa (A8).
10. Heads per meaningful act, bulk content under composite roots (A9).
11. Cross-context evidence is carried with its origin, contexts are not merged (A10).
12. Name the budget row you move and the receipt that will show it (§5).

## 7. What this spec does not decide

Whether Holochain stays, the fork deepens, or witnessing is re-implemented is the §8 decision
experiment and needs the T12 budgets first. Group clone spaces remain an operator-selected
candidate behind D0, D3 and the budget (D6). The surface (buttons, chips, cards) and the hub-side
install experience are owned by the UX addendum written alongside this spec.
