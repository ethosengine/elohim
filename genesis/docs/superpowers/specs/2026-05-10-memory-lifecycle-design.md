# Memory Lifecycle Design — Comet-Shaped Memory and Deliberate Forgetting

**Status**: proposal
**Date**: 2026-05-10
**Owner**: elohim-protocol research
**Implements**: `/dream` skill (forthcoming)
**Forward-references**: elohim-agent autonomous memory; EPR substrate decay; DHT entry lifecycle; lamad content archival; mishpat scenario corpus hygiene

---

## Why

The household network is a bounded compute substrate. Every long-lived datum — Claude memory entry, EPR gossip, DHT notarization, content node, scenario, sprint result — costs storage on every replica, indexing on every search, traversal on every query. Long-term unbounded retention is structurally unsustainable for household-scale infrastructure.

Yet **forgetting must not be accidental**. Letting things fade ungraciously is the same failure mode as remembering everything forever; both are bugs. The protocol needs **principled lifecycle policy** that governs when data stays hot, when it compacts, when it merges with kin, when it closes its interval, when it earns memorialization, and when it can be released.

This spec defines the lifecycle primitives. They generalize across data types — Claude memory, agent working memory, EPR/DHT, content, scenarios — because the underlying problem is the same at every scale: **bounded compute meets unbounded conversation**.

The pattern: **practical today, governable tomorrow.** We prototype this in `/dream` against our own dev memory (fast feedback, observable outcomes), then graduate the principles into elohim-agent's autonomous memory hygiene, then into the protocol substrate's data lifecycle. Same circularity loop at every layer.

## Core principle: comet-shaped memory

Memory has three regions, gradient not binary:

- **Head** (~99% of recent traffic) — bright, dense, fully present. New artifact, hot working state, recent decisions. High access frequency, high resolution, full detail.
- **Tail** — long, dwindling. Compacted essences, distilled principles, merged heads from earlier subject-coalescences. Lower access, lower resolution, but referenceable. Carries trajectory.
- **Memorialized core** — small, anchored, never forgotten. Manifesto-tier facts, foundational principles, structurally-load-bearing decisions. The things that have *earned* permanence through repeat citation across many heads.

Most data lives in the tail, fading. Only what earns it stays in searing memory. **Memorialization is the asymptote of repeated reference**, not a default.

The comet shape applies fractally: each lifecycle data type (a memory entry, an EPR cluster, a DHT entry-set) has its own head/tail/core relative to its scope.

## Lifecycle primitives

Every memory entry has **three orthogonal attributes**:

1. **Tier** — episodic / semantic / manifesto (importance and permanence)
2. **Visibility** — conscious / subconscious / quarantined / forgotten (who can see)
3. **Validity interval** — `[start, end?]` (when it was/is true; closed-interval is supersession, not deletion)

Lifecycle operations transform these attributes independently. Seven operations (six terminal-or-transformational, one paired bidirectional) cover the full surface.

### 1. `promote`
Move an entry up a tier — episodic → semantic, or semantic → manifesto.
- **When**: earning criteria met (see below).
- **Effect**: the entry's tier label changes; freshness clock resets; cited-by edges accumulate.
- **Authority**: operator (for personal memory), qahal (for collective principles), author (for own EPR couplings).

### 2. `compact`
Distill one entry to its essence plus a pointer to source detail.
- **When**: the entry's *content* is principle-shaped but its *body* is verbose; or feature shipped and the spec body has been superseded by code-as-truth.
- **Effect**: body shrinks (target ≤2-3 sentences for memory entries; ≤1 paragraph for specs); pointer attaches (git tag, file path, content hash).
- **Authority**: same as the entry's tier.

