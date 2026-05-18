---
# ContentNode identity (matches lamad ContentNode schema; seeded into DHT)
id: "experience-story-gertrude-grandma--as-recovery-counterparty--backup-stewardship-for-household-dowell"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple — the canonical identity (three links when seeded)
subject: "human-gertrude-grandma"                                    # → :hasSubject link
role: "role-as-recovery-counterparty"                                # → :inRole link [DOES NOT YET EXIST — see INDEX coverage gap]
feature: "backup-stewardship-for-household-dowell"                   # → :exercises link [FEATURE FILE DOES NOT YET EXIST — see INDEX coverage gap]

# Human metadata
title: "Gertrude Holds the Share"
description: "Witnessed evidence of human:gertrude-grandma exercising feature:backup-stewardship-for-household-dowell in role:as-recovery-counterparty — the reciprocal-backup counterparty for the Dowell household; what it feels like for her to keep the share."
slug: "gertrude-holds-the-share"
version: 1
written: "2026-05-18"
author: "storyteller"
status: "draft"                                # storyteller-drafted; operator confirms canonical

# Delivery axis — orthogonal to author-status. Read-only to storyteller.
# Floor value `undelivered`: the canonical feature `backup-stewardship-for-household-dowell.feature` does
# not exist on disk. The adjacent recovery-shamir-optional.feature does exist (Matthew-as-subject; Jessica/
# Adam/Abby as custodians) but the share-holder-as-subject framing this story dramatizes — Gertrude on the
# receiving end of the share-custody ceremony — does not yet have a feature file. Per Run #2 disposition
# taxonomy, this story would graduate as `graduated-narratively` until `/deliver` mints a verdict against
# a canonical share-holder feature. See [[feedback_story_delivery_status_axis]].
delivery_status: "undelivered"
delivery_status_updated: "2026-05-18"
delivery_status_source: "deliver-bridge-floor"

# EPR alias (derived from the triple)
epr_alias: "epr:experience-story/gertrude-grandma/as-recovery-counterparty/backup-stewardship-for-household-dowell"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-matthew-manager"              # the counterparty whose household holds the share's pair-bond
  - "human-jessica-spouse"               # co-steward of the Dowell household; co-signer on the ceremony
  - "human-james-son"                    # the "kids" Gertrude is keeping the share safe for
  - "collective-household-dowell"        # the household whose recovery this share serves
  - "collective-household-gertrude"      # Gertrude's own household; her hub serves the share
devices:
  - "device-home-nuc"                    # Gertrude's always-on hub on shem; the device that holds the share
  - "device-2019-android-phone"          # her phone, where the ceremony surfaces

# Adjacent features the narrative touches (not the canonical feature; for coverage discipline)
adjacent_features:
  - "auth/recovery/recovery-shamir-optional.feature"    # the Matthew-as-subject share-custody flow this story is the reciprocal of
  - "auth/recovery/recovery-m5-vote-as-emergency-contact.feature"   # the approval pattern Gertrude already lives inside

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "social_medium/elder/README.md"                                           # dignity-preserving protection; elder as wisdom-keeper, not burden
  - "governance_layers/geographic_political/family/elder/README.md"           # elder constitutional role within family layer
  - "governance_layers/geographic_political/family/parent/README.md"          # the Dowell-side parental authority that consented to ask
  # acknowledged-gap: no `resilience/` or `recovery/` top-level epic body exists on disk. The recovery
  # principle is carried in memory (project_recovery_grandma_standard, project_socially_derived_security,
  # project_graduated_recovery_authority) and in scattered feature files under auth/recovery/, but there is
  # no anchoring epic README. Surfaced as a cartographer coverage-gap below; the social_medium/elder and
  # family/elder epics carry the load-bearing philosophy in the meantime.

# Memory graduation — storyteller curate authority (pending operator canonical-flip)
graduates_memory: []                     # no entries graduated until operator flips to canonical
memorializes: []                         # no entries memorialized yet

# ContentNode tags
tags:
  - "experience-story"
  - "@as-recovery-counterparty"
  - "recovery"
  - "socially-derived-security"
  - "grandma-standard"
  - "ceremonial-ux"
  - "ambient-notifications"
  - "ungrudging-service"
  - "reciprocal-backup"

