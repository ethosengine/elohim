---
title: "Slug leftovers 2026-10-08 — a name's answer carries who elected it and when"
id: slug-leftovers-sprint-20261008
date: 2026-10-08
status: in-flight
author: "claude-fable-5-1 (orchestrator), plans by three claude-opus-5-5 planners, at the operator's direction"
habits: [dataplane-convergence, notary-authority, doorway-failover, reach-enforced-everywhere, served-under-standing, authority-in-integrity]
cites:
  - "elohim-protocol-specification | protocol-specification | sha256:d0975dc0f31fab70 | path: genesis/docs/content/elohim-protocol/protocol-specification.md"
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - genesis/data/timeline/backlog/epr-head-envelope-differs-per-peer.md
  - genesis/data/timeline/backlog/security-doorway-blob-pantry-ungated.md
  - genesis/data/timeline/backlog/doorway-coherence-servedbundle-naming-and-relay-window-followups.md
  - genesis/data/timeline/backlog/2026-09-04-doorway-projection-never-carries-app-row-heads.md
  - genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md
  - genesis/data/timeline/backlog/head-authority-carried-with-content-sync-unit.md
  - "gradient-sprint-20261008 | Gradient sprint 2026-10-08 | sha256:4fec108f82d70b3d | path: genesis/docs/superpowers/sprints/2026-10-08-gradient-sprint.md"
  - genesis/data/timeline/chronicle/2026-10-08-lamad-spa-two-cids-one-name.md
  - "name-binding-design | A name is answered by a binding: the NameBinding design gate | sha256:0182b4ae84f407b0 | path: genesis/docs/superpowers/specs/2026-10-08-name-binding-design.md"
---

# Slug leftovers sprint 2026-10-08

This is a plan. Results go to each habit's own atom as a DELTA line, never here.

## What the reading found

On 2026-10-08 elohim.host and doorway-alpha answered `lamad-spa` with two CIDs for 2.5 hours
(dataplane-convergence DELTAs 2026-10-08 through 2026-10-08d; chronicle entry cited above). Read as "a slug
is a value" that is divergence. Read as "a slug is a binding" it is two peers reporting two elections of one
elector, one stale, and the defect is that the answer carried no election clock and no elector, so a reader
could not tell stale from different. The operator ruled the same day (spec Open Issue "Names are bindings",
`0387fcabf`; authority backlog row 17): a slug is a projection of a content-addressed NameBinding — name ×
elector × scope-declaration CID × head × election clock × standing proof — and the label is never the
authority.

The code, read today:

- `/epr-head/{slug}` (`elohim/elohim-storage/src/epr_head.rs:77-169`, `http.rs:15576`) answers `author` =
  the row's creator, not the elector, and `updated` = `declared_head_at`. It carries none of
  `canonical_declared_at`, `canonical_earned`, `canonical_link_hash`, although the `content` row has all three.
  `ContentHeadView` (`crates/elohim-views/src/lamad.rs:94`) likewise.
- There is no elector column. The zome keeps the declarer only as `link.author` and drops it before
  `CanonicalElectionOutput` (`content_store/src/lib.rs:3132-3140`, `:6394-6425`). The canonical-head
  declaration has no link type of its own: it rides `IdToContent` off the `canonical_head` anchor, tagged
  `canonical-head:staging|earned`, clocked by the CreateLink timestamp.
- The release channel is a layer of the canonical-head mechanism, not a second one: the key
  `releaseChannel` in `metadata_json` (`release_channel.rs:26`), a channel row whose canonical head is the
  release manifest, and `AppBundleVehicle` as the only writer of a bound slug's pointer.
- `ProjectionInventoryEntry` (`crates/elohim-views/src/shared.rs:341`) carries no `canonical_*`;
  `classify_content_gap` compares the anchor only. Both are the other session's files (below).
