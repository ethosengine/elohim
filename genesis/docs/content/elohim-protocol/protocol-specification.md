---
id: elohim-protocol-specification
---

# The Elohim Protocol: Content Addressing for Human Flourishing

A specification for content that travels with its knowledge, value and governance context.

Version 0.2 (September 2026)

---

## Executive Summary

AT Protocol, ActivityPub and IPFS address content as signed records, server URLs and hashes, and none of them addresses it together with the knowledge it belongs to, the people who care for it, or the rules that decide who may see it.

The Elohim Protocol addresses content through an **Elohim Protocol Record (EPR)**: a reference that resolves to a record whose three dimensions travel together.

- **lamad** (knowledge): what this content is, how it relates to other knowledge, and how people have learned from it.
- **shefa** (value): who stewards it, what recognition has accumulated, and what economic flows attach to it.
- **qahal** (governance): what authority ratified it, how far it may travel (its [reach](glossary.md), one of eight levels from `private` to `commons`), and which constitutional layer, from the individual to the global, binds it.

This specification reads the same EPR as a Reference while it is being composed and as a Record once it is at rest. Appendix E.1 explains the two names.

The protocol is designed so that content is reached through its record. A conforming peer checks the governance in an EPR's head before it serves the content bytes, and a delivery made through EPR resolution credits the people who steward the content. Bytes fetched by their content address alone, outside EPR resolution, carry none of this context.

A CID tells you what the bytes are. An AT URI tells you who published them. An Elohim EPR tells you what they mean, who cares for them, and who said they could be here.

---

## Scope

This specification defines:

- the three resolution tiers of an EPR (Part II) and the rules that couple knowledge, value and governance within them (Part III);
- the resolution protocol `/elohim/epr/1.0.0` (Part IV);
- the feed protocol `/elohim/feed/1.0.0` (Part V), which is specified here and not yet implemented;
- the `epr:` URI scheme and the content-address formats (Appendix E), and the alignment with IPLD (Appendix F).

Version resolution and search are outside version 1.0.0 of the resolution protocol.

The record shapes and coupling rules (Parts II and III), the wire messages (Parts IV and V) and their framing (Appendix C), the `epr:` URI syntax (Appendix E.1), content-address canonicalization (E.5), the head encoding (F.1 and F.2) and the stored-head migration rule (G.2) are normative, and a **conforming peer** implements them, Part V only if it serves feeds; the worked examples and the rest of the appendices, Appendix A included, are informative.

