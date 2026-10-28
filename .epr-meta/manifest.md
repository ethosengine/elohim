---
epr-meta-version: 1
id: repo-root-governance
root: true
purpose: >
  The repo-root constitutional base. Carries the build-time ci-trigger: leg (the cross-cutting
  CI-ignore set, projected into the flat .ci-ignore), and author-time rules including the repo-wide
  binding of the source-file-LoC-ceiling policy (measure class — never blocks) and the
  governance-escalation-ladder (the agency charter — definitions live in the policy registry,
  .claude/epr-meta/policies.yaml; bound for both manifest forms), the brand-vocabulary boundary
  lint, a context-blind README review obligation, three developer-valueflow authoring signals, and
  two placement routes for the prose classes that kept being born at the repo root (handoffs,
  vulnerability-cluster sheets). The body records the top-level allow-list, which the gate cannot
  express. It anchors the cascade and hosts the
  ignores and basename-wide rules that cannot decentralize, plus the subtree/orchestrator exact-path
  entries kept inline for the first cut.
policy-recipe: .claude/epr-meta
policy-recipes:
  default:
    name: default
    dir: .claude/epr-meta
    measures: .claude/epr-meta/measures.yaml
    policies: .claude/epr-meta/policies.yaml
    why: >
      The declared DEFAULT recipe, named as an intentional act rather than left implicit
      (operator plurality ruling, 2026-09-10, recorded on
      genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md). This policy
      set is ONE lens over this substrate, never the lens: a second recipe may be added here
      and evaluated simultaneously over the same records, with every outcome labelled by its
      recipe. Rows carry a PRECEDENT_BINDING level and lineage so a set can fork, propagate
      and be ratified upward without a rewrite; ratification recommends, never overwrites
      local tightening. `policy-recipe:` is the key the native report reads
      (elohim/eprfs/epr-cli/src/flow/report.rs declared_default -> Recipe::at_dir): its value is
      the DIRECTORY holding the recipe's `measures.yaml` + `policies.yaml`. The entry below is
      that same recipe under its declared name `default` — joined to the scalar by its `dir:`,
      and carrying `name:` so a label reader can take the name from the key or the field. The
      declared name and the reported label agree at `default`; a second recipe is added here.
