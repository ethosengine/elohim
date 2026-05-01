---
title: AT Protocol Lexicon Projection at Doorway
status: Draft
created: 2026-05-01
related:
  - genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md
  - genesis/research/README.md
---

## The recognition

Doorway is the protocol's federation / web2-projection layer — DNS, OAuth relying party, federated discovery, bootstrap server. AT Protocol is structurally another federation flavor (server-as-truth, lexicon-driven, Firehose-stream). It belongs where federation lives.

Pulling AT Proto interop into elohim-storage would pollute peer-native primitives with lexicon-level concerns and hard-bind the protocol to AT Proto's substrate assumptions (PDS-as-server-of-record, did:plc, Ethereum commitment). At doorway, AT Proto becomes one optional projection surface among several. Different doorways can choose different federation postures; the protocol stays uniform.

This spec is the sibling of `2026-05-01-computation-attestation-graduated-rigor-design.md`. The peer-native primitive defines the attestation; this spec defines the projection adapter that lets AT Proto consumers participate in computation auditing without the protocol becoming captive to AT Proto.

## The primitive

A projection adapter translates between peer-native EPR variants and AT Proto lexicon records. Trait surface is loose; specific mappings (which EPR maps to which lexicon, how fields translate) are implementations.

```rust
trait LexiconProjectionAdapter {
    /// Project a peer-native EPR into an AT Proto lexicon record.
    /// Returns None if no mapping applies.
    fn project_outbound(
        &self,
        epr: &Epr,
        target_lexicon: LexiconId,
    ) -> Option<AtProtoRecord>;

    /// Translate an inbound AT Proto record into an ingest signal.
    /// Returns None if no mapping applies or the record is filtered.
    fn project_inbound(
        &self,
        record: &AtProtoRecord,
    ) -> Option<IngestSignal>;
}

trait ProjectionAttestation {
    /// Doorway signs: "I faithfully projected this record from peer X's
    /// EPR Y at scope Z." The peer never appears as the AT Proto signer;
    /// the doorway is the relying party, not the identity.
    fn attest(
        &self,
        record: &AtProtoRecord,
        source: &EprRef,
        scope: ProjectionScope,
    ) -> ProjectionClaim;
}
```

`AtProtoRecord` and `IngestSignal` are opaque to the trait — concrete lexicon shapes are implementations. `IngestSignal` flows into the protocol's existing ingest path; peers subscribe to it through their manifests rather than receiving pushed content.

## Outbound projection

A doorway operator who chooses outbound posture publishes selected peer-native EPRs into AT Proto's Firehose as lexicon records. The doorway signs each record as itself, attaching a `ProjectionClaim` field that references the source EPR's content-addressed handle. Anyone reading the Firehose can verify the projection by fetching the source EPR through any doorway and checking that the projection corresponds to the EPR.

Peers don't have `did:plc` identities. Their attestations live on the peer-native substrate; the doorway's role is to *present* them in lexicon shape, not to claim them as its own. This matches the existing OAuth-RP pattern — doorway presents identity from elsewhere, never owns it.

Operators choose what to project. A civic-tech doorway might project public deliberation results (`org.dds.result.pca`, `org.dds.result.summary`); a private-collective doorway might project nothing outbound. Defaults are conservative: outbound projection is opt-in per EPR variant, configured in the doorway's federation manifest.

## Inbound ingest

A doorway operator who chooses inbound posture subscribes to an AT Proto Firehose (or a filtered slice of one) and translates incoming records into protocol ingest signals. Ingest signals don't bind peers to consume them — they are *available*, registered against the doorway's content registry, and peers subscribe by manifest.

This matches the inside-out registration pattern. Peers register interest in content with the doorway; the doorway makes external content available; peers subscribe at their own discretion. AT Proto records flowing inbound get the same treatment as any other federated content — they pass through the doorway's projection layer and become protocol-shaped.

Inbound projection raises an attribution question. The lexicon's `did:plc` identity does not exist in our identity graph. Inbound ingest signals carry the original `did:plc` reference plus the doorway's translation claim, but they do not import `did:plc` as a peer-native identity. Peers consuming the ingest signal see the foreign identity as foreign and treat its standing accordingly — no inferred standing, no automatic admission to constitutional floors.

## Walkthrough

