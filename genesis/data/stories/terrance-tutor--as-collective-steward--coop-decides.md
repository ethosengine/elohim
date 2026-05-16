---
# ContentNode identity (matches lamad ContentNode schema; seeded into DHT)
id: "experience-story-terrance-tutor--as-collective-steward--collective-governance"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple — the canonical identity (three links when seeded)
subject: "human-terrance-tutor"              # → :hasSubject link
role: "role-as-collective-steward"           # → :inRole link [DOES NOT YET EXIST — see acknowledged-gap]
feature: "collective-governance"             # → :exercises link

# Human metadata
title: "The Coop Decides"
description: "Witnessed evidence of human:terrance-tutor exercising feature:collective-governance in role:as-collective-steward — a homeschool coop ranks history curricula by ranked-choice and the elohim's justification."
slug: "the-coop-decides"
version: 1
written: "2026-05-14"
author: "storyteller"
status: "canonical"                          # ceremony Run #5 Wave 6 author; operator flipped to canonical at Run #6 Wave 4 (2026-05-15) — no body changes per Wave 2 storyteller read

# Delivery axis — orthogonal to author-status. Read-only to storyteller.
# `collective-governance.feature` EXISTS at genesis/a2o/features/qahal/collective-governance.feature
# but no `/deliver` verdict has been minted against it yet (no tier-3 stewardship pass on the
# ranked-choice curriculum scenario this story dramatizes). Therefore the substrate-axis sits
# at `undelivered` floor — a feature on disk is necessary but not sufficient; only `/deliver`
# can confer `active.*`/`stable`/`regression`. The story's `delivery_status_source` records
# this as a ceremony-direct-author entry, not a deliver-bridge poll.
delivery_status: "undelivered"
delivery_status_authored_by: "storyteller"
delivery_status_last_polled: "2026-05-14"
delivery_status_source: "ceremony-direct-author"   # Wave 6 author; deliver-bridge will overwrite when it next polls

# EPR alias (derived from the triple)
epr_alias: "epr:experience-story/terrance-tutor/as-collective-steward/collective-governance"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-jessica-spouse"          # the proposer; brings the curriculum question to the coop
  - "collective-homeschool-coop"    # community-homeschool-coop as attestable subject (collective-as-actor)
  # James, Sarah are referenced in the story body but are scenario-rooted (sarah/james appear in
  # the canonical feature's ranked-choice example, lines 47-56); not foregrounded as characters.
devices:
  - "device-family-node-base"
  - "device-chromebook-edu"

# Adjacent features the narrative touches
adjacent_features:
  - "qahal/collective-governance.feature"        # canonical (also feature: triple-slot)
  - "qahal/feedback-dialogue-panel.feature"      # exists; the dialogue panel where dot-voting and reasoning surface

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "social_medium/epic.md"                                          # earned reach, attention-as-sacred — the philosophical floor
  - "governance_layers/functional/educational/epic.md"               # functional governance layer for education
  # acknowledged-gap: governance_layers/functional/qahal/README.md does NOT exist on disk —
  # the `qahal` functional layer is implied by the `qahal/` feature directory but has no epic
  # body. Closest available epic is the social_medium epic (which carries the earned-reach
  # philosophy the coop instantiates) and the educational functional layer (which has the
  # student/parent/educator records). Surfaced as a cartographer coverage-gap below.
  # acknowledged-gap: governance_layers/functional/educational/epic.md may not exist as a flat
  # file; the functional/educational/ directory contains sub-epics but no top-level README.
  # If the validator flags this, fall back to "governance_layers/functional/educational/student/.gitkeep"
  # (precedent from james-and-the-spoke), which marks the educational governance layer as a
  # first-class anchor even when its body is unwritten.

# Memory graduation — storyteller curate authority (pending operator canonical-flip)
graduates_memory: []                  # no entries graduated yet; this is a `draft` author state
memorializes: []                      # no entries memorialized yet

# ContentNode tags
tags:
  - "experience-story"
  - "@as-collective-steward"
  - "governance"
  - "ranked-choice"
  - "collective-decision"
  - "community-attestation"
  - "ceremonial-ux"
  - "elohim-justification"

