---
name: project-continuity-witness-stepup-check
title: "Continuity witness triggers a p2p step-up check"
description: "Operator 2026-10-29: a private behavioural fingerprint (trustful-pool custody; device-keyed only at bootstrap) notices when someone else acts as the steward of record and triggers a p2p 2FA check that accelerates elohim intercession."
metadata:
  node_type: memory
  title: Continuity witness triggers a p2p step-up check
  type: project
  originSessionId: 5365acd4-e50b-4288-8f26-536d931715d7
  modified: 2026-10-29T17:48:48.504Z
---

Operator, 2026-10-29: "'I'm the steward of record' and if something like my 8 year old son starts typing.. there's
going to be a fingerprint there, (later biometrics and IoT), that triggers that stop/check p2p-2fa to assint and
accelerate the intercession."

Design shape agreed in conversation (not yet specced):
- **The signal is held in a trustful pool, not a public one.** Operator correction, same day: "there' has to be a
  pool though.. it's just a trustFULL pool, but evenutally that encrytped sharded data that's replicated among those
  peers to protect your resiliency. So it's only.. device-keyed, because right now.. we're just in development and are
  refining the boostrap path." Typing rhythm / biometrics / IoT are attention-class data: never in a *commons* pool
  (public records, per the commons-pool spec) and never readable by holders, but encrypted, sharded and replicated
  across the person's own holon peers so they survive device loss — custody by a threshold of independent holders
  (diversity, not headcount; holders learn sizes and timing only). Device-keyed custody is the current bootstrap rung,
  not the design ceiling. Only the outcome crosses as an attested-private (B2) record: continuity broken on device X,
  check requested.
- **The check is the device-join consent act pointed at a session.** The session's `affirmsStewardOfRecord`
  covenant clause is *suspended* (not revoked) until another of the person's devices, or a holon member at higher
  stakes, answers through the existing approve-on-the-approving-node flow (`crates/consent-grant`). Agents notice and
  accelerate; they never decide alone ([[feedback_human_loop_not_terminal_authority]]).
- **Outcome is truthful attribution, not punishment.** A child gets mediated agency ("his keystrokes, under the
  guardian") per the cradle-to-grave gradient; a stolen device gets revoked recognition so readers stop admitting the
  signer while its provenance stays honestly reported.

**Precondition gaps:** `epr covenant` honours clauses without verifying their signatures
(`elohim/eprfs/epr-cli/src/covenant.rs` (the honouring logic in `Covenant`, ~L502-640 on origin/dev; re-verify the no-signature-check claim before relying on it)); sibling-verdict admission does not read revocation.
Related: [[feedback-holon-diversity-makes-root-inviolable]], [[feedback_elohim_witness_human_identity]].