rules:
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
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
  - id: governance-escalation-ladder
    policy: governance-escalation-ladder@1
  - id: governance-escalation-ladder-dir-form
    policy: governance-escalation-ladder@1
    when: { write: "manifest.md" }
  - id: brand-vocabulary-boundary
    policy: brand-vocabulary-boundary@1
  - id: habit-declaration-at-birth
    policy: habit-declaration-at-birth@1
  - id: dev-lifecycle-context-sync-npm
    class: inject
    when: { write: "package.json" }
    dedupe-of: genesis/data/timeline/backlog/2026-08-16-dev-cli-hygiene-script-census.md
    why: >
      You are editing a dev command surface (package.json scripts). The local-dev lifecycle
      context is COUPLED to it and drifts when it changes silently — keep them in sync in the
      same pass: the hc-dev-orchestrator skill (.claude/skills/hc-dev-orchestrator/SKILL.md,
      local stack lifecycle), the root CLAUDE.md Build & Test Commands section, the root
      justfile recipes, and the script census/consolidation ledger this rule points at. If you
      added a script: does one of the eight root verbs already cover it (the census ledger
      tracks the measured baseline and burn-down)? If you renamed/removed one: grep docs+CI
      for the old name. Out-of-date
      instructions are cognitive load — the census doc records what is verified-alive.
  - id: dev-lifecycle-context-sync-just
    class: inject
    when: { write: "justfile" }
    dedupe-of: genesis/data/timeline/backlog/2026-08-16-dev-cli-hygiene-script-census.md
    why: >
      justfile recipes are the consolidation target for the dev CLI (census doc). When a recipe
      changes, sync the coupled context: root CLAUDE.md Build & Test Commands, the
      hc-dev-orchestrator skill, and the census ledger. Native-cargo recipes MUST set
      CARGO_TARGET_DIR at the cargo-pool slot (the disk-guard denies plain native cargo) —
      the pre-2026-08 root justfile drifted exactly here.
  - id: brief-is-a-claim
    class: inject
    when: { write: "task-*-brief.md" }
    route-to: { dest: "epr flow claim" }
    why: >
      A dispatched task brief is a claim on promised work. Record that act with
      `epr flow claim --on <gap-id> --as agent:implementer@<model> --brief <this file>` so the
      implementer's commitment is durable and attributable instead of existing only in orchestration
      prose.
    retire-when: >
      when the authoring surface records every task-brief dispatch as a claim by construction before
      the brief can be written
  - id: report-is-a-fulfilment
    class: inject
    when: { write: "task-*-report.md" }
    route-to: { dest: "epr flow fulfill" }
    why: >
      A completed task report discharges promised work. Record that act with
      `epr flow fulfill --on <gap-id> --report <this file> --status <DONE|DONE_WITH_CONCERNS>` so
      completion changes the commitment stock instead of remaining an unjoined report artifact.
    retire-when: >
      when the authoring surface records every discharging task report as a fulfillment by
      construction before the report can be written
  - id: rulings-are-notes
    class: inject
    when: { write: "progress.md" }
    route-to: { dest: "epr flow note --kind ruling" }
    why: >
      A ruling belongs in the valueflow record: use
      `epr flow note --on <gap-id|plan> --kind ruling --reason '...'`. The progress file is a
      projection and never the record, so do not leave a binding decision only as prose here.
    retire-when: >
      when progress projections are rendered entirely from ruling notes and cannot be authored as a
      competing decision record
  - id: handoff-routes-to-sprints
    class: dispatch
    when: { write: "handoff*.md", new: true }
    route-to: { dest: genesis/docs/superpowers/sprints/ }
    parameters:
      dispatch-agent: librarian
      dispatch-prompt: >
        Classify the proposed new handoff document. Report exactly one of ROUTE (the sprint result
        under genesis/docs/superpowers/sprints/ that should carry it), MERGE (the existing sprint
        result, backlog entry or memory file that already holds its content) or DROP (session
        narration that git history already keeps), with the shortest supporting reason. Do not
        create, move or rewrite files; report to the editing agent and let development continue.
    why: >
      Handoff documents are not an artifact class here: the handoff pattern was retired in
      df250665f (2026-06-11) and the decompose discipline replaced it on 2026-06-23; the last
      holding pen (.claude/handoffs/) was drained to zero residue and deleted on 2026-09-24
      (genesis/data/timeline/backlog/handoff-sprawl-decompose-2026-06-23.md). A concluded
      session's durable carrier is its sprint
      result in genesis/docs/superpowers/sprints/. Decompose the rest: open work to
      genesis/data/timeline/backlog/, a lesson to a memory file or to
      genesis/docs/content/elohim-protocol/history/, and the narration to git history.
    retire-when: >
      when no handoff*.md has been born outside genesis/docs/superpowers/sprints/ for two
      consecutive quarters, so the convention holds without a prompt
  - id: vulnerability-cluster-routes-to-backlog
    class: dispatch
    when: { write: "vulnerability_cluster*.md", new: true }
    route-to: { dest: genesis/data/timeline/backlog/ }
    parameters:
      dispatch-agent: librarian
      dispatch-prompt: >
        Check whether this security concern is already tracked by a security-*.md entry in
        genesis/data/timeline/backlog/ (its CLUSTERS.md groups them). Report exactly one of ROUTE
        (the security-<slug>.md name it should be born under), MERGE (the existing entry to
        extend) or DROP, with the shortest supporting reason. Do not create, move or rewrite
        files; report to the editing agent and let development continue.
    why: >
      A vulnerability cluster is a security concern, and security concerns live as
      security-<slug>.md entries in genesis/data/timeline/backlog/, where the deprecation-stasis
      loop reconciles them against .claude/data/deprecations.jsonl and drains them. Thirteen
      VULNERABILITY_CLUSTER_*.md sheets born at the repo root on 2026-07-30 had to be swept out
      by hand. Name the entry security-<slug>.md and write it in the backlog.
    retire-when: >
      when security concerns reach the backlog only through the deprecation sentinel's own
      writer, so no hand-authored cluster sheet has a reason to exist
  - id: readme-blind-reader-review
    class: inject
    when: { write: "README*.md" }
    route-to: { dest: blind-reader }
    parameters: { review-profile: readme }
    why: >
      After the README authoring pass, dispatch a fresh-context blind-reader with ONLY the completed
      README path and the `readme` review profile. The reader must be able to recover who the README
      serves, what the thing is for, prerequisites, its mental model, a first successful path, and the
      next useful action without inheriting the author's repository context. Revise and repeat with a
      new blind reader until READY or the operator explicitly defers named findings.
      Each finding carries a class (correctness | interpretability | preference); the loop closes when a
      fresh reader returns no correctness or interpretability finding, and preference findings never
      reopen it. Record the loop's cost where the work already reports — the commit message or the
      owning habit delta — as rounds, per-class counts per round, and findings resolved, so review
      efficiency is measured rather than felt. Every round still uses a new reader.
