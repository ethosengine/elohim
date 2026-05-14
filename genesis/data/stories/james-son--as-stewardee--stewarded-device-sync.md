---
# ContentNode identity (matches lamad ContentNode schema; seeded into DHT)
id: "experience-story-james-son--as-stewardee--stewarded-device-sync"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple — the canonical identity (three links when seeded)
subject: "human-james-son"                # → :hasSubject link
role: "role-as-stewardee"                 # → :inRole link  [DOES NOT YET EXIST — see report]
feature: "stewarded-device-sync"          # → :exercises link  [FEATURE FILE DOES NOT YET EXIST — see report]

# Human metadata
title: "James and the Spoke"
description: "Witnessed evidence of human:james-son exercising feature:stewarded-device-sync in role:as-stewardee."
slug: "james-and-the-spoke"               # short reference; INDEX uses this
version: 2                                # bumped from v1 (legacy frontmatter) → v2 (triple schema)
written: "2026-05-14"
author: "storyteller"
status: "canonical"

# Delivery axis — orthogonal to author-status. Read-only to storyteller; written by deliver-bridge auto-poller.
# See `.claude/scripts/memory-kit/LIFECYCLE.md` "The author/delivery axis split" section.
# Values: undelivered | envisioned | backlog | refined | wip | active.alpha | active.beta | active.latest-stable | stable | regression
# active.*/stable/regression are conferred ONLY by `/deliver`'s tier-3 verdicts.
# This story is `undelivered` because `genesis/a2o/features/.../stewarded-device-sync.feature` does not yet exist.
# Per Run #2 disposition matrix, the 2026-05-14 graduation is `graduated-narratively`; delivery-debt flag attached
# (cartographer backlog: "author stewarded-device-sync.feature + run through /deliver"). See [[feedback_story_delivery_status_axis]].
delivery_status: "undelivered"
delivery_status_updated: "2026-05-14"
delivery_status_source: "deliver-bridge-floor"

# EPR alias (derived from the triple; recorded for navigation)
epr_alias: "epr:experience-story/james-son/as-stewardee/stewarded-device-sync"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-jessica-spouse"
  - "human-matthew-manager"
  - "human-terrance-tutor"
devices:
  - "device-chromebook-edu"
  - "device-family-node-base"

# Adjacent features the narrative touches (not the canonical feature)
adjacent_features:
  - "lamad/learning-journey.feature"
  - "lamad/path-adaptation.feature"
  - "lamad/assessment-completion-feedback.feature"
  - "content/stewardship-allocation.feature"

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "social_medium/child/README.md"
  - "governance_layers/geographic_political/family/child/README.md"
  - "governance_layers/geographic_political/family/parent/README.md"
  - "governance_layers/functional/educational/student/.gitkeep"

# Memory graduation (storyteller curate authority — pending operator canonical-flip)
graduates_memory:
  - "project_household_fabric"
  - "project_multi_device_humans"
  - "project_ungrudging_service"
memorializes:
  - "project_stewarded_child_identity"          # STALE — names Terrance as the stewardee; data layer moved to James. Memorialize with note; see report.
  - "project_stewardship_philosophy"            # six-principle frame stays in deep archive; story carries the lived shape
  - "project_bootstrap_to_elohim_security_gradient"   # structural/social Stage-1 security pattern dramatized here

# ContentNode tags
tags:
  - "experience-story"
  - "@as-stewardee"
  - "stewardship"
  - "ceremonial-ux"
  - "graduated-authority"
  - "community-attestation"
  - "household-fabric"
  - "ungrudging-service"

# Sourcing — five-stream composition provenance (backfilled 2026-05-14 Run #3, Wave 4; C8 disposition)
# Story authored pre-schema; this block retrofits the storyteller's 5-stream discipline onto the canonical text.
sourced_from:
  epics:
    - "social_medium/child/README.md"
    - "governance_layers/geographic_political/family/child/README.md"
    - "governance_layers/geographic_political/family/parent/README.md"
    # NOTE: anchors_epics also lists "governance_layers/functional/educational/student/.gitkeep" but that path
    # is empty (no README, no .gitkeep) as of 2026-05-14. Carried as a coverage-gap flag for cartographer;
    # not cited here because nothing resolves to read.
  personas:
    - "human-james-son"           # subject
    - "human-jessica-spouse"      # the spoke-opener; carries the ceremony
    - "human-matthew-manager"     # builder of the family node; named the spoke
    - "human-terrance-tutor"      # community-attestation correspondent
    # No role-* record files exist in genesis/data/humans/ (roles encoded in tags + body); the @as-stewardee
    # role lives in the `role:` triple field + tags, not a separate persona record.
  scenarios:
    - "genesis/a2o/features/lamad/learning-journey.feature"
    - "genesis/a2o/features/lamad/path-adaptation.feature"
    - "genesis/a2o/features/lamad/assessment-completion-feedback.feature"
    - "genesis/a2o/features/content/stewardship-allocation.feature"
    # CANONICAL FEATURE GAP: the story's `feature:` triple-slot is `stewarded-device-sync` but
    # `genesis/a2o/features/**/stewarded-device-sync.feature` does not yet exist (delivery-debt; see
    # delivery_status: undelivered above). Cartographer backlog entry already tracks the authoring.
  devices:
    - "device-chromebook-edu"
    - "device-family-node-base"
  historian_precedents:
    - "archive:.claude/archive/2026-05-14/graduated/project_household_fabric.md"
    - "archive:.claude/archive/2026-05-14/graduated/project_multi_device_humans.md"
    - "archive:.claude/archive/2026-05-14/graduated/project_ungrudging_service.md"
    - "archive:.claude/archive/2026-05-14/memorialized/project_bootstrap_to_elohim_security_gradient.md"
    - "archive:.claude/archive/2026-05-14/memorialized/project_stewarded_child_identity.md"
    # Sixth memorialized precedent (project_stewardship_philosophy) also cited in `memorializes:` above;
    # capped this list at the spec's "up to 5" forensic-precedent guidance.
