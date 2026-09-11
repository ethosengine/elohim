---
id: "backlog-brit-act-level-attribution-identity-chain"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit: attribution below the commit — each act carries the identity chain and steward that produced it, not author-plus-trailer"
slug: "brit-act-level-attribution-identity-chain"
written: "2026-09-10"
author: "operator (Matthew Dowell), captured by orchestrator during the memory-kit replacement"
status: "backlog"
priority: "medium"
jobs: [elohim]
cites:
- "agent-provenance-collective-affiliations | Agent provenance and collective affiliations | sha256:a4dec7f2a88ad0ce | path: genesis/docs/superpowers/plans/2026-09-10-agent-provenance-and-collective-affiliations.md"
- "private-thought-governed-fruit | private-thought-governed-fruit | sha256:80a19011beed18a8 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
---

## The gap

Git can attribute a change to one author and any number of co-author trailers. Commit
`975a6faab` (the private-thought canon) shows the ceiling: the operator authored the
ruling and edited the text, the orchestrating agent drafted the prose, and git can only say
"author: Matthew, co-author: Claude". Which words were whose, under whose standing, is
unrecoverable from the object.

## What brit should carry

brit is the covenantal VCS and the eprfs seam where an act becomes a governed object. An
act (a hunk, a document revision, a merge) should carry, as content-addressed pins on the
object rather than as trailer text:

- the **identity head** of the acting participant (a person, or an agent's chain minus its
  ephemeral layer) and, for an agent, the **session head** as provenance;
- the **steward** the act was performed under (the person or collective whose standing it
  drew on), which is where value and accountability route;
- the **signature** of the acting persona over the act's canonical bytes (the provenance
  plan's station one gives local personas a key and a `did:key`);
- for a revision that composes several participants' words, one attribution record per
  contributing act, so "writer's credit" is a fold over acts, never a single field.

This stays on the fruit side of the privacy line: it records who placed which words into
shared space, never the reasoning behind them.

## Dependencies and order

Blocked on the provenance plan (signed persona claims, identity chain, affiliations) and on
brit being buildable in the devspace (its `elohim-epr` and `rakia-brit` deps resolve only
from the auth-gated Nexus, 2026-09-10). Design first as an eprfs object shape reusable by
`epr flow` (the flows sidecar already records provider, steward and co-author slots on
produce events), then as a brit object extension, so the same attribution reads identically
from a repository sidecar and from a brit history.
