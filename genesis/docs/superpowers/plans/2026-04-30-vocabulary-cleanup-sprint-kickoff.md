# Vocabulary Cleanup Sprint — Kickoff

**Created:** 2026-04-30
**Why this sprint exists:** Before dispatching the parallel substrate-replication + doorway-cache team for the "where's my thumbnail" fix, we need stable design vocabulary. Without it, the two agents converge on subtly-different word choices for new things they create (signals, events, configmap keys, admin endpoints, tracing spans) and we end up with another inconsistency to clean up. With this sprint done, both agents work from the same dictionary.

There's a deeper reason too: the `blob/store/s3` framing is hostile to the protocol's actual shape. "Blob" implies "opaque lump in a bucket"; "store" implies "destination you PUT to." The protocol is none of those things — it's peers stewarding shards in pantries, weaving redundancy through reed-solomon. The vocabulary should make the truth visible.

## Goal

Land three things:

1. **Design vocabulary register** — formal entries in `genesis/graphos/` for: `quilt`, `pantry`, `stock`, `draw`, plus the relationships to existing terms (`blob`, `store`, `upload`, `download`).
2. **Naming-collision resolution** — `weave` rejected (collides with Moss `@theweave/api` / Weave Tool); `quilt` chosen. Resolved 2026-04-30 in Task 1 below.
3. **Legacy path consolidation** — migrate `/store/<hash>` and `/api/blob/<hash>` callers (~11 TypeScript files) to canonical `/blob/<hash>`; delete the legacy doorway dispatch arms. This is out-of-scope follow-ups #1 and #2 from `2026-04-28-doorway-blob-registry-routing.md`, brought into scope here because vocabulary cleanup is the natural moment to retire competing path families.

## Out of scope (deliberately)

- **P2P byte replication (Gap 1)** — separate substrate sprint after this
- **Doorway blob-tier cache write-on-fetch (Gap 2)** — separate doorway sprint after this
- **Renaming Rust types** (`BlobStore`, `blob_path`, `blob_hash`) — internal Rust language stays. New vocabulary applies to design docs, signal/event names, user-facing strings, and NEW concepts that don't yet exist in code.
- **Wire-level addressability** (`sha256-{hex}`, CID) — universally understood; do not touch
- **Protocol schema enums / DNA constants** — orthogonal
- **Orchestrator `lastSuccessfulCommit` advance bug** — separate diagnostic; logged at the bottom
- **HEAD `/blob/<hash>` 404 mismatch** — small route-registration bug; logged at the bottom

## Tasks

### Task 1 — Resolve the `weave` collision (RESOLVED 2026-04-30)

**Decision: `quilt`.**

**Rationale:**

1. **The Moss collision is concrete, not hypothetical.** RNO sub-project #8 (Moss Weave Tool packaging, lamad-as-applet) is High priority on the cross-wave guidance with a concrete v0.4.0 reference implementation pattern: `@theweave/api 0.6.3`, `weave.service.ts`, `weave.store.svelte.ts`, a `weave/` directory with `weave.dev.config.json`. Sharing `weave` would collide in identifier space, not just prose.
2. **The substrate spec already treats Weave as Moss-specific.** `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md:475`: *"Moss groups federate via Weave for group-scale coordination. Elohim serves ecosystem-scale graph composition across such groups. They compose."* Adding elohim-`weave` on top muddles that composition framing.
3. **`quilt` fits the metaphor better than `weave`.** A quilt of N patches naturally survives losing some — matches RS(N,K) reconstruction. Threads in a weave do not.
4. **`quilt` fits the register.** Domestic; pairs cleanly with `pantry`/`stock`/`draw`. Reinforces the household-as-resilience-unit framing.
5. **`quilt` is collision-free.** The only existing reference is a content scenario about a person joining a quilting affinity network — not a protocol concept.

**Verb pairings:** `quilt content into N shards` · `the quilt for content X` · `re-quilt` (restitch after losses) · `RS(N,K) quilt`.

**Eliminated alternatives:**

| Candidate | Eliminated because |
|---|---|
| `weave` | Moss `@theweave/api` / Weave Tool collision; identifier-space conflict |
| `lattice` | Already taken — "the holonic lattice" for cross-collective governance (`genesis/plans/2026-04-10-collectives-schema-design.md`) |
| `weft` | Poetic but obscure; readers won't know it |
| `scatter`/`gather` | Loses the redundant-fabric frame |
| `rs` | Too clinical; doesn't make the truth visible, which is the point |
| `rsweave`/`eweave` | Awkward in prose; preserves the collision in the root word |

Transcribe this rationale (condensed) into the vocabulary register in Task 2.

### Task 2 — Write the design vocabulary register

Add a "Design Vocabulary — Storage & Distribution" section to `genesis/graphos/elohim-protocol-design-spec.md` (or a new `genesis/graphos/vocabulary.md` if you want to keep it separable). For each term, state:

- **Meaning** at protocol level
- **Verbs it pairs with** ("you stock a pantry", "you draw from a pantry", "you quilt a content unit into N shards")
- **Relationship to wire-protocol terms** it replaces or coexists with
- **Where it appears** (specs, narrative, signal/event names, code identifiers); where legacy terms stay (HTTP routes, internal Rust types like `BlobStore`)

