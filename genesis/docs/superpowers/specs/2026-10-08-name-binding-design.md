---
title: "A name is answered by a binding: the NameBinding design gate"
id: name-binding-design
status: Draft
class: architecture
serves: [notary-authority, dataplane-convergence, doorway-failover, authority-in-integrity]
date: 2026-10-08
context-tier: disclosed
steward: agent:integrator@claude-fable-5-1
graduation-trigger: retires to history once a coordinator read answers any name with its elector, clock, head and standing reference beside the CID, two doorways that answer one name differently can be told apart by those fields alone, and row 17's two integrity rules have landed in the authority cluster's DNA crossing
cites:
  - "elohim-protocol-specification | Open Issues: \"Names are bindings\" (the ruling designed for) and \"Reach is a per-plane declaration\" (the scope component) | sha256:d0975dc0f31fab70 | path: genesis/docs/content/elohim-protocol/protocol-specification.md"
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/elohim-storage/src/release_channel.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/stewardship.rs
  - genesis/data/timeline/backlog/plural-mishpat-lenses-binding-key-slug-id-spec-followup.md
  - genesis/orchestrator/data/deployments.json
  - genesis/a2o/features/dataplane/notary-authority.feature
---

# A name is answered by a binding

## Scope

Row 17 of `arch-authority-in-integrity-backlog.md` records the ruling below, guesses that a channel or council binding is Notarized and a petname Attested-Private, names two integrity rules (a binding's author is its elector or carries the elector's standing proof; the election clock is monotone per name and elector), and requires this design gate before any code. This document runs that gate. It changes nothing; items marked "owed" are what a later implementation must supply.

The ruling (protocol specification, Open Issue "Names are bindings"): a slug is a projection of a content-addressed NameBinding, resolved as the bindings the asker trusts in the asker's declared order, varying only along replayable inputs the asker declared or holds. Its six components:

- **name**: the human-facing string looked up (`lamad-spa`).
- **elector**: who declares what the name means: a person's key, a channel, or a council.
- **scope-declaration CID**: the address of a declaration of how far the binding travels, plane by plane.
- **head or CID**: what the name points at.
- **election clock**: the signed time that orders one elector's declarations.
- **standing proof**: evidence the elector may declare for this name.

The incident (2026-10-08): the two alpha doorways answered `lamad-spa` with two CIDs. Doorway-alpha reads storage peer matthew, elohim.host reads storage peer adam. Matthew followed the app-bundle release channel and adopted the new build; adam followed none, and since the channel's adoption vehicle is the only writer of a bound slug's pointer, adam kept the old one. Neither answer named its elector or clock.

## What exists today (verified at source)

**The canonical-head declaration carries four of the six components.** It has no link type of its own. `create_canonical_head_link` (content_store `lib.rs` ~:3054) writes `LinkTypes::IdToContent` from `StringAnchor("canonical_head", id)` to the head action, tagged `canonical-head:staging` (any agent may declare; loses to any earned declaration) or `canonical-head:earned` (counted only if the declarer holds standing), optionally followed by `|delegation:` and a signed `HeadDelegation` (~:3042-3043, :4296). The integrity zome accepts every `CreateLink` unconditionally (`content_store_integrity` ~:4260). Line numbers prefixed ~ are approximate. The progenitor below is the DNA's bootstrap steward key. Mapping:

| Component | Where it rides today |
|---|---|
| name | the anchor's id string |
| elector | `link.author` |
| head | the link target (a Content action) |
| election clock | the link timestamp, or the root author's signed acceptance time for a delegate (`use_accepted_ordering`) |
| standing proof | the tag, checked by `holds_earned_declaration_standing` (~:6882): progenitor, root author, or a delegation the root author accepted |
| scope-declaration CID | absent. `HeadDelegationPayload.scope` is a free `String` |

The elector is read and then dropped: the standing check consumes `link.author` (~:3410, :3437), but `CanonicalCandidate` (~:3132) and `CanonicalElectionOutput` (~:6394-6425) carry no author, and `ContentHeadOutput.author` is the version's author, not the declarer. One read keeps it: `get_canonical_election_evidence` (~:6636) returns the winning declaration link's Record, whose action names its author.

