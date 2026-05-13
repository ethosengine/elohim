---
name: Socially derived security — peer-held shares, elohim-attested identity
description: Foundational recovery principle — identity recovery goes through peers' elohim-agents attesting to you, not through doorway custody or seed phrases
type: project
originSessionId: a00556ea-66be-405a-985e-1a7a309d43a8
---
Identity and key recovery are **socially derived**. No single party (not doorway, not any peer, not the protocol itself) can recover your identity alone. Recovery requires your people's elohim-agents, plus your people themselves, to attest you are who you say you are.

**Mechanics:**
- A master *recovery seed* (distinct from the everyday agent key) is Shamir-split across emergency contacts' devices at setup time.
- On recovery, each contact's **elohim-agent** evaluates plausibility from shared history, anomaly signals, behavioral patterns, and the human contact's own confirmation.
- On affirmative elohim + human judgment, the peer's device releases their Shamir share.
- Doorway is a *blind proxy* — coordinates the challenge, reassembles shares in memory, commits a `KeyRotation` DHT entry, zeroizes the seed. Doorway never has custody of the seed or any share.
- The reassembled seed authorizes a `KeyRotation` entry on DHT → fresh agent key generated for a hosted cell → old agent key can be revoked (`KeyRevocation`). Stolen device does not mean permanent compromise.

**Why:** This collapses three failure modes in one pattern —
1. Seed phrases lost/stolen are not a risk because there is no seed phrase.
2. Device theft does not mean identity theft — the thief has the old key but cannot authorize a key rotation without quorum approval from the victim's contacts.
3. Doorway compromise does not expose recovery — doorway is blind.

**How to apply:**
- When designing any recovery or re-authorization flow, peer elohims and the humans they steward are the trust oracle. Doorway is coordination only.
- Biometrics, 2FA, hardware tokens, passphrases are *pluggable additional factors* that individual contacts' elohims can weight in. They are optional strengthening, never replacements for social attestation.
- "Blind proxy" is the doorway contract for recovery — if any design step requires doorway to hold plaintext keys or shares at rest, it violates this principle.
- The Shamir *seed*, not the agent key itself, is what gets split. This is the invariant that makes revocation possible.