Maria's neighborhood clustering produced a `ComputationAttestation` EPR at the *Audit* station after Tom contested the first run. The qahal's doorway operator has chosen outbound AT Proto posture for governance-input results — the city's civic-tech consortium uses DDS-style tooling and benefits from cross-app comparison.

The doorway projects the EPR as an `org.dds.result.pca` record into the Firehose. The record carries the input Merkle root (so anyone can fetch the comments via any doorway and re-execute), the algorithm + version pin, the output hash, and the doorway's `ProjectionClaim` ("faithfully projected from peer/Maria's-qahal/EPR cid:bafyrei… at scope 2026-05-01T00:00Z"). The doorway signs as itself; Maria's qahal never appears as a `did:plc`.

A DDS-using analyst across town reads the Firehose, fetches the peer-native EPR through any doorway in the protocol, re-executes the clustering, and confirms the result. They publish their own `org.dds.result.pca` alongside, citing the projection. The conversation that started in Maria's neighborhood now has external auditors. None of them needed an account in the protocol; none of them needed Maria to have a `did:plc`.

If, later, an `org.dds.result.summary` record from a separate AT Proto deliberation seems relevant to Maria's qahal, an inbound-posture doorway can subscribe to that Firehose slice and surface the record as an ingest signal. Maria's qahal subscribes by manifest; the foreign deliberation's output becomes one input among many, treated as foreign-standing content rather than auto-trusted.

## Breadcrumbs

- The peer-native primitive: `2026-05-01-computation-attestation-graduated-rigor-design.md`. This spec consumes its EPR variants as projection sources; does not redefine them.
- AT Protocol interop lives at doorway — memory `project_doorway_is_federation_surface_atproto`. Constitutional principle for any federation-flavor interop.
- Doorway routes are manifest-driven — memory `project_doorway_manifest_driven_routes`. Federation manifest declares which EPR variants project to which lexicons.
- Doorway as peer registration point — memory `project_doorway_peer_registration`. Inbound ingest follows the same inside-out pattern; peers subscribe rather than receive push.
- Account layer shape — memory `project_peer_native_account_canonical_surface`. Doorway-as-relying-party is the principle the signing approach inherits from.
- DDS-WG lexicon catalog: `org.dds.module.polis`, `org.dds.module.sense`, `org.dds.module.survey`, `org.dds.result.pca`, `org.dds.result.summary`, `org.dds.identity.*`, `org.dds.auth.*`, `org.dds.org.*`, `org.dds.ref.*`. Concrete mappings are implementations.
- ActivityPub is structurally a sibling federation flavor. The trait should generalize cleanly when an ActivityPub driver appears; defer until then.
- Cold-path archival projection (Arweave / Filecoin / Logos) is doorway-side and sibling to AT Proto outbound. See `genesis/research/README.md` "The Archival Problem."

## Open questions

- Opportunistic `did:plc` per peer with doorway co-signing rotation events — higher interop, parallel identity surface. Defer until a concrete driver requests peer-as-AT-Proto-identity rather than peer-via-doorway-projection.
- Lexicon mapping completeness for v1 — which `org.dds.*` lexicons get implementations first. Tentative shortlist: `org.dds.result.pca` and `org.dds.result.summary` outbound, `org.dds.module.polis` and `org.dds.module.sense` inbound. Defer concrete mapping work to first driver.
- Inbound filtering — what subset of the Firehose does an inbound-posture doorway subscribe to? Probably manifest-declared topic filters per peer-subscriber rather than doorway-wide. Coordinate with doorway routes manifest schema.
- Multi-doorway coordination — when peer X is registered with doorways A and B and both project outbound, does the same EPR appear twice on the Firehose with two `ProjectionClaim`s? Probably yes (different operators, different posture), but worth deciding deduplication semantics for downstream consumers.
- `ProjectionClaim` hash anchoring — should the doorway's claim be content-addressed against the source EPR, or reference it by URI? Content-addressing makes the claim self-verifying but adds a lookup hop. Probably content-addressed; defer to first implementation.
- ActivityPub generalization — when does the trait surface generalize from `AtProtoRecord` toward a `LexiconRecord<L>` parameterized over federation flavor? When an ActivityPub driver appears.
- Sybil hardening on inbound — a hostile Firehose publisher could flood the doorway with low-value records intended to ingest-amplify. Probably solved by per-publisher rate limits in the federation manifest, but worth confirming the threat model.