---

# James and the Spoke

The Chromebook lives on the bookshelf in the living room. Jessica put it
there on purpose. It is not Mom's laptop, and it is not the family node
humming quietly downstairs in the utility closet. It is James's, and
James is eleven.

When James wants to do his lessons, he asks his mom. He has asked his
mom every weekday morning for two years now, and she still expects to be
asked. This is not because the Chromebook is locked in any obvious way —
no password, no parental-control PIN, no "ask permission" screen with a
big friendly button. It is because the Chromebook, by itself, cannot
reach the rest of the family. The path between James's device and the
home that holds his lessons, his progress, his teacher's next assignment
— that path does not exist until Jessica opens it.

She opens it by sitting next to him for two minutes.

That is the ceremony. There is no other way to start.

---

James calls it the spoke. Matthew, his dad, started calling it the spoke
months ago — "let's bring up the spoke, buddy" — and it stuck. The
family node in the closet is the hub; James's Chromebook is one of its
spokes; the moment Jessica authorizes the connection, the spoke is in
the wheel and the wheel can turn.

It works like this. Jessica taps her phone against the Chromebook. Her
phone is paired to her — it knows her face, her PIN, the family-node's
key. The Chromebook wakes up and asks Jessica's phone, in the careful
language the protocol uses for things that matter: *for the next
ninety minutes, may this device act for James inside the family ring?*
Jessica reads what the spoke is being asked to do. She has read it
hundreds of times. She reads it again. She approves.

The Chromebook is now a spoke. James can see his lesson queue. He gets
to work.

---

This sounds like a lot, and that's the point.

The first weeks, Jessica wondered if it was too much. James wondered if
the family node was broken. Matthew, who built it, wondered if he had
made the wrong call. They had moved James off a stock school-issued
device three months earlier, after a long evening at the kitchen table
where Jessica had read out loud what their old district contract said
the school could collect, store, and sell about an eleven-year-old's
attention patterns. They had decided, that night, that James's learning
would live inside the household. The protocol made that possible. The
ceremony was the price.

It turned out the ceremony was the feature.

What James used to do — open a Chromebook, log in, drift — was no longer
possible. What he does now is: sits next to his mother for two minutes
while she authorizes a window of work, then works until the window
closes. He does not get to scroll past his lessons into anything else.
The spoke goes where Jessica said the spoke could go.

When the window closes, the Chromebook is not "off." It is just a
laptop again. A laptop that cannot reach the family ring without his
mom or his dad. James puts it back on the bookshelf and goes outside.

---

The other thing the ceremony does, James doesn't see.

Inside the family node, his elohim — the patient little agent that's
been listening to his work since he was eight — reviews what the spoke
brought home. It looks at the math problem he got wrong twice, then
right on the third try. It notices he read for twenty-two of the
twenty-five minutes. It notices the pause in the middle, when his
attention left the page; it logs this honestly, without judgment, as a
fact about a Tuesday morning in May.

Then, because the spoke is up, the family node reaches out to the
homeschool community. Terrance is on the other end — the mentor who
meets James most Wednesdays. Terrance's node has been waiting all week
for this sync. James completed the fractions unit. The community
attests: yes, this is how he did, here is what we saw, here is what we
recommend next. The attestation is not a grade. It is a record that
several people who care about James reviewed his work and agree it
counts.

The next assignments — the ones Jessica picked out, the ones Terrance
suggested, the ones James himself flagged when he asked a curious
question last week — flow back to the Chromebook. Quietly. Without
ceremony, because the spoke is already up; the ceremony was for
opening it, not for what travels through it.

When the family node is done, it tells Jessica. Not with a
notification — Jessica isn't trying to be on her phone right now — but
with a small green dot on the spoke's status screen, the kind of light
you only notice when you're already looking. The cycle completed.

Jessica looks. Jessica notices. Jessica picks up her phone.

She closes the spoke.

---

The Chromebook does not protest. James does not protest. The protocol
does not buzz with reminders or beg her to keep things synced. The
network gave what was needed and is content to be set down. If James
opens the Chromebook in an hour and tries to work, he can — alone, on
the device, without the ring — and his work will be there for the next
ceremony. The household does not punish him for working between syncs.
It just waits.

This is the part Jessica didn't expect to love. The protocol does not
sulk when she disconnects. It does not retaliate against her schedule.
It serves her family on her family's terms and then steps back.

There are evenings she lets the spoke stay up after dinner because
James and Matthew are working on a project together, and the family
node is happy to keep the connection open. There are afternoons she
brings it down ten minutes into a session because the dog got out the
back gate and homework can wait. The protocol gets out of the way both
times. It is not in a hurry to be important.

---

James does not yet know that the spoke he calls the spoke is a
capability grant signed by his mother's key against his father's
family-node key, scoped to a ninety-minute window over a specific list
of his sources. He does not need to know. He knows that Mom opens the
spoke, the work happens, Mom closes the spoke, and he goes back to the
living room.

In a few years, his mom will start asking *him* if he's ready to keep
the spoke open by himself. The protocol will give him a key. The
ceremony will become his own — different, lighter, but still a
ceremony, because the household will still be a household, and the
work will still matter. The shape of his authority will graduate. The
shape of the ritual will not disappear.

For now, on a Tuesday morning in May, James puts the Chromebook back
on the bookshelf, picks up his shoes, and lets the screen door slam
behind him. The spoke is down. The family is at rest. The protocol
keeps watch on the things that matter — quietly, ungrudgingly, without
needing to be thanked.
