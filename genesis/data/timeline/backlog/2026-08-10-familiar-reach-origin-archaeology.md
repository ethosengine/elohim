---
id: "backlog-familiar-reach-origin-archaeology"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Where did reach=familiar come from on 1987 lamad rows (matthew), and what is the intended repair surface? Read-only archaeology"
slug: "familiar-reach-origin-archaeology"
written: "2026-08-10"
author: "batch-3 integration session (uncertainty-reduction dispatch)"
status: "backlog"
priority: "medium"
tags: [dataplane, reach, serving-damage, git-archaeology, read-only-probe, codex-claimable]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
---

# The familiar-reach serving damage — origin unknown, repair surface undecided

READ-ONLY probe. Live fact (2026-08-10): matthew holds 1987 lamad rows with
reach=`familiar` (+39 private, +2 intimate) that adam holds as `community`.
Consequences: excluded from the anchor-advertisement inventory
(DISTRIBUTION_SAFE_REACH) and hidden from unauthenticated HTTP reads ("Reach
denied"). Seeds declare no reach (null); storage default is `public`; no
seeder/import default of `familiar` was found in a first sweep. The RC-4
guard comment blames pre-guard conductor-answer stamping — but WHERE the
conductor answers got `familiar` is unestablished.

Questions (cite file:line / commit):
1. Which write path can put `familiar` on a lamad content row? Enumerate all
   writers of `content.reach` (signal projection arms, heal stamps, seeder,
   bulk import, witness re-author) and which of them could have carried
   `familiar` for the value-scanner corpus.
2. Git archaeology: when did these rows first read familiar? (Candidates: an
   import run, a heal wave, a zome default in a past coordinator.) Any commit
   or deploy window that correlates.
3. What is the honest repair surface — a projection-side re-widen against
   seed-declared reach (dev repair), a canonical re-declaration through
   governance (protocol repair), or a seeder re-run? Recommend, don't build.

No code changes; append findings under `## Findings`.

## Findings

### Verdict

The 1,987-row cardinality is an exact fingerprint of the Susan account
package, not of the content corpus or a conductor default:

```text
$ jq -r '.content | group_by(.reach)[] | "\(.[0].reach) \(length)"' \
    genesis/data/account-packages/susan-household.json
commons 62
community 1320
familiar 1987
```

`account-package.ts` classifies a human's affinity-matching content as
`familiar` (`genesis/seeder/src/account-package.ts:480-488`). `seed-accounts.ts`
then POSTs each whole human package to one hash-selected storage peer, with
fallback to sibling peers (`genesis/seeder/src/seed-accounts.ts:146-178`). The
receiver treats that viewer-relative assignment as a write to the one shared
Lamād projection: `/account/import` loops over the package and executes
`UPDATE content SET reach = assignment.reach` (`elohim/elohim-storage/src/http.rs:10146-10155,10199-10219`). There is no human/beneficiary key in that update.
Whichever human package last touches an ID on a peer therefore becomes that
peer's global serving reach for the ID. Different package routing/order is
enough to produce Matthew=`familiar` and Adam=`community` for identical rows.

Confidence is **high** that this is the source of the 1,987-row class: Susan's
package has carried exactly 1,987 affinity/`familiar` assignments since it was
introduced by `f4883ed1a` on 2026-04-11. The generator had already changed its
affinity label from legacy `neighborhood` to `familiar` in `77a84bb64` on
2026-03-26. Before `921e1694a` (2026-04-17), every account import went through
the single doorway; that commit introduced optional hash-mod peer splitting,
and `1828562c5` (2026-05-04) made active compute peers the normal targets.
Thus the earliest repository-supported window for the exact Susan fingerprint
on Matthew is the first successful seed after 2026-04-11; the repository has no
per-row reach audit log, so source archaeology cannot name the exact live build.

The July migration `d9fcd353c` is a real second writer: it maps every stored
`neighborhood` to `familiar`
(`elohim/elohim-storage/migrations/2026-07-23-140000_content_reach_canonicalize/up.sql:7`).
It may have converted other legacy rows, but it is not needed to explain this
class and does not explain the exact Susan-package cardinality nearly as well.

The adjacent `+39 private, +2 intimate` observation is consistent with the
same overwrite shape. Content seeding now defaults ungraded content to
`private` (`genesis/seeder/src/seed-sqlite.ts:517-521,575-605`), while the two
love-map path sources explicitly author `intimate`. IDs absent from Susan's
package retain those seed values instead of receiving her affinity assignment.

### Reach writers (production paths)

1. `create_content` and `bulk_create_content` insert the caller's reach
   (`content_diesel.rs:407-435,465-513`). Bulk seed is skip-on-exists, so it
   cannot repair an existing row (`content_diesel.rs:475-489`).
2. Ordinary content PATCH/update writes `input.reach`, preserving the old value
   only when the field is absent (`content_diesel.rs:547-575,619-638`). The
   reach-carrying HTTP service path re-notarizes through the conductor; the
   seeder's `stampProvenance` uses that route
   (`genesis/seeder/src/seed-sqlite.ts:983-1006,1018-1058`).
3. `/account/import` directly updates `content.reach` in SQLite, with no
   conductor, canonical-head, widening, or beneficiary-scope guard
   (`http.rs:10199-10219`). This is the writer that matches the live count.
4. DHT `ContentCommitted`, head-heal, and witness/re-author paths carry a
   `ContentProjectionPatch.reach`, but on an **existing row** the shared patch
   applier does not write reach at all (`content_diesel.rs:714-796,947`). The
   repository's own RC-4 test records this explicitly
   (`projection_reconcile.rs:6679-6691`). These paths can carry the conductor's
   reach only on the defensive **insert-missing-row** branch
   (`content_diesel.rs:948-982`). Therefore pre-guard conductor-answer stamping
   is not the origin of the existing 1,987 rows.
5. The CRDT reverse projector calls `update_content`, but constructs a patch
   containing only `blob_hash` plus the amber timestamp
   (`sync/projector.rs:483-506`); it does not write reach. Shard acquisition can
   insert a missing row with the received row's reach, but does not explain a
   mass in-place flip of pre-seeded rows.
6. The one-time 2026-07-23 SQL migration is the only other direct bulk rewrite
   found; it canonicalizes legacy vocabulary as described above.

### Repair recommendation

Do **not** use a plain seeder rerun. Bulk content seeding skips existing rows,
and the later account-import phase still repeats the last-writer bug. Nor is a
blind projection-side widen from seed data honest: most affected source rows
have no authored reach, and the current inverted-burden interpretation of that
absence is `private`, not `community`.

First stop `/account/import` from projecting viewer-relative affinity into the
global `content.reach` field. That assignment is agent-scoped access/context,
whereas `content.reach` drives cross-peer distribution and unauthenticated
serving (`DISTRIBUTION_SAFE_REACH` is only `community|public|commons` at
`content_diesel.rs:1671-1684`).

Then repair according to canonical evidence, per ID:

- If Matthew's own conductor resolves the canonical head at `community`, use a
  bounded, conductor-verified projection repair to re-project that reach. This
  is projection drift; it does not justify a new governance action.
- If the canonical DHT head itself says `familiar`, the bad projection has
  already been re-authored. Establish the intended authored reach, then move it
  through the normal governed/conductor update-and-declare channel. Do not
  silently widen a scoped canonical entry in SQLite.

So the honest immediate repair surface is **verified projection re-widen when
the canonical head proves `community`; canonical re-declaration only for IDs
whose canonical entry is also wrong**. A reseed becomes safe only after the
account-import writer is corrected and the source data explicitly carries the
intended global reach.
