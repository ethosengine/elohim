---
id: shared-rust-discipline-network-authority-report
status: HOLD
gap: plans__2026-11-02-shared-rust-discipline#3
actor: agent:implementer@codex
date: 2026-11-02
serves: rust-style
cites: []
---

# Network policy adoption authority

No implementation or network acceptance is claimed. Base inspected:
`d7dc62fdd328f3c50b87ded46b34f599802a97bb`.

## Missing station

chain: shared-rust-discipline / SRD-3

between: addressed policy offer -> independently authorized space adoption

missing node: the receiving peer resolves a space-controller authority anchor that
was admitted independently of the arriving policy claim, and verifies the signer,
scope, delegation and revocation against that anchor. Probe: an Elohim signer offers
an otherwise valid adoption naming Brit; Brit refuses it, while Brit's legitimately
appointed signer succeeds and a revoked delegation subsequently fails.

current state: unresolved by source inspection. No repository-space controller
resolver was found in the inspected Mishpat coordinator and storage services.

## Existing mechanisms and their limits

- `mishpat/src/commitment_record.rs::get_commitment_record` reads an exact signed
  Commitment Create, verifies its action and entry hashes, and preserves the author.
  Its contract explicitly excludes current delegation and non-revocation.
- `mishpat/src/invocation.rs::authorize` checks cell-owner provenance or an assigned,
  function-listed exact-payload mandate. That answers who may request signing in a
  cell, not who may govern the named independent repository.
- `mishpat/src/bootstrap_steward.rs` explicitly says bootstrap identity is not an
  authority check; stewardship-grant resolution must supply authority instead.
- Imagodei's `StewardshipGrant` governs a human subject's filtering, time limits,
  features, monitoring and policy delegation. It is not an existing repository-space
  controller anchor and cannot be repurposed by matching a slug.
- `create_commitment` checks action payload shape. Its dispatch currently refuses
  `binds-policy` as an unhandled action. Adding a JSON validator would remove that
  refusal without supplying authority.
- `create_commitment_state_link` documents GAP G7: arbitrary peers can author
  well-shaped active/revoked lifecycle links. The link is a discovery claim, never
  a verified verdict. A receiver must validate the author and referenced event.
- Mishpat integrity admits unknown commitment actions with generic shape checks.
  Therefore the coordinator's unknown-action refusal does not prove arbitrary
  source-chain writes impossible. No negative test may claim that broader property.

## Implementation boundary

Reuse exact Commitment read-back and invocation mandates once space-authority
admission exists. Bindings can remain immutable commitments, with receiver-side
verification of their evidence. Neither bootstrap identity, a signed self-claim,
the supplying peer's local projection, nor a lifecycle tag may select the binding.
Do not add a generic binds-policy operation until that authority is enforceable.
If admission requires integrity changes, state the DNA-hash-moving cost explicitly;
do not promise coordinator-only completion from the existence of Commitment alone.

## Evidence

Gate evidence: no gate or Cargo test run; no code changed. Read-only source inspection
and `brit revision resolve HEAD` completed successfully. `epr flow context` found no
current projected assertions for the new plan; no native SRD-3 acceptance was minted.
No network probe, conductor operation, push, commit or real-space adoption occurred.
The terminal `epr flow note --on SRD-3` attempt exited 2: SRD-3 is not yet a
projected sidecar label or known commitment. No terminal observation was recorded.

## Steward clarification

The steward subsequently authorized a micro council of two independently grounded
repository representatives under their own stewardship, allowing unresolved imagodei
linkage to remain explicit. That encounter ran and reached a conditional trial
recommendation; its public outcome is in `task-srd-4-report.md`. The local delegation
question is answered. This does not establish a peer-verifiable controller resolver,
signature, network adoption or revocation proof; SRD-3 remains open for those reasons.