**Standing proof is not a `StewardshipGrant`.** `StewardshipGrant` exists (`stewardship.rs:133`), but the canonical-head check never reads it.

**A release channel is a layer of the same mechanism.** A slug's record names its channel in `metadata_json` under `releaseChannel` (`release_channel.rs:26`). `release-ceremony.ts channel bind` writes it by `update_content` on the slug, then declares that version canonical. A channel id is its own content row whose canonical head is the release manifest. `AppBundleVehicle` (`services/release_adoption/apply.rs` ~:1719) is the only writer of a bound slug's pointer. A bound slug is therefore two bindings in series: slug to channel, channel to manifest.

**The resolution order is declared, in the compute plane.** `ELOHIM_RELEASE_CHANNELS` in `deployments.json`: adam, matthew and jessica follow `runtime:app-bundle:alpha:dev=canary`; james, gertrude, susan and eve follow only the coordinator channel. `runtime-config-render.test.mjs` (~:239-256) derives the required follower set from each doorway manifest's `STORAGE_URL`.

## 1. Local-first reading

**Alone, one device, nothing reachable.** Own petnames resolve in full. Cached public bindings resolve read-only, each checked against its address (the head's CID and the declaration link's signed action) and shown as "cached, freshness unknown", never as current.

**Who holds the key.** Key on the device: the person can author a petname and, where they hold standing, a channel binding (the `CreateLink` commits to their chain and publishes when a peer is reachable). Key held by a doorway: petnames are kept on the device and signing waits; the person is told the binding is not yet saved to their account. The doorway can see and sign every binding it hosts; this document does not describe it as blind. Nobody's key: cached bindings resolve, nothing is kept.

**What arrives.** A followed channel adds fresher bindings from a chosen elector; a doorway adds reach and the bindings it holds; another own device adds the same petnames. None becomes required: without them, petnames and cached bindings still answer. A channel's holders learn that the person asks; a doorway learns every name a hosted person resolves.

## 2. Gradient reading, plane by plane

- **Reach.** The bytes have their own reach; the binding and each reference to it have another. A petname is intimate while pointing at commons bytes: two reaches.
- **Custody.** A channel binding is held by the DHT authorities for its anchor; threshold and independence are the DHT's, not declared per binding. A council binding would need a holder set and threshold of its own; that shape is unbuilt. A petname is held by the person's own devices.
- **Freshness.** A channel binding is current only when read from an election through a conductor, not a served projection. A petname may be served from the device.
- **Linkability.** Following an elector reveals the follow to its holders and to the resolver; a serving node's follow set is public today in `deployments.json`.
- **Cost bearer.** The elector pays: authoring, standing upkeep, carrying declaration links. Askers pay one resolve per name.

## 3. Gate output

### P2P Design Gate: NameBinding