### 3. `merge`
Fuse N entries (or N copies, or N adjacent records) into a new head.
- **When**: two or more entries are detected as same-concept-from-different-angles (e.g., conversation-A noted X, conversation-B touched the same X from a different aspect), or N replicas of the same content exist across the network without distribution justification.
- **Effect**: produces a new head; **lossy by design** — overlap is shed, distinct material is preserved, synthesis is added. Specific compression ratio depends on overlap and is not normative; the discipline is "earn the loss" not "hit a number." **Lineage edges from the new head back to each predecessor are mandatory**; predecessor entries close their validity intervals (superseded, with date).
- **THE most graph-shaped primitive.** Detection requires graph-aware proximity AND governance signals (see below); synthesis requires producing a coherent new statement that captures all sources without redundancy; lineage requires explicit provenance edges.
- **Why critical**: merge prevents unbounded growth *without* requiring forgetting. When predecessors earned their existence, coalescing is more sustainable than letting either fade. At protocol scale this is **content deduplication across the network** — see Network-Scale Merge below.
- **Authority**: never identity-match alone. Authority + multi-factor governance signals must concur. For personal entries: author or operator. For collective records (manifesto, qahal-governed, content-distribution): the responsible body, gated on the governance signals.

### Network-Scale Merge (load-bearing implication)

The merge primitive scales from memory entries to **whole-network content deduplication**. A movie copy does not need to live on every household. Many EPRs about the same subject do not need to all stay hot. If the protocol can attest "thing (a) we observed *is definitively* thing (a) THEY observed" via content addressing + governance, merging the storage records is the protocol's native deduplication primitive.

Identity-matching alone is **not sufficient** to authorize a merge. The decision must satisfy a multi-factor governance check:

| Signal | What it enforces |
|---|---|
| Content-reach | Who needs access from where? Merging cannot strand readers — replicas remain wherever reach demands. |
| Authoritative-governance | Who steward this artifact? Their authority must consent before consolidation. |
| Valueflows-to-stewards | Stewards holding the content earn flow (shefa). Consolidating must not silently strip their participation. |
| Resiliency | What replica count survives expected loss? Merging cannot drop below the resilience floor. |

Detection (the "yeah, definitively this is the same thing" judgment) is an **elohim-agent discernment concern**, not a UI/skill concern. Skills surface candidates and proposals to operators; the underlying signal extraction, identity attestation, and governance gating is substrate work in the elohim-agent layer.

**Cross-references for downstream specs**: `epr-content-addressing` (content identity), `rea-economics` (valueflows-to-stewards), `qahal` (governance authority), `libp2p-discovery` + `automerge-sync` (replica/distribution mechanics).

### Consolidation Events as First-Class Economic/Social Feedback

A consolidation event is **not janitorial**. It is among the protocol's most signal-rich operations because it simultaneously conveys: a truth-discovery (`a = b` or `a = bad`), an economic shift (compute freed, restitutions owed), a relationship update (shared-insight or strained-trust), a reach update (earned increment or reversal decrement), and a governance moment (was authority exercised well). Treating merge as silent graph rewrite forfeits all five.

Every consolidation primitive in the substrate **must emit structured events to four pillars**:

**Positive feedback (when consolidation reveals connection / equivalence):**

| Signal | Pillar | Emission |
|---|---|---|
| Discoverer reward | shefa | REA economic event — flow to the agent(s) who recognized the equivalence; truth-finding is valuable work |
| Compute freed | shefa | reduced stewardship-load on N replicas → can shard, redistribute by reach/governance/resiliency |
| Relationship | imagodei | shared-insight history accrues between agents who recognized equivalence together; trust signal updates |

**Negative feedback (when consolidation reveals bad-propagation: misinformation, malware, harm):**