- `PUT /epr-head/{id}` has no auth and no client; `/apps/{slug}` is served from `slug_index` without a reach
  verdict; the doorway coherence digest folds in each doorway's own commitment id, so two doorways never
  agree; the 2026-09-04 projection-drop row is already cured on dev (`f64d8c5bf`, `658a05218`, `32cd99ce4`).

Operator decisions: the NameBinding gate pass is IN as a design lane; this sprint runs IN PARALLEL with the
gradient sprint on DISJOINT FILES; the two security leftovers are IN as one lane.

**The other session's files.** Peer session `sprint-gradient-open-items` holds `.worktrees/wt-lane1`
(`feat/head-ordering-carried`), which edits `db/content_diesel.rs`, `sync/projector.rs`, `sync/mod.rs`,
`services/head_adoption*.rs`, `services/live_earned.rs`, `services/liveness_contract.rs`,
`services/reanchor_backfill.rs`, `p2p/projection_reconcile.rs`, `metrics.rs`, `signals.rs`,
`rea_projection.rs`, `seam-registry.yaml`, `features/dataplane/federation-version-convergence.feature` and
`steps/federation-deploy.steps.ts`, and has uncommitted main-tree edits to `habits.yaml`, the
dataplane-convergence and idle-is-free atoms, `head-authority-carried-with-content-sync-unit.md` (items 9,
10), `arch-dataplane-refactor-backlog.md` and `performance-concern-index.md`. This sprint edits none of them;
their functions are called, never edited. The projection check and the dataplane-convergence DELTA wait for
wt-lane1 to land.

## Lanes

### Lane A — `notary-authority` + `doorway-failover` (fleet read to `dataplane-convergence`): the answer carries the election

Worktree `.worktrees/wt-slug-a`, branch `feat/head-answer-carries-election`.

**Decision, closing `epr-head-envelope-differs-per-peer`:** election fields are envelope, not addressed.
`EprHead` and the DAG-CBOR body stay byte-identical across peers; `cid` stays a function of the declared head
alone; the election rides beside it as an unaddressed witness, omitted when `canonical_declared_at` is NULL.
A peer whose projection is behind now serves a visibly different witness, which is the legibility the
incident lacked.

- **A1.** `epr-head-view.schema.json` gains optional `election {canonicalDeclaredAt (RFC3339, µs), earned,
  linkHash}`; `content-head.schema.json` gains `canonicalDeclaredAt`, `canonicalLinkHash`.
  `views_convert/epr.rs` `EprHeadElectionView`; `epr_head.rs` `derive_epr_head_with_election` with
  `derive_epr_head` as the thin wrapper; `crates/elohim-views/src/lamad.rs` + `views_convert/lamad.rs` for the
  content head view. Tests in `tests/schema_contract.rs`, `tests/epr/epr_head_centralization.rs` (cid
  invariant to election columns; witness omitted when none; clock, tier, tiebreak rendered), one `tests/api/`
  read. `cargo test export_bindings` in `crates/elohim-views`; `pnpm run schema:codegen:ts`.
- **A2.** Elector on the wire, coordinator-only: `CanonicalCandidate.author`,
  `CanonicalElectionOutput.winner_author` (`serde(default)`), `CanonicalElectionWire.winner_author`,
  `ContentHeadView.elector` + `electorSource: live|cached` set in `apply_live_earned` under the "says so" rule
  of `tests/api/content_head_live_election.rs:107`. No integrity crate touched; DNA hash unchanged. Never an
  `elector` field without a producer. If the hApp roll cannot be measured on the household this sprint, A1
  lands alone and A2 is the follow-on `elector-on-the-wire`.
- **A3.** `features/dataplane/notary-authority.feature` (`@concern:notary-authority`): "A head answer names
  the election that chose it, so a stale election reads differently from a different one"; elector assertion
  `@wip` until A2 is on a household binary. `features/delivery/content-addressing.feature`
  (`@concern:content-addressing`): "Attaching the election witness does not move the head envelope's
  address". Blind-reader loop to READY.