# Sourcing — five-stream composition provenance (ceremony Run #5 Wave 6, AUTHOR-CANONICAL-STORY)
sourced_from:
  epics:
    - "social_medium/epic.md"
    # acknowledged-gap: no governance_layers/functional/qahal/README.md or governance/qahal/
    # directory exists on disk; the `qahal` (council/coop) pillar is currently substrate-only
    # (a feature directory at a2o/features/qahal/ and a pillar in elohim-app, but no epic body
    # in docs/content/elohim-protocol/). Story body anchors to the social_medium epic's
    # "earned reach" and "attention as sacred" principles, which are the load-bearing
    # philosophical floor for collective-governance regardless. Cartographer should rank
    # authoring `governance_layers/functional/qahal/README.md` as a candidate Objective.
  personas:
    - "human-terrance-tutor"          # subject; facilitator at Valley Learning Collective + homeschool coop member (per record line 10-11)
    - "human-jessica-spouse"          # proposer; coop member (per record line 11)
    - "collective-homeschool-coop"    # collective-as-subject per `community-homeschool-coop` in collectives.json
    # acknowledged-gap: no `role-as-collective-steward` record exists in
    # `genesis/data/lamad/content/`. Per the james-and-the-spoke precedent, role-* records
    # crosscut pillars and aren't always materialized; the story uses the @as-collective-steward
    # tag + body framing. Cartographer to rank "create role-as-collective-steward".
  scenarios:
    - "genesis/a2o/features/qahal/collective-governance.feature"
    # Specifically anchors to:
    #   - "Community uses ranked-choice to pick a curriculum path" (lines 40-58) — the moment dramatized
    #   - "Elohim builds governance disposition from voting history" (lines 240-247) — the disposition Terrance trusts
    #   - "Elohim votes as proxy when human hasn't engaged" (lines 249-256) — the proxy moment for the parent who missed it
    - "genesis/a2o/features/qahal/feedback-dialogue-panel.feature"
    # acknowledged-gap: no `community-attestation.feature` exists in genesis/a2o/features/ —
    # the thematic tag `community-attestation` on this story flags it as a coverage-gap for
    # cartographer. James-and-the-spoke also touches community-attestation thematically; the
    # canonical feature has been deferred twice now, which raises its leverage_score for the
    # next cycle's NEEDS-NEW-FEATURE bucket.
  devices:
    - "device-family-node-base"       # the coop's quorum runs across family nodes — each household carries its own
    - "device-chromebook-edu"         # Sarah's mother brings up Sarah's ballot on her child's school-issued device
  historian_precedents:
    - "archive:.claude/archive/2026-05-14/graduated/project_household_fabric.md"
    # The lesson the household fabric memory carried — that the family node is the trust
    # boundary, not the individual device — extends laterally here: the COOP's fabric is
    # composed of family-node fabrics. The coop never reaches a device directly; it reaches a
    # family node, which reaches a device when a steward opens a spoke. The same household
    # principle, one ring out.
    - "archive:.claude/archive/2026-05-14/memorialized/project_stewardship_philosophy.md"
    # The six-principle frame (graduated capability / accountable authority / visible shape /
    # ungrudging service / structural protection / cradle-to-grave lifecycle) — the story
    # honors visible shape (the tally is round-by-round, every member can see the elimination)
    # and accountable authority (every block carries a written reason, every proxy vote names
    # the elohim that cast it and the disposition it referenced).
    - "memory:feedback_no_hebrew_pillar_names_in_narrative"
    # The translation discipline — "qahal" stays in slugs/anchors/frontmatter; the body says
    # "the coop" or "the council." "Elohim" stays because it is the protocol's and the agent's
    # name. This story carries the discipline into a story where the temptation to say "qahal
    # decides" was real (the original ceremony brief used the word seven times); restraint
    # honors the parent-at-the-kitchen-table test reader.
---

# The Coop Decides

Jessica is the one who brought the question. Six families homeschool together — different
houses, different curricula, different convictions, one shared morning twice a week when
the kids work side-by-side at the church basement. They have been arguing politely about
history for nine months. Sarah's mom wants Story of the World; the new family wants History
Odyssey; Terrance, who facilitates the Wednesday session and knows what the eleven-year-olds
can actually carry, has been quietly recommending Classical Conversations for a year.

Nobody wants to vote. Voting feels like making someone lose.

