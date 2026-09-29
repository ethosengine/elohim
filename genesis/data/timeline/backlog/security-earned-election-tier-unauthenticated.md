---
id: "backlog-security-earned-election-tier-unauthenticated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "An EARNED canonical-head election is judged by link-tag prefix alone — any peer can forge one (carried, gossiped, or via the delegated head-declare shortcut); it blocks earned-adoption reach widening (F19)"
slug: "security-earned-election-tier-unauthenticated"
written: "2026-09-28"
author: "claude-opus-5-5 (red-team review of held commit 1869b8fab, FCT v2 shift)"
status: "open"
priority: "high"
area: "lamad-dna/content_store + storage/head-adoption"
domain: "protocol"
jobs: [elohim-holochain, elohim-edge]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "habit:dataplane-convergence"
cites:
  - genesis/data/timeline/backlog/security-declare-carries-record-carried-evidence-bounds.md
  - genesis/data/timeline/backlog/security-storage-direct-caller-unauthenticated.md
  - genesis/data/timeline/backlog/lamad-teacher-authoring-backlog.md
tags: [security, election, earned-head, reach, coordinator, integrity]
---

# The earned tier is not authenticated

## What is true (red-team, 2026-09-28)

`earned` is computed in wasm but is NOT validated DHT data: the tier is a link-tag prefix
(`canonical-head:earned`, `content_store/src/lib.rs` ~3056, ~3320) and nothing checks the
link's author against the root author / verified delegate / progenitor.

1. **Carried election (critical once reach travels).** `try_carried_election_supply`
   (`elohim-storage/src/services/head_adoption.rs` ~2461-2485; `ELOHIM_OBEY_CARRIED_ELECTION`
   ON in hc-mesh.sh, adam-firstman.yaml, `_edgenode-consolidated.template.yaml`) →
   `verify_carried_election` (`lib.rs` ~6629-6645) → `prove_carried_declaration` checks
   self-hash, signature-under-its-own-key, anchor and tier only. `holds_authoring_standing`
   is applied on the courier path only. Any peer can self-sign an earned-tagged link plus a
   Content with the target id and win as newest.
2. **Gossiped link.** The integrity zome accepts any CreateLink
   (`content_store_integrity/src/lib.rs` ~4260); `genesis_self_check` admits anyone; a peer
   can `create_link` an earned-tagged IdToContent link directly.
3. **Delegated head-declare shortcut.** `http.rs` ~9290-9330 accepts `X-Agent-Cid: K` with a
   junk delegation; the zome's `authorize_author_or_delegate` returns `Ok(None)` whenever
   `me == root_author` (`lib.rs` ~4246) without inspecting the delegation — on the author's or
   steward's own peer an unauthenticated caller mints an earned election for any Content with
   that id.

Today (reach insert-only) these are head hijacks. With earned-adoption reach widening (F19,
held at branch `f19-earned-widen-hold`, commit 1869b8fab, reverted on dev as b31100358) they
become fleet-wide disclosure: a forged empty-body commons head widens a private row while
`body_from_verified_entry` (`content_diesel.rs` ~1047) keeps the private body.

## Also found in the F19 commit (fix before it returns)

- An older earned election re-widens a row its author narrowed on purpose (Declare clears
  the stored ordering; "election beats none", `content_diesel.rs` ~2128).
- Widening re-stamps every authored edge of the source, including edges the new head did not
  restate; an edge to an id with no local row then exposes the target id.
- Refuse to widen when the adopted entry's body/blob is kept from the previous version.

## Direction

Coordinator (hot-swap, no DNA hash move): re-check link author (+ delegation) in
`gather_election_candidates` and apply `holds_authoring_standing` in
`verify_carried_election`; refuse a supplied delegation when `me == root_author`.
Integrity (moves DNA hash): validate canonical-head link tags in `IdToContent`.
Then re-land F19 with the three fixes above.

## Current decision

The coordinator-only authentication mitigation and the storage-side F19 widening safeguards are
implemented and locally tested. Ordinary, carried-election, and carried-head-evidence paths
re-authenticate every EARNED link author as the id's immutable first root author, a currently valid
in-scope delegate, or the progenitor. Root authority is derived from all signed id-link creation
facts, including later-deleted links, so an attacker cannot delete the live index and substitute
its own root. Supplied delegation proofs are never ignored, self-delegation is refused consistently,
and storage refuses to exercise the delegated route through a root-author/progenitor conductor.
F19 now widens only when the adopted verified version is complete and carries its own body/blob,
never while retaining an older blob pointer; it replaces title/description with the adopted
version, holds a narrower or unknown-grade version rather than serving it under an older wider
grade, preserves an authenticated election-ordering floor across both unordered author-declaration
write paths, and widens only edges the adopted head restates. Those additional guards close four
disclosure paths found by the independent local adversarial review. The integrity-layer
canonical-head tag gate remains later work (it moves the DNA hash); this re-land changes storage
only. Household delivery was checked on 2026-09-29: the course serves the same earned head and body
anonymously on all three rebuilt peers, and the active HTTP reach checks pass 3/3. This was already
true before the rebuild: public and commons are equivalent here, as the operator confirmed.
The run therefore does not prove a restricted-to-commons transition; F19 stays partly verified.
Intimate reads remain 403 everywhere; private fixtures remain 403 on Matthew/Jessica, while James
has no private fixture. Report: `genesis/docs/superpowers/sprints/2026-09-29-f19-household-proof-report.md`.
World/standing/Mishpat affirmation remains distinct from these HTTP observations; the report
records the a2o lane's unknown stage rather than claiming a sealed proving ground.