| Signal | Pillar | Emission |
|---|---|---|
| Restitution obligation | mishpat | propagators owe those who received bad through them; restitution path is first-class affordance, not optional |
| Reconciliation obligation | imagodei | strained relationships need active repair; protocol offers the path |
| Reach drop | shefa + reach-accounting | reach earned at authoring is **reversible at consolidation**; bad-propagation costs reach at the moment of discovery; the dropped-reach content **defaults to submerge, not forget** (preserves recovery loop) |
| Quarantine | substrate | discovered-bad is **contained, not relabeled** — quarantine is structurally distinct from `forget`; forgetting a known-bad is irresponsible |
| Submerge | substrate + imagodei | reach-dropped or decayed content moves to encrypted elohim-visible tier; subject can later face it (canonical path) or elohim can ambiently surface when patterns recur — recovery loop, not permanent disenfranchisement |

**Reach accounting is bidirectional.** Reach earned at authoring AND adjusted at consolidation. Full history visible to qahal/mishpat for audit. This extends the existing principle (reach earned at authoring) into a closed-loop economy: authoring earns; bad-propagation discovered through consolidation reverses.

**Quarantine ≠ forget.** Two structurally distinct forget-shaped operations:
- `forget` — release fully (transient, no longer relevant, audit trail kept)
- `quarantine` — contain known-bad, track its propagation paths, support restitution/reconciliation; the bad fact is *retained as evidence*, not released

**Implementation note**: this section names obligations the substrate must make available, not implementations to draft today. Downstream specs (shefa REA event types, mishpat restitution flow, qahal consolidation-authority, imagodei relationship-update) consume these signals.

**Values-forward disclosure obligation**: the mechanics named here (restitution, reach-drop, reconciliation, quarantine) will be experienced as oppressive by users with accountability-resistant patterns. The protocol does not soften this; being crystal-clear about it IS the consent. The Icarus framing — structure constrains because the alternative is fatal to the unconstrained — is the recommended onboarding metaphor. See manifesto-tier disclosure language (forthcoming under `genesis/docs/content/elohim-protocol/`) and project memory `project_values_forward_disclosure_accountability.md`.

### 4. `close-interval`
Mark an entry superseded with an end-date.
- **When**: a fact's referent renamed/moved/deleted, or a successor entry has been promoted/merged/created.
- **Effect**: structurally distinct from delete — the entry remains queryable as historical record; its validity window is now `[start, end]` rather than open-ended; current resolution skips it.
- **Why**: trajectory queries can walk backward through closed intervals; learning the protocol's history requires not erasing it.
- **Authority**: whoever has authority over the entry's tier.

### 5. `memorialize`
Anchor an entry to manifesto-tier; permanent retention.
- **When**: the entry has been cited in M+ specs/plans across N+ months and represents a principle the project has structurally adopted.
- **Effect**: tier becomes manifesto; never compacted, never merged, never forgotten. Eligible for protocol substrate publication.
- **Why-cap**: memorialization is rare by design; flooding manifesto-tier defeats its purpose. Earning standard must stay high.
- **Authority**: explicit reviewer pass — operator for personal manifesto, qahal for collective manifesto. Never auto-promoted.

### 6. `forget`
Release an entry fully.
- **When**: episodic-tier entry is genuinely transient (one-off debugging note, stale temp directive) AND has not been cited or referenced for the freshness window AND no successor exists.
- **Effect**: entry removed; an audit-trail record persists (`forgotten_at: <date>, reason: <why>, scope: <what>`) but the body is gone.
- **Why**: forgetting is first-class. Not failure — sustainability. Aligns with *ungrudging service*: the gift flowed, the trace can fade.
- **Authority**: same as the tier; never automatic without explicit policy declaration.

### 7. `submerge` ↔ `surface` (paired bidirectional)
Transition an entry between conscious and subconscious visibility.

