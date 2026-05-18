---
# ContentNode identity (matches lamad ContentNode schema; seeded into DHT)
id: "experience-story-gertrude-grandma--as-account-claimant--social-recovery-with-help-from-family"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple — the canonical identity (three links when seeded)
subject: "human-gertrude-grandma"                                    # → :hasSubject link
role: "role-as-account-claimant"                                     # → :inRole link [DOES NOT YET EXIST — see INDEX coverage gap]
feature: "social-recovery-with-help-from-family"                     # → :exercises link [FEATURE FILE DOES NOT YET EXIST — see INDEX coverage gap]

# Human metadata
title: "Gertrude Logs In with Help from Her People"
description: "Witnessed evidence of human:gertrude-grandma exercising feature:social-recovery-with-help-from-family in role:as-account-claimant — locked out on a new device, recovered via her people; the grandma-standard met."
slug: "gertrude-logs-in-with-help-from-her-people"
version: 1
written: "2026-05-18"
author: "storyteller"
status: "draft"                                # storyteller-drafted; operator confirms canonical

# Delivery axis — orthogonal to author-status. Read-only to storyteller.
# Floor `undelivered`: the canonical feature does not yet exist. The adjacent feature suite under
# auth/recovery/ — particularly recovery-shamir-optional.feature and recovery-m5-vote-as-emergency-
# contact.feature — covers the substrate-side mechanics, but no feature foregrounds the claimant's
# experience from inside the grandma-standard. /deliver has not minted a verdict against any
# claimant-vantage feature. Per Run #2, this story would graduate as `graduated-narratively`.
delivery_status: "undelivered"
delivery_status_updated: "2026-05-18"
delivery_status_source: "deliver-bridge-floor"

# EPR alias (derived from the triple)
epr_alias: "epr:experience-story/gertrude-grandma/as-account-claimant/social-recovery-with-help-from-family"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-matthew-manager"              # Gertrude's first call; one of her five share-holders
  - "human-jessica-spouse"               # second of the five share-holders responding
  - "collective-household-gertrude"      # the household whose recovery is in motion
  - "collective-household-dowell"        # the household answering the call
devices:
  - "device-2019-android-phone"          # the new phone; the recovery's surface
  - "device-home-nuc"                    # Gertrude's hub at home; the resting state being restored

# Adjacent features the narrative touches (not the canonical feature; for coverage discipline)
adjacent_features:
  - "auth/recovery/recovery-m5-lost-key-entry.feature"          # the entry-point flow
  - "auth/recovery/recovery-m5-vote-as-emergency-contact.feature"  # Matthew's approval card
  - "auth/recovery/recovery-shamir-optional.feature"            # the substrate share-custody this story stresses
  - "auth/recovery/recovery-m5-portal-host-discovery.feature"   # the doorway-discovery the substrate handles

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "social_medium/elder/README.md"                                           # protection without infantilization; dignity-floor
  - "governance_layers/geographic_political/family/elder/README.md"           # elder family-layer constitutional role
  # acknowledged-gap: no recovery/ epic body; carried by social_medium/elder. Cartographer-rank
  # high — three stories in this batch flag the gap.

# Memory graduation — storyteller curate authority (pending operator canonical-flip)
graduates_memory: []                     # no entries graduated until operator flips to canonical
memorializes: []                         # no entries memorialized yet

# ContentNode tags
tags:
  - "experience-story"
  - "@as-account-claimant"
  - "recovery"
  - "grandma-standard"
  - "socially-derived-security"
  - "graduated-authority"
  - "ceremonial-ux"
  - "ambient-notifications"
  - "no-customer-support"