Initial entries:

| Term | Meaning | Pairs with | Replaces |
|---|---|---|---|
| `quilt` | RS-encoded distribution of a content unit across N shards, any K of which reconstruct | "quilt N-of-K", "the quilt for content X", "re-quilt" (restitch after losses) | conceptually replaces "blob as monolithic unit"; doesn't replace `/blob/<hash>` HTTP route. Distinct from Moss `weave`/`@theweave/api` — see Task 1 rationale. |
| `pantry` | Peer-tended container that holds shards on behalf of the household; multiple households tend overlapping pantries | "stock the pantry", "draw from the pantry" | replaces "bucket", "store-as-destination" |
| `stock` (verb) | Deposit content into a pantry | "stock the local pantry with this quilt" | replaces `upload` (where new) |
| `draw` (verb) | Retrieve content from a pantry | "draw the thumbnail from the pantry" | replaces `download` (where new) |
| `shard` (already in use) | One piece of an RS-encoded quilt; held by a peer; addressed by `sha256-{hex}` of its bytes | n/a | unchanged; document the relationship: a quilt is N shards |
| `RS(N,K)` | Contract policy: N total shards, any K reconstruct; archetype-tunable per "Cadences are archetype-tunable" memory | n/a | new; replaces "S3-style replication factor" framing |

The register should also state the **boundary rule**: wire-level (HTTP routes, file paths, Rust struct names) keeps existing terminology because it's externally legible (HTTP `/blob/`, IPFS-style addressing). The new vocabulary applies one layer up — design discussion, signals, events, narrative, anything we're inventing fresh.

### Task 3 — Migrate legacy `/store/` and `/api/blob/` callers

**11 TypeScript files** still reference legacy paths. Audit list:

```
app/elohim-library/projects/elohim-service/src/connection/connection-strategy.ts
app/elohim-library/projects/elohim-service/src/connection/connection-strategy.spec.ts
app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts
app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts
app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts
app/elohim-app/src/app/elohim/services/storage-client.service.ts
app/elohim-app/src/app/elohim/services/storage-client.service.spec.ts
app/elohim-app/src/app/elohim/services/doorway-client.service.ts
app/elohim-app/src/app/elohim/services/doorway-client.service.spec.ts
app/elohim-app/src/app/elohim/services/epr-resolver.service.ts
app/elohim-app/src/app/elohim/services/content.service.ts
app/elohim-app/src/app/elohim/services/helia-fetch.service.spec.ts
genesis/seeder/src/blob-manager.ts
genesis/seeder/src/doorway-client.ts
```

Replace `/store/${hash}` and `/api/blob/${hash}` with `/blob/${hash}`. Run elohim-app vitest + seeder tests + doorway-service `cargo test` to confirm.

Then in `doorway/doorway-service/src/server/http.rs`, delete:
- `(GET|HEAD) /store/*` arms (~lines 1322-1346)
- `(GET|HEAD) /api/blob/*` arms (~lines 1348-1390)

The wildcard `classify_dispatch` arm picks up `/blob/<hash>` via the registry — already shipped 2026-04-28.

### Task 4 — Document & commit

- Update `doorway/CLAUDE.md` to remove legacy-paths language now that they're gone; cross-reference the new vocabulary register
- Update `elohim/elohim-storage/CLAUDE.md` (and any other relevant CLAUDE.md) to reference the vocabulary register where they currently use `blob`/`store` for design-level concepts
- Commit with conventional message; push; let the orchestrator pick it up

## Done criteria

- [x] Task 1: `weave` rejected, `quilt` chosen; rationale recorded above (transcribe condensed form into the register in Task 2)
- [ ] Task 2: `genesis/graphos/...` has a vocabulary section covering ≥6 terms with meanings, verb pairings, replacement relationships, and the boundary rule
- [ ] Task 3: zero references to `/store/<hash>` or `/api/blob/<hash>` in the audited TS files; doorway dispatch has only the canonical `/blob/<hash>` arm via registry
- [ ] Task 4: CLAUDE.mds cross-reference the register
- [ ] Build green, tests green, alpha deploy unchanged in user-visible behavior

## When this sprint completes

Report back to the chat: confirmation that the legacy paths are gone, the vocabulary register link, and any naming questions that surfaced during Task 3 migration. Then we relaunch the two-agent team for Gap 1 + Gap 2 — both speaking the same language.

## Out-of-band follow-ups (NOT in this sprint, just logged)

1. **Orchestrator `lastSuccessfulCommit` advance bug** — orchestrator advanced past 44e02608 even though no pipelines actually ran in #765 (the `matchesGlob **` corruption + empty-graph case bypassed the gate at Jenkinsfile lines 699-704). Separate diagnose-and-fix once this sprint and the two-agent team have shipped.
2. **HEAD `/blob/<hash>` returns 404 when GET returns 200** — route-registration bug in `elohim-storage/src/http.rs:533` (only registers GET, HEAD falls through to a 404 catch-all). File-and-forget; touch when whichever sprint next modifies that handler.