**`submerge`** — route from conscious view to a stewardship destination.
- **When**: entry loses reach (e.g. discovered bad-propagation triggers reach-drop), naturally decays from active reference, or is superseded by a merge/promotion. Default for reach-drop on bad-propagation: **submerge, not forget** (preserves recovery loop).
- **Effect**: entry routes to a destination, where it is held under that destination's attestation/anonymization/consent/lifecycle rules; not surfaced on default human-facing surfaces; not network-discoverable through normal queries; **dream/compaction lifecycle continues to apply at every destination** — no destination holds content "forever by default."
- **Destinations** (each is an *attested stewardship collective*; declared as app-manifests in the substrate's federation layer; non-attested entities cannot be destinations):
  - *Personal subconscious* — default for individual-bound content; visible only to subject's own elohim + trusted-relation elohim per governance
  - *Therapist collective (licensed)* — traumatic memories, intimate patterns needing professional support; anonymized; held by attested licensed therapists' mutual-attestation collective; their elohims have read access; provides anonymized clinical insight
  - *Research observatory* — sociological/public-health patterns; anonymized at routing; citation-anchored lifecycle
  - *Government-encrypted evidence store* — harm-class content (CSAM, fraud, abuse evidence) requiring law-enforcement chain of custody under judicial oversight + restorative-justice witness
  - *Cultural archive* — archival-class content cooled from active distribution; library-of-record collectives
  - *Lineage archive* — family/household trajectories; graduated authority across generations
- **Routing determinants**: memory class + content character + consent attached (explicit-at-submission OR inferable-from-standing-trust-contract; never inferred from technical convenience) + purpose served by destination
- **Why**: pure delete forfeits the recovery loop AND wastes potential structural service (therapeutic insight, research, restitution-evidence). Routing preserves trajectory while honoring consent and purpose.
- **Authority**: subject + their authoritative-governance tier (operator for personal, qahal/mishpat for collective destinations; explicit license/attestation required for non-personal destinations).
- **Imagodei test**: any routing/destination/lifecycle choice that subordinates the image-bearer to the institution or system fails the design test, regardless of technical merit.

**`surface`** — inverse: move from subconscious back to conscious.
- **When**: subject chooses intentional facing (the canonical path); OR elohim detects pattern recurrence connecting to submerged content and proposes ambient surfacing (gentle, non-interruptive); OR governance authority requires it (e.g. mishpat for restitution audit).
- **Effect**: entry returns to conscious visibility, often in transformed form (integration markers, recovery context, or marked as "faced and resolved").
- **Why**: healing happens through facing, not through forgetting. The protocol must offer paths through, not only paths out.
- **Authority**: subject consent for self-surface; elohim discernment with governance-tier authorization for ambient surfacing.

**Why this primitive is load-bearing**: the protocol's accountability mechanics (consolidation events, restitution, reach-drop) cannot be just punitive — they require a recovery loop to be sustainable AND to honor the human's agency. Submerge/surface mirrors how the human psyche actually heals: patterns held in subconscious until ready to face, surface-able through intentional work or trusted-other insight. The elohim plays a role analogous to a counselor with full access; the subject chooses when to face. Without this primitive, reach-drop becomes silent permanent disenfranchisement, which is brittle and unjust.

**Distinguishing from related primitives:**
- *submerge ≠ forget* — forget releases (audit-only); submerge holds, accessible to elohim
- *submerge ≠ quarantine* — quarantine contains-as-evidence (often known-bad, restitution-relevant); submerge changes visibility regardless of content character
- *submerge can co-exist with quarantine* — known-bad content can be both quarantined (for restitution) AND submerged (out of conscious view); orthogonal
- *surface ≠ promote* — promote moves up a tier (importance); surface moves up a visibility layer (access)

## Memory classes

**Lifecycle primitives are the operations. Memory classes are the defaults.** A film and a conversation use the same primitive vocabulary, but they live with radically different defaults — which primitives apply, at what cadence, with which governance authority. The class an entry declares determines half its design.

Every entity in the protocol substrate **must declare its memory class at creation**. Untyped entries are a design failure — they admit no principled lifecycle policy.

| Class | Examples | Lifecycle character | Primary primitives | Governance authority |
|---|---|---|---|---|
| **Contextual** | Conversations, decisions, in-the-moment interactions, household coordination, dev intent log | Comet-shaped, decay-active; `submerge`/`surface` heavy when content carries consequence; `forget` aggressive at tail. Days to months. | promote, compact, merge, submerge, close-interval, forget | Operator (personal), household-steward (shared) |
| **Archival / canonical** | Films, books, recorded music, photographs, recipes, scientific datasets, heirloom-as-data | Stable-by-design; `merge` heavy across network (dedup equivalent copies via content addressing); rarely `forget`; rarely `submerge`. Generations. | merge (network-scale), memorialize | qahal-governed distribution; mishpat when harm-class |
| **Identity** | Profile, attestations, displayed self, key material, recovery shares | Durable-while-you-live; evolves; `memorialize` for core; `close-interval` for superseded states | promote, close-interval, memorialize | Author + recovery-circle; some sub-classes structurally inviolable |
| **Relational** | Bonds, shared histories, trust signals, accumulated insight-with | Fades with cooling, warms with re-engagement; `close-interval` natural when relations end; never fully forgotten | close-interval, compact | Co-authored — both parties have stake |
| **Operational / transient** | System state, working buffers, in-flight task data, configs, CI artifacts | `forget` aggressive once operation completes; minimal audit | forget | Substrate-managed; minimal governance |
| **Attestation / truth** | Factual records, contracts, governance decisions, REA commitments, notarizations | `close-interval` heavy for supersession; **never `forget`** (historical record sacred); citation-anchored | close-interval, promote (to DHT), memorialize | Protocol-substrate-managed; immutability is structural |
| **Wisdom / principle** | Extracted learnings, manifesto-tier statements, principles, distilled experience | `memorialize`-by-default for core; slow-evolving; high earning threshold for new entries AND revisions | promote, merge, memorialize | qahal-promoted with explicit reviewer pass |

**Class + primitive matrix — selected examples**:

- A `merge` of two contextual entries requires operator consent and triggers all consolidation-event signals.
- A `merge` of two archival entries is a network-efficiency operation gated on content-addressing equivalence; no operator consent (no individual owns the artifact uniquely); freed compute redistributes.
- `merge` is **structurally not available** for attestation entries — `close-interval` (supersession) is the only path; original attestation remains queryable forever.
- `submerge` fits contextual memory (reach-dropped post) and relational memory (cooled friendship out of active view); is ill-defined for archival (an artwork doesn't leave culture because users stop engaging); is impossible for attestation (truth doesn't move to subconscious).
- `forget` fits operational and tail-contextual entries; is forbidden for attestation and identity-core; is rare for archival and wisdom.

**Class informs storage tier** (cross-reference protocol substrate decisions):

- Attestation → DHT (notarized truth layer)
- Archival → quilt / RS-distributed storage (resilience replicas across network)
- Contextual → household pantry (one household per user, with selective sharing)
- Operational → ephemeral / not persisted past operation lifetime
- Identity → DHT-anchored (durable, recovery-protected) with household-local active state
- Wisdom → DHT-promoted core + libp2p propagation for adoption
- Relational → bilateral household storage with REA-witnessed flow

When ambiguous (e.g., a household photo: archival because cultural, contextual because personal), default to the more-protective class (archival in this case) and require explicit declaration for variance.

**`/dream` v1** consumes the memory-class declaration when proposing lifecycle operations. Class-inappropriate proposals (e.g., proposing `forget` on an attestation entry) are filtered before reaching the operator review surface.

## Earning criteria

Lifecycle decisions are **principled, not arbitrary**. Each operation has earning criteria the proposal must demonstrate:

- **Promotion**: cited in K+ artifacts (specs/plans/scenarios/EPRs) across T+ time window; recurrence pattern stable across multiple authors or contexts.
- **Compaction**: feature shipped (scenarios green, branch merged); OR entry's body verbose relative to its principle; OR active references resolve to the principle, not the detail.
- **Merge**: detection signals concur (semantic neighborhood + shared citations + judgment); compression ratio honest (not hiding novel content as "redundancy"); both predecessors had earned their existence.
- **Memorialize**: recurrence in M+ specs across N+ months AND no contradiction surfaced AND project would change behavior if the principle were lost.
- **Forget**: no citations within freshness window AND superseded or genuinely transient AND audit trail acceptable.

**Repetition is the proof.** Earning is by reference, not assertion.

## Provenance and lineage

Every transformation **must preserve traceback**. The corpus is its own evidence.

- Every `promote` carries citations to the artifacts that demonstrated the earning.
- Every `compact` carries a pointer to the un-compacted source (git tag, file hash, archived path).
- Every `merge` carries lineage edges from the new head to each predecessor; predecessors carry "superseded-by" edges forward.
- Every `close-interval` records the date and the trigger (which referent moved, which successor was promoted).
- Every `memorialize` carries the citation set that earned it.
- Every `forget` carries the audit record (date, reason, scope).

A consolidation without provenance is forgery. The graph must remain walkable backward through every transformation.

## Authority tiers (governance)

**Forgetting and merging are governance decisions.** Centralizing them is the antipattern. Different scales need different authorities:

| Scale | Authority | Examples |
|---|---|---|
| Personal memory | Operator | The operator's own MEMORY.md and topic files |
| Personal EPR / authored content | Author | Own EPRs, own posts, own commentary |
| Sprint / project / household | Project steward (often operator + collaborators) | Sprint results, plans, dev-intent log |
| Collective principles | qahal governance | Manifesto-tier facts, protocol-level principles |
| Collective content | mishpat governance + author | Lamad content archival, scenario corpus |

The runtime must support each tier; no operation should require central permission for a scope where the authority is local.

## Generalizability targets

The same primitives govern lifecycle for every long-lived data type in the protocol:

| Data type | head/tail/core mapping | Primary primitives in active use |
|---|---|---|
| Claude/agent memory entries | recent/tail/manifesto-tier | promote, compact, merge, close-interval |
| MEMORY.md index entries | recent/tail/permanent | promote, merge |
| Specs (`genesis/docs/superpowers/specs/*.md`) | drafted/landed/canonical | compact (after ship), merge (related specs), memorialize (principle-shape) |
| Plans (`genesis/docs/plans/*.md`) | active/historical | compact, close-interval |
| dev-intent.jsonl | recent/coalesced | merge (related intents), forget (one-off) |
| Sprint results | latest/historical | compact, merge |
| EPR couplings | head/trail/durable | merge (subject coalescence), close-interval, promote-to-durable |
| DHT entries | hot/cool/notarized | close-interval (rotation), promote (durable-anchored) |
| Content nodes | live/archived/canonical | compact, close-interval, memorialize |
| Scenarios | active/passing/regression-anchored | compact, memorialize (regression value) |

The same proposal schema serves all of these. v1 (`/dream`) implements for memory entries; downstream consumers adopt the same schema.

## v1 implementation: `/dream` skill

The `/dream` skill is the dev-loop prototype. It implements the lifecycle primitives for Claude/dev memory, scoped narrowly to validate the design at a fast-feedback scale before graduating downstream.

**Inputs (corpus axis Anthropic Dreams won't natively touch):**
- `genesis/docs/superpowers/specs/*.md` since last dream
- `genesis/docs/plans/*.md` since last dream
- `.claude/data/dev-intent.jsonl` since last dream
- Sprint-result artifacts from `/shift` and `/deliver`
- `git log --since=<last-dream>` (commit messages, PR descriptions)
- Current `MEMORY.md` topic files (canonical state — to deduplicate against)
- Skill `SKILL.md` frontmatter and bodies (principle-shaped corpus)
- Manifesto headers (rare-velocity tier, for promotion-detection)

**Outputs (proposals, never auto-applied):**
Each proposal carries: `kind` (promote/compact/merge/close-interval/memorialize/forget), `target_entries` (1+), `proposed_change` (the new state), `receipts` (citations), `earning_demonstration` (why this meets the criteria), `authority_required` (who needs to approve).

Staged to `.claude/dreams/<YYYY-MM-DD>/proposals.md` for operator review.

**v1 scope (narrow on purpose):**
- Implements: promote, compact, merge, close-interval (the four most useful for dev memory)
- Defers: memorialize (rare; manual operator action for now), forget (high-stakes; manual)
- Detection for merge: starts with shared-vocabulary + shared-citations heuristics + LLM judgment; can adopt vector-similarity later if needed.
- No automatic application — every proposal is operator-reviewed.

**Out of scope for v1:**
- Reading sessions / conversation transcripts (Anthropic Dreams handles this surface)
- Cross-repo dreams
- Federated consolidation across households

## Forward references — specs that should adopt these primitives

The lifecycle schema is generalizable. As downstream specs are written, they should reference this spec and consume the primitives:

- **elohim-agent autonomous memory hygiene** — agent's own working/long-term memory uses promote/merge/forget under operator authority.
- **EPR substrate decay design** — EPR couplings use merge (subject coalescence), close-interval (supersession), memorialize (durable anchors). The compute-footprint sustainability case for the household network.
- **DHT entry lifecycle** — close-interval for rotation, memorialize for permanently-notarized facts, forget never (DHT-tier is by definition durable, but the *promotion* into DHT is gated by earning).
- **Lamad content archival** — content nodes compact and close-interval as superseded by new versions; memorialize for canonical reference content.
- **Mishpat scenario hygiene** — scenarios merge when redundant, compact after stable, memorialize when proven regression-anchors.
- **Shefa event compaction** — economic events compact into period summaries; raw event tail closes intervals.

Every consumer should declare its lifecycle policy explicitly in its spec: hot-tier criteria, decay function, merge candidacy, memorialization threshold, forgetting policy.

## Open questions

These need answers before the design is final, or as `/dream` v1 surfaces evidence:

1. **Detection threshold for merge** — what's the false-positive cost (incorrectly merging distinct concepts) vs the false-negative cost (missing legitimate merges)? Start conservative; let evidence calibrate.
2. **Minimum "earning" thresholds** — exact values for K (citations), T (time window), M (specs cited in), N (months). v1 picks defaults; evidence adjusts.
3. **Dream cadence** — per-sprint, weekly, on-demand only? Probably on-demand for dev memory; periodic for protocol substrate.
4. **Provenance representation** — JSON-Schema for proposals + audit records. Should align with view-schema conventions.
5. **Cross-tier merge rules** — can a manifesto-tier entry merge with a semantic-tier entry? (Probably not — promotion paths flow upward only; merges happen within-tier.)
6. **Rollback** — if a merge proves incorrect, can `c` be split back into `a` and `b`? (The lineage edges make this technically possible; the policy question is whether to support it.)

## Sources

- Brainstorm conversation 2026-05-10 (Matthew + Claude)
- Industry surveys: agent memory systems, RAG/graph context, Claude Code/peer codebase exploration
- Anthropic Dreams documentation (research preview, beta `dreaming-2026-04-21`): https://platform.claude.com/docs/en/managed-agents/dreams
- Project memory: agentic context graph model, comet-shape memory lifecycle, DHT vs libp2p scoping, trust as efficiency signal, reach earned at authoring, stewardship as graduated authority, ungrudging service, household horizontal scaling
- Field references: Zep Graphiti (temporal validity intervals), Mem0g (conflict detection at write time), Microsoft GraphRAG (Leiden community detection — analog to merge), GAM/TiMem (episodic→semantic consolidation, named-fragile)

## Status

Proposal. Pending implementation of `/dream` v1 to validate the design against actual corpus before promoting to "adopted." After v1 surfaces evidence, this spec graduates and downstream specs (elohim-agent, EPR, DHT, lamad content) can begin adopting the primitives.
