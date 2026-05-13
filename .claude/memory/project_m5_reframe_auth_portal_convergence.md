---
name: M5 reframed — auth-portal convergence + stub defender, not full defender backend
description: M5 pivots from full elohim-defender implementation to connecting hosted-doorway + peer-native-steward auth portals via the account management surface; defender stays stubbed
type: project
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
The kickoff prompt at `genesis/docs/plans/2026-04-24-recovery-m5-elohim-defender-and-revocation-ux-kickoff-prompt.md` originally scoped M5 as defender backend + revocation UX. **2026-04-25 user reframe** changed scope:

**Defender — stub only.** Elohim calls remain stubbed in the design. Scaffold the specialist pattern (manifest, detection signal subscription, `submit_specialist_revocation` coordinator gate) but do NOT ship real detection logic. Real detection follows when elohim integration is actually ready.

**The real M5 work — auth-portal convergence.** Two auth portals coexist:
- Doorway (hosted) — `doorway/doorway-app/components/login/threshold-login.component.ts` + `components/account/doorway-account.component.ts`. Web2 portal for unsgraduated humans.
- Peer-native (steward) — supersedes doorway when a hosted human upgrades to have a peer-steward presence. EPR 2B Batch A landed the substrate: `AgentPeerBinding` entry, `/elohim/identity/handshake/1.0.0` libp2p protocol, `peer_identity_bindings` projection.

M5 connects them via the **account management surface** in elohim-app (Surface 3 from `project_imagodei_three_surfaces`). The Security & sign-in pane is where:
- M4's revocation primitives become human-visible
- The handoff between hosted-doorway login and peer-native login is visible to the human
- Stub defender attestations would surface

**Why now (2026-04-25):**

EPR 2B Batch A merged today (`79181f8e`) and landed:
- `HolochainAppSignalStream` — DNA signals subscribable from storage
- `ReconcileController` — already dispatches `RevocationAttestation` (defender's path is wired structurally)
- `AgentPeerBinding` — the peer-native identity primitive
- Identity handshake protocol — the bridge mechanic

These give M5 the substrate to ship the convergence without inventing new primitives.

**Out of scope (deferred):**

- Real defender detection signals — stay stubbed
- Self-knowledge surface (Surface 2) — separate sprint
- Hosted-cell migration / browser session handoff in detail — M6
- Hashcash / rate limiting — M6+

**How to apply:**

- M5 design must NOT end up implementing defender detection beyond stubs.
- M5 design must include the account-management shell + Security & sign-in pane.
- Treat the hosted ↔ peer-native handoff as a first-class concern, not a side note.
- Do not foreclose the OAuth-pattern shape (`project_peer_native_account_canonical_surface`).
