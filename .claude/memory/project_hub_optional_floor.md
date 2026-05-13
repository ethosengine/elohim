---
name: Hub-optional floor — laptop is a full participant
description: The protocol's design floor is one device, no hub required. Hubs are graduations that add convenience/scale, never gates that grant participation. A laptop in a Kenyan village is a full participant — DHT entries, libp2p reach, REA commitments, recovery — sized to the smallest sovereign unit. References Kolibri/community-credit/mutual-aid patterns where no provider is required.
type: project
originSessionId: 155036b0-387a-441c-91c5-7a1333fb2f07
---
The protocol's primitives are sized so a single device can be a complete participant. Adding a hub increases convenience (a teacher-laptop syncing a Khan-style library to student devices when they show up at school) and aggregates scale, but does not unlock new categories of participation.

**Why:** This is the capture-resistance invariant. If hubs are required, whoever runs the hub becomes a rent extractor. Kolibri-on-RPi, local wallets, mutual-aid ledgers, and community credit prove the floor is feasible: full participation with no provider. The protocol must hold that bar.

**How to apply:**
- Any feature that *requires* a hub for the basic learner/contributor/steward experience is a smell. Re-check the design.
- Hub designs (HouseholdHub, CollectiveHub, school-hub, parish-hub) are graduations on top of the laptop-floor, sized at "desktop with extra drives + reliable internet" before scaling further.
- Brainstorm flow: design the laptop-only path FIRST. Then layer hub-as-bonus. Never the inverse.
- "Hubs needed at this scale" is a real constraint at higher node counts, but it is reached by graduation, not by gate.
- Test framing: "Could a village in Kenya, with no hub and no doorway operator nearby, run this feature on laptop-class devices?" If no, push hub-coupling out of the primitive into a graduation path.