**Done-when.** `just gate elohim-storage` EXIT=0 · `just gate doorway` EXIT=0 · `just gate schema-codegen`
EXIT=0 · `BERTH_CLASS=verify BERTH_TTL=1800 just test mesh features/dataplane/notary-authority.feature`
twice, the new scenario green · DELTA on `notary-authority` (stays GREEN, check widened) and on
`doorway-failover` (RED preserved, envelope decision) · after the operator's roll, both
`doorway-alpha.elohim.host/epr-head/lamad-spa` and `elohim.host/epr-head/lamad-spa` return one `cid` and
one `election`; that reading is the `dataplane-convergence` DELTA, written after wt-lane1 lands.

**Deferred, named.** `elector-column` (persist `ContentHeadDeclared.author`; `rea_projection.rs` + a
migration, both constrained). `announce-carries-ordering` (`ProjectionInventoryEntry` + `classify_content_gap`;
waits for Lane 1A's `DocHeadOrdering` as the payload — fields with no producer and no consumer are a lie on
the wire).

### Lane B — `authority-in-integrity`: the NameBinding design gate, document only

Main tree. Deliverable `genesis/docs/superpowers/specs/2026-10-08-name-binding-design.md` with the specs
`.epr-meta` frontmatter and `serves: [notary-authority, dataplane-convergence, doorway-failover,
authority-in-integrity]`. Order inside: local-first reading, gradient reading per plane, then the
p2p-design-gate Output Format with one Entity each for the channel/council binding and the person's petname,
the scope declaration named as a referenced missing node (measure-family row 32), Design Constraints
Discovered, and the back-fill detector's three answers. Every count read from the repo, never from a prompt.
Guesses to test: channel binding = Linked (A2) with the CreateLink as the atom; petname = Attested-Private
(B2) in imagodei; address = content-derived dag-cbor over name, elector, scope CID, head, clock, standing
reference; bridge view neutral to the DNA hash, row 17's two integrity rules hash-moving and batched into the
authority cluster's crossing. Then `cites seal`, a `@wip` scenario in `notary-authority.feature` ("Two doorways
answering one name with two addresses name two electors or two clocks, never one binding"), a blind-reader
pass, and a DELTA on `authority-in-integrity` (stays RED; no new `checks:` line while the scenario is `@wip`).

**Done-when.** The spec seals; the blind reader returns READY or named residuals; row 17 and the spec Open
Issue point at the spec; the DELTA is on the atom. Nothing coded for the binding until the spec is reviewed.

### Lane C — `reach-enforced-everywhere` + `doorway-failover` + `served-under-standing`: the slug path's security and doorway leftovers

Worktree `.worktrees/wt-slug-c`, branch `fix/slug-path-reach-and-doorway-coherence`.

- **C0.** Closure only: the 2026-09-04 projection-drop row closes on `f64d8c5bf`, `658a05218`, `32cd99ce4`;
  the declared-head-blob row on `06e9587f8` (Lane D records both).
- **C1.** `PUT /epr-head/{id}`: storage reuses the this-machine predicate
  (`device_consent::remote_caller_refusal(caller_is_local(&req))`); the doorway narrows its arm to GET.
  `reach-enforced-http.feature`: "An anonymous PUT of a declared head is refused".
- **C2.** `/apps/{x}` is judged exactly as `/db/content/{x}` or `/blob/{x}` would be: the dispatch extracts the
  caller identity as the `/blob` arm does; `blob_reach_refusal` for the CID form, `content_reach_gate::
  reach_refusal` for the slug form, both before the extraction-cache fast path; `slug_index` widens to carry
  the content id; the capability probe hides a restricted blob hash. `reach-enforced-http.feature`: "A
  restricted app bundle is refused by slug and by content address alike" (`@wip` until a restricted
  html5-app fixture exists). Verify whether a cached doorway `/apps/{slug}` hit consults reach.
