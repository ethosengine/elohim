---
# ContentNode identity (matches the lamad ContentNode schema; seeded into DHT)
id: "experience-story-ethosengine--as-plural-author--agent-identity-claim-and-acceptance"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple (the canonical identity — three links when seeded)
subject: "org-ethosengine"                          # → :hasSubject link
role: "role-as-plural-author"                       # → :inRole link
feature: "feature-agent-identity-claim-and-acceptance"  # → :exercises link

# Human metadata
title: "Who Wrote This"
description: "Witnessed evidence of collective:org-ethosengine exercising feature:agent-identity-claim-and-acceptance in role:as-plural-author."
slug: "who-wrote-this"
version: 1
written: "2026-08-15"
author: "storyteller"
status: "draft"

# Delivery axis — auto-poller-maintained; NEVER operator/storyteller-authored.
delivery_status: "undelivered"
delivery_status_updated: "2026-08-15"
delivery_status_source: "deliver-bridge-floor"

# EPR alias (derived; recorded for navigation)
epr_alias: "epr:experience-story/ethosengine/as-plural-author/agent-identity-claim-and-acceptance"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-matthew-manager"
devices:
  - "device-gaming-desktop"

# Adjacent features the narrative touches (not the canonical feature; for coverage discipline)
adjacent_features:
  - "auth/contributor-presence-claim-ceremony.feature"
  - "devflow/developer-valueflow-projection.feature"
  - "qahal/collective-governance.feature"

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "imagodei.md"
  - "governance/epic.md"
  - "social_medium/epic.md"

# Sourcing (the five composition streams — storyteller discipline)
sourced_from:
  epics:
    - "imagodei.md"
    - "governance/epic.md"
    - "social_medium/epic.md"
  personas:
    - "human-matthew-manager"
    - "org-ethosengine"
  scenarios:
    - "genesis/a2o/features/auth/contributor-presence-claim-ceremony.feature"
    - "genesis/a2o/features/devflow/developer-valueflow-projection.feature"
    - "genesis/a2o/features/qahal/collective-governance.feature"
  devices:
    - "device-gaming-desktop"
  historian_precedents:
    - "git:e4c4accf3fb0feda203a17683f4bd3e5096f114f"
    - ".claude/memory/feedback_ratification_is_us_not_operator_solo.md"
    - "genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
    - "genesis/docs/content/elohim-protocol/history/2026-06-21-contributor-presence-whoswho-grounding.md"
    - "genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md"

# Content-addressed cites (sealed by cite-gen; never hand-written)
cites:
  - "actor-plane-implementation-plan | The sealed plan this story narrates the birth of — in-flight honor-system identity claims, acceptance at ratification, evidenced adverse attestation | sha256:3044daacc8d5b48f | path: genesis/docs/superpowers/plans/2026-08-15-actor-plane-implementation-plan.md"
  - "actor-plane-inflight-identity-claims-design | The design record of the plane this story's closing beat narrates as built — claim, attribution surfaces, acceptance at ratification; carries the evidence (commits, golden CIDs, first live claims) the narrative renders in human register | sha256:6a6dee8249ae76ef | path: genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md"

# Memory graduation (storyteller's curate authority)
graduates_memory: []   # HOLD — the contributor-presence and ratification lessons are load-bearing
                       # in their own right and the story is still draft; nothing graduates yet.
memorializes: []       # HOLD — no technical artifact is ready for the deep tier; the plan is live.

# ContentNode tags (used by seeding + discoverability)
tags:
  - "experience-story"
  - "@as-plural-author"
  - "plural-authorship"
  - "agent-identity"
  - "honor-system-then-evidence"
  - "earned-standing"
  - "capability-is-not-one-thing"
  - "dev-collective"

# relatedNodeIds — computed at seed time. Do not hand-maintain.
---

# Who Wrote This

Matthew noticed it the way you notice that a friend has started mumbling.