ci-trigger:
  ignore:
    - .claude/
    - .github/
    - .husky/
    - genesis/orchestrator/Jenkinsfile
    - genesis/orchestrator/build-graph.groovy
    - CLAUDE.md
    - AGENTS.md
    - GEMINI.md
    - .no-claude.md
---

# repo root — constitutional base

Carries the cross-cutting `ci-trigger:` ignore set (projected into `.ci-ignore` by
`.claude/scripts/ci-ignore-projector.py`) and the author-time rules listed above. **`.ci-ignore` is GENERATED from
this leg — never hand-edit it.**

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

The [portable-app lifecycle roadmap](../genesis/docs/superpowers/specs/2026-10-14-portable-app-lifecycle-design.md)
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
owner. Hub has a declared home but no runtime implementation there yet. Review,
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

## Reach context — claim, human standing, Mishpat affirmation

Operator declaration, 2026-09-29: Matthew is the human operator of record for this developer
environment. The developer effort proceeds under his authorization. Claude, Codex and Gemini
exercise delegated review and stewardship while the fuller network of human peer developers and
Elohim inference is not yet participating. The claim is that this developer effort and its public
demonstrations have earned public reach under that authorization.

A reach inquiry anywhere in the repository should be able to follow the applicable local
`.epr-meta` declarations through their authority context to three connected pieces of evidence:

1. **The claim:** what is proposed to have which reach, for which artifact/version, world and act.
2. **The human with standing:** who is accountable for the claim, the holon/context in which that
   standing applies, and the delegation by which an agent acts for them.
3. **The Mishpat affirmation:** the network governance decision affirming that claim and standing,
   with its scope, bounds and applicable supersession or withdrawal evidence.

This manifest records the operator's declaration; it does not certify its own claim. **No
network-witnessed Mishpat decision or verified identity-binding reference is linked here yet.**
That missing evidence remains explicit. Mature-network governance determines whether and where
the claim is recognized; a filesystem location, persona name or stage flag is not that decision.

Public delivery and world-local standing are distinct. A simulacra artifact can legitimately be
published on the public web under the operator's authorization while Adam's simulated memberships
and grants remain facts of that world. Publicly serving the artifact does not promote those facts
into authority over real humans. More specific content and holon constraints still matter; this
project claim is not a recursive disclosure grant for every byte beneath the directory.

This is the manifest's explanatory context, not a new executable rule or frontmatter schema.
Today's resolver follows filesystem ancestry and stops at `root: true` (and repository boundaries);
it does not yet resolve the claim/standing/Mishpat trace across nested roots or verify network
affirmations. The reach solutioning session must specify that trace without treating nearest-rule
override as permission to manufacture standing. Its existing home is
`genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md`.

## rs-loc-ceiling — repo-wide source-file LoC ceiling (measure class)

Binds `source-file-loc-ceiling@1` from the policy registry: `*.rs` writes are measured against a
soft (edit-time nudge) and hard (fingerprinted architecture finding → `.claude/data/
architecture-findings.jsonl` → modularization-plan dispatch) LoC ceiling. Observation tier — it
never blocks a write. Vendored submodules (brit, rakia) are outside this cascade by construction
(their own `.git` roots terminate the ancestor walk). A region with a legitimately different
ceiling (e.g. a table-driven test harness) overrides locally by re-binding the policy with
`params:` in its own `.epr-meta` — never by editing the registry version in place.

## governance-escalation-ladder — the agency charter (ask class)

Binds `governance-escalation-ladder@1`: agents self-grant `measure`/`inject`/`dispatch` freely;
authoring or promoting an `ask`/`deny` rule requires an operator-ratified policy pin. Bound HERE
rather than a `.claude/epr-meta/.epr-meta` manifest deliberately — `when` patterns match by
BASENAME only (`_matches_when` in `_lib/epr_meta.py`) and the cascade is a strict ancestor walk,
so a manifest placed anywhere other than the repo root can only ever govern writes BENEATH
itself; it cannot reach out to `.epr-meta` files elsewhere in the tree. A repo-wide charter needs
the repo-root anchor — the minimal-diff fallback the design explicitly allows when a
`.claude`-located manifest can't scope repo-wide. See the policy row for the full why.

