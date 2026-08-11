---
id: "backlog-post-decay-adjudication-cascade-trace"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Source-trace the post-decay-author steady state: does the adjudication cascade actually land divergent_actionable <= 2, or stall at a phantom-candidate loop?"
slug: "post-decay-adjudication-cascade-trace"
written: "2026-08-10"
author: "batch-3 integration session (uncertainty-reduction dispatch)"
status: "backlog"
priority: "high"
tags: [dataplane, head-adoption, ghost-decay, contest, election, read-only-probe, codex-claimable]
cites:
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/services/content_service.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/db/content_diesel.rs
---

# Post-decay-author steady state — the last-mile question, answerable from source

READ-ONLY source trace (no file changes). The ghost-decay cure (46e853521)
authors fresh heads for phantom-declared rows via
`update_via_conductor(.., HeadElection::PreserveExistingDeclaration)`. The
open uncertainty is what the NEXT sweeps converge to. Answer each question
with file:line citations:

1. **Row state after a decay-author.** With `PreserveExistingDeclaration`,
   does the row's `declared_head_action_hash` remain the PHANTOM hash while
   `dht_anchor_hash` moves to the fresh authored action — or do both move?
   Trace the stale-anchor heal branch of `update_via_conductor` and the stamp
   it performs.
2. **What can contest nominate afterwards?** Once the conductor holds a chain
   (`note_local_chain_arrived` fires), the next contest attempt runs. Arm 1
   nominates the PEER's head (also phantom — retrievable?); self-candidacy
   nominates the row's own declared head (still the phantom —
   `not_retrievable`?). Is there ANY arm that nominates the FRESH authored
   head? If not, is that a stall loop (contest_failed{not_retrievable} or
   declare_error every sweep) — and does it re-enter backoff, bounding the
   cost?
3. **Does the gate metric care?** `divergent_actionable = divergent_anchor -
   divergent_refused`, and refused = rows with local declared heads
   (`declared_heads_for`). Post-author the rows REMAIN declared (per Q1?), so
   cross-pod anchor disagreement should classify as REFUSED (adjudicated),
   not actionable. Confirm from `discover_content`'s (4b) split that a
   declared row's divergence can never re-enter the actionable count, i.e.
   the quiesce gate settles even if full canonical adjudication is slow.
4. **Gap-class steady state.** After authoring, matthew's conductor resolves
   the id, so the heal leg's answer flips from Ok(None) to Some(head). For a
   reach-narrowed (familiar) row the id still classifies AnchorGap every
   sweep a peer advertises it (familiar is excluded from local_anchors).
   Confirm the heal then completes (stamp refused-or-noop counted how?) and
   that this contributes to neither `failed` nor `divergent_actionable` —
   i.e. bounded churn, not gate pressure.

Deliverable: a written verdict per question + an overall call — "cascade
lands actionable<=2: YES / NO / stalls at <named loop>" — appended to this
file under a `## Findings` heading. No code changes.

## Findings

Source-traced against `46e853521` / `dev` on 2026-08-10. The narrow unit
contracts named under **Verification** were also run against that tree.

### Overall call

**Cascade lands `divergent_actionable <= 2`: YES for the decay-author cohort;
canonical-head adjudication can still stall at a bounded phantom-candidate
loop.**

That is a cohort verdict, not a proof that the fleet-wide value is literally
`<= 2`: unrelated actionable rows can still keep the global value above two.
What source does prove is that every successfully decay-authored row which
retains its non-empty declaration contributes zero to `divergent_actionable`.
The decay cure therefore removes this cohort's gate pressure even though it
does not itself replace the phantom declaration with the fresh authored head.

### 1. Row state after a decay-author

**The declaration stays phantom; only the anchor moves to the fresh authored
action.**

- The ghost witness calls `update_via_conductor` with
  `HeadElection::PreserveExistingDeclaration`
  (`elohim/elohim-storage/src/p2p/projection_reconcile.rs:2176-2185`).
- On the stale-anchor branch, failed `update_content` falls back to
  `create_content` (`elohim/elohim-storage/src/services/content_service.rs:399-417`).
  The returned fresh action hash is passed to `upsert_with_anchor`
  (`content_service.rs:441-465`).
- For an existing row, `PreserveExistingDeclaration` names only
  `dht_anchor_hash` and `updated_at`; it does not touch any declaration or
  election column (`elohim/elohim-storage/src/db/content_diesel.rs:902-944`).
- The asynchronous `ContentCommitted` projection uses the same preserve mode,
  so it cannot undo the eager write by re-crowning the fresh root
  (`elohim/elohim-storage/src/rea_projection.rs:710-728`).

Steady row shape immediately after authoring is therefore:

`dht_anchor_hash = <fresh authored action>` while
`declared_head_action_hash = <old phantom action>` (and the old declaration /
election ordering columns are unchanged).

### 2. What contest can nominate afterwards

