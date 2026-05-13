---
name: DNA upgrade path — lineage regression + rna module
description: HC 0.6 gates lineage behind unstable-migration; rna module is a backburnered upgrade-path exploration worth its own brainstorm
type: project
originSessionId: 79de6926-64bf-4e59-b5d7-f1a35d2d83c8
---
DNA manifest `lineage` field was the protocol's designed forward-compat mechanism
(each manifest carries the previous DNA hash so breaking-change chains are
reconstructable). As of Holochain 0.6.0 the field is gated behind the
`unstable-migration` cargo feature; the stable `hc` CLI in holonix 0.6 rejects
it as an unknown top-level key. All 5 alpha DNAs had `lineage: []` removed on
2026-04-24 (commit `9855133d`, after build elohim-holochain/dev #1141) to unblock
the pipeline.

`elohim/holochain/rna/` is an existing module positioned as the upgrade path
for the current Holochain codebase. Currently on the backburner per the user.
The lineage regression makes it more relevant — without lineage, `rna/` may
need to carry more weight as the migration-tracking layer.

**Why:** User explicitly flagged (2026-04-24) that upgrade/migration capability
is "actually an important note" and that "lineage and 'upgrade' or whatever is
worth its own brainstorming session." The fix on the pipeline retreated from
lineage rather than solving upgrade paths.

**How to apply:** When the user returns to this topic — or when Holochain
stabilizes `unstable-migration` — kick off a brainstorming session that
considers rna revival + lineage reintroduction together. Don't propose bolt-on
fixes; this is a protocol-layer design decision. In the meantime, upgrade
history lives in git + network_seed rollover (`_alpha` → `_alpha2`), documented
in `elohim/holochain/dna/NETWORK_UPGRADES.md`.

References:
- NETWORK_UPGRADES.md STATUS banner (2026-04-24)
- manifest-hygiene test deletion rationale comment at
  `elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs:157`
- rna module at `elohim/holochain/rna/` (backburner)