The policy's own scope is `write: ".epr-meta"`, which matches only the flat manifest file. The
directory form, `<dir>/.epr-meta/manifest.md`, has a different basename, so the charter never saw
it. `governance-escalation-ladder-dir-form` binds the same `@1` policy a second time with a
`when: { write: "manifest.md" }` override. That closes the gap without a new registry row. The
validator checks the path first (`is_manifest_path`), so a `manifest.md` that is not an `.epr-meta`
manifest draws nothing. A subtree under another `root: true` manifest inherits neither binding;
such a subtree must bind the ladder in its own manifest.

## brand-vocabulary-boundary — accessible code vocabulary (inject class)

Binds `brand-vocabulary-boundary@1` repo-wide. Architecture and product prose retain the domain
brands; newly introduced code, routes, schemas, configuration identifiers, persistence names, and
wire values receive an advisory to use their semantic capability name. There is no internal
compatibility exemption while the protocol is still in development: role names, zome names, package
ids, discriminators, and wire literals should be renamed too. The rule is net-new-only and never
blocks, so cleanup can proceed as each existing surface is touched without turning legacy vocabulary
into a maintenance toll.

## Developer-valueflow authoring — briefs, reports, and rulings (inject class)

The three authoring signals keep orchestration artifacts joined to the valueflow they change: a task
brief claims a commitment, a discharging task report fulfils it, and a progress-file ruling is first
recorded as a ruling note. The files remain useful projections for people, but none is the durable
record of the act it describes.

## Placement at the repo root (dispatch class, plus an allow-list the gate cannot express)

Two prose classes kept being born at the top level and swept out by hand: session handoffs
(`HANDOFF-*.md`) and vulnerability-cluster sheets (`VULNERABILITY_CLUSTER_*.md`). Each has a named
route at birth. `handoff-routes-to-sprints` sends a handoff to `genesis/docs/superpowers/sprints/`
(the `.claude/handoffs/` holding pen that once widened this rule was drained and deleted on
2026-09-24, so the root rule is the only one). `vulnerability-cluster-routes-to-backlog` sends a cluster sheet to a `security-*.md` backlog entry.
Both dispatch a report-only librarian review and never block.

**Top-level allow-list — authoritative, not gate-enforced in v1.** The repository root holds only:

- directories: `app/`, `bridges/`, `che-devworkspaces/`, `crates/`, `doorway/`, `elohim/`,
  `genesis/`, `patches/`, `scripts/`, `sophia/`, `steward/`, `vendor/`, plus `docs/`, which is a
  tombstone holding only its own routing manifest;
- workspace-root files: `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `justfile`,
  `deny.toml`, `VERSION`, `devfile.yaml`, `Jenkinsfile`, `sophia.Jenkinsfile`, `README.md`,
  `CLAUDE.md`, `AGENTS.md`, `.gitmodules`, `.gitignore`, `.ci-ignore`, `.npmrc`;
- tool dotfiles and dot-directories (`.claude/`, `.codex/`, `.agents/`, `.epr-meta/`, `.eprfs/`,
  `.github/`, `.husky/`, `.cargo/`, `.vscode/`, `.mcp.json`).

`storage-iroh/`, `iroh-relay/`, `relay-addr-beacon/`, `docs/`, `rakia/` and `tools/` were relocated
or retired on 2026-09-23. Anything else belongs inside one of the homes above. The gate cannot
enforce this list. `when.write` matches the basename only, so it cannot tell a depth-1 file from
any other. A root rule also cascades to every descendant, and `no-new-subdirs` and `require-sibling`
fire at any depth, so either one here would toll every new module directory in the tree. Review
holds the list. A tool that writes machine-local state at the root (a dependency store, a build
pool, a crash log) needs a `.gitignore` line, not a rule: the resolver sees only Write and Edit.

## readme-blind-reader-review — newcomer legibility (inject class)

Every `README*.md` authoring pass routes to the general `blind-reader` using the `readme` profile.
The post-write adapter makes the obligation visible after successful edits. It is a semantic review,
not a formatting lint: a context-isolated reader must be able to orient, understand purpose and
boundaries, satisfy prerequisites, reach a first success, and know what to do next. One rich review
loop runs after the completed authoring pass—not one dispatch per edit.

Native seam guidance references the owner-local `*.seam.json` declarations bound above.
They refine the composition standards; lexical matches are candidates, not ownership rulings.