# Sourcing — five-stream composition provenance
sourced_from:
  epics:
    - "social_medium/elder/README.md"                                          # adaptive interface without condescension; agency preserved
    - "governance_layers/geographic_political/family/elder/README.md"          # elder constitutional role in the family layer
    # acknowledged-gap: no recovery epic; the social_medium/elder principles carry the load.
    # Cartographer should now strongly rank "author a recovery/ epic body": three Gertrude/Dowell
    # stories converge on it.
  personas:
    - "human-gertrude-grandma"           # subject; community profileReach; large-text + simple-navigation
                                          # accessibility (per record line 11); communities include garden-club + local-church
    - "human-matthew-manager"            # her first call; her trusted person; one of five share-holders
    - "human-jessica-spouse"             # Matthew's spouse; second to respond
    - "collective-household-gertrude"    # the household; the substrate-side actor restored to wholeness
    - "collective-household-dowell"      # the household answering the call
    # ROLE GAP: no `role-as-account-claimant` record exists. Distinct from role-social-medium-elder
    # (an archetype, not a role), role-as-stewardee (a different shape — claimant is asking for
    # herself, not being stewarded by another), or @as-recovery-counterparty (the inverse direction;
    # holding for someone vs claiming for self). Crosscuts imagodei (identity), social_medium/elder
    # (dignity-preserving recovery), and family/elder (constitutional standing). Cartographer-rank:
    # "create role-as-account-claimant" — useful beyond Gertrude (any human under recovery is in
    # this role at the moment of asking).
  scenarios:
    - "genesis/a2o/features/auth/recovery/recovery-m5-lost-key-entry.feature"        # entry-point routing
    - "genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature"  # share-holder side
    - "genesis/a2o/features/auth/recovery/recovery-shamir-optional.feature"          # share-custody substrate
    - "genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature" # doorway discovery
    # CANONICAL FEATURE GAP: `social-recovery-with-help-from-family.feature` does not exist.
    # The shape this story dramatizes — the claimant's experience from the moment of being locked
    # out through the moment of being back in, carried in plain language end-to-end — is not in
    # any Gherkin. The existing recovery-m5-* features test the substrate behaviors; none test the
    # grandma-standard UX invariant (no jargon shown to the claimant; no seed phrases ever; ambient
    # notifications throughout; the elohim doing the translation between substrate and human). This
    # feature would be the load-bearing test for the entire grandma-standard memory.
  devices:
    - "device-2019-android-phone"        # the new phone; carrier-grade NAT; large-text capable; the surface
    - "device-home-nuc"                  # the hub at home that already holds her data; substrate continuity
  historian_precedents:
    - "memory:project_recovery_grandma_standard"
    # The bar. The whole memory is the substrate this story is the demonstration of. *"Recovery must
    # feel like 'log in on a new device with help from your people' — not a crypto ritual."* This is
    # the story version of that sentence.
    - "memory:project_socially_derived_security"
    # The mechanic. Five share-holders, threshold three. Each contact's elohim evaluates
    # plausibility; the humans confirm; the shares assemble in a blind doorway proxy; a KeyRotation
    # entry lands on DHT; the old key revokes. Gertrude sees none of this. Gertrude sees a call to
    # Matthew, a green light on her phone, and her photos coming back.
    - "memory:project_graduated_recovery_authority"
    # The community can always make it right. Gertrude exercises layer 1 (intimate circle quorum)
    # through Matthew/Jessica and three other share-holders. She did not opt into "crypto-only," and
    # the protocol would not have let her if she had tried. The community-mediated path is always
    # the floor.
    - "memory:project_subsume_g_f_a_via_it_just_works"
    # The standard the story is held against. *"Credible to a grandmother whose family photos are
    # on the line."* The story is the test case. If it does not pass the parent-at-the-kitchen-table
    # test, the protocol has not yet earned the right to compete with corporate custody.
    - "memory:feedback_less_pushy_notifications"
    # The UX discipline carried throughout. Gertrude is not interrupted with modals during recovery;
    # the elohim translates substrate-events into ambient status; the share-holders receive a single
    # gentle ask, not a daily nudge; the closing of the flow surfaces as a quiet green light, not a
    # celebration.
    - "archive:.claude/archive/2026-05-14/graduated/project_ungrudging_service.md"
    # The principle the james-and-the-spoke story graduated: *the protocol does not sulk when she
    # disconnects. It serves her family on her family's terms and then steps back.* Extended here
    # to recovery: the protocol does not bill her, does not make her sign up for premium, does not
    # send her a marketing email afterward. It restores her and steps back.
---

# Gertrude Logs In with Help from Her People

The phone Gertrude has been carrying since Christmas — the one her son David gave her
when her old phone finally died — fell in the church parking lot on Sunday morning,
and Gertrude did not notice until she got home and tried to call James to wish him
happy birthday for the next day. The screen lit up; the screen went dark; the screen
lit up; the screen went dark in a different way, with a small spinning circle that
would not stop spinning.

On Monday, David took the phone to the place at the mall and they told him the
mainboard was done. On Tuesday, Gertrude went with him to pick out a new one. The
man behind the counter handed her the new phone, said *we just need to log you into
your account,* and Gertrude looked at the screen the way she has been looking at
screens for fifteen years, which is to say with the calm patience of someone who
expects to be asked something she will not understand.

But the screen, this time, did not ask her something she did not understand.

The screen said, in plain large letters: *Welcome back, Gertrude. This is a new
phone. To make sure it's really you, we're going to ask a few of your people. You
can choose who. There is no hurry.*

Underneath was a list of names. Her son David, who was standing next to her. Matthew
and Jessica, the Dowells, who lived three states away. Carol from the garden club.
Pastor Anne. Five names, each one a small picture, each one with a smaller-letter
note beneath: *available, will get a quick message on their phone, can answer in a
minute or two.* At the bottom, the screen said: *Pick at least three. The more you
pick, the faster this will be. You can also just call them, and we'll know.*

Gertrude considered the list. She tapped David, because he was next to her. She
tapped Matthew, because Matthew was Matthew. She tapped Carol, because Carol was
the most likely of the five to be sitting in her kitchen with her own phone in her
hand at eleven on a Tuesday morning.

The screen said: *Thank you. I'm asking them now.*

---

What happened next, Gertrude did not see. She put the phone in her purse and went
with David to the food court because David said this would probably take a few
minutes and the iced tea at the food court was decent.