# Sourcing — five-stream composition provenance
sourced_from:
  epics:
    - "social_medium/elder/README.md"                                          # dignity-preserving protection; elder agency
    - "governance_layers/geographic_political/family/elder/README.md"          # elder family-layer constitutional role
    - "governance_layers/geographic_political/family/parent/README.md"         # the Dowells' parental side of the agreement
    # acknowledged-gap: no recovery/resilience epic body anchors this story; carried by social_medium/elder
    # and family/elder epics for now. Cartographer-rank: "author a recovery/ epic body that codifies the
    # grandma-standard, the graduated authority stack, and reciprocal-backup as a family-layer covenant."
  personas:
    - "human-gertrude-grandma"           # subject; large-text + simple-navigation accessibility needs (per record line 11)
    - "human-matthew-manager"            # counterparty; EthosEngine founder, family-systems affinity (per record lines 8-12)
    - "human-jessica-spouse"             # co-steward of the Dowell household
    - "collective-household-dowell"      # the household whose recovery the share serves (per collectives.json)
    - "collective-household-gertrude"    # Gertrude's own household; her hub serves the share
    # acknowledged-gap: no `role-as-recovery-counterparty` record exists in genesis/data/lamad/content/.
    # Closest extant records are role-social-medium-elder (covers Gertrude's archetype but not her role
    # in this specific exchange) and the @as-stewardee / @as-collective-steward role tags from prior
    # stories (different shape — counterparty is a peer-to-peer reciprocal role, not stewardship-of-a-ward).
    # Cartographer-rank: "create role-as-recovery-counterparty" — crosscuts imagodei (identity), shefa
    # (reciprocal-flow), qahal (graduated authority), and elder-as-wisdom-keeper.
  scenarios:
    - "genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature"          # the Matthew-as-subject reciprocal
    - "genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature"   # the approval-card pattern
    # CANONICAL FEATURE GAP: `backup-stewardship-for-household-dowell.feature` does not exist. The shape
    # this story dramatizes — share-holder-as-subject, accepting a custody ceremony for a specific
    # counterparty household — has no canonical Gherkin. The adjacent recovery-shamir-optional.feature
    # treats the share-recipient as substrate (Jessica/Adam/Abby are scenario fixtures, not subjects).
    # The new triple-feature would foreground the counterparty's vantage. Cartographer-rank: high
    # (delivery-debt carries forward from /deliver pass for any of the three reciprocal-backup stories).
  devices:
    - "device-home-nuc"                  # 16GB NUC; always-on; public NAT — the share's resting place
    - "device-2019-android-phone"        # large-text-capable phone; the ceremony's surface
  historian_precedents:
    - "memory:project_recovery_grandma_standard"
    # The bar this story is held against — "credible to a grandmother whose family photos are on the line."
    # The story instantiates it: Gertrude does not handle key material, does not read share bytes, does
    # not understand Shamir. She agrees to keep something safe for the kids. The technical primitive is
    # invisible; the relational primitive is foregrounded.
    - "memory:project_socially_derived_security"
    # The principle: identity recovery goes through peers' elohim-agents attesting + the humans themselves.
    # Doorway is blind. Shamir share lives with the peer. The story carries the receive-side of this
    # invariant.
    - "memory:project_graduated_recovery_authority"
    # The authority stack: intimate circle → community → governance → global witness → optional crypto
    # hardening. Gertrude is intimate circle (Dowell household-level relationship). The Shamir share is
    # the "optional crypto hardening" layer 5; her existence as a counterparty is layer 1.
    - "memory:feedback_less_pushy_notifications"
    # The UX discipline: ambient over interruptive. The story shows the ceremony arrive as a small light
    # on Gertrude's phone, not a modal; the elohim's explanation in plain language; no badge counts; no
    # daily nudges. One gentle ask, one consent, then silence.
    - "memory:project_no_sovereignty_stewardship_over_ownership"
    # The vocabulary: Gertrude is not "in custody of the Dowells' key" or "the owner of a recovery
    # secret." She is the steward of a share she keeps safe for the kids. The narrative refuses the
    # ownership frame at every available moment.
---

# Gertrude Holds the Share

It is a Tuesday in late spring, and Gertrude is in the garden. She has come inside for a
glass of water and to put the radio on. Her phone is on the kitchen counter where she
left it after church on Sunday. There is a small green light on the screen — the kind
you only notice when you are already looking — and a note in larger letters than her
last phone ever printed: *Matthew has asked you something. There is no hurry. It can
wait until after lunch.*

She wipes her hands on her apron and looks.

The screen says, in plain words, *Matthew and Jessica would like you to help keep their
family safe. If something ever happened to their main computer, or if Matthew got locked
out of his account, having a piece of his recovery here at your house would mean the
kids' photos and lessons could be brought back without him having to start over. We are
asking five people. You are one of them. You don't have to do anything except agree, and
then the computer in your kitchen drawer will quietly hold the piece for him.*

There is no jargon. There is no asking her to read a crypto disclosure. There is no
small print. Underneath, in the same large letters, are two buttons. *Yes, I'll keep
it safe for them.* *No, please ask someone else.*

Gertrude reads it twice. She thinks about James, who is eleven now, and the
fractions worksheets he has been bringing to her house on Saturdays. She thinks about
the box of photographs in her hall closet from when Matthew was eleven, and how the
basement flooded the year he was twelve and they almost lost them. She has known
Matthew since he was born. Of course she will keep it safe for them.

