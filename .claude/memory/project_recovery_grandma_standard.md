---
name: Recovery's user-experience bar — grandma + trillion-dollar convenience
description: Recovery design must let non-technical users trust their data and identity are safe, at the same convenience as big-tech custody, via social/peer trust instead of corporate custody
type: project
originSessionId: a00556ea-66be-405a-985e-1a7a309d43a8
---
The recovery protocol exists so grandma can trust that her family photos, her identity, and her perspective on the network are safe — at the same experience standard a trillion-dollar tech company provides, but without the corporate custodian.

**Applied to design:**
- The user never handles seed phrases, cryptographic keys, or technical artifacts directly. Those are protocol-internal concerns.
- Setup ritual must feel like "choose who you trust" — emotionally, not technically.
- Recovery must feel like "log in on a new device with help from your people" — not a crypto ritual.
- Errors, edge cases, and crisis scenarios must be handled so the user is never left holding the complexity.
- The elohim-agent is the mediator that makes this possible — it carries the technical burden on the user's behalf, both at setup and during recovery.

**Why this framing matters:**
Social trust can beat corporate trust on durability and alignment, but only if the UX matches. A correct-but-ceremonially-hard recovery flow loses to Apple's seed-phrase custody even if ours is architecturally superior. The bar is not "recovery works" — it is "grandma can recover without fear, without understanding the mechanism, and without calling her grandchild for IT support."

**How to apply:**
- When any design choice adds a step the user has to understand or remember, flag it. Most should be removable.
- When comparing against "should the user do X or should the system do X for them," default to the system doing it and surfacing only what the user actually needs to decide.
- For every technical artifact (seed, share, rotation), there is a user-facing concept (your recovery, your people, your new device). Keep the two separate; the user never sees the first.
