---
id: "backlog-citation-apparatus-claim-grain-typed-edges-reach-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Citation as a book does it — claim-grain markers, typed cite edges, a derived reverse index, and support judged once at the reach gate"
slug: "citation-apparatus-claim-grain-typed-edges-reach-gate"
written: "2026-10-02"
author: "operator (Matthew Dowell), captured by Claude during the gradient-view session"
status: "envisioned"
priority: "medium"
jobs: [elohim]
relatedNodeIds:
  - "backlog-dependency-graph-cited-contributor-economy"
  - "backlog-brit-act-level-attribution-identity-chain"
  - "backlog-performance-concern-index"
tags: [citation, attribution, reach-gate, progressive-discovery, retraction, trustful-earned, friction-verify, plane-reference, plane-projection, fused-planes, unit-once, unit-history, design-input]
---

## The operator's thought

A book compresses the time a reader must spend on it through four separate parts: the marker in
the sentence, the footnote, the bibliography entry and the index. A reader can judge the shape of
a book's support before reading it; a bad advice book shows itself by having almost no citations.
The operator's direction (2026-10-02): EPRs and `.epr-meta` should be structured to give the same
linking and the same compression, upgraded with content addressing, machine-speed indexing,
peer-to-peer hosting, agentic review, and trust linked to the bibliographic claim through REA
flows. Agents that author content cite their sources, so attribution can flow back to the claims
they came from and trust can be minted at the reach gate.

The operator frames this as restoration and reconciliation: for authors, and for developers whose
work was taken into the commons without the link back being kept.

## What exists, read on 2026-10-02

Source: `.claude/skills/semantic-links/SKILL.md`.

| Book part | Here today |
|---|---|
| Marker in the sentence | Absent. A cite belongs to the whole document. |
| Footnote | Partly. Each cite carries a one-sentence `desc` written so a reader can decide without resolving the target. It is free text. |
| Bibliography | Present. Permanent id plus a content fingerprint that is a short form of the target's CIDv1; a changed target reads `STALE`. |
| Index | Present as tags, built by hand per topic. |

Timeline and backlog entries are outside the cite graph by design and use plain paths. So the
performance concern index and the shem handoff's inline figures carry no fingerprints.

Not verified: whether a "who cites this?" query exists. The tooling tracks inbound cites well
enough to flip their status when a document moves.

## Design observations (unverified; brainstorm input)

1. **Claim-grain cites.** When a source drifts, only the sentence that rests on it needs re-reading.
2. **Typed cite edges.** "Relies on", "disputes" and "see also" are different edges. The content
   graph already has typed couplings; the document cite graph does not. Retraction needs the type,
   or it floods everything that ever mentioned the source.
3. **Cites live inside the content's own bytes.** One content address then covers the claim and
   its citations; many cites add nothing to what the network must notarize.
4. **The envelope answers before the source is fetched.** Id, edge type, hint, fingerprint.
5. **Support is judged once, at the reach gate, and carried as a signed proof.** A content address
   makes "the source exists and is unchanged" free. "The source says this" costs judgement; that
   friction is owed, and is paid once. Private reach needs no cites; wider reach needs more.
6. **The reverse index is a projection each peer derives**, never a notarized record.
7. **Retraction is pulled, not pushed.** One record on the source; citers find it at their next
   verification. Eager propagation only for high-reach content.
8. **Two roles in the signature chain.** The gate that judged support answers for the judgement. A
   channel that relayed on a valid proof answers for relaying, unless it relayed after a
   retraction was visible. If every relay is equally answerable, every relay re-checks and the
   compression is lost. (Operator: a fake citation is a gated decision with real consequence, and
   feedback governs the channels that re-propagated as well as the source.)
9. **Positive feedback for repair.** Self-correction costs less than being caught; a correction is
   a citable act that supersedes the original; speed of repair is the measure. The consequence on
   a relay is restoration of the record.
10. **Source types that are not publications.** First-hand witness, and unsourced model knowledge,
    each graded on its own terms, so a person reporting what happened to them does not read as
    "uncited".

## Agents re-citing what training absorbed

A model cannot recall where it learned something; asked from memory it reconstructs a source and
can invent one. Re-citation therefore means: find the source, read it, check it says the thing,
then link. The edge type is "corroborated by, found after the fact", not "learned from". The
operator names this as the agent's own reconciliation work.

## The scale question the spec must answer first

The operator, 2026-10-02: part of the challenge is how we let ourselves cite sources at the scale
we need, and index them, without overwhelming ourselves or the architecture. No spec is agreed;
nothing is to be built until one is.

Observed in this session, as a small specimen of the cost. Three canon documents were edited (one
subsection in the manifesto, one paragraph in `succession.md`, one sentence in `values-forward.md`).
Ten inbound cites were healthy before and went stale: one on the manifesto, one on succession,
eight on values-forward. Clearing them meant re-reading each citing sentence against the change,
by hand, though every change was an addition that touched none of the cited passages. That is
document-grain staleness, paid per citer, on every edit, by an attentive reader.

The same pass showed what happens when it is not paid: fifteen of the sixteen cites on the
manifesto were already stale before the edit, and a dry run of the status pass reports 268 stale
edges across 170 of 1,024 documents. A verdict that is nearly always "stale" stops carrying
information.

The costs to size before designing, in the vocabulary of CONVENTIONS.md §Cost unit, phase and lane:

- **Authoring** — per claim, paid once. Who pays: the author, or the harness that witnessed the reads?
- **Support verification** — per claim, paid once at the reach gate and carried. What does a re-check cost when the source changes?
- **Staleness** — per citer, per edit of the source. Grows with history unless it is claim-grain and pulled.
- **The reverse index** — per peer, derived. How large is it at one year, and what rebuilds it?
- **Human attention** — the scarce one. Which of these verdicts ever reach a person, and how are they batched?

This entry is not tagged `performance`: nothing here is a measured cost yet. The `gradient-reading`
skill's design-time use is the method for the pass that sizes it.

## Where the vision is stated

`genesis/docs/content/elohim-protocol/manifesto.md`, Part IV-B, "A Covenant From Below: Citing What
Was Taken" (added 2026-10-02). It frames both this entry and the dependency entry as a covenant
taken up from below, in contrast with treaties made from above, and keeps the manifesto's rule
that the record states facts and defers valuation.

`succession.md` §9.4 and `values-forward.md` Stance I.3, where the debt of AI training is named,
each carry a short pointer to it (added the same day): re-citation as a partial, in-kind payment,
stated as a proposal that is not built.

## Open questions

- Do backlog and timeline entries join the cite graph? That reopens a recorded decision.
- What carries a claim-grain marker in markdown without harming readability?
- Which reach levels require which cite strength?

No design is chosen. This entry graduates through `/brainstorm` and the `p2p-design-gate`.
