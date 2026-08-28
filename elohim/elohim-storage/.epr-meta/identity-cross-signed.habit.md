---
epr-habit-version: 1
id: identity-cross-signed
invariant: >
  Agent-to-transport identity bindings are cross-signed and verifiable; no
  economic attribution rides a self-asserted binding.
status: red
active: false
checks:
  - "cargo test --test binding_attribution_refuses_sentinel (elohim/elohim-storage — the admissibility decision: a sentinel is inadmissible AND a real cross-signature is admitted; no longer #[ignore]d, so it counts in the gate)"
  - "cargo test --test binding_attribution_cut (elohim/elohim-storage — the same decision end-to-end through the projection: sentinel projects unverified, cross-signed projects cross_signed, Enforce admits only the proven peer and COUNTS the refusal, Observe is behaviour-preserving, a poisoned signature neither panics nor attributes, a lifted proof does not attach)"
  - "live: elohim_attribution_bindings_examined_total{posture=\"enforce\"} > 0 AND elohim_attribution_unverified_bindings_total{posture=\"enforce\"} == 0 on the alpha fleet — the flip-to-green measure, stated as a CONJUNCTION because the numerator alone cannot carry it. A bare zero is ambiguous between 'every binding examined was cross-signed' (the green) and 'no attribution join reached a binding' (silence), and MEASURED 2026-08-20 the live series is the second: max_over_time(...[7d]) == 0 across all 56 pod instances, and no posture=\"enforce\" series exists at all. Flipping the posture against the numerator alone would turn this habit green having verified nothing."
first_move: >
  C2-S2 — the minting path. Storage signs the transport half locally (it
  owns the libp2p Keypair) and the agent half through the existing
  `ConductorSigningClient::sign` -> `sign_for_agent` zome fn, emits the
  proof into the binding's signature field via
  `binding_proof_wire::encode_proof`, and the classification chokepoint
  already waiting on the other side turns it into a cross_signed row with
  no further change. Then flip one peer to posture=enforce and watch the
  counter.
refs:
  - "genesis/data/timeline/backlog/agent-peer-binding-signing.md — the C2-S1..S7 decomposition + the 2026-07-18 four-lens red-team review this slice implements (S4 + the attribution half of S5)"
  - "genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md — why SELECTION is safe on an unverified binding and ATTRIBUTION is not"
  - "genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md"
  - "elohim/elohim-storage/CLAUDE.md — Identity & Transport-Identity Coherence"
  - "NOT in this slice, still open: durable PLACEMENT is not yet on the cross_signed cut — services/transport_resolve.rs's source-2 fallback (the load-bearing path in prod) can still redirect shard PUSH bytes to a spoofed peer while shard_locations keeps recording the victim. Named + file-anchored in agent-peer-binding-signing.md; it needs C2-S2 first, because gating it today would stop shard distribution outright."
retire-when: >
  when an unverified binding is unrepresentable rather than merely refused — the binding
  type itself carries the cross-signature — so no economic attribution can be constructed
  from a self-asserted one even by a caller trying to.
---
DELTA 2026-08-28 (STAYS RED; first_move CORRECTED, not advanced). The
first_move text below is stale: C2-S2 landed 2026-08-19 as
p2p/binding_mint.rs (2aedf0947) — transport half signed with the libp2p
keypair, agent half via sign_for_agent, encode_proof, self-classified
cross_signed, spawned from main.rs, default-ON. The open rung is the
2026-08-20c one: read bindings_examined{enforce} on a deploy that carries
the counters (joins absent vs bindings absent), then decide the iroh
transport half under dual. Recorded as `epr flow note --kind correction`;
campaign spec ratchet-to-delivery-dataplane-sdk-lanes, lane P rung P8.
DELTA 2026-08-20c (STAYS RED; the flip measure was unfalsifiable and now
is not — a measure fix, never a status change). The live check read
`unverified{enforce} == 0` alone, which is satisfied by two opposite fleet
states: every binding examined was cross-signed, or no attribution join
reached a binding. MEASURED: max_over_time(unverified[7d]) == 0 across all
56 alpha pod instances, and no posture="enforce" series exists — so the
zero is silence. That matters because the gate was corrected 2026-08-20 to
key on the live count rather than on a minter existing, and the live count
IS zero: by the letter of the old check, someone could have flipped
ELOHIM_ATTRIBUTION_CROSS_SIGNED=enforce today and turned this habit green
having verified nothing. Note the reading also contradicts the two
structural reasons the count was expected NON-zero (minting is
libp2p-only while alpha runs dual since 2026-08-05; the minted proof
reaches 2 of 3 writers) — an expectation of non-zero and 7 days of zero
cannot both hold, and the denominator is what settles it. Landed:
elohim_attribution_joins_total and
elohim_attribution_bindings_examined_total (both posture-labelled, both
incremented UNCONDITIONALLY — a counter that fires only on the interesting
path is how the numerator got here), AttributableBindings::examined()
counted BEFORE the posture filter so Enforce cannot shrink its own
denominator, and 2 tests pinning the three-way read. 13 passed / 0 failed
across both attribution targets. NOT proven: whether the joins are absent
or the bindings are — that needs one deploy carrying the counters, and it
is the next rung, not this one.
DELTA 2026-08-18 (unwired -> RED; C2-S4 + the attribution half of C2-S5
landed, desk-proven, DEPLOY-INERT by default). The red was already
written but was #[ignore]d — a CI no-op — and measured 2 failed / 0
passed against honest stubs (binding_admissible_for_attribution returned
`!signature.is_empty()`, faithfully reproducing the only gate a binding
passes today: imagodei_integrity's non-empty check, which a sentinel
satisfies). It now measures 8 passed / 0 failed across two files that
both run unignored. What landed: (a) the projection RETAINS the
signature it used to drop (peer_identity_bindings.signature +
proof_status, migration 2026-08-18-104500, DEFAULT 'unverified' NOT
NULL, backfilled) — without it no consumer could have verified anything
even if it wanted to; (b) `p2p::binding_proof_wire` — a panic-free
envelope codec over the landed C2-S1 algebra plus the classification
chokepoint, which is the ONLY constructor of the cross_signed value
(BindingProofStatus's inner enum is private and is the insert model's
field type, so a careless writer gets a compile error, not an
attributable row); (c) all four binding writers classify at write
(gossip, handshake against the TRANSPORT-verified PeerId, DHT arrival,
and the DHT-arrival signal now carries the entry's signature through
instead of discarding it — the gossip re-publish forwards a real proof
rather than substituting a sentinel); (d) the typed attribution cut,
`AttributableBindings`, which economic joins take BY TYPE (reciprocity
ledger REST + GraphQL, cluster totals' external_committed_bytes, the
per-device stewarded triptych) while routing/display keeps the honest
self-asserted set. WHY IT IS STILL RED, precisely: the cut's posture
defaults to observe (ELOHIM_ATTRIBUTION_CROSS_SIGNED=enforce flips it),
so a deploy changes no behaviour and blanks no economic surface — which
is correct, because NO PEER CAN YET MINT A PROOF (C2-S2 unbuilt);
enforcing first would be a self-inflicted outage in the name of a
property nothing can satisfy. Verification is also receiver-local, not
notarized, until the integrity-zome fold (C2-S7). Genesis fixtures
verified compatible: the seeder writes signature=[0], which classifies
unverified without panicking, and no View type changed, so no TS codegen
moved. Flip-to-green needs, in order: C2-S2 (storage assembles a real
proof for its own agent_cid — transport half local, agent half via the
existing ConductorSigningClient::sign -> sign_for_agent), then the
counter draining to 0 under posture=enforce on alpha.
