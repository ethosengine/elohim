---
name: project_epr_atom_home_slice1_landed
title: EPR atom home — slice 1 landed
description: EPR atom home Slice 1 landed 2026-09-02 — shell-owned /epr/{id} (EprHomeComponent), focal extracted count-neutral, legs, gate, lens; commons layer is the next plan; start:alpha env artifacts; habit red until the fleet renders it.
metadata:
  type: project
---

# EPR atom home — Slice 1 landed (2026-09-02)

`/epr/:resourceId` is now served by the shell's `EprHomeComponent`
(`app/elohim-app/src/app/elohim/components/epr-home/`), not lamad's ContentViewer. Spec:
`genesis/docs/superpowers/specs/2026-09-02-epr-atom-home-shell-component-design.md`; plan (Slices 0+1):
`genesis/docs/superpowers/plans/2026-09-02-epr-atom-home-frame-plan.md`; story
`genesis/a2o/features/content/epr-atom-home.feature` (`@concern:epr-atom-home`, 7 frame scenarios pass locally,
3 commons scenarios `@wip`); habit `app/elohim-app/.epr-meta/epr-atom-home.habit.md` (RED until the fleet
renders it — the wave-2 app build #1682 died on an agent-pod flake). Design canvas:
https://claude.ai/code/artifact/50e6f942-e332-43c4-b619-66f0a5d2ccb0 (v2 community layer).

**Why:** the operator chose a shell-owned atom home with a community (reddit-like, New_ Public-shaped) layer;
lamad is one lens away ("Open in Lamad" minted from generated all-bundle route claims, `bundle-lens.ts`).

**How to apply:**
- Next plan = spec §2.3 the commons (conversation over `queryDiscussions`, statements over `getStatements`,
  who's here, how we talk here, the tender/newcomer welcome) + §2.6 phone accordion + `elohim-qahal/register`.
  The a2o steps for those scenarios already exist with provisional test ids — reconcile them.
- Seams to respect: `EprFocalComponent` is the ONLY shell file importing lamad renderers (import ratchet);
  `StewardRow` keeps the lamad stewardship type out; services come from `@elohim/service/public-api`; generated
  views from `@app/generated/*`; `toSignal` needs exact `initialValue` (an explicit `toSignal<T>` collapses to
  `T | undefined` under strictTemplates).
- `pnpm start:alpha` artifacts: the content resolver and `StorageApiService` use environment.ts absolute
  localhost hosts (markdown body shows lamad's placeholder; steward row empty); alpha login cookies don't reach
  :4200 (no "Your mark"). Local a2o runs need `ELOHIM_CAP_OWNED_SUBSTRATE_STATUS=available` because `@act:i`
  gates on owned-substrate. Light theme = localStorage `elohim-theme=light` (the shell defaults dark).
- Filed, not built: containing-paths reverse projection; three holding instruments disagree on alpha
  (resilience 5 peers / household 1 of 3 / blob-distribution 0 of 7 — the page shows only feltStatus);
  governance state/mechanism/accumulation 404s; `relatedNodeIds` all unseeded on alpha.

**Fleet status at session end (2026-09-02 ~10:30Z):** the atom home is on origin/dev (wave 5, 7513654f6) and its
blobs are uploaded, but NOT visible on alpha: the app pipeline's canonical-head declares for the root bundle were shed
twice (#1684: alpha 503 catching-up during the storage roll; #1686: alpha 502 "conductor admission: shed … class=interactive,
capacity=5, in_flight=5", elohim.host persistent 503). alpha serves a stale index (main-R7523XGF.js → 404). The flip to
green = the first `pnpm look https://alpha.elohim.host/epr/evolution-of-trust` that carries `data-testid="epr-home"`;
until then the habit stays red and the fix lives in the dataplane (top red), not the bundle. Re-trigger with an empty
`[build:app]` commit once the doorway's `/db/content/elohim-host-landing/head-record` stops answering 503.

**Update ~10:40Z:** the head-declare sheds were a symptom — all seven alpha conductor pods have been crash-looping
since ~09:10Z (SQLite read-pool saturation → admin listener dies → restart storm; NotReady → no headless DNS record →
doorways see no bridge → storage reconcile circuit open → doorway write gate 503), and the wave-3/4 hApp's INTEGRITY DNA
hash moved for all five roles (hc-rna link-flag commits 03f331f21/d5fd9642b were not hash-neutral under holonix; my
content_store commits are integrity-byte-identical). Operator-owned recovery; escalation atom
`genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md`. Do not `[build:app]`
until conductors are Ready.