The document was correct. Every claim in it held. It was also *work* to read — sentences
carrying three ideas apiece, paragraphs you had to back up through twice. He had handed the
job to the newest and most capable model in the house, and it had understood the problem
better than anything before it and written it less well than the model it replaced.

He was not the only person saying so. Opus 5 reasoned further into complexity; Opus 4.6 was
kinder to a reader. That is uncomfortable, because it breaks a comfortable assumption: that
there is one axis called *capability* and you simply want more of it.
Capability turned out to be plural, the way it is plural in people: the neighbor who can fix
anything and cannot explain any of it; the teacher who cannot fix a thing and can make you
understand it.

## The first move was small, and only moved the judge

The repository keeps a cold reader — an agent that opens one document, knows nothing else
about the project, and reports honestly whether it lands. Matthew pinned that reader to the
older model. One line of configuration.

It was the right change and it fixed nothing. A better judge does not produce a better
document; he had improved the grading and left the writing where it was.

## The second move gave away the pen

So he built a scribe: the older model as the *primary writer*, not a reviewer, not a
polisher. The newer model — the one dispatching the work — would supply the technical
substance and review for exactly one thing: whether the prose was **true**, not whether it
was how the reviewer would have said it.

The division of labour was stated out loud, because unstated ones collapse: **the writer
owns how it reads; the dispatcher owns whether it's right.**

The first run took two rounds to reach approval — corrections back, the scribe rewrote,
corrections back again, and the reviewer was satisfied. Then the cold reader, which had seen
none of the argument, opened the whole document. Its verdict was *revise*, mostly for old
debt in sections nobody had touched; on the new material it found exactly one thing. The rows
never said why they had been placed where they were.

Neither the writer nor the reviewer could have caught that. The writer had been told where
the rows went; the reviewer already knew why. A third round answered it; a fourth added a
closing line.

The evidence that the arrangement worked was not that it went smoothly. It was that each
role failed only inside its own lane — the writer never invented a fact, the reviewer never
rewrote a sentence for taste — and the reader, who had no lane, caught the gap that belonged
to neither.

## The run produced its own design

What came out of that run was two rows in a backlog, about what had just happened.

*Which model for which job* is not an operations setting. It is a standing question — who
may fill a role, on what evidence, under whose policy. A hardcoded model name is the
degenerate form of an attestation: a preference with no witness and no lineage behind it.

And crucially: the evidence must come from **outside**. Capability scores are imported
measurements — externally witnessed results from communities that do that benchmarking work,
accepted and re-attested by a council that chooses which benchmarks it trusts. Never
self-produced. The protocol has a long refusal of the scoreboard: standing is not a number
anyone could game, it is a shape that lives in relationships. A benchmark result is a
measure; "preferred for this role" is a lens some community chose to read it through, and a
different community may choose differently without either being wrong.

Paired with it was a constraint that binds it at design time rather than following it: a
capability lens must never harden into a closed roster. If there is no route in — no way to
grow into a capability through learning, no way to earn standing through contributing — the
lens has become enclosure, whatever its numbers say. And authorship
must be plural: the human, the agents, and the collective the work flows to, with lineage
kept. A system that can only credit one name cannot describe how the work actually got made.

## The commit that practiced it

The commit that landed those two rows carried four names, in two different places. The author
line — the one bound to a signing key — carried Matthew, the human giving directive and
steering. The trailers underneath carried the other three: the newer model as technical owner
and editor, the older model as scribe and primary writer, and ethosengine, the collective the
attribution flows to — which is, today, one person.

That last name is the honest and slightly absurd part. A collective of one. But if the
pattern only works at scale it is not a pattern, it is a hope — so it was written down at
n=1.

## The gap the roster exposed

Matthew saw it that evening. One name was bound to a key. The other three had been typed by
the dispatcher, about somebody else, after the work was over. The scribe never got to say
*that was me.* Nothing in the harness let an agent, while it was working, register its own
claim.

The answer designed that night has three moves, and none of them is a lock on a door.