So Jessica brings the question to the coop the way she has brought every other question to
the coop this year: she opens the panel on her phone, taps the council she sits inside,
and types one short proposal. *Which history curriculum?* Three options. Ranked-choice.
Open until Sunday at noon. Every family carries one vote.

---

The coop is not on a server. The coop is six family nodes humming in six utility closets,
gossiping to each other when they have something to say and otherwise leaving each other
alone. When Jessica's proposal lands, her family node tells the other five. Each one
quietly wakes the parent on duty — Sarah's mom on her chromebook, the new family on the
laptop in their kitchen, Terrance on the panel above the basement chalkboard he hasn't
finished wiping. *Jessica proposes a thing. You have until Sunday. Here is what she said.*

No notification noise. No badge count. A small light on a status screen, the kind you only
notice when you're already looking.

---

Terrance reads the proposal twice. He is the only one in the coop who runs a session every
week with all the kids in one room, and the only one whose ballot the elohim will weight by
that fact — the protocol knows what he attests to and how often, and it carries that into
the disposition it offers up alongside his vote. He doesn't see the disposition as a
number. He sees it as the elohim's careful sentence above the ballot: *Terrance has scored
curriculum revisions seven times in the last year and weighted readability over coverage
every time.* He nods. That's true. He votes Classical Conversations first, History Odyssey
second, Story of the World third, and adds two lines of reasoning into the field where the
panel asks him to.

Sarah's mom ranks the three. The new family ranks the three. Two more parents rank by
Saturday morning. One parent — the one with a newborn and a roof leak — has not opened the
coop panel in eleven days.

---

This is where, in the old web, things would have stalled.

In the coop, the elohim that knows that parent — the one she has been listening to for
three years, the one that has watched her vote on forty-one proposals and file two
challenges and consent to twenty-nine consent-rounds — composes a careful proxy. The
elohim doesn't guess. The elohim reads back the parent's own pattern: she has consistently
chosen the option Terrance recommends when she has not had time to read the proposal
herself, because she trusts Terrance. The elohim records this aloud in the ballot — *I am
voting as proxy for this parent; her pattern shows high trust in Terrance's curriculum
recommendations; her disposition's consensusPreference is 0.8; she has the right to
override my vote when she next opens the panel* — and ranks Classical Conversations first
on her behalf.

The parent will see this on Monday. She will either confirm it (the notification dismisses,
the proxy stands) or override it (her direct vote replaces the proxy, and the elohim
records the divergence into its model so it learns where this parent's trust runs out).
The protocol does not punish her for not voting. The protocol does not punish the elohim
for being wrong. The protocol records what happened, makes it visible, and lets the parent
correct the record at her leisure.

---

Sunday at noon, the tally runs.

The coop panel shows the round-by-round elimination: History Odyssey eliminated first,
its first-choice votes redistributed to second choices. Classical Conversations wins by a
margin nobody contests. Underneath the tally, the elohim's justification appears — not a
verdict, not an algorithm's explanation, but a paragraph in plain language that any parent
could have written: *Three families ranked Classical first; two families ranked it second;
the proxy vote for the parent who didn't open the panel was cast on the basis of her
consistent pattern of trusting Terrance's recommendations. The redistribution preserved
the consent of the families whose first choice was eliminated. Every block opportunity was
declined. The decision is held by the coop.*

The coop did not erase Sarah's mom's preference. The protocol logged her ranking, her
reasoning, and her family node carries the record forever. Next year, when the curriculum
question reopens, that reasoning will be visible to her future self and to the new
families who join. The decision is held lightly. It can be revisited. Nothing was settled
forever; one thing was decided for now.

---

Terrance closes the panel and goes back to wiping the chalkboard. The chromebook on the
basement table is still open to the morning's grammar exercise. He will not need to tell
the kids on Wednesday that a decision was made. They will notice when the new books
arrive. They will notice when their mothers bring different worksheets. The decision
travelled by its own weight, through the families it concerned, and stopped where it
was useful.

The coop didn't win. The families didn't lose. A question was opened, a question was held,
a question was answered with the care six households can muster between them. The protocol
made the holding possible — the elohim weighted what was already true about how these
families trust each other; the substrate carried the vote without any one family's node
becoming the place where the decision lived. No platform decided. No moderator decided.
Six homes decided, and the protocol remembered honestly.

Jessica puts her phone down and goes outside to find James.
