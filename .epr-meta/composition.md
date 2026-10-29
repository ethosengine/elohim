# Composition map — application lifecycle and native developer tooling

This page is navigation for the repository root, kept beside `manifest.md` rather than in it: it
records where cross-cutting concerns live and how the developer tools compose. It binds no rule.
The owner-local `*.seam.json` declarations cite its sections as authoritative contracts.

## Application lifecycle — authoritative homes and composition

Every application, including developer tools, composes the same lifecycle. Local-first
is a requirement across it, not a transport or a maturity tier. Private work, shared
editing, published versions and web exposure are independent choices: sharing does
not publish, publication need not be public, and a doorway is optional. Presence is
transient; it is not a document change or evidence of authority.

The [working-version standard](../genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md)
owns working-state and publication lifecycle semantics. The
[sync-state contract](../genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md)
owns per-copy freshness vocabulary. The [hub boundary design](../genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md)
owns the peer-serving/doorway split. Extend these existing standards when a common
contract changes; this map routes work to them rather than restating their schemas.

| Concern | Authoritative implementation home | What consumers may compose |
|---|---|---|
| Application declarations and reusable client lifecycle | [SDK governance](../elohim/sdk/.epr-meta): app-manifest schema and domain declarations; `storage-client-ts`; matching [Rust SDK](../crates/elohim-sdk/.epr-meta) | Apps choose whether a type has working state, its publication rule and passing-state needs. Framework adapters bind the common contract; they do not invent another lifecycle or vocabulary. |
| Durable working state and merging | Shared SDK client persistence on the person's device; [storage](../elohim/elohim-storage/.epr-meta/manifest.md) document store and authorized sync on a node; Automerge merges values | Apps supply domain edits. Solo drafts stay out of published projections, listing and automatic peer sync. Editor admission comes from signed authorization, never a merged field. |
| Presence and gestures | The working-version standard's passing-state contract; reusable SDK and peer transport implementation when built | Apps declare transient meanings. No durable presence log, notary write per gesture or authority inferred from a heartbeat. The shared transport remains unchosen in that standard. |
| Publication and canonical authority | [DNA governance](../elohim/holochain/dna/.epr-meta/manifest.md): existing integrity/coordinator authority; storage's verified projection; SDK composes fixing and submission | Apps choose a declared submission policy and display observed outcomes. CRDT convergence, a build result or byte possession cannot elect a head. |
| Published byte custody and replication | Storage's blob, reach, replication and release-adoption services; EPRFS for filesystem projection | Apps and SDKs request and verify held content. They do not create a separate replication network or infer authority from availability. |
| Peer-native app serving and rendering | [Hub governance](../elohim/elohim-hub/.epr-meta), composed into the node runtime | Apps provide bundles and declared capabilities. Serving by peer key and render/deliverability behavior are shared runtime concerns, even on a single person's device. |
| Web-facing projection and hosted access | [Doorway governance](../doorway/doorway-service/.epr-meta/manifest.md) | Mount the same underlying capabilities for browsers/web2: names, TLS, host routing and edge caches. Do not make doorway the sole owner of drafts, publication or app execution. |
| Domain UX and framework bindings | [Application governance](../app/.epr-meta), domain app manifests and shared client libraries | Compose the contracts above into screens and domain actions. Reusable persistence, sync or authority fixes go to their owner before adding another application-specific implementation. |

**Sprint routing.** A brief names the existing habit/scenario it serves, the standard
it changes (if any), the authoritative producer, consuming adapters, and the owning
gate. A shared missing capability is work at the producer with an application journey
as its proof. Keep implementation work and consumer wiring in the same acceptance
chain; a frontend workaround does not close a missing SDK/storage contract. Changing
a shared wire shape updates its generative source and projections together.

The current [placement contract](../genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md) owns app needs and their first-reader gates. The [portable-app lifecycle roadmap](../genesis/docs/superpowers/specs/2026-10-14-portable-app-lifecycle-design.md)
adds resident execution and executable residency to this composition. Foundation contracts precede
Che access, Rakia adoption and platform integration. Rust primitives own behavior; SDKs expose it
and environment bridges translate it. Passing-state sessions compose the separate live-session
standard. Private observation continuity across explicitly admitted own devices never authorizes
pooling, public gossip, or cross-person access. Declaration, implementation and delivery evidence
remain separately reported.

**Proof at the seams.** Qualify the relevant transitions: reopen local work offline;
admit authorized editors while refusing unauthorized listing/serving/sync; merge
without silently publishing; fix and submit with pending/current/not-current outcomes;
serve verified published bytes from a peer with no doorway; then expose the same
capability through the doorway without widening reach. Use component contract tests
and the existing [shared-sheet story](../genesis/a2o/features/federation/local-first-shared-sheet.feature)
where applicable. A solo publication needs no collaborative editing, and a one-act
type need not acquire a working document to satisfy a universal checklist.