#### Entity: ChannelBinding (an elector's binding of a name: person, channel or council)
- **Classification**: Linked (A2)
- **Justification**: The `CreateLink` on the `canonical_head` anchor is the atom; it has no meaning without the anchor and its target, so row 17's "Notarized" guess is corrected to Linked.
- **Head-Plane Cost Budget**: Adds no heads. Names with an anchor at seed: at most 3,556 (the parseable rows in `genesis/data/lamad/content/`), plus 2 release channel ids (`runtime:coordinators:elohim:workspace`, `runtime:app-bundle:alpha:dev`). Slugs bound to a channel: 2 (`elohim-host-landing`, `lamad-spa`: default `APP_RELEASE_BUNDLES` in `publish-app-release.sh`, matching both doorways' `SSR_BUNDLE_SLUGS`). At 1 year: head count unchanged. Link count grows by one per staged release and one per promotion on each channel; the repository holds no release rate, so no number is given. The recurring cost joined is election work: `get_links` per anchor grows with declarations; pruning belongs to release retention. No bundling shape is needed.
- **Network Stakes**: All four stages. The standing check and the elector's identity are Constitutional (floor-protected: never cheapened at any stage). A refused or contested binding is CounterEvidence and always reaches the elector.
- **Content Address Strategy**: Content-Derived (CID)
- **Address Justification**: The binding's address is the dag-cbor CIDv1 of `{name, elector, scopeCid?, head, clock, standingRef}`, derived from the link and never stored as a second identity. The elector declares the applicable head; the election picks among electors the asker follows. Tiebreak stays where it is: the earned tier first, then clock, then ordering hash or link hash (`select_canonical_winner`). The slug is the lookup key, never the address.
- **Transport Affinity**: n-a (no bytes).
- **Source of Truth**: Holochain DHT (the `CreateLink` on the `canonical_head` anchor).
- **Integrity Zome + DNA-hash class**: `content_store_integrity` (lamad DNA, packed from `dna/elohim/`). The bridge view is DNA-hash-NEUTRAL. Row 17's two rules (the author is the elector or carries the elector's standing; the clock is monotone per name and elector) are DNA-HASH-MOVING and batch into the authority cluster's crossing.
- **Coordinator Zome**: `content_store::declare_canonical_content_head` and `declare_earned_canonical_head` (existing) write the link. A new read `content_store::resolve_name_binding(name)` returns, per candidate, the `CreateLink` ActionHash, the elector, the head ActionHash, the clock, the tier and the standing reference. It is built on what `get_canonical_election_evidence` already reads.
- **Projections**: SQLite: the existing canonical-head projection, extended to keep the elector (dht_anchor_hash: the parent anchor; the link's ActionHash is the binding's locator). Automerge sync: n-a for Linked.
- **HTTP Route**: Existing name-keyed reads: `/epr-head/{slug}` (doorway forwards `/api/epr-head/{id}`) and `/db/content/{id}/canonical-head`. Owed: a read keyed by the binding's address (the `CreateLink` ActionHash or the derived CID), declared in elohim-storage `build_manifest()`, answering all six components. The name-keyed reads then add the elector and clock beside the CID.
- **SDO/RWA Test** (social dominance orientation / right-wing authoritarianism: what a dominating holder could see, join and compel; the six numbered hard boundaries are in the p2p-design-gate skill and `private-thought-governed-fruit.md` §4): The worst holder sees which names are bound by whom and when, which is public by design; it cannot compel a binding without the elector's key or a root-accepted delegation, once the integrity rules land. Until then a modified coordinator can mint a staging link any peer accepts (boundary 4: constitutional records floor-protected; this is the gap row 17 closes).
- **Anti-Pattern Check**: Caught and corrected: a slug as address (the slug is a lookup key); a second identity beside the link (the CID is derived from it); a resolution order held in `deployments.json` (the compute plane; see constraints). No new entry type on a second DNA.

#### Entity: Petname (a person's own name for a binding or a CID)
- **Classification**: Private (B)
- **Justification**: A petname belongs to one person and confers no standing; no peer validates it, and it changes no answer anyone else receives. The row 17 guess (Attested-Private) is corrected: nothing about its effect needs a public attestation. A third party replays a disputed resolution from the person's disclosed, signed record.
- **Head-Plane Cost Budget**: n-a (Private). Count at seed: 0.
- **Network Stakes**: All four stages; no floor-protected cost.
- **Content Address Strategy**: Agent-Scoped Composite
- **Address Justification**: `(person's agent key, petname string, target binding CID or head CID)`.
- **Transport Affinity**: n-a.
- **Source of Truth**: Private Source Chain.
- **Integrity Zome + DNA-hash class**: `imagodei_integrity` (row 17 names imagodei; neither imagodei nor any DNA has a petname type, and imagodei has no private entry type at all; the only private type in these two DNAs is `AttentionTending` in `content_store_integrity`). DNA-HASH-MOVING. Until that crossing, petnames live in the device's own store, which satisfies the alone case and does not travel between devices.
- **Coordinator Zome**: `imagodei::set_petname` -> ActionHash of the private create; `imagodei::get_petnames` -> the person's own records.
- **Projections**: SQLite local index for the owner only (dht_anchor_hash: no). Automerge sync: no.
- **HTTP Route**: None for other agents. The owner's client reads its own device store.
- **SDO/RWA Test**: A doorway hosting the person sees every petname it signs (boundary 6: hosted participants are visible to their doorway). Nothing is published, so nothing joins across people.
- **Anti-Pattern Check**: Standalone shared table for agent state: refused. Granular data on the DHT: refused.