What happened next, from the protocol's vantage, was this. The doorway in the
mainboard of the new phone — Gertrude had never heard the word *doorway* and would
not know to use it — broadcast a small careful request out to the network. *A
person who says she is Gertrude is sitting in front of a new phone. Here is what
she knew about herself: she could name three of her people from the small set the
protocol offered her. Two of the three have already accepted; we are waiting for the
third.* The request carried no key material, no seed bytes, no anything that could
hurt Gertrude if it leaked. It carried only a claim, a list of names, and a polite
request for those names to weigh in.

David, sitting next to her, felt his phone buzz softly. He looked. *Gertrude is
setting up a new phone. She is sitting next to you. Can you confirm?* He tapped
Yes.

Carol, in her kitchen, heard her phone make the small sound it made for things that
were not urgent. She looked. *Gertrude is setting up a new phone with David at the
mall. She named you as one of her people. Can you confirm she's who she says she
is?* Carol almost laughed. She had known Gertrude for forty years. She tapped Yes.

Matthew, in his utility closet two thousand miles away, was rebuilding a kernel and
had his phone face-down on his desk. The notification did not interrupt him; the
small green dot on the panel above the family-node lit up gently. He glanced at it
twenty minutes later. *Gertrude is on a new phone. David and Carol have confirmed.
Will you?* He thought about whether he would have heard from David first if
something were actually wrong. He decided he would have. He tapped Yes, and the
panel asked him one further question, in the careful language the protocol used
for things that mattered: *Do you have any reason to think this is not Gertrude?*
Matthew thought. He did not. He tapped *No reason.*

---

Underneath all of this — somewhere in the substrate Gertrude did not need to see —
three elohim-agents had also been listening. David's elohim had checked: yes, David
was where his phone said he was, his speech patterns in his Yes-tap context matched
his baseline, the request was not happening at a time-of-day that David's pattern
called unusual. Carol's elohim had checked: same. Matthew's elohim had checked:
same, and also flagged that Matthew had taken slightly longer than usual to respond,
which was consistent with him being in the closet with the kernel. All three
elohim-agents had then attested separately — *I confirm this is consistent with my
human's pattern* — and the protocol had folded those three substrate-attestations
into the same vote the humans were casting.

Three humans plus three elohims. Quorum, twice over.

The doorway-side of the recovery — the blind proxy — gathered the responses, never
holding any of them at rest, and assembled what it needed to assemble. A
KeyRotation entry was prepared. A new agent key for Gertrude was generated, locally
on her new phone, never leaving her new phone. The KeyRotation entry was signed by
the quorum and committed. Her old key was revoked. The home-NUC in her kitchen, two
hundred miles away, noticed quietly that Gertrude was on a new device, accepted the
new device into the household ring, and began the small sync that would bring
Gertrude's photos and recipes and the videos of James reading to her on Sunday
nights back onto the new phone.

None of this took more than four minutes.

---

Gertrude was halfway through her iced tea when her phone buzzed once, gently. She
took it out. The screen said: *Welcome home, Gertrude. David, Carol, and Matthew
confirmed. Your phone is ready. Your photos are coming back now — it might take a
few minutes for all of them; the new ones will arrive first. You can use the phone
while it works.*

Underneath, in smaller letters: *Nobody had to give us a password or a code or
anything technical. They just said yes because they know you. The old phone, if
you ever find it, won't work anymore — it doesn't need to. Welcome back.*

Gertrude looked at the screen for a long moment.

She had not been afraid, exactly, when the old phone died — David had told her
the photos were "in the cloud," which she had taken on faith, because David said
things were *in the cloud* the way her own father had said the radio was *on the
air*. But she had not, until this moment, known that the photos coming back was
something that depended on her family rather than on a faceless company. And she
had not known that the family had been there, on the other side of the screen the
whole time, waiting to be asked.

She opened the photos app. There was James, on Saturday at her kitchen table,
working on his fractions. There was the tomato patch from last August. There was
the picture David had taken of the three of them at Easter, the one she had set as
her wallpaper on the old phone, restored to its place on the new one without her
asking. She scrolled. Everything was there.

She put the phone away. She finished her iced tea.

---

In the car on the way home, she said, "I should call Matthew and thank him."

David, driving, said, "He said you might. He said to tell you no need."

She laughed. "Of course he did."

"He also said," David added, glancing over, "to tell you the tomatoes you sent him
seeds for came up."

Gertrude smiled. The phone in her purse buzzed once, gently, with the last of the
photos finishing their sync. The protocol, having done what it was for, did not
brag and did not bill and did not try to keep her attention. It put the photos
where they belonged and went quiet. The new phone, in her purse, was Gertrude's
phone now. The work that made it Gertrude's phone had been done by three people
who knew her, two phone calls she did not have to place, and a kind of substrate
she would never need to learn the name of.

That night, on Tuesday, she called James to wish him happy birthday, and the
camera on the new phone showed him her face exactly the way the old phone had,
and James said *Hi Gee-Gee*, and she said *Hi sweetheart*, and the protocol did
its quiet work in the background and did not announce itself even once.

The grandma-standard had been met. The grandma had not had to know.