**Declared versus delivered.** The working-version standard is Draft and records
missing persistence, admission and publication composition. Its verbs and proposed
manifest fields are not an implemented API merely because this map names their
owner. Hub has a declared home but no runtime implementation there yet, and the local-view reader from the rescue remains a separate port. Review,
local manifest advisories and owning gates hold the boundary; there is no automatic
cross-project architecture checker. Do not turn a missing shared primitive into a
second per-app standard or claim a draft privacy guarantee before the serving and
sync paths enforce it. New entities/routes still require the P2P design gate;
this map adds none and does not choose the unresolved presence transport.

## Native developer tooling — composition and authority

This root records how the developer tools compose; component manifests own their
local contracts. The [native developer through-line](../elohim/brit/docs/plans/2026-09-27-native-developer-through-line.md)
defines the intended experience: one public `brit` executable connects intent,
scoped source, checks, builds, authorized release and verified retrieval. This is
the target contract, not a claim that every station is delivered.

| Concern | Owner and local authority | Boundary |
|---|---|---|
| Developer journey and repository transitions | Brit's [standalone root](../elohim/brit/.epr-meta/manifest.md) | Brit composes the journey; gix and Git adapters interpret objects, index, refs and worktrees. Git mechanics do not establish Elohim authority. |
| Shared governance and evidence | EPR's [CLI manifest](../elohim/eprfs/epr-cli/.epr-meta) and [decision registry](../elohim/eprfs/epr-cli/seam-registry.yaml) | Intent, attribution, review, acceptance and readiness use the existing shared records and evaluator. Brit, other domain tools and human interfaces must not create competing acceptance ledgers. |
| Immutable content and local materialization | EPRFS [core](../elohim/eprfs/eprfs-core/.epr-meta), [local](../elohim/eprfs/eprfs-local/.epr-meta) and [storage adapter](../elohim/eprfs/eprfs-storage/.epr-meta) manifests | Artifact-neutral trees, verified bytes and host projection; no Git interpretation, build recipes or election of release heads. These manifests inherit the [substrate manifest](../elohim/.epr-meta); EPRFS has no separate workspace root manifest. |
| Developer services, artifacts, automation and their GUI | Rakia's [repository contract](../elohim/rakia/CLAUDE.md), [composition design](../elohim/rakia/docs/composition.md) and [executor/registry design](../elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md) | Owns the developer-service roles exemplified by Artifactory/Nexus, GitHub, Jenkins, Harbor and SonarQube: package/artifact catalogs and registry workflows, forge collaboration, build execution and graph visualization, release operations, static analysis and collaborative quality review. Composes Brit's source operations and shared governance; consumes substrate custody/transport rather than creating another storage network. |

The `epr` executable is built in `elohim/eprfs/epr-cli`; `elohim/epr` is the
canonical codec/envelope library. Their similar names do not make the codec a
developer orchestrator. The wider [concern-routing atlas](../genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md)
owns routing beyond these developer-tool concerns.

**Composition rule.** Domain-independent identity, commitments, review and
acceptance belong in the shared protocol/governance layer. Repository interpretation
belongs in Brit or an explicitly identified Git adapter. Build, artifact and
developer-service interpretation belongs in Rakia, including work with no source
repository. Rakia's forge experience composes Brit's source lineage and EPR's
review/acceptance; analysis findings and quality thresholds are evidence and declared
gate inputs, not independent grants of authority. A public command or GUI may compose
these capabilities without duplicating records or decision logic. Command names,
executable packaging and crate ownership are distinct boundaries.

### Rakia's local-first developer surface

Operator clarification, 2026-10-12: Rakia's scope includes the roles above and their
local-first GUI, with a web surface/server runnable on the person's own node and
composable for P2P use. The same composition can be published through a doorway to
Web 2.0. A doorway adds hosting and reach; it is not the exclusive home of the app,
its working state or its server-side capabilities. The named products identify
responsibilities, not a requirement to run those vendors or copy each product.
Static analysis includes the social experience around findings: discussion,
attributed remediation, quality trends and review. Personal attention and private
notes remain on the person's device.

Local-first reading of this intended surface:

- **Alone, key on device:** browse held source, artifacts, build graphs, results and
  findings; retain private working changes; run checks/builds whose inputs and tools
  are available locally; fix, address and sign the person's own contribution. Missing
  inputs or unavailable compute are named dependencies, not reasons to lose the GUI.
- **Alone, key held by doorway:** the same held-byte reading and private working
  changes work; versions can be fixed and addressed. Account signing waits for the
  key holder and is shown as not saved to the account. A hosting doorway can see what
  passes through it and sign as its hosted person.
- **Alone, no key:** inspect held published material as a visitor; no personal state
  persists past the visit, and the UI says so.