The substrate beneath these protocols is specified elsewhere. The `elohim-epr` crate is the content-addressing root: it encodes the EPR atom, the signed record itself, as a DAG-CBOR envelope addressed by CIDv1 and carrying Ed25519 proofs. Holochain DNAs notarize structural commitments. A DNA is an application's rule set, divided into modules called zomes, and its hash defines a distinct network. To notarize a commitment is to record it on the Holochain DHT, an agent-centric distributed hash table with no global consensus, in which peers validate each entry against the rules of the DNA they share. A conductor is the Holochain runtime that runs a participant's cells (a cell is one participant's running instance of a DNA), on their own device or, for a hosted participant, on a doorway. elohim-storage moves and reconciles bytes between peers over libp2p and over iroh, a QUIC-based peer-to-peer transport. Doorways project canonical content to the web; they are plural and replaceable, and none of them owns the content. The [glossary](glossary.md) defines DNA, conductor and source chain in plain terms. The [core graph substrate design](architecture/2026-04-21-elohim-core-graph-substrate-design.md) specifies the atom, and the [substrate trust contract](architecture/2026-07-12-substrate-trust-contract-runbook.md) states the invariants that govern how heads are declared, propagated and repaired.

An **elohim** (the word is both singular and plural) is one of the protocol's AI agents ([glossary](glossary.md)). An elohim acts for a person and their community, under that person's authorization and within the constitution, and holds no authority of its own. In this specification an elohim may issue attestations, explain a reach negotiation, add restrictions to a policy chain and apply a community's economic rules.

Several concerns that the [manifesto](manifesto.md) and the [constitution](constitution.md) raise belong to companion specifications:

| Concern | Governed by | Why it is outside this specification |
|---------|-------------|--------------------------------------|
| Physical privacy controls | [Observer Protocol](observer-protocol.md) and [Hardware Specification](hardware-spec.md) | Hardware and firmware layer, not wire protocol |
| Harm response and protection of those harmed | Elohim Agent Constitution (planned) | Agent behavior, not content addressing |
| Recognition circulation: decay, thresholds, redistribution | Shefa Economic Protocol (planned; see [Shefa](shefa.md)) | Economic policy, not content format |
| How ratifiers are chosen; cryptographic sortition is one possible mechanism | Constitutional Council Protocol (planned) | Governance mechanism, not content delivery |
| Identity across the participation stages: Visitor (browsing, no account), Hosted (a doorway holds the person's keys and runs their cell), App Steward (the desktop app, with self-custodied keys) and Node Steward (always-on infrastructure) | Identity Portability Protocol (planned) | Identity lifecycle, not content references |

This specification provides the hooks these companions attach to: the `ratifierId` on each stewardship allocation connects to how ratifiers are chosen, `recognitionPolicy` connects to the economic rules, and `reach` connects to privacy. It does not define the rules themselves.

---

## Part I: The Protocol Gap

### Why Existing Protocols Fail

> "A dollar spent on weapons is identical to one spent on medicine, it's a system without values feedback."
> ([Manifesto](manifesto.md), on currency)

A CID on IPFS carries no values. An AT Protocol record carries one owner's signature. An ActivityPub URL carries one server's authority. None of them can express:

- Multiple non-owning stewards (people or collectives who hold an attested, revocable caretaking relation to the content) who share fractional responsibility for a piece of knowledge
- Visibility gated by governance and enforced at the protocol layer, not the application layer
- Recognition triggered by content delivery, so that serving knowledge produces value for those who care for it
- Constitutional verification: before a peer serves content, it checks that the constitutional documents it operates under (see the [constitution](constitution.md)) match their notarized versions, and it refuses to serve if they do not (Part IV)

| Protocol | Content addressing | Ownership model | Governance | Value flow | Peer-to-peer |
|----------|-------------------|-----------------|------------|------------|--------------|
| AT Protocol | CIDs in signed repos | Single owner (one DID = one repo) | None (app-layer labeling) | None | No |
| ActivityPub | URLs on servers | Server owns URL | Advisory (`to`/`cc` fields) | None | No |
| IPFS | CIDv1 (content hash) | None | None (no access control) | None | Yes |
| Holochain | EntryHash + ActionHash | Agent provenance | DHT validation zomes | None native | Yes |
| Elohim Protocol | CIDv1 plus a three-pillar EPR Head | Multiple stewards, no owner | Reach and constitutional layer on every head | Recognition event on delivery | Yes |

### What Value-Blind Content Costs

When content is addressed without its economic and governance context:

- Stewards are invisible. Someone curated, translated or maintained this knowledge, and the protocol forgets them.
- Governance is optional. Reach levels, constitutional constraints and community ratification become application conventions that any client can bypass.
- Value extraction is frictionless. Content can be served, cached and redistributed without any recognition reaching those who created it and care for it.
- Context is severed. A concept torn from its knowledge graph, its learning paths, its prerequisite relationships is diminished, like a verse torn from its chapter.

The Elohim Protocol is designed to close these gaps in the protocol itself rather than in the policy of any one application.

---

## Part II: The EPR and Its Three Tiers

Every piece of content, every learning path and every assessment is reached through an EPR: a reference that resolves to a record whose three dimensions travel together. For content, that record resolves in three tiers, and the head of a content EPR always carries all three dimensions. Other kinds of EPR, such as economic events and attestations, carry the dimensions their kind requires; the core graph substrate design lists them.

### Three Tiers of Resolution

An EPR resolves in three tiers, each serving a different availability and privacy requirement:

```
Tier 1: EPR Head        (~500 bytes; fits a Kademlia record or a feed message)
  ↓ resolve
Tier 2: EPR Document    (~5-50 KB, cached by interested peers)
  ↓ resolve
Tier 3: Content Bytes   (any size, steward-delivered, described by a shard manifest)
```

This tiered model solves the **steward offline problem**: even when every steward is asleep, Tier 1 EPR Heads remain available in Kademlia, so peers can discover, preview and queue content for later retrieval.

### Tier 1: EPR Head

The **EPR Head** is the magnet link of the Elohim Protocol. It is small enough to store in a record of Kademlia (the distributed hash table libp2p peers use for discovery), embed in a feed message or send to any peer, and it carries enough to render a preview card, assess access rights and fetch the bytes by their content address. An earlier draft carried a flat signed head; the canonical shape is the nested record below, which the `elohim-epr` crate implements and Appendix F.2 encodes as DAG-CBOR.

```
EPR Head {
  version:        u32                  // schema version of this record (1)
  id:             String               // stable slug; survives updates
  content:        CIDv1                // current content bytes (raw codec 0x55),
                                       // carried as a string

  lamad: {                             // knowledge
    title:          String
    contentType:    ContentType        // Appendix A
    description:    String?
    contentFormat:  String?            // markdown | sophia-quiz-json | gherkin |
                                       // html5-app | epr-composite | ...
    tags:           [String]
  }

  shefa: {                             // value
    stewards:       [String]           // steward identifiers: a DID, or a
                                       // ContributorPresence ID that resolves to one
                                       // once its creator claims it
    allocations:    [f64]              // ratios in [0, 1], parallel to stewards;
                                       // they should sum to 1.0
  }

  qahal: {                             // governance
    reach:          Reach?             // Appendix A; absent reads as private
    layer:          ConstitutionalLayer?   // Appendix A; absent reads as individual
    attestationRequirements: [String]  // gates the serving peer checks before it
                                       // serves this head (Part IV), written
                                       // "type:reference", e.g.
                                       // "prerequisite-mastery:calculus-101"
  }

  relationships: [{                    // typed edges to other heads
    type:           Relationship       // Appendix A
    target:         String             // target content id
    targetCid:      CIDv1?             // target content address, as a string
  }]

  author:         DID?                 // the content's creator
  updated:        String?              // RFC 3339 UTC time of the declaration
                                       // on the Holochain DHT (Authority, below)
}
```

`contentFormat` names a format from the lamad domain manifest, such as `sophia-quiz-json` (an assessment rendered by Sophia, the protocol's assessment renderer) or `epr-composite` (Appendix D). A **ContributorPresence** holds a creator's attribution, and the recognition their work earns, in trust until the creator joins the network and claims it; once claimed, it resolves to the creator's DID.

#### Canonical encoding

The canonical form of a head is its DAG-CBOR encoding, and the head's own address is the CIDv1 of those bytes (codec 0x71, SHA-256), which prints with the prefix `bafyr`. DAG-CBOR's deterministic encoding sorts map keys and uses the shortest integer forms, so two peers that encode the same head produce the same bytes and the same CID. Absent optional fields, and empty lists other than `relationships`, are left out of the encoding rather than written as null or empty.

Inside `/elohim/epr` messages, feed messages and Kademlia records, a head travels as a MessagePack encoding of the same record (Appendix C). A receiver that needs the head's CID re-encodes it as DAG-CBOR.

#### Authority

A head carries no signature of its own. Its authority comes from two kinds of record on the Holochain DHT, both in the lamad DNA:

- The **content record** holds a content EPR's id, title, description, content type and format, tags, reach and content address; for content stored inline it also holds the body. Each revision of the content is a new version of the record, and the versions form a chain back to the first.
- A **declaration** names one version of the content record as current for its `id`. The author of the first version, or a device the author has delegated, declares within that chain, and a canonical declaration can choose among independent chains recorded for the same `id`. `updated` is the time of the declaration. While no declaration has named a current version, `updated` is absent and a peer may serve the head of the author's newest version, but such a head never replaces a declared one.

A peer derives the head from the declared version. `id`, `content`, the `lamad` fields, `reach` and `author` come from the content record. The `shefa` summary comes from the peer's records of the content's stewardship allocations, `layer` from the content's governance state, and `relationships` and `attestationRequirements` from the content graph.

#### Kademlia storage

Heads are published to Kademlia under the key `epr:{id}`, as a discovery index.

#### Size

A head with a short description, a few tags and a few stewards encodes to roughly 400 to 600 bytes, well within the size of a Kademlia record.

### Tier 2: EPR Document

The **EPR Document** holds the full pillar context of a content EPR for peers with affinity to it, meaning peers who learn from, steward or govern the content; those peers cache it locally. It is too large and too sensitive to publish on the Holochain DHT or in Kademlia, so it is retrieved directly from known peers. Version 1.0.0 of the resolution protocol defines no message that delivers it.

```
EPR Document {
  head:            EPR Head      // embedded, not referenced

  ── Lamad (full knowledge context) ───────────────────────
  relationships: [{
    targetId:      String        // related content ID
    targetCid:     CIDv1         // integrity of related content
    type:          Relationship  // Appendix A
  }]
  tags:            [String]
  contentFormat:   ContentFormat // markdown | sophia-quiz-json | gherkin |
                                 // html5-app | epr-composite | ...
  pathMemberships: [{
    pathId:        String
    stepIndex:     u32
    chapterId:     String?
  }]
  attestationsRequired: [String] // gate conditions for access
  bloomLevel:      BloomLevel?   // not_started | remember | understand |
                                 // apply | analyze | evaluate | create

  ── Shefa (full economic context) ────────────────────────
  allocations: [{                // full StewardshipAllocation records
    id:            String
    stewardDid:    DID
    presenceId:    String        // ContributorPresence ID
    ratio:         f32
    method:        AllocationMethod
                                 // manual | computed | negotiated
    contribution:  Contribution
    governance:    Governance
    ratifiedAt:    u64?
    ratifierId:    DID?
    recognition:   f64           // accumulated for this steward
    evidence:      Value?        // contribution evidence JSON
  }]
  economicEvents: [{             // REA (Resource–Event–Agent) economic
                                 // events touching this content
    eventType:     String
    resourceType:  TokenType     // care | time | learning | steward |
                                 // culture | infrastructure | recognition
    agents: {
      primary:     DID
      supported:   DID?
      witnessed:   [DID]
      benefited:   [DID]
    }
    timestamp:     u64           // Unix milliseconds
  }]
  recognitionPolicy: {           // how recognition distributes on interaction
    onView:        f64           // micro-recognition on content view
    onComplete:    f64           // on path step completion
    onAttest:      f64           // on attestation earned
    onTeach:       f64           // on sharing/teaching forward
    distribution:  String        // "proportional" | "equal" | "primary_weighted"
  }

  ── Qahal (full governance context) ──────────────────────
  constitution: {
    layer:         ConstitutionalLayer
    version:       String        // semver
    hash:          String        // SHA-256, verifiable against the
                                 // constitution's notarized entry on the
                                 // Holochain DHT
    contextId:     String?       // which community, household, etc. the
                                 // constitution belongs to
  }
  policyChain: [{                // composable restriction stack
    source:        PolicySource  // org | guardian | elohim | subject
    restrictions:  [String]      // policy IDs applied
  }]
  reachNegotiation: {            // present when the author asked for one reach
    requestedReach: Reach        // and was granted another
    grantedReach:   Reach
    reasoning:      String       // the elohim's explanation of the outcome
    trustScore:     f32?         // an input local to this one negotiation;
                                 // never a person's standing
  }?
  disputeHistory: [{             // governance state transitions
    from:          Governance
    to:            Governance
    reason:        String
    initiator:     DID
    timestamp:     u64           // Unix milliseconds
  }]

  ── Federation ───────────────────────────────────────────
  knownLocations: [{
    doorwayDid:    DID?          // doorway serving this content
    peerId:        String        // libp2p PeerId (base58)
    multiaddrs:    [String]      // libp2p multiaddrs
    lastSeen:      u64           // Unix milliseconds
    tier:          NodeTier      // hardware tier: network | home_node |
                                 // home_cluster | laptop
    capabilities:  [String]      // ["shard", "sync", "relay"]
  }]
}
```

### Tier 3: Content Bytes

The content blob itself, retrieved over the shard protocol (`/elohim/shard/1.0.0` on libp2p, `/elohim/shard/2.0.0` on iroh) and verified against the head's `content` address.

```
Content Bytes:
  Retrieval:     ShardRequest::Get { hash: head.content }
  Verification:  the SHA-256 digest of the received bytes matches head.content
  Sharding:      per ShardManifest encoding (none | chunked | rs-4-7)
  Access:        governed by head.qahal.reach and the EPR Document's policyChain
  Delivery:      any peer holding the blob may serve it
  Caching:       Cache-Control: public, max-age=31536000, immutable
                 (content-addressed bytes never change)
```

Blob stores, shard manifests and the `/blob/{hash}` route (Appendix E.3) serve Tier 3. The tiers above add context and do not change how bytes move.

---

## Part III: Pillar Coupling

A conforming peer applies these rules in its storage layer rather than in any client, so a client cannot bypass them.

### The Four Coupling Rules

**Rule 1: No value-blind content.**

The head of every content EPR carries a stewardship summary, and a head with no stewards is invalid: a conforming peer neither announces nor serves one. At minimum the original creator is the bootstrap steward, with an allocation of 1.0. This follows the manifesto's creator-presence model, in which a creator's work is attributed, and the recognition it earns is held in trust, before the creator joins the network.

```
VALID:    shefa: { stewards: ["did:key:z6Mk..."], allocations: [1.0] }
INVALID:  shefa: { stewards: [] }
```

**Rule 2: No governance-free content.**

The head of every content EPR carries reach and a constitutional layer. Content without explicit governance context defaults to `reach: private, layer: individual`, the most restrictive setting, so an omission cannot expose anything. You must explicitly grant reach; the protocol does not assume openness.

```
DEFAULT:  qahal: { reach: "private", layer: "individual" }
          (invisible to everyone except the author)
EXPLICIT: qahal: { reach: "community", layer: "community" }
          (content visible to community members, governed at community layer)
```

An explicit reach starts with the author, who asks for one when composing the content; the author's peer then checks the request against the author's standing. Reach within the author's own relationships, from `private` up to `familiar`, needs no standing; `community`, `public` and `commons` need the standing that a standing policy sets for each level. The check grants the request, refuses it, or refers it for discernment, where an elohim may explain the outcome; the EPR Document's `reachNegotiation` records a grant that differs from the request. The granted reach is written into the content record, so changing it means declaring a new version of that record (Part II). A serving peer reads reach from the declared record and does not re-evaluate the grant.

**Rule 3: Content delivery triggers recognition.**

When a peer delivers Tier 3 bytes in answer to a valid EPR resolution, it logs a recognition event against the content's stewardship allocations, and recognition reaches the stewards in proportion to their allocations. The event is recorded as an REA economic event:

```
EconomicEvent {
  eventType: "content_delivery"
  resourceType: "learning"        // or "care" for care-economy content
  agents: {
    primary:   delivering_peer_did
    benefited: [steward_dids]     // per allocation ratio
  }
}
```

To a steward, recognition is credited value, and it circulates as information: each event carries which delivery produced it, which stewards it credited and in which community's context. Token types (Appendix A) are readings of these events by resource type, such as learning or care; they are not separate currencies. How recognition circulates belongs to the Shefa economic protocol: it decays over time so that it keeps moving (demurrage), recognition beyond constitutional thresholds is redistributed to the next community or to infrastructure commons, per the [manifesto](manifesto.md)'s wealth transition architecture, and each community sets the rates and bounds in its constitution, which the community's elohim apply. This specification defines only the event.

**Rule 4: Dimensions are queried independently and stored together.**

A content EPR always carries all three dimensions, and a query may filter by any one of them:

- "All content I steward" (shefa, by steward)
- "Community-reach concepts" (qahal and lamad)
- "Recognition for this path" (shefa, aggregated across the path's steps)

The data for all three is always present, so each context can select by the dimension it needs. The query interface itself is not specified here.

### Why Always-Coupled Matters

Consider a piece of content, say the manifesto's chapter on economic architecture, addressed only by its CID:

`bafkreih7mdkh5gn3mkjbagj3n22w2eyrwya3eqzhdqr2t6lnk7iyn2blea`

This CID tells you nothing about:

- Who wrote it and who maintains it (Alice as original creator, Bob as editor; both are example identities)
- Who can see it (public reach, global constitutional layer)
- What knowledge it connects to (builds on "poverty of currency" concept, contrasts with "engagement optimization")
- What happens when you learn from it (recognition flows to stewards, engagement tracked for affinity)

The EPR Head for the same content:

```
{
  version: 1,
  id: "manifesto-economic-architecture",
  content: "bafkreih7mdkh5gn3mkjbagj3n22w2eyrwya3eqzhdqr2t6lnk7iyn2blea",
  lamad: {
    title: "Economic Architecture: The Poverty of Currency",
    contentType: "article"
  },
  shefa: {
    stewards: [
      "did:web:doorway.example.org:humans:alice",
      "did:web:doorway.example.org:humans:bob"
    ],
    allocations: [0.7, 0.3]
  },
  qahal: { reach: "public", layer: "global" },
  ...
}
```

The content is the same bytes. But the EPR carries its meaning.

### Attestations

An **attestation** is a signed witness claim by an issuer about a subject; here, about a person's relationship with a piece of knowledge. In the [manifesto](manifesto.md)'s terms, an attestation comes from a person, refers to specific content and accumulates at the creator's presence, and the constitution protects it from being sold or manipulated. An elohim that issues one does so for the person it acts for, under that person's authorization (Scope).

```
Attestation {
  id:              String        // unique identifier
  issuer:          DID           // who attests: the subject, a peer, or an elohim
  subject:         DID           // the person being attested
  contentCid:      CIDv1         // the content this attestation concerns
  claim:           String        // "engagement" | "reflection" | "application" | "teaching"
  evidence:        String?       // optional narrative or hash of an assessment result
  timestamp:       u64           // Unix milliseconds
}
```

An attestation's history is append-only. An attestation cannot be sold or transferred, and it is never edited or deleted in place: withdrawing one means issuing a superseding attestation that points back to it, so the record of what was attested, and when it was withdrawn, stays auditable. Attestations travel as EPRs of kind `Attestation`. The envelope carries the issuer's Ed25519 signature over the record's canonical DAG-CBOR bytes and, for a withdrawal or revision, a `supersedes` link to the attestation it replaces. Holochain source chains (each participant's own append-only, tamper-evident log of actions) keep this history, and a conforming peer refuses `Update` and `Delete` operations on attestation records.

Attestations are designed to meet access gates (a head's `qahal.attestationRequirements`, a Document's `attestationsRequired`), and they feed the three meaning maps that [lamad](lamad.md) builds for a learner (knowledge, love and self: how they relate to a subject, to other people and to themselves). They are the protocol's alternative to engagement metrics.

---

## Part IV: Resolution Protocol, `/elohim/epr`

### Protocol Definition

A request-response protocol carried over libp2p and, with the same message types, over iroh QUIC.

```
Protocol ID:   /elohim/epr/1.0.0     (libp2p request-response)
ALPN:          /elohim/epr/2.0.0     (iroh; same messages)
Codec:         4-byte big-endian length + MessagePack payload
Max request:   1 MB on libp2p
Max response:  64 KB on libp2p (a head is ~500 bytes; a batch of 100 is ~50 KB)
Max frame:     16 MiB in each direction on iroh
```

Both transports frame the messages the same way; the reference implementation encodes them positionally on libp2p and with field names on iroh (Appendix C). A receiver refuses a response larger than its limit rather than truncating it.

### Request Messages

```rust
pub enum EprRequest {
    /// Resolve the current EPR Head for a content ID. `agent_pubkey`
    /// identifies the requester so the serving peer can apply the reach gate.
    Resolve {
        id: String,
        agent_pubkey: Option<String>,
    },

    /// Announce a new or updated head (MessagePack-encoded EPR Head)
    Announce {
        head: Vec<u8>,
    },

    /// Resolve several heads in one request (for composite content).
    /// Carries no requester identity.
    ResolveBatch {
        ids: Vec<String>,
    },

    /// Get the Tier 2 EPR Document
    GetDocument {
        id: String,
    },

    /// Ask whether a peer can serve a blob, and from which cache tier
    QueryDelivery {
        blob_hash: String,
    },
}
```

### Response Messages

```rust
pub enum EprResponse {
    /// Single EPR Head (MessagePack-encoded)
    Head(Vec<u8>),

    /// One entry per requested ID; an empty entry for an ID not found
    HeadBatch(Vec<Vec<u8>>),

    /// Announcement acknowledgment
    Announced {
        accepted: bool,
        reason: Option<String>,
    },

    /// Content not found
    NotFound,

    /// Access denied: the reach gate failed
    AccessDenied {
        required_reach: String,
        reason: String,
    },

    /// Error
    Error(String),

    /// Delivery capability for a specific blob
    DeliveryInfo {
        serves_extracted: bool,
        serves_compressed: bool,
        cache_tier: String,   // "projection" | "extraction" | "blob-only"
        warm: bool,           // this blob is extracted and ready
    },
}
```

`QueryDelivery` needs no reach authorization, because it asks whether a peer can serve a blob rather than asking for the blob. Its answer says in which forms the peer can serve it: `serves_extracted` means the blob's unpacked form, such as the files inside a packaged `html5-app`, and `serves_compressed` means the blob as stored. `cache_tier` says where the peer would serve it from: `projection` (a doorway's projection cache), `extraction` (a disk cache of rendered or unpacked content, with `warm` set when this blob is already in it) or `blob-only` (raw bytes). A peer answers `GetDocument` with `Error`.

### The Reach Gate

A serving peer checks the requester against a head's reach before it serves the head or its bytes.

The requester is the agent named by `agent_pubkey`, a Holochain agent public key. The serving peer maps the requester's key to the person it belongs to, maps the head's `author` to a person, and maps each steward to a person, a ContributorPresence through the person who has claimed it. An unclaimed ContributorPresence maps to no one, so it admits no one at the levels that depend on stewards. A request that names no requester, or whose key maps to no person the serving peer knows, passes only for `public` and `commons`.

The reference implementation applies the gate as follows (informative):

| Head's reach | Who passes |
|--------------|------------|
| `public`, `commons` | Anyone; no identity is needed |
| `community` | A person with a consented membership in any collective |
| `familiar` | A person with a consented membership in a collective that one of the content's stewards also belongs to |
| `trusted` | A person in a relationship with one of the stewards at intimacy `trusted` or closer |
| `intimate` | A person in an intimate relationship with one of the stewards, to which both have consented |
| `self`, `private` | The content's author, as named by the head's `author`; no one, if the head names no author |

After the reach check, the requester must meet each gate in the head's `qahal.attestationRequirements`. A gate is written `type:reference`. The type this specification uses is `prerequisite-mastery:{contentId}`, met when the requester has a recorded mastery of that content beyond `not_started`; the serving peer derives these gates from the content's `PREREQUISITE` relationships. A serving peer also refuses content that a policy governing the requester excludes, such as a reach ceiling a guardian has set. Every refusal is answered with `AccessDenied { required_reach, reason }`.

### Resolution Flow

A peer wants to read the content with ID `manifesto-economic-architecture`. Resolution goes from the head to the bytes.

```
0. CONSTITUTIONAL VERIFICATION (prerequisite)
   Before serving any EPR resolution, a peer verifies its own constitutional stack:
   - Load its constitutional documents, from individual to global
   - Check each document's SHA-256 hash against that document's notarized
     entry on the Holochain DHT
   - If any document fails verification: refuse to serve
   - If every document verifies: proceed with resolution

1. DISCOVERY
   Peer → Kademlia: GET key="epr:manifesto-economic-architecture"
   Kademlia → Peer: EPR Head (Tier 1, ~500 bytes)
   or
   Peer → a resolving peer: EprRequest::Resolve { id, agent_pubkey }

2. AUTHORIZATION
   The resolving peer applies the reach gate before it answers Resolve.
   A refused request receives AccessDenied { required_reach, reason }.

3. CONTENT RETRIEVAL (if authorized)
   Peer → a delivering peer (a steward, or any peer holding the blob):
          ShardRequest::Get { hash: head.content }
   Peer receives: Content Bytes (Tier 3)
   Peer verifies: the SHA-256 digest of the bytes matches head.content

4. RECOGNITION (on successful delivery)
   Delivering peer logs: EconomicEvent { eventType: "content_delivery", ... }
   Recognition reaches the stewards in proportion to their allocations
```

The resolving peer, which answers `Resolve`, and the delivering peer, which sends the bytes, can be different peers.

A peer's **constitutional stack** is the set of constitutional documents that apply to the agent the peer acts for, from the agent's own individual constitution through those of the household, communities, province, nation and bioregion the agent belongs to, up to the global layer. The peer's elohim read the stack as their [constitution](constitution.md), so step 0 makes sure that the peer and its elohim do not act under a tampered or outdated copy. The stack is separate from the constitution that governs a piece of content, which a head identifies only by its `layer` and an EPR Document by its `constitution` field.

### Steward Offline Fallback

```
Tier 1 (EPR Heads):
  Stored in Kademlia across all participating peers.
  Available as long as any peer in the network is online.
  TTL: republished every 24 hours by stewards.
  Stale detection: if updated more than 30 days ago and no steward seen,
  mark as "dormant".

Tier 2 (EPR Documents):
  Designed to be cached by peers with affinity (learners on a path,
  community members, etc.), so that a document survives the downtime of
  individual stewards. A cached document's embedded head is stale once
  it is no longer the current head for its id.

Tier 3 (Content Bytes):
  Every blob has a shard manifest. A blob of up to 16 MB is one shard;
  up to 64 MB, a sequence of chunks; above 64 MB, Reed-Solomon coded as
  4 data and 3 parity shards, recoverable from any 4 of the 7.
  Doorways keep projection caches of frequently requested content.
  Content-addressed bytes never change, so any peer that ever cached the
  blob can serve it.
```

---

## Part V: Feed Protocol, `/elohim/feed/1.0.0`

> "Free speech does not mean free reach. There is no right to algorithmic amplification."
> (A formulation associated with Renée DiResta and Aza Raskin, quoted in the [manifesto](manifesto.md).)

Feeds in the Elohim Protocol are curated, not algorithmic. Content reaches a reader through channels that communities and stewards maintain, and no ranking optimizes for engagement.

### Protocol Definition

```
Protocol ID:  "/elohim/feed/1.0.0"
Codec:        FeedCodec (4-byte big-endian length + MessagePack payload)
Max Request:  64 KB
Max Response: 1 MB (a feed page carries many EPR Heads, never full content)
```

### Feed Types

| Feed type | Curator | What it contains | Example |
|-----------|---------|------------------|---------|
| `path` | Path stewards | EPR Heads of the steps in a learning path, in order | The `elohim-protocol` path as a homepage feed |
| `steward` | A steward's portfolio | EPR Heads of content they steward | Follow a creator to see their work |
| `community` | Qahal governance | EPR Heads curated by community consensus | A church community's learning feed |
| `layer` | Constitutional layer | EPR Heads at a governance layer | All community-level governance decisions |

A feed serves only heads its requester may see under the reach gate (Part IV).

### Request Messages

```rust
pub enum FeedRequest {
    /// Subscribe to a feed (push updates)
    Subscribe {
        feed_type: String,              // "path" | "steward" | "community" | "layer"
        feed_id: String,                // path ID, steward DID, community ID, layer name
        since: Option<u64>,             // Unix ms: only updates after this time
        filters: FeedFilters,
    },

    /// Unsubscribe from a feed
    Unsubscribe {
        subscription_id: String,
    },

    /// Pull a page of feed content (for catch-up or non-subscribers)
    GetPage {
        feed_type: String,
        feed_id: String,
        offset: u64,
        limit: u64,
        filters: FeedFilters,
    },
}

pub struct FeedFilters {
    pub content_types: Vec<String>,     // filter by content type
    pub min_reach: Option<String>,      // most restrictive reach to include
    pub max_reach: Option<String>,      // most open reach to include
    pub tags: Vec<String>,              // topic filters
}
```

The reach filters only narrow what a requester asks for; the reach gate decides what the requester receives.

### Response Messages

```rust
pub enum FeedResponse {
    /// Subscription confirmed
    Subscribed {
        subscription_id: String,
        feed_type: String,
        feed_id: String,
    },

    /// Unsubscribed
    Unsubscribed,

    /// Feed update (pushed to subscribers)
    Update {
        subscription_id: String,
        entries: Vec<Vec<u8>>,          // MessagePack-encoded EPR Heads
        sequence: u64,                  // monotonic, for ordering
    },

    /// Feed page (response to GetPage)
    Page {
        entries: Vec<Vec<u8>>,          // MessagePack-encoded EPR Heads
        total: u64,
        has_more: bool,
    },

    /// Error
    Error(String),
}
```

### Push and Pull

Push: subscribers receive a `FeedResponse::Update` whenever the feed changes. When a path steward adds a chapter, every subscriber to that path's feed receives the new step's EPR Head; when a community ratifies new content, every subscriber to that community's feed receives the update.

Pull: any peer can send `FeedRequest::GetPage` without subscribing. This serves catch-up after time offline, browsing a feed before subscribing, and rendering feed previews in composite content.

### No Engagement Optimization

Feed ordering is determined by the feed type's natural order:

- Path feeds: step order (the learning sequence the steward designed)
- Steward feeds: chronological (most recent first)
- Community feeds: governance-determined order (community consensus on what surfaces)
- Layer feeds: chronological with severity weighting (disputed items surface)

A feed never orders its entries by recognition, by a count of deliveries or by any other measure of attention. No field in these messages carries such a measure.

---

## Security and Privacy Considerations

These points collect what the rest of this specification already requires. A full threat model is deferred to a companion specification.

- Reach at serving. A serving peer applies the reach gate before it serves a head or its bytes, and answers `AccessDenied` when the gate fails (Part IV).
- Tier 2 sensitivity. EPR Documents hold the full stewardship, economic and governance context; they are too sensitive to publish on the Holochain DHT or in Kademlia and are retrieved directly from known peers (Part II).
- Constitutional verification. Before it serves, a peer checks the SHA-256 hash of each constitutional document it has loaded against that document's notarized entry, and refuses to serve if any check fails (Part IV, step 0).

---

## Open Issues

These are the places where the design is not complete.

- Requester identity on batch and feed requests. `ResolveBatch` and the feed messages carry no requester identity, so the reach gate has no requester to check for heads narrower than `public`.
- Requester authentication. `agent_pubkey` is a claim. Nothing in version 1.0.0 binds it to the connection's authenticated peer identity.
- The gate for Kademlia. Every head may be published to Kademlia, and a Kademlia record, readable by any peer, cannot apply the reach gate, so where the gate sits for heads served from Kademlia is open.
- The gate for bytes, and the delivery event. `ShardRequest::Get` names only a hash, with neither a requester nor the EPR it serves. A Bitswap request also names no requester, and one blob can sit behind several EPRs with different stewards. How to gate bytes that sit behind an EPR narrower than `public` is therefore open. Bytes fetched by address over HTTP, from an IPFS node or over Bitswap pass outside EPR resolution. Rule 3 needs a byte request that names the EPR, and a witness to the event other than the delivering peer, so that a peer cannot log deliveries it never made. Whether the delivering peer shares in the recognition is left to the Shefa economic protocol.
- Evidence for the gate. The gate reads collective memberships, relationships between people, mastery records and the records that bind an agent key to a person and their DID; this specification defines none of them. The `Attestation` record binds a person to a content CID, which changes when the content is revised, while gates name a content id; the fields that would let an attestation meet a gate are not specified, nor whether one an elohim issues can meet a gate that asks for a person's witness.
- Private content on the Holochain DHT. The lamad DNA's content entry has no visibility attribute, so the title, reach and content address of private content, and its body when stored inline, are readable by any participant in that DNA's network.
- Who declares, and what a declaration fixes. How the content record that a declaration names relates to the content EPR's atom is not specified, nor who may make a canonical declaration that chooses among independent chains for one `id`. A declaration names a version of the content record but not the stewardship, governance or graph records that a head's `shefa`, `layer`, `relationships` and `attestationRequirements` are derived from, so two peers can derive different heads from one declaration.
- Granting reach. A serving peer reads reach from the declared record and cannot tell a reach granted by standing from one its author assigned without the check (Part III, Rule 2). Whether a community can narrow the reach of content it governs is not specified.
- Which community. A head carries no community identifier: `reach: community` admits a member of any collective, and a head with `layer: community` does not say which community's constitution governs it. Only the EPR Document's `constitution.contextId` names it. This specification also does not define what distinguishes `self` from `private`, or `commons` from `public`, beyond the standing a policy may require to grant each.
- Document resolution. No 1.0.0 message delivers an EPR Document. Still to specify: the document response and its access rule; how `policyChain` restricts access to Tier 3; bounds or paging for `economicEvents`, which grows by one event per delivery; an invalidation rule for accumulating fields such as `recognition`; where per-learner state such as `bloomLevel` belongs, given that many peers share and cache one document; and what the `supported` agent role in an economic event means.
- `QueryDelivery` disclosure. `QueryDelivery` needs no authorization, so anyone who knows a hash can learn whether a peer holds that blob warm in its extraction cache, including a blob behind non-public EPRs.
- Evidence of constitutional verification. A peer verifies its own stack, but no 1.0.0 message carries evidence of the check to a requester. How a peer establishes the household, communities and places its stack draws on is not specified, nor whether it must hold the constitution that governs a piece of content before serving it.
- Checking a received head directly. A head travels as MessagePack, and its Holochain declaration names a version of the content record rather than the head's CID. Carrying the head's DAG-CBOR bytes, with a CID that the declaration records, would let a receiver check a head without re-deriving it.
- MessagePack and absent fields. The reference implementation encodes a head in MessagePack as a positional array (Appendix C) and leaves absent optional fields and empty lists out of it, as the DAG-CBOR form does. That moves every later field to an earlier position, so a decoder cannot always tell which field a value belongs to. How absent head fields are encoded in MessagePack is not specified.
- HTTP projections. `GET /db/content/{id}` returns a flat content record, with the content address as `blobCid` and possibly an inline body, not a head. `GET /epr-head/{id}` returns a head-shaped projection derived from the declared content record alone, so its stewardship, relationships and attestation requirements can be empty and it is not the head a peer serves over `/elohim/epr`. Content whose body is stored inline has no blob, so its head has no address to carry in `content`. The alignment of the HTTP projections with the head is open.
- Batch size. `ResolveBatch` sets no cap on `ids`, and a response over the size limit is refused rather than truncated.
- Feed push. A request-response exchange returns one response per request, and the mechanism that delivers an unsolicited `FeedResponse::Update` to a subscriber is not specified.
- Identifiers. How `id` slugs are allocated and kept unique across communities is not specified. Converting a DID to an `epr:` URI drops the host, so the conversion is lossless only while ids are unique across hosts. A client cannot tell from `epr:{id}` alone whether it names content or a path. The shape of a path record, and whether step positions start at 0 or 1, are left to the lamad domain.
- Steward identity and the system issuer. A hosted steward's DID names a doorway's host (`did:web:{host}:humans:{id}`), while doorways are replaceable; keeping that identity when the steward moves is the Identity Portability Protocol's concern (Scope). Who operates the protocol system issuer, `did:web:elohim-protocol.org:system`, what it may issue, and where recognition credited to it goes (for example from deliveries of the landing page in Appendix D), are not specified here.
- Dormant heads. What a peer does differently with a head marked dormant (Part IV) is not specified.
- Reserved. `epr:{id}@{version}` is reserved until content versions are defined (Appendix E.1). The protocol ID `/elohim/cluster/1.0.0` is reserved.

---

## Appendix A: Enumeration Reference

This appendix is informative. Where the protocol schema (`elohim/sdk/schemas/v1/` in the Elohim Protocol repository) or the lamad domain manifest (`elohim/sdk/domains/lamad/`) declares a vocabulary, that declaration is authoritative, and the lists below mirror it.

### ContentType

Core types, notarized on the Holochain DHT with all three dimensions bound (the atom's knowledge, value and governance coupling references):

```
epic | concept | lesson | scenario | assessment | reflection | discussion |
exercise | article | path
```

Entity references: content records that stand for a person, a role or a collective, whose own records are notarized in their own DNAs:

```
human | role | collective
```

Communities register further types (for example `quiz`, `simulation`, `bible-verse`, `feature`, `course-module`) without a protocol change; storage accepts them and the DNA does not validate them.

### Reach (from most restrictive to most open)

```
private | self | intimate | trusted | familiar | community | public | commons
```

Part IV shows who passes the reach gate at each level. For access, `private` and `self` behave alike, as do `public` and `commons`; a standing policy can require different standing to grant `public` and `commons` (Part III, Rule 2).

### ConstitutionalLayer (from narrowest to widest)

```
individual | family | community | provincial | nation-state | bioregional | global
```

- `individual`: the person; the most flexible layer
- `family`: household norms
- `community`: a community's local values and membership rules
- `provincial`: a province or state
- `nation-state`: a nation's cultural and constitutional expressions
- `bioregional`: ecological limits
- `global`: existential boundaries; the hardest layer to change

A head's `layer` names the layer whose rules govern the content. The rules of every wider layer still apply to it, and where two layers conflict the wider one takes precedence.

### Contribution

```
original_creator | editor | translator | curator | maintainer | inherited
```

### Governance

```
active | disputed | pending_review | superseded
```

### AllocationMethod

```
manual | computed | negotiated
```

### TokenType (REA resource types)

```
care | time | learning | steward | culture | infrastructure | recognition
```

Token types are readings of REA economic events by resource type. They are not separate currencies.

### PolicySource

```
org | guardian | elohim | subject
```

### NodeTier (hardware)

```
network | home_node | home_cluster | laptop
```

- `network`: an always-on node that stores content in full, replicates it to the network and serves anyone
- `home_node`: an always-on node that serves its household
- `home_cluster`: several nodes that pool storage for one household
- `laptop`: an intermittently connected device with capped local storage, which does not serve others

These name hardware tiers, not participation stages (Visitor, Hosted, App Steward, Node Steward). A doorway is a replaceable web2 projection of canonical content, not a node tier: it proxies to elohim-storage nodes, and each backing node declares its own `NodeTier`.

### Relationship

The lamad domain manifest is the source for relationship types, and it gives each type a description. Its relationship vocabulary:

```
CONTAINS | BELONGS_TO | DESCRIBES | IMPLEMENTS | VALIDATES | RELATES_TO |
REFERENCES | DEPENDS_ON | REQUIRES | FOLLOWS | ATTACHED_TO | STEP
```

The protocol schema's relationship enum (`elohim/sdk/schemas/v1/enums/relationship-type.schema.json`) declares the same core types in lowercase and adds `derived_from` and `source_of`. The manifest's content-graph edges add `PREREQUISITE`, `TEACHES` and `SUPERSEDES`. elohim-storage also reads `FOLLOWUP`, `SIBLING`, `PARENT`, `CHILD`, `SIMILAR_TO`, `CONTRASTS_WITH`, `ELABORATES`, `SUMMARIZES`, `EXAMPLE_OF` and `DEFINITION_OF` on stored relationships.

The types this specification relies on, in the manifest's terms:

- `DEPENDS_ON`: the source depends on the target for correctness or completeness
- `REQUIRES`: learners complete the target before the source
- `PREREQUISITE`: the target must be mastered before the source can be approached; the edge the prerequisite-mastery gate reads (Part IV)
- `TEACHES`: the source teaches the target concept
- `STEP`: the source path's step at a given position is the target, with the position carried as `orderIndex`

---

## Appendix B: DID Methods

The Elohim Protocol uses two DID methods: `did:web`, in several roles, and `did:key`. `{host}` is the domain of the deployment that issues the identifier.

| Method | Usage | Example |
|--------|-------|---------|
| `did:web` | Doorway identity | `did:web:doorway.example.org` |
| `did:web` | Hosted participant (a doorway holds their keys) | `did:web:{host}:humans:{humanId}` |
| `did:web` | Session identity (ephemeral) | `did:web:{host}:session:{sessionId}` |
| `did:web` | Content identity | `did:web:{host}:content:{contentId}` |
| `did:web` | Learning path identity | `did:web:{host}:paths:{pathId}` |
| `did:web` | Agent identity, such as an elohim | `did:web:{host}:agents:{agentId}` |
| `did:web` | Protocol system issuer | `did:web:elohim-protocol.org:system` |
| `did:web` | Steward issuer (for W3C Verifiable Credentials) | `did:web:elohim-protocol.org:stewards:{id}` |
| `did:key` | Steward identity (local keypair) | `did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK` |

DID Documents are served at `/.well-known/did.json` with the context:

```json
[
  "https://www.w3.org/ns/did/v1",
  "https://w3id.org/security/suites/ed25519-2020/v1",
  "https://elohim-protocol.org/ns/v1"
]
```

The `elohim-protocol.org/ns/v1` context defines these extensions:

- `elohim:capabilities`: node capabilities
- `elohim:region`: geographic region
- `elohim:holochainCellId`: Holochain cell identifier

A doorway also publishes its Ed25519 signing key as a JWKS at `/.well-known/doorway-keys`.

---

## Appendix C: Wire Format Reference

The request-response protocols in this specification share one framing: a 4-byte big-endian length followed by a MessagePack payload.

```
┌─────────────┬──────────────────────────────────────┐
│ Length (4B) │ MessagePack Payload                  │
│ big-endian  │ rmp_serde serialized enum variant    │
│ u32         │                                      │
└─────────────┴──────────────────────────────────────┘
```

### MessagePack mapping

The payload is the Rust definition of the message (Parts IV and V). The reference implementation, elohim-storage, encodes it with the `rmp_serde` library, version 1:

- On libp2p (`/elohim/epr/1.0.0`), messages use the library's default mapping. A struct is an array of its field values in the order the definition lists them, and field names are not encoded. An enum variant that carries data is a map with one entry, from the variant's name as a string to its data: a struct variant's fields as an array, or the single value of a one-value variant such as `Head` or `Error`. A variant without data, such as `NotFound`, is its name as a string. An absent `Option` is nil, and a `Vec<u8>` is an array of integers, one per byte, not a MessagePack `bin`.
- On iroh (`/elohim/epr/2.0.0`), messages use the library's named mapping: the same, except that a struct, including a struct variant's fields, is a map from field name to value.
- The heads it places in `Head` and `HeadBatch` responses and in Kademlia records are encoded with the default positional mapping on both transports, with their fields in the order of Part II, and are carried as bytes within the message.

For example, on libp2p `EprRequest::Resolve { id: "x", agent_pubkey: None }` is the map `{"Resolve": ["x", nil]}`, 13 bytes, framed as:

```
00 00 00 0d    81      a7 52 65 73 6f 6c 76 65    92         a1 78    c0
length 13      map(1)  "Resolve"                  array(2)   "x"      nil
```

Protocol IDs this specification refers to:

```
libp2p                       iroh ALPN              Purpose
/elohim/shard/1.0.0          /elohim/shard/2.0.0    Blob shard transfer
/elohim/storage-sync/1.0.0   /elohim/sync/2.0.0     Automerge document sync (CRDTs,
                                                    which merge concurrent edits)
/elohim/epr/1.0.0            /elohim/epr/2.0.0      EPR resolution (Part IV)
/elohim/feed/1.0.0                                  Feed subscription (Part V)
/elohim/id/1.0.0                                    Peer identification (libp2p identify)
/elohim/cluster/1.0.0                               Cluster coordination (reserved)
```

---

## Appendix D: Worked Example: A Landing Page Built from EPRs

A front page maintained outside the content graph cannot reflect what is in it. It goes stale as paths are added and stewards change, and it carries none of the stewardship or governance that the content behind it carries. This example builds a landing page as content.

A composite is an ordinary content record whose `contentFormat` is `epr-composite`: its body is a layout of references to other EPRs. Because it is content, it has stewards and governance, and serving it triggers recognition like anything else. The example uses the content type `collective`, as the protocol's own landing page does, though Appendix A describes `collective` as an entity reference. It names the protocol system issuer (Appendix B) as its steward.

```
EPR Head {
  version:   1
  id:        "elohim-protocol-home"
  content:   <CID of the composite body>
  lamad:     { title: "Elohim Protocol", contentType: "collective",
               contentFormat: "epr-composite" }
  shefa:     { stewards: ["did:web:elohim-protocol.org:system"],
               allocations: [1.0] }
  qahal:     { reach: "commons", layer: "global" }   // the front door
}
```

The body follows the `epr-composite` body schema in the lamad domain manifest: sections of items, each item a reference to another EPR.

```json
{
  "schemaVersion": 1,
  "layout": "exploratory",
  "sections": [
    {
      "id": "why",
      "title": "Why this exists",
      "items": [{ "ref": "epr:manifesto" }]
    },
    {
      "id": "paths",
      "title": "Where to begin",
      "items": [
        { "ref": "epr:elohim-protocol" },
        { "ref": "epr:governance-policy-maker" },
        { "ref": "epr:know-thyself-path" }
      ]
    }
  ]
}
```

Rendering it:

1. Resolve: `EprRequest::Resolve { id: "elohim-protocol-home", agent_pubkey }` returns the head.
2. Fetch the body: `ShardRequest::Get { hash: head.content }` returns the composite body.
3. Batch resolve: `EprRequest::ResolveBatch { ids: [...every ref in the body...] }` returns their heads.
4. Render each section's items as preview cards from those heads (title, description, stewards).
5. A click on a card begins a full resolution of that EPR, from its head to its bytes.

Because the cards come from heads, the page stays readable while particular stewards are offline, and it changes as paths are added, stewards change and communities curate new material. Sections driven by a query or by a feed (Part V) would extend the body schema; this specification does not define them.

---

## Appendix E: The `epr:` URI Scheme and Content Addressing

HTML has `href`, IPFS has `ipfs://{cid}` and AT Protocol has `at://{did}/{collection}/{rkey}`. The Elohim Protocol has the `epr:` URI. It names content by a stable id; the knowledge, value and governance context lives in the EPR Head the URI resolves to, not in the URI itself.

### E.1: The `epr:` URI Scheme

```
epr:{id}                                    Resolve EPR Head (Tier 1)
epr:{id}@{version}                          Reserved: a specific version (see below)
epr:{id}/doc                                Resolve EPR Document (Tier 2)
epr:{id}/blob                               Resolve Content Bytes (Tier 3)
epr:{id}#step/{index}                       Fragment: path step position
epr:{id}#chapter/{chapterId}                Fragment: path chapter
epr:{id}#rel/{relationship}/{targetId}      Fragment: graph edge
epr:{id}?via={did-or-peer-id}               Hint: prefer this resolver
epr:{id}?reach={reach-level}                Hint: the access level I expect
epr:human/{id}                              A person (E.4)
epr:agent/{id}                              An agent, such as an elohim (E.4)
```

Syntax (informal ABNF):

```
epr-uri      = "epr:" epr-id [ "@" version ] [ "/" tier ] [ "?" query ] [ "#" fragment ]
epr-id       = [ namespace "/" ] slug
namespace    = "human" / "agent"
slug         = 1*( ALPHA / DIGIT / "-" / "_" / "." )
version      = 1*DIGIT
tier         = "doc" / "blob"
query        = via-param / reach-param / ( via-param "&" reach-param )
via-param    = "via=" ( did / peer-id )
reach-param  = "reach=" reach-level
fragment     = step-frag / chapter-frag / rel-frag
step-frag    = "step/" 1*DIGIT
chapter-frag = "chapter/" slug
rel-frag     = "rel/" relationship-type "/" slug
```

When the text after `epr:` begins with `human/` or `agent/`, that segment is a namespace, so the `/doc` and `/blob` forms cannot address content whose id is `human` or `agent`. The `?reach=` hint grants nothing: the serving peer's gate decides access. The `@{version}` form is reserved. A head's `version` field is the schema version of the record, so `@` has no content version to name until one is defined.

Examples:

```
epr:manifesto-foundations                    The manifesto's foundations
epr:manifesto-foundations/blob               The raw content bytes
epr:manifesto-foundations/doc                Full EPR Document
epr:elohim-protocol#step/2                   Step 2 of the protocol learning path
epr:elohim-protocol#chapter/economic         The "economic" chapter of that path
epr:systems-thinking#rel/PREREQUISITE/feedback-loops
                                             The prerequisite edge from systems-thinking
                                             to feedback-loops
epr:manifesto-foundations?via=did:web:doorway.example.org
                                             Prefer resolving through that doorway
```

Why `epr:` and not `elohim://`: the desktop app already uses `elohim://` for deep links (OAuth callbacks on peer-to-peer native devices). `epr:` is short, unambiguous and matches the EPR name.

On the name. EPR expands two ways, on purpose: Elohim Protocol Reference and Elohim Protocol Record. It began with HTML's `href` and two questions: what a reference would look like if it had to carry value and governance, and earn its reach, before it could resolve; and how to address content that any number of peers might host, none of them its fixed home. With no fixed home there is no location to point at, so the reference points at the content itself, through a content address that resolves to the same bytes whichever peer serves them. That makes it the Reference, `href`'s accountable, location-independent cousin, and the sense this specification uses. Once authority over it is notarized on the Holochain DHT, it is evidence that stands on its own: a Record, the sense the [Records Lifecycle design](architecture/2026-05-24-records-lifecycle-design.md) (§A.1) uses. Reference → Record is one atom seen at compose time and at rest; the missing letter is the whole arc.

### E.2: The Four-Layer Hierarchy

Every content reference passes through four layers. Transport is not part of the URI: the connection strategy resolves it at runtime, which keeps references portable across every deployment mode.

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 4: GRAPH POSITION (fragment: how it connects)             │
│   #step/3    #chapter/economic    #rel/PREREQUISITE/feedback    │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: PROTOCOL CONTEXT (carried by the EPR Head)             │
│   lamad: title, contentType, contentFormat, description, tags   │
│   shefa: stewards, allocations                                  │
│   qahal: reach, layer, attestationRequirements                  │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: RESOLUTION (query hint: where to find it)              │
│   ?via=did:web:doorway.example.org   (prefer this doorway)      │
│   ?via=12D3KooW...                   (prefer this peer)         │
│   (omitted)                          (use Kademlia or a default)│
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: TRANSPORT (implicit: resolved at runtime)              │
│   web:     HTTPS via a doorway (browsers, hosted participants)  │
│   native:  HTTP to the local elohim-storage node                │
│   p2p:     libp2p or iroh request-response (device to device)   │
└─────────────────────────────────────────────────────────────────┘

         epr:systems-thinking#rel/PREREQUISITE/feedback-loops
              ───────┬───────  ──────────┬──────────────────
                   id           graph position (Layer 4)

    Protocol context (Layer 3) is not in the URI. It is in the EPR Head
    that the URI resolves to: the URI is the key, the head is the value.
```

### E.3: Resolution Matrix: `epr:` to Transport-Specific Operations

Given an `epr:` reference, the connection strategy resolves it to a transport-specific operation. Here:

- `{doorway}` is a doorway's base URL (for example `https://doorway.example.org`)
- `{storage}` is an elohim-storage node's base URL (on a peer-to-peer native device, the local node)

Over HTTP the routes return projections: views of a record shaped for web clients, which a doorway or storage node derives from the records it holds.

#### Tier 1: EPR Head (metadata, stewardship, governance)

| EPR URI | Web (doorway) | Peer-to-peer native device | Peer to peer | App route |
|---------|---------------|----------------------------|--------------|-----------|
| `epr:{id}` | `GET {doorway}/epr-head/{id}` | `GET {storage}/epr-head/{id}` | `EprRequest::Resolve { id, agent_pubkey }` | `/lamad/resource/{id}` |
| `epr:{id}` (path) | `GET {doorway}/db/paths/{id}` | `GET {storage}/db/paths/{id}` | `EprRequest::Resolve { id, agent_pubkey }` | `/lamad/path/{id}` |

The head route returns a projection in the head's shape: DAG-CBOR when the client asks for `application/vnd.ipld.dag-cbor`, and JSON otherwise. Web clients can also read the content projection, `GET {doorway}/db/content/{id}` (the same route serves against `{storage}`). The content and path routes return the record as a flat object, with the content address as `blobCid` (E.6); they are not heads.

#### Tier 2: EPR Document (full pillar context)

| EPR URI | Web (doorway) | Peer-to-peer native device | Peer to peer |
|---------|---------------|----------------------------|--------------|
| `epr:{id}/doc` | No Document route | No Document route | `EprRequest::GetDocument { id }` |

Over HTTP a client can compose part of the Tier 2 context, such as the content's relationships, from `GET {doorway}/db/content/{id}` and `GET {doorway}/db/relationships/graph/{id}`. The result is not an EPR Document.

#### Tier 3: Content Bytes

| EPR URI | Web (doorway) | Peer-to-peer native device | Peer to peer |
|---------|---------------|----------------------------|--------------|
| `epr:{id}/blob` | Two steps: resolve `epr:{id}`, read its content address, then `GET {doorway}/blob/{hash}` | Two steps: resolve, then `GET {storage}/blob/{hash}` | `ShardRequest::Get { hash }` |
| (by address directly) | `GET {doorway}/blob/{hash}` | `GET {storage}/blob/{hash}` | `ShardRequest::Get { hash }` |

The canonical blob route is `/blob/{hash}` on both doorways and elohim-storage nodes. On a doorway, the request passes through the projection and caching layer (HTTP Range, ETag, CDN-friendly caching, shard-resolution fallback) and is routed to a storage node through the doorway's registry. On a peer-to-peer native device, the local elohim-storage node answers directly on the same path. `POST /api/blob/verify` is a separate verification endpoint.

#### Path Steps (fragment resolution)

| EPR URI | Resolution |
|---------|------------|
| `epr:{pathId}#step/{n}` | Resolve `epr:{pathId}` as a path, take its step at position `n` (the target of its `STEP` relationship with that `orderIndex`; Appendix A), then resolve that step's content EPR |
| App route | `/lamad/path/{pathId}/step/{n}` |

Fragments are resolved on the client after the parent EPR resolves; the fragment is never sent to the server.

#### Batch Resolution (for composite content)

| EPR URI | Web or direct | Peer to peer |
|---------|---------------|--------------|
| Several `epr:{id}` references in a composite body | Parallel `GET /epr-head/{id}` calls | `EprRequest::ResolveBatch { ids: [...] }` |

### E.4: DID and EPR Identifiers

Two identifier systems coexist. DIDs serve W3C interoperability (Verifiable Credentials, or VCs; federation discovery; DID document resolution). EPR URIs serve protocol operations (content resolution, feed subscriptions, graph traversal, links in the interface).

Conversion rules:

| DID | EPR URI | Use the DID for | Use the EPR for |
|-----|---------|-----------------|-----------------|
| `did:web:{host}:content:{id}` | `epr:{id}` | VCs, federation, external references | Resolution, feeds, interface |
| `did:web:{host}:paths:{id}` | `epr:{id}` | VCs about path completion | Resolution, interface |
| `did:web:{host}:humans:{id}` | `epr:human/{id}` | Identity, authentication, VCs | Steward feed subscriptions |
| `did:web:{host}:agents:{id}` | `epr:agent/{id}` | elohim identity | Agent-related queries |

Examples:

```
did:web:doorway.example.org:content:manifesto-foundations  →  epr:manifesto-foundations
did:web:doorway.example.org:paths:elohim-protocol          →  epr:elohim-protocol
did:web:doorway.example.org:humans:alice                   →  epr:human/alice
```

Converting a DID to an EPR URI drops the host. The EPR URI names no host and resolves through whatever transport is available, so `epr:manifesto-foundations` works the same on a doorway, on a peer-to-peer native device and on a headless storage node. Converting back needs a host, which the resolver supplies, and the round trip is lossless only while ids are unique across hosts.

### E.5: Content-Address Canonicalization

In this section and in Appendix G, the capitalised words MUST, SHOULD and MAY carry their RFC 2119 meanings.

The canonical content address of a blob is a CIDv1 (Appendix F.1):

```
CANONICAL:  bafkreibm6jg3ux5qumhcn2b3flc3tyu6dmlb4xa7u5bf44mcplnzjhclme
            CIDv1 · raw codec 0x55 · SHA-256 multihash · base32lower
```

All components MUST produce a CIDv1 when they generate a blob reference. All components MUST accept every form below on input. Every form names the same SHA-256 digest, so normalizing changes the representation and never re-hashes.

| Format | Example | Status |
|--------|---------|--------|
| `bafkrei...` | `bafkreibm6jg3...` | Canonical. Produce and accept. |
| `sha256-{hex}` | `sha256-a7ffc6f8...` | Legacy. Accept, never produce. |
| Raw hex | `a7ffc6f8bf1ed766...` | Legacy: 64 lowercase hex characters. Accept, never produce. |
| `sha256:{hex}` | `sha256:a7ffc6f8...` | Deprecated: appears in older content bodies. Accept, never produce. |

Normalization (pseudo-code for any component that accepts a content address):

```
fn normalize(input: &str) -> Result<Cid> {
    if let Ok(cid) = Cid::parse(input) { return Ok(cid) }     // canonical
    let hex = input.strip_prefix("sha256-")                   // legacy
        .or_else(|| input.strip_prefix("sha256:"))            // deprecated
        .unwrap_or(input);                                    // raw hex
    if hex.len() == 64 && is_hex(hex) {
        let digest = hex_decode(hex);
        return Ok(Cid::v1(RAW_0x55, Multihash::wrap(SHA2_256_0x12, digest)))
    }
    Err("unrecognized content address format")
}
```

### E.6: Two-Step Content Resolution (Metadata, then Blob)

Most content retrieval takes two steps: learn the blob's content address, then fetch the blob by that address. Over the peer-to-peer protocol the address is the head's `content` field (Part IV). Over HTTP a client reads it from the head route, or from the content projection, which carries it as `blobCid`:

```
Step 1: Read the content projection (or the head, GET /epr-head/{id})
  epr:manifesto-foundations
    → GET /db/content/manifesto-foundations
    → {
        id: "manifesto-foundations",
        blobCid: "bafkreibm6jg3ux5qumhcn2b3flc3tyu6dmlb4xa7u5bf44mcplnzjhclme",
        contentType: "article",
        contentFormat: "markdown",
        title: "Foundations of the Elohim Protocol",
        ...
      }

Step 2: Fetch the blob (if the content body is a blob reference)
    → GET /blob/bafkreibm6jg3...   (through a doorway with CDN and Range support,
                                    or directly from a storage node)
    → ShardRequest::Get { hash: "bafkreibm6jg3..." }   (peer to peer)
    → raw bytes (markdown text, image data, video, etc.)

Step 3: Verify integrity
  The SHA-256 digest of the received bytes matches the content address.
  On a mismatch: discard, and try another peer or doorway.
```

In the content projection, Step 2 is needed when the content body is a blob reference, meaning it starts with `bafk`, `sha256-` or `sha256:`, or when the body is empty and the record carries `blobCid`. Otherwise the body is inline, stored directly in the record, and has no blob. The raw-hex form, which E.5 accepts wherever a field is known to hold an address, is not read as a blob reference inside a body.

```
content.contentBody
  ├── starts with "bafk", "sha256-" or "sha256:" → BLOB: fetch from /blob/{hash}
  ├── empty, and the record carries blobCid       → BLOB: fetch blobCid
  ├── starts with "{" or "["                      → INLINE JSON: parse directly
  │                                                  (sophia-quiz-json, html5-app config)
  └── otherwise                                   → INLINE TEXT: render directly
                                                     (short markdown, descriptions)
```

### E.7: Delivery Mode Decision Tree

Which operation to use, by the kind of device doing the retrieving:

```
Am I a peer-to-peer native device? (own conductor, own storage, own keys)
  ├── YES: I talk to my local elohim-storage node over HTTP,
  │     │  and to the network over libp2p or iroh. Both paths are mine.
  │     │
  │     │  Local storage node (HTTP):
  │     │     Heads:    GET {storage}/epr-head/{id}
  │     │     Content:  GET {storage}/db/content/{id}   (projection)
  │     │     Blobs:    GET {storage}/blob/{hash}
  │     │     Paths:    GET {storage}/db/paths/{id}
  │     │
  │     │  Network (peer to peer):
  │     │     Heads:    EprRequest::Resolve { id, agent_pubkey }
  │     │     Blobs:    ShardRequest::Get { hash }
  │     │     Feeds:    FeedRequest::Subscribe { ... }   (Part V)
  │     │     Sync:     Automerge sync protocol
  │     │
  │     └── My local storage node may also fetch, over the network,
  │         content I don't have yet. The two paths compose.
  │
  └── NO: I'm a browser, and a doorway serves me.
        Heads:    GET {doorway}/epr-head/{id}
        Content:  GET {doorway}/db/content/{id}      (projection)
        Blobs:    GET {doorway}/blob/{hash}    (CDN-cached, Range-capable,
                                                registry-routed to a storage node)
        Paths:    GET {doorway}/db/paths/{id}
        Graphs:   GET {doorway}/db/relationships/graph/{id}
        WS:       wss://{doorway}/hc/app/{port}?apiKey=...&token=...
```

---

## Appendix F: IPLD Alignment: EPR as an IPLD Extension

The Elohim Protocol composes on IPFS primitives rather than building beside them. EPR Heads are IPLD-compatible documents: any IPLD tool can decode them, and only EPR-aware tools understand the three-pillar semantics.

### F.1: Content Addressing: CIDv1 as Canonical

EPR uses IPFS Content Identifiers (CIDv1) as its canonical content address:

- Codec: raw (0x55); content bytes are opaque, not structured
- Hash: SHA-256 (0x12) via multihash
- Base: base32lower, which produces `bafkrei...` strings
- Example: `bafkreibm6jg3ux5qumhcn2b3flc3tyu6dmlb4xa7u5bf44mcplnzjhclme`

Appendix E.5 lists the legacy forms that are accepted on input and never produced.

### F.2: EPR Head as an IPLD Document (normative)

This is the normative encoding of the head defined in Part II. It is shown here as JSON for reading; the canonical bytes are its DAG-CBOR encoding (multicodec 0x71).

```json
{
  "version": 1,
  "id": "rea-foundations",
  "content": "bafkreibm6jg3...",

  "lamad": {
    "title": "REA Foundations",
    "contentType": "concept",
    "contentFormat": "markdown",
    "description": "Resource-Event-Agent accounting model",
    "tags": ["economics", "rea", "accounting"]
  },

  "shefa": {
    "stewards": ["did:web:doorway.example.org:humans:contributor-1"],
    "allocations": [1.0]
  },

  "qahal": {
    "reach": "commons",
    "layer": "global"
  },

  "relationships": [
    {
      "type": "PREREQUISITE",
      "target": "systems-thinking",
      "targetCid": "bafkrei..."
    },
    {
      "type": "TEACHES",
      "target": "hrea-agent-model"
    }
  ],

  "author": "did:web:doorway.example.org:humans:contributor-1",
  "updated": "2026-02-27T00:00:00Z"
}
```

Links: the head carries its CIDs (`content` and `relationships[].targetCid`) as strings. A generic IPLD tool decodes them as text, not as links; encoding them as native DAG-CBOR links (CBOR tag 42) would let such tools traverse them.

Three-pillar extension: the `lamad`, `shefa` and `qahal` fields are EPR's extension to IPLD. They mean nothing to a generic IPLD tool and carry the three-pillar context for EPR-aware ones.

### F.3: What IPLD Provides and What EPR Adds

| Capability | IPLD primitive | EPR extension |
|------------|----------------|---------------|
| Content addressing | CIDv1 (multihash + multicodec) | Used as it is |
| Immutable links | DAG-CBOR CID links | Typed relationships (PREREQUISITE, TEACHES, etc.) |
| Graph traversal | IPLD Selectors | Three-pillar-aware traversal (planned) |
| Mutable naming | IPNS (name → CID pointer) | EPR Head (name → rich metadata → CID) |
| Block exchange | Bitswap | Shard protocol with stewardship economics (planned) |
| Data model | IPLD Data Model (maps, lists, links) | Three-pillar coupling (lamad, shefa, qahal) |
| Content verification | Hash verification | Hash, plus governance (the reach gate) |

### F.4: Interoperability Guarantee

Any IPFS node can:

1. Store EPR content bytes (they are blobs with CIDs)
2. Pin EPR content (standard IPFS pinning)
3. Exchange EPR blobs via Bitswap (standard block exchange)
4. Decode EPR Heads (standard DAG-CBOR)

Only EPR-aware nodes can:

1. Interpret the three-pillar semantics (lamad, shefa, qahal)
2. Enforce governance (reach levels, constitutional layers)
3. Route stewardship economics (recognition)
4. Resolve context-aware navigation (path-aware link resolution)

If the three-pillar coupling proves itself in use, it becomes a candidate for an IPLD specification extension.

### F.5: Multicodec Codes

EPR defines three application-specific codes in the multicodec private-use range `0x300000–0x3FFFFF`, alongside the standard DAG-CBOR codec it uses to encode heads:

| Code | Name | Description |
|------|------|-------------|
| `0x71` | dag-cbor | Standard IPLD DAG-CBOR codec (used to encode EPR Heads) |
| `0x300001` | epr-head | EPR Head metadata envelope (private use) |
| `0x300002` | epr-document | EPR Document body (private use) |
| `0x300003` | epr-relationship | EPR Relationship edge (private use) |

The private-use range follows the [multicodec specification](https://github.com/multiformats/multicodec). These codes are not registered upstream and mean something only within the Elohim Protocol. If EPR is adopted more widely, they will be submitted to the multiformats multicodec table.

A head's CID uses the standard dag-cbor codec (0x71). The EPR-specific codes are reserved for a later phase of IPLD integration (Appendix G), in which they would identify the semantic type of an encoded document, so an EPR-aware tool could tell an EPR Head from a generic DAG-CBOR document without inspecting the payload.

### F.6: CID Format Convention

A CID's prefix encodes its codec as well as its hash, so the prefix separates structured metadata from raw bytes at a glance:

| Prefix | Codec | Meaning | Example |
|--------|-------|---------|---------|
| `bafyr...` | `0x71` (dag-cbor) | Structured IPLD document (EPR Head, EPR Document) | The EPR Head for "rea-foundations" |
| `bafkrei...` | `0x55` (raw) | Opaque content bytes (markdown, images, video) | The blob for a concept's markdown body |

Both are base32lower CIDv1 over SHA-256. A system that processes EPR references can tell from the prefix alone whether a CID points to traversable structured data or to opaque bytes, without resolving it.

---

## Appendix G: Migration

Migration is additive. Existing content and HTTP routes keep working while EPR resolution is added beside them.

### G.1: Wrapping Existing Content in Heads

1. For each content record, derive an EPR Head from its fields (id, content address, content type and format, description, tags, reach), its stewardship allocations (shefa), its relationships, and the constitutional layer of its governance state (qahal).
2. Declare each content record on the Holochain DHT through the canonical declaration channel, and publish its head to Kademlia under `epr:{id}`.
3. Keep the HTTP content API (`/db/content/{id}`) working. EPR resolution is an additional path, not a replacement.

### G.2: Stored Head Encoding: JSON to DAG-CBOR

Stored heads move from JSON to DAG-CBOR (codec 0x71) in three phases:

1. Phase 1: heads are stored as DAG-CBOR, and readers tell them apart from the older JSON form by the first byte: `0x7B` (ASCII `{`) is JSON, and anything else is DAG-CBOR. All readers MUST support both formats during this phase.
2. Phase 2: all new heads are written only as DAG-CBOR. Readers keep reading JSON so that existing content can be migrated, and new implementations MAY omit JSON writing.
3. Phase 3: the JSON fallback is deprecated and all content is IPLD-native. Implementations MAY drop JSON reading after a migration period set by constitutional governance.

This rule covers heads exchanged or stored as canonical bytes, such as those served by the HTTP head route. A head received in an `/elohim/epr` message, a feed message or a Kademlia record is MessagePack (Part II) and is decoded as MessagePack.

First-byte detection (pseudo-code):

```
fn decode_epr_head(bytes: &[u8]) -> Result<EprHead> {
    match bytes.first() {
        None       => Err("empty input"),
        Some(0x7B) => json::decode(bytes),       // '{': legacy JSON
        Some(_)    => dag_cbor::decode(bytes),   // canonical DAG-CBOR
    }
}
```

### G.3: Content Addresses: `sha256-{hex}` to CIDv1

New code produces CIDv1 and accepts the legacy forms listed in Appendix E.5, so content addressed as `sha256-{hex}` stays reachable while references move to CIDv1.

### G.4: Deeper IPLD Integration

- Submit the EPR multicodec codes (Appendix F.5) to the multiformats multicodec table if EPR is adopted beyond this network
- IPNS for EPR mutable naming
- IPLD Selectors for knowledge-graph traversal
- Trustless blob retrieval in the browser (verified fetch) and Bitswap block exchange
- Propose the three-pillar pattern as an IPLD specification extension

---

## License and Openness

This specification is published as open documentation under the same terms as the Elohim Protocol codebase. All protocol specifications, reference implementations and constitutional documents are publicly auditable, modifiable and community-maintained.

No entity, including the Elohim Protocol organization, holds exclusive rights to implement, extend or restrict this specification. The protocol's anti-capture design applies to the specification itself: it belongs to the commons, stewarded by the community that uses it.

Implementations of this specification should be open source. A proprietary implementation that restricts auditability conflicts with the manifesto's first principle, that no single entity should control the infrastructure of human connection.