#### Entity: FollowSet (the asker's declared electors, in order, with mode)
- **Classification**: Attested-Private (B2)
- **Justification**: The list belongs to its holder, but its effect must be checkable: an answer a serving node gives others is legitimate only if it follows from a declared follow set. The attestation is the follow set's CID carried beside each answer; the list is disclosed by policy for a serving node and on demand for a person.
- **Head-Plane Cost Budget**: n-a for the raw list. The attestation is a field on an answer, not a head.
- **Network Stakes**: All four stages; the serving node's follow set is Constitutional (it decides what others are served).
- **Content Address Strategy**: Content-Derived (CID) of the ordered list.
- **Address Justification**: Replay requires the exact list; any change is a new CID.
- **Transport Affinity**: n-a.
- **Source of Truth**: today `deployments.json` `ELOHIM_RELEASE_CHANNELS` (compute plane); owed: the holder's source chain.
- **Integrity Zome + DNA-hash class**: unresolved; held until the scope declaration's shape is decided, since both answer "which inputs may vary an answer".
- **Coordinator Zome**: owed; none designed here.
- **Projections**: storage's release-adoption state already reads the list; it would report the list's CID.
- **HTTP Route**: `/db/p2p/adoption` (existing) reports followed channels; owed: the follow-set CID on every name answer.
- **SDO/RWA Test**: An outside observer learns which electors a serving node follows (boundary 6); a person's follow set is never published without their act.
- **Anti-Pattern Check**: A protocol concern modelled in the deployments plane: caught; named as debt, not designed here.

#### Referenced, not designed: the scope declaration
The fourth component is the per-plane scope declaration of `measure-family-borrows` row 32 and the specification's Open Issue "Reach is a per-plane declaration". Chain / between ChannelBinding → reach gate / missing node: a content-addressed scope declaration a binding names by CID, answering reach of bytes and of each reference, custody, freshness, linkability, cost bearer, retention / probe: a binding whose `scopeCid` names a declaration the reach gate reads / current state: `HeadDelegationPayload.scope` is a free string; no declaration exists. Row 32 holds its shape and its own gate.

#### Design Constraints Discovered
- The elector is dropped at two points (`CanonicalCandidate`, `CanonicalElectionOutput`) and only `get_canonical_election_evidence` keeps it. Carrying it through is the whole coordinator-side change, and it is DNA-hash-neutral.
- A bound slug resolves through two bindings in series. The answer must name both electors and both clocks, or the 2026-10-08 incident remains indistinguishable from a contested election.
- A council elector needs a threshold over members. `Collective` and `Membership` exist in imagodei, but membership authority is coordinator-only (authority backlog row 14), so council bindings wait on row 14.
- Standing for names is `HeadDelegation`, not `StewardshipGrant`. Acting for a subject still rides `StewardshipGrant`; joining the two is out of scope here.
- The plural-mishpat lenses follow-up binds lens scope on the EPR slug-id; that holds only while the slug stays a lookup key in front of a binding.
- Concern canon (C0-C14, `.claude/epr-meta/policies.yaml` and `concerns.yaml`) for `resolve_name_binding`: answered C0, C4, C5, C9, C12; partial C1 and C2 (self-electable staging links and the clock rule are coordinator-only until the integrity rules), C6a (links per anchor unbounded), C7 (an answer omits the follow set behind it), C10 (new fields must default for old coordinators), C13 (council tier unbuilt); unbound C8, C14; n-a C3, C6b, C11. Seam-registry registration is owed with the read.

**Back-fill detector** (the gate's three questions that cannot be answered from a route backwards).
1. The read returns the `CreateLink` ActionHash per binding. The existing name-keyed routes accept a name, not that hash; that mismatch is why an address-keyed read is owed.
2. `content_store_integrity`; the bridge view does not move the hash, the two row 17 rules do. Petnames would move `imagodei_integrity`'s hash.
3. At 1 year, heads are unchanged from seed (at most 3,556 names plus 2 channels); declaration links grow per release. Quiesce is unchanged in head count; election cost per anchor grows with declarations until retention prunes them.