- **Working version:** starts private to the author in the holder's document store;
  shared editing requires an explicit grant. Agent work is a draft the person may
  merge with attribution. Reopening offline restores retained changes. Publishing
  submits a fixed version through existing declaration/election rules; a collaborator's
  submission cannot silently replace the elected version.
- **Each kind of state:** selections and private notes stay on device; presence and
  cursors use an unstored channel; working documents use the merging layer separately
  from published projections; published artifacts and evidence are fixed/addressed.
  Graph layouts and dashboards are views, not another authority ledger.
- **Arrivals:** another owned device adds a trusted copy and can run the same local
  service; peers add collaboration and execution; an always-on copy adds availability;
  a doorway adds hosted/browser access; a pool adds capacity for public work. Owned
  devices require no per-item trust ceremony; other relationships require explicit
  trust. Each learns only admitted content plus relevant request, size and timing
  metadata. A copy outside the authorized circle receives no private draft; pool
  membership/use is public, so sensitive work stays outside it. Loss of any addition
  leaves held bytes, drafts, local inspection and locally runnable work usable; pending
  remote work remains visibly pending rather than falsely complete.
- **Shown to the person:** device-only changes; each copy caught up, behind or unknown;
  unsigned/pending/elected/not-current versions; checks and artifact availability with
  their evidence scope. Another person's consent, missing bytes and external execution
  may wait; the person's own contribution is prepared and retained first.
- **Refusals / stands as is:** reject a host-only state store or a GUI requiring an
  initial server call when held content suffices. Keep Brit source semantics, EPR
  authority, EPRFS projection and substrate custody in their existing homes; Rakia
  composes their developer-facing service and quality experience.

This declares product ownership and acceptance expectations, not new entity schemas,
routes or an implemented server/GUI. Detailed work still passes the P2P design gate
and owns runnable local/offline and publication checks. Rakia currently has no local
`.epr-meta` root; its existing repository contract places the developer CLI in Brit.
That packaging does not reduce Rakia's product scope to a build-planning library.
The independent repository must adopt its own enforcement; this parent declaration
does not cross its Git boundary.

**Current adapter placement.** Brit delegates context to installed `epr`; the
integration adapter being qualified also lives in EPR CLI. That adapter may bind
acceptance to exact Git source/target revisions and execute an atomic ref change.
Its placement does not transfer developer orchestration to EPR or permit Git
semantics in EPRFS core. Shared acceptance and repository execution must remain
coupled through revalidation at mutation time. The installed-EPR dependency is an
explicit limit of today's composition, not proof of the one-executable target.

**Where enforcement lives.** Local `.epr-meta` rules provide authoring signals;
component `seam-registry.yaml` files name decision surfaces and their limitations;
owning manifest gates and executable contract/journey tests verify behavior.
Cross-component changes must update the affected local contracts and consuming
tests together, retaining one shared evaluator. Parent pin gates attest the pinned
component's CI; they do not prove architectural ownership. Review must check this
composition when a capability moves or an adapter exception expands.

These links are navigation, not executable governance imports. The resolver stops
at repository boundaries and `root: true`; this root cannot impose rules inside
Brit or Rakia. There is no automatic cross-root composition validator today.
Independent roots must adopt their own rules, and ownership remains a review
obligation until a real check exists. Do not use inert `extends:` declarations or
an unimplemented validator to claim otherwise.

## Native seam guidance — opt-in

`epr govern seam` and the validator `epr:validator-native-seam-guidance` classify a proposed
write against the owner-local `*.seam.json` declarations, using the governed method in
[`elohim/algorithms/seam-guidance.json`](elohim/algorithms/seam-guidance.json). It is advisory
and opt-in. Until a `.epr-meta` cascade binds the validator, the evaluator reports
`enabled: false` with an empty packet, so the hook wrappers stay silent and ordinary governed
writes do not run the classifier. Lexical matches are candidates, not ownership rulings.

To enable it for the whole repository, the steward adds this rule to the root manifest's
`rules:` (a root manifest edit counts as touching every crate under RS-GOV-04, so it lands as
its own reviewed change):

```yaml
  - id: native-seam-guidance
    class: inject
    when: { write: "*" }
    validator: epr:validator-native-seam-guidance
    parameters:
      algorithm: .epr-meta/elohim/algorithms/seam-guidance.json
      declarations:
        - elohim/sdk/working-version.seam.json
        - elohim/sdk/app-lifecycle.seam.json
        - elohim/elohim-storage/.epr-meta/publication-authority.seam.json
        - elohim/elohim-storage/.epr-meta/peer-serving.seam.json
        - elohim/eprfs/epr-cli/developer-composition.seam.json
    why: "Advisory candidates from owner-local seam declarations; active-agent review distinguishes implementation from composition."
    retire-when: "when authoring clients surface verified seam contracts without this advisory binding"
```

A narrower `.epr-meta` may bind the same rule for its own subtree instead.
