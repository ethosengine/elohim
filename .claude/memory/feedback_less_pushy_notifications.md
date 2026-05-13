---
name: Less pushy notifications — prefer ambient over interruptive
description: UX default: ambient status > passive prompts > gentle one-time nudges > never interruptive notifications; system does sensible thing automatically, surfaces result quietly
type: feedback
originSessionId: a00556ea-66be-405a-985e-1a7a309d43a8
---
Default strongly toward ambient, non-interruptive UX. The fewer notifications, modals, badges, and "you should do X" prompts the user sees, the better.

**Why:** Notification fatigue destroys trust. Every interruption says "we needed your attention to justify our existence." The elohim-agent's job is to carry the user's technical burden, not to keep poking them. People don't want to think about security, recovery, or governance infrastructure until they have to — and when they have to, it should just work. Until then, it should be silent.

**How to apply, in priority order:**
1. **Do the sensible thing automatically.** Use defaults, infer from context, act on the user's behalf. Recovery provisioning on the 2nd emergency contact shouldn't require clicking "set up" — it just happens.
2. **Surface result ambiently.** A passive status chip in a profile panel ("5 people are protecting your account") beats a success toast beats a modal.
3. **Batch, don't serialize.** If multiple small decisions need the user, bundle them into one occasional check-in, not N separate prompts.
4. **One gentle nudge, then never again.** If something truly needs user input (e.g., a contact you selected refused), say it once, in-context, and let it settle. Don't re-prompt daily.
5. **Interruptive notifications are for crisis only.** "Someone is trying to recover your account" — yes. "We'd like to remind you to back up" — never.

**Examples where this applies:**
- Recovery setup (ambient, auto-provisioned when relationships reach threshold)
- Reshare triggers (surfaced once in profile, not notified)
- Share-holder acceptance (one prompt, then silent; elohim handles the rest)
- Progress indicators during recovery (passive panel, not push notifications)
- Gate/discernment outcomes (only surface to user when action is genuinely required)

Applies equally to elohim-agent behavior, frontend UI, and any future notification surface across the protocol.