- **C3.** Coherence: `declaredHead` → `servedBundle` with a serde alias for the roll; the digest no longer folds
  in each doorway's own commitment id (consequence: a re-issue on an unchanged route no longer rings the
  doorbell); the relay window becomes a named Background wait in `name-routing.feature` bounded at 120 s
  (`BUNDLE_HEADS_TICK_SECS` + one discovery tick); the dev bundle resolves its storage URL through the serving
  origin. The missing station `bound-slug-authority-and-serving-provenance` is written `@wip` beside its
  sibling in `app-bundle-elected-delivery.feature`; its cure is constrained and deferred.

**Done-when.** `just gate elohim-storage` EXIT=0 · `just gate doorway` EXIT=0 · household runs of
`reach-enforced-http.feature` (C1 green) and `name-routing.feature` (the Background holds) · DELTAs on
`reach-enforced-everywhere` (C1, C2; RED preserved, a flip needs a fleet build), `doorway-failover` (C0,
3.1, 3.2, 3.4), `served-under-standing` (3.1, 3.3); `dataplane-convergence` (3.5 `@wip` only) after
wt-lane1 lands.

### Lane D — closure: the incident record, dispositions, the measure

Main tree, docs only. The chronicle entry `2026-10-08-lamad-spa-two-cids-one-name.md` is the one durable
record of the incident (the habit DELTA and row 17's parenthetical point at it); it also states why decision
R1 in `content-head-election-vs-reach-fork-arbitration` is not falsified (a declaration gap, not a
two-operator fork). Dispositions: row 17 folds into B; the plural-mishpat lens-binding spec edit closes its
follow-up; `epr-head-envelope-differs-per-peer` folds into A and closes on the fleet read; the two
`derive-epr-head` / `epr-head-cid-schemas` rows defer behind B; the projection-drop and declared-head-blob rows
close on their anchors; the coherence and blob-pantry rows mark lane C in flight; workspace row 41 records
A's residual. Out: psephos naming drift (a component rename), the household-id vocabulary column, workspace
row 26 (release plane), the three `task-release-*` entries (open with named residual stations; nothing closed
by `e553c7e12`), the two `/epr-head` 404 harness rows. One measure, registered after A1 fixes the field names:
`slug-answers-disagree-across-doorways@1` (for each slug in the doorway manifests' served set, read
`/epr-head/{slug}` on both doorways; count those whose `cid` or `election` differ) with lens
`slug-answers-disagree-ceiling@1`, hard 0 within 30 min of a roll, concern `dataplane-convergence`.

## Sequence

1. Now, main tree, docs only: this doc, the chronicle entry, the dispositions, the name-binding spec with its
   `@wip` scenario and DELTA. Commit by path; `git push origin HEAD:dev`.
2. Worktrees `wt-slug-a` (A1, A3, then A2) and `wt-slug-c` (C1, C2, C3) in parallel with each other and with
   the gradient sprint's three worktrees. Cargo gates serialized through `berth claim cargo`. A1 before A2; C1
   before C2.
3. The household is one berth: `berth status` first, `BERTH_CLASS=verify BERTH_TTL=1800`, A's and C's feature
   runs after the gradient session's Lane 1 runs, twice each.
4. Land each worktree onto dev with `git switch` and `git merge --ff-only` (merging dev in first when a
   feature file was appended by another lane); stage by path; `git push origin HEAD:dev`; `RUN_SWEETTEST=1`
   for A2 if the push does not already target dev.
5. After wt-lane1 lands: `python3 .claude/scripts/habits-project.py --check`; the dataplane-convergence
   DELTA (A's fleet read, C's 3.5); row 17's cross-cite to head-authority items 9–10; the measure registered;
   re-project; push.
6. The operator rolls; the fleet read is A's Done-when. Status flips only on fleet evidence. Everything else
   lands as a RED-preserved DELTA.