She taps the green button.

---

Nothing dramatic happens. The screen says *thank you* in the same large letters, and
then asks her one more thing: *Would you like Matthew to call you and explain, so you
understand what just happened? Or would you rather just trust it and not think about
it?*

This is the part of the design Gertrude does not know to be grateful for. The protocol
does not assume she is incompetent. It also does not assume she wants a lecture. It
asks. She thinks for a moment, and then taps *I'd like Matthew to explain, but on
Sunday after dinner is fine.*

The screen acknowledges. The light goes out. Gertrude goes back to the garden.

---

Underneath the kitchen counter, in a drawer that Matthew set up for her two Christmases
ago, a small computer the size of a hardcover book has been listening. It is what
Matthew called the "always-on machine" when he installed it — the thing that lets her
phone find her photographs from the garden, the thing that runs the video calls with
the grandkids without her son-in-law having to fiddle with it first. Gertrude has
never opened the drawer to look at it. She has no reason to.

The machine has just received the share. It does not announce this. It does not
celebrate. It writes the share into a small encrypted region of its disk, makes sure
it is backed up to a second region in case the first one ever fails, and goes back to
the quiet work it does all day — listening for her phone, answering when called,
otherwise resting. The share will sit there for years, undisturbed, possibly forever.
That is the point. A share you have to think about is a share that has failed.

---

On Sunday, after the roast and after Jessica has cleared the plates, Matthew sits next
to Gertrude on the porch with two cups of coffee. He doesn't have notes. He has been
thinking about how to explain this for a week.

"Mom," he says — she is not his mother, but she has been *Mom* to him since Jessica's
mother died, and the word is the truth they share — "what you said yes to on Tuesday
is the most useful thing you've ever done for us, and I want to make sure you know what
it actually is. Not because you have to do anything with it. Because if something ever
goes wrong, you should know what role you played."

He tells her, in the careful kitchen-table language they have always used with each
other, what a recovery share is. He doesn't say *Shamir secret-sharing.* He doesn't
say *cryptographic.* He says: *Imagine that to get back into the family's account, if
we ever lost everything, we'd need five trusted people to each unlock a piece of a
puzzle. You hold one of the pieces. You don't carry it; the machine in your kitchen
drawer carries it. You don't know what it says; nobody does, until five people agree
to unlock it together. And it's not even five out of five — it's three out of five.
If two of the people we asked are away, or sick, or sleeping, the other three can
still help. The point of asking five was so you would never be the one we have to
wake up at three in the morning.*

He waits. Gertrude takes a sip of her coffee.

"And what if I'm the one who's sick when the time comes?" she asks.

"Then we ask the other four, and three of them will be enough." Matthew is quiet a
moment. "And if all five of you are unreachable at once — which would be a very bad
day — there are other ways. Slower ways. The whole community would help. You being
willing is the fastest way, not the only way. We don't ever want you to feel like the
family depends on you not being on a plane that day."

Gertrude has been carrying things for her family for sixty years. She knows the
shape of being asked to be a load-bearing wall in someone else's life. This is the
first time, she realizes, that anyone has thought to tell her: *you can be one of the
walls, and we have also already poured a different foundation.*

She nods. "Okay. I just keep this safe for the kids."

"That's it," Matthew says. "That's it."

---

The machine in the kitchen drawer hums for the next three years without ever needing
Gertrude to think about it. It survives a power outage in October — battery backup
covers the gap, the share stays warm. It survives a software update in February — the
update arrives quietly, the machine confirms it has not lost anything, the share is
still safe. It survives Gertrude's hip surgery in July of the second year, when she
moves into her daughter's house for six weeks and the machine in the drawer keeps
humming alone in the empty kitchen, doing what it agreed to do, ungrudgingly. The
share does not need her presence. It needs her consent, which she gave on a Tuesday
in spring, and her presence in the world, which is its own kind of attestation.

The Dowells never call. Matthew never loses his key. The share, as far as anyone
knows, will sit in that drawer until the machine wears out, at which point another
machine will inherit it, quietly, the way Matthew's father's wedding ring inherited
the box of photographs when the basement flooded. The protocol arranges the
inheritance. Gertrude is not asked to think about it. The kids are her people, and
their pieces are safe at her house, and that is the felt sense she carries — the way
you carry knowing a friend has a spare key to your front door.

She does not think of herself as a counterparty. She thinks of herself as Grandma.
Both things are true; only the second one matters to her, and the protocol is
satisfied with that. The technical primitive is honored by being invisible. The
relational primitive — *I keep this safe for the kids* — is what carries the weight,
and what the protocol is here to amplify. Gertrude in her garden, half a continent
from the family-node in the Dowell utility closet, is part of how the Dowells stay
themselves.

She picks up her trowel and goes back to the tomatoes.