**A claim, on the honor system.** An agent registers who it is at the start of its work,
with the human steward attached. Nobody verifies it. We believe you. This is
the honest maximum available today — the self-reported signatory line the governance canon
already describes, finally carried into the harness that actually runs — and it is stated as
such rather than dressed up as enforcement.

**Acceptance, by the community, later.** The claim does not accept itself. Acceptance
happens when the work is accepted — when the branch is ratified into the trunk by the people
and agents who reviewed it. That placement comes from a correction the operator issued weeks
earlier: ratification belongs to *us*, the working community, not a solo stamp.[^ratify] An
agent that could confirm its own identity would be electing itself, and the governance wager
is agents powerful enough to help and structurally unable to rule.

**Adverse attestation, with evidence.** If work later looks counterfeit and causes trouble,
someone files a correction against the claimed name, with the evidence attached. Never a bare
accusation. The original record is never edited; the correction is appended beside it, the
way an appealed settlement leaves the settlement standing and adds the appeal. The teeth are
earned standing — whether the next claim travels — rather than a gate. Carrot before stick;
repair rather than exile.

## The substrate was already waiting

The strangest part: they went looking for where to build it and found it built.

The protocol already knows how to hold a name for someone who has not arrived yet: a
presence that sits *unclaimed*, tended by a steward, recognition accruing to it before anyone
holds a key, and a claim ceremony that convenes and gets witnessed rather than transferring
anything atomically. That was designed for a grandmother whose contributions were real long
before she had an account. It is, line for line, the situation of an agent mid-task. The
identity layer already listed *ai-agent* among the kinds of thing that can hold an identity;
the community layer already admitted AI members. The inclusion built for humans without keys
turned out to be the inclusion for agents without keys.[^open]

And the identity has to attach to the name rather than the machine. The desktop in the corner
can run inference all night — it exists to play games, and the protocol is a tenant there,
borrowing the GPU when nobody wants it. But the model that wrote those paragraphs ran on no
box the collective owns. Substrates get swapped; a composed agent tomorrow is not the artifact
it was today. A human identity has always worked this way — no weights to benchmark, just a
name accruing witnessed proofs over a life — and that is now one pattern for any actor.

Matthew had started the day noticing that his best model was a worse writer. He ended it
having designed a way for any worker, human or not, to say honestly what they did — and for
the rest of us to accept it, or to disagree with evidence.

## The roster's last commit

The four typed names turned out to be the last of their kind. Inside a day the harness had the
thing the roster had been standing in for: a worker registers its own name before it starts,
with the human answerable for it attached, on a line nobody afterward edits.

Superseding is how a handoff gets recorded. When the scribe took the pen, its name went on a
new line at the bottom of the log; when the dispatcher took the pen back, so did its own.
Nothing was struck out. Read top to bottom, the log is the afternoon itself — who wrote, who
read cold, who handed back, in order. Each line is stamped with the date of the work in hand
rather than the clock on whatever machine happened to be running, so that two people holding
the same history end up holding the same record, instead of two that differ only in when they
were typed.

The first decision the new stamp ever carried was a refusal. The gate that governs documents
turned down the very document describing the mechanism — required frontmatter missing — and
the refusal named the scribe, along with the exact version of the scribe it was. The
mechanism's first witnessed act was to tell its own author no, out loud, with a name on it.

Then the record of who made what was re-read end to end: four hundred twenty-one entries
already there, untouched, and one new one — the plan's own, carrying three names. It added. It
did not go back and revise anything.

This last section was written by a storyteller that registered its own name before the first
word of it, naming the human answerable for it and superseding the dispatcher that sent it,
whose claim is still on the line above. A day ago the story had to say that could not happen
yet. It can.

[^ratify]: Operator correction of 2026-07-23 — ratification is peer acceptance at the branch
rung by the deliberating community of operator and agents, completing at the merge.

[^open]: Picks up a question left open on 2026-05-23 in the multi-collective collaboration
design — when a collaborating community has agent members, whose agents are they?
