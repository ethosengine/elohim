---
id: chronicle-substrate-currency-hub-doorway-web-seam
kind: chronicle
contentType: chronicle-entry
contentFormat: markdown
title: "The hub and doorway documents were brought to one statement of the web seam"
slug: substrate-currency-hub-doorway-web-seam
written: '2026-10-06'
occurred_at: '2026-10-06'
author: "claude-fable-5-1 (memory ceremony, operator present)"
status: noted
significance: meaningful
ceremony: substrate-currency
tags:
- memory-ceremony
- coherence
- hub
- doorway
- web-seam
- forward-looking-docs
relatedNodeIds:
- habit:recall-reaches-authority
cites:
- elohim/elohim-hub/README.md
- doorway/doorway-service/EDGE-DESIGN.md
- genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
- genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
- genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md
surfaces_rewritten:
- elohim/elohim-hub/README.md
- doorway/doorway-service/EDGE-DESIGN.md
- doorway/CLAUDE.md, doorway/doorway-service/CLAUDE.md, elohim/elohim-storage/CLAUDE.md, steward/node/CLAUDE.md, steward/device/CLAUDE.md (seam pointers)
- genesis/docs/content/elohim-protocol/architecture/ (hub-boundaries design, seam map 3.9 to 3.12, MAP, INDEX, iroh/libp2p complementarity, doorway SSR runtime, doorway access-tier patterns; notes and pointers)
- genesis/docs/content/elohim-protocol/resilience/README.md (retired-spec citations)
- genesis/docs/superpowers/specs/ (two rendering specs; one placement note each)
- genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md (open question answered in place)
- doorway/doorway-service/src/.epr-meta and elohim/elohim-storage/src/.epr-meta (rule web-seam-placement)
- app/elohim-elements/elohim-core/src/elohim-seam-map.ts (two strings of the seam map's mirror)
- genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md (one amended note: the node is the envelope's child)
- genesis/docs/content/elohim-protocol/architecture/ and genesis/docs/architecture/ (claims that rested on retired memory anchors; INDEX note on reading one)
- fifteen citation descriptions across specs, plans, history and gospel (reworded with the cite tool)
diff_review_verdict: GREEN on the fourth pass (code-reviewer). Passes one to three were YELLOW and each was repaired inline; no dead path and no misquote survived. The node decision and the anchor repairs had two further passes, the first YELLOW (eight findings, all repaired) and the second GREEN.
coherence_verdict: YELLOW-resolved (three fresh implementation readers; the last routed five sample tasks correctly from the seam map alone and after following pointers, and its leftovers were repaired in the final batch)
next_topic_sampled: none in the documents; the next work is the first slice of the device footprint spec (holds and autoclean in elohim-storage)
agent_minutes: unknown
surfaces_read: unknown
tokens: unknown
recall_correctness: not sampled; neither session could retain a finding (see the habit delta)
review_rounds: blind-reader README, 8 rounds, every round a new reader. Rounds 1 and 2 REVISE (1 correctness, 7 interpretability each); rounds 3 to 8 READY with 0 correctness. Round 8 returned no finding above minor (5 interpretability, 2 preference). The loop was stopped there on diminishing returns with the README exactly as round 8 read it; those five minor findings were applied afterwards. Rounds 9 to 11, each a new reader, were READY with 0 correctness; the two round-10 findings above minor (no list of the crates the test routes between; the stop-here section too long) were repaired before round 11, and the README stands as round 11 read it, with that round's three interpretability and three preference findings unapplied. Round 12 read the README after the round-11 repairs (conductor and iroh glossed, the shell defined with the conventions, the decision heading led by the decision) and was READY with 0 correctness, 2 interpretability (no template for creating the crate; blade and fabric not defined inline) and 3 preference; the README stands as round 12 read it.
rework: nine repair batches, each validated across all files before any write
correction_cids_closed:
- bafyreiezw7lze73ie7skt7ebgfkmu2tfrkrk3edpsyh2gbkfebh5yomhoq
---

# The hub and doorway documents were brought to one statement of the web seam

The operator marked a seam on 2026-10-06: web-level work a peer needs when it is reached by key
belongs to the hub, home `elohim/elohim-hub/`, and the doorway keeps only what exists because the
other end is a browser. Writing that into the hub README exposed how far the seam's documents had
drifted. Eighteen of the README's nineteen `memory` anchors named entries drained into the
architecture documents on 2026-06-03. The spec it called canonical was retired the day before.
Neither file is a member of the cite graph, so none of it had ever counted as drift.

The operator then set the standard for the repair: these documents state where concerns belong, and
the code is expected to move toward them. So each rule is written as the target, and what the code
has not caught up with sits in one debt table in the hub README. Reviewers who did not write the
text checked four batches against the tree. The last cold reader started from the seam map and
placed five tasks correctly without following a pointer.

A census changed the scope. 171 tracked documents carry memory anchors and 127 distinct names no
longer resolve, but most sit in frontmatter provenance lists and dated plans, which the 2026-07-21
ceremony already ruled acceptable. Only claims that rested on an anchor nobody could open were
repaired.

One error of the ceremony's own, caught before closing. 62 citations of the edited documents were
re-blessed on the strength of reading each claim against the edit. A fingerprint check against the
last commit showed 47 of them were already stale before the session, so that reading was not
enough. An independent reader then checked each claim against the current target: 17 are
title-only, 26 hold, 5 name things the target does not contain. Those 5 were restored to their
committed stale state. The cite graph ends at 200 stale edges, from 228.

The five things left open at the second close were then taken up, at the operator's request.

**Which process links the hub on a personal device.** Decided, by the agent, and recorded as a
decision the operator can overturn: `elohim-node` is the node process on every peer-capable
device, one composition root linking storage, the hub and the agent crates. The compute-envelope
spec already calls its root envelope "the peer runtime", so the decision does not use that phrase:
`ark` stays the parent of a device's processes and the node is the child it supervises, in the
place the spec gave to storage. What the code lacks for this is in the README's debt table.

**The citations of documents that gained a placement note.** An independent reader found 24, not
29. Ten held, five were title-only, and nine named a section, a term or a placement the target no
longer has. Those nine were reworded with the cite tool. **The five unconfirmed citations** were
all legitimate cites with inaccurate descriptions; they were reworded the same way.

**Dead memory anchors in living documents.** All 94 distinct retired names are recoverable from
git. Of 244 occurrences, 198 are provenance and were left; the architecture INDEX now says that
`memory_anchors:` is provenance and how to read a retired entry. The claims that rested on one
(about 26, across a dozen documents) now name the living document that carries the principle, or state
it. Three had no living home and state it inline.

**The README's minor findings** were applied.

The last re-bless went edge by edge under a written rule: only citations whose claim a reader had
checked today, or that were healthy at the last commit, or that were written today. 111 edges on
86 documents. The cite graph ends at 182 stale edges, from 228.

After the commit of the seam documents (`5c4c4d0ac`) the two remaining document items were closed.
Two independent readers checked the 32 citations that had been left stale: 14 held, 6 were
title-only, 11 were partly wrong and 1 described a model the target does not contain. The 18 that
did not hold were reworded with the cite tool, one that held was refined to match today's seam
map, and all 32 were re-blessed. Every citation of a document this work edited is now at that
document's current fingerprint. One reader did not read the tiered-quilt design in full: about half
of it was searched for the claimed terms and not read line by line. `HouseholdHub` became
`DwellingHub` in the feature file, the two specs and the memory entry; generated a2o layering
reports and build output still carry it until they are next generated.

Left open, by name: the semantic fold, about 2,900 files behind, which does not rebuild in this
workspace; 168 stale edges graph-wide, none of them citing a document this work touched; and the
code the debt table names, none of which has moved. The design that moves it is
`genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md`,
written the same day; its first slice is holds and autoclean in `elohim-storage`.

## Horizon-scan reference

- **Latest scan**: `genesis/docs/analysis/horizon-scans/2026-09-06.md`
- **Next recommended scan**: 2026-12-05
- **Trigger**: if today is on or after that date and the latest scan is still this one, run `/mem-horizon-scan` before the next ceremony's first surface.
- **Summary**: the scan has no Summary section to quote; its two parts are "Horizon delta" and "Refinement plan: keep the memory HEAD fresh".