**No contest arm nominates the fresh authored action.** The fresh action is
visible as the own conductor's non-canonical fallback head: absent a resolvable
canonical election, `resolve_content_head_inner` selects the newest
root-author record (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:3539-3568`).
But the adoption decision reduces that answer to `conductor_canonical: bool`;
for a non-canonical answer plus two declarations and no election it selects
`ContestPeer` (`elohim/elohim-storage/src/services/head_adoption.rs:1085-1107,
1280-1286`). The contest call then receives only the peer hint and the SQL
row's preserved declaration (`head_adoption.rs:1334-1337`):

- Arm 1 declares the peer-advertised head (`head_adoption.rs:1968-1977`).
- Arm 2, reached after Arm 1's expected not-retrievable refusal, declares the
  SQL row's own `local_declared` head (`head_adoption.rs:2082-2121`).
- Neither arm receives or names the fresh fallback `ContentHeadWire` action.

The resulting steady state has three shapes:

1. If peer and local phantom declarations are equal, the first idempotence gate
   returns `Held` before either round trip (`head_adoption.rs:1922-1932`). This
   is a cheap per-sweep hold, not convergence.
2. If the phantom declarations differ, Arm 1 normally fails because the peer
   phantom has no carried/retrievable record; Arm 2 then tries the local
   phantom and fails for the same reason. A failed self-candidacy releases its
   de-dup claim and records `SelfCandidacyBackoff`
   (`head_adoption.rs:2133-2158`). The backoff is finite (default contest window
   one hour), and authoring proactively clears any earlier `no_local_chain`
   backoff (`projection_reconcile.rs:2188-2199`;
   `elohim/elohim-storage/src/services/contest_backoff.rs:42-62,195-214`).
   With no peer record, the failure counter is `fetch_none`, not
   `not_retrievable`, because that label split is based on whether bytes were
   carried (`head_adoption.rs:1935-1962`).
3. A non-`ERR_NOT_RETRIEVABLE` Arm-1 error is counted as `declare_error` and is
   retried next sweep without entering this backoff (`head_adoption.rs:2063-2077`).
   Fanout and the sweep budget still bound aggregate work, but there is no
   per-id contest-backoff write on that branch.

So the named residual is a **phantom-candidate loop**: bounded by idempotence or
contest backoff in its expected forms, but not a path to the fresh authored
head. A later real canonical election can still move the row; the decay author
alone does not supply that election.

### 3. Whether the gate metric cares

**No, under a healthy declaration-classification read.** Discovery increments
`divergent_anchor` only for `ContentGap::Divergent`
(`elohim/elohim-storage/src/p2p/projection_reconcile.rs:3125-3144`). It then
batch-reads declarations for exactly those divergent ids; every id with a
non-empty `declared_head_action_hash` enters `declared_ids`
(`projection_reconcile.rs:3147-3182`;
`elohim/elohim-storage/src/db/content_diesel.rs:1134-1154`). Finally,
`divergent_refused` includes the declared set (`projection_reconcile.rs:3184-3220`)
and publication computes:

`divergent_actionable = divergent_anchor.saturating_sub(divergent_refused)`

(`projection_reconcile.rs:1150-1172`). Because Q1 proves preserve mode retains
the phantom declaration, a decay-authored divergent row remains refused /
adjudicated and contributes zero actionable pressure even while its contest
loop is unresolved.

There is one deliberate honesty exception to the word "never": if the pool or
batch declaration query fails, discovery conservatively knows no declarations
and counts the divergence as unadjudicated for that sweep
(`projection_reconcile.rs:3157-3180`). A later channel that actually removes
the declaration would also change the classification. Neither exception is
caused by decay-authoring.

### 4. Gap-class steady state for `reach = familiar`

**Processed heal attempts complete as refused/no-op and add neither `failed`
nor `divergent_actionable`; the row remains a re-detected `AnchorGap`.**

- `familiar` is a valid core reach, so the ghost authoring guard permits it
  (`elohim/elohim-storage/src/generated_enums.rs:314-324`;
  `projection_reconcile.rs:2088-2100`).
- It is intentionally absent from `DISTRIBUTION_SAFE_REACH`, whose only values
  are `community`, `public`, and `commons`; consequently it cannot enter
  `list_content_anchor_inventory` even with a non-null fresh anchor
  (`elohim/elohim-storage/src/db/content_diesel.rs:1675-1699,1738-1757`).
- Presence is reach-agnostic, so `present = true` plus absence from
  `local_anchors` classifies as `AnchorGap`, not `Divergent`
  (`projection_reconcile.rs:2815-2837,3096-3144`). It therefore never increments
  `divergent_anchor` in the first place.
- On the post-author `Some(fresh fallback head)` answer, the pre-stamp self-
  election guard does not fire because the row is already declared; that guard
  is limited to `local_declared.is_none()` (`projection_reconcile.rs:870-890,
  3593-3641`). `heal_content_one` chooses `GapFill` for the non-canonical answer
  (`projection_reconcile.rs:4640-4655`), and `GapFill` returns
  `SkippedDeclared` when the fresh head differs from the preserved phantom
  (`content_diesel.rs:1423-1441`). The heal leg marks that outcome completed and
  increments `refused_declared`, not `failed` or `healed`
  (`projection_reconcile.rs:3687-3698,3769-3779`).

Thus this is bounded churn rather than divergence-gate pressure. Discovery can
re-admit the `AnchorGap` on later sweeps, and the cross-sweep `MissLedger`
eventually exhausts/defers it (`projection_reconcile.rs:3184-3219`). During a
budget-limited sweep an admitted item can still remain `pending`, so source does
not prove that scoped-reach churn is cost-free; it proves that once this item is
processed its result is completed/refused and never actionable divergence.

### Verification

Targeted library tests, all green (`2 + 6` tests, zero failures):

- `preserve_election_leaves_an_existing_declaration_untouched`
- `preserve_election_does_not_create_a_declaration`
- `declared_heads_for_batches_the_skipped_declared_predicate`
- `gapfill_stamp_never_resurrects_over_a_declared_head`
- `declared_divergence_admission_requires_all_four_conditions`
- `content_gap_classification_absent_null_divergent`
- `a_backoff_always_expires`
- `ghost_decay_requires_every_positive_observation`
