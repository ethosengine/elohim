---
title: "Backstitch — Realtime SCM Cross-Pollination: what a shipped CRDT version-control tool proves, and what it punts to us"
id: backstitch-realtime-scm-cross-pollination-2026-08-15
status: Capture
date: 2026-08-15
---

# Backstitch Cross-Pollination — August 2026

**[Backstitch](https://backstitch.dev)** ([inkandswitch/backstitch](https://github.com/inkandswitch/backstitch)) is Ink & Switch's **real-time version control for the Godot game engine**: an alpha editor plugin with a Rust core that stores a project as Automerge documents — branches ARE documents, scenes are parsed into structured CRDT trees, merges are structural auto-merge plus a mandatory human "Merge Preview" ceremony, and sync rides [samod](https://github.com/inkandswitch/backstitch-sync-server) over WebSocket/TCP to an optional relay. It is the field's first shipped answer we have surveyed to the question the operator asked this session: **is realtime SCM possible?**

The one-line verdict: **Yes — demonstrated, alpha-grade, for structured artifacts at small-team scale, and the demonstration is load-bearing precisely because of what it is NOT: it is not git (zero git objects, no history round-trip — "git compatibility" means two uncoordinated VCSs sharing one working directory), it is not keystroke-realtime (save-granularity, debounced), it does not abolish merge ceremony (CRDT auto-merge converges structure; a human Confirm gate owns semantics), and it has no identity layer (plaintext usernames, unauthenticated relay). The first three non-facts are design wisdom we adopt; the fourth is the hole our actor plane, steward slots, and brit `AgentKey` already fill — this pollination runs both ways.**

Method: repo cloned and read directly (registered in `research-manifest.json`; shallow clone, so activity history came from the GitHub API); `backstitch.dev` is a client-rendered SPA whose docs were recovered verbatim from the built JS bundle (primary text, but marked ⚠ web-only per house rules); one Sonnet external researcher + one Opus internal-grounding researcher fanned out; synthesis by the orchestrating session. **Verification key:** ✅ verified in source · ◐ single-source/plausible · ⚠ web-only/unverified.

---

## ⚠️ Name / lineage guard (read first)

- **`inkandswitch/backstitch` ≠ backstitch.io** — the latter is an unrelated HR/marketing company. The project's domain is **backstitch.dev**. ⚠
- **Product-in-alpha, not a lab probe** ✅◐: MIT, three named core engineers, Endless Foundation funding ("we're able to work full-time on Backstitch" ⚠), `v2.0.3-alpha` (2026-07-23), last push the day before this survey, 231 stars, near-daily cadence. An Automerge/samod core maintainer (alexjg) has commits ✅. Their own disclaimer: *"This is alpha-grade software"* with an explicit several-GB memory ceiling ⚠. Cite capabilities, not longevity.
- **No stated lineage to Patchwork or Cambria** ✅-by-absence: the connection is organizational (same org, same Automerge substrate). Do not cite a "universal version control" program — none is asserted in any primary text found.

## Anatomy (compressed; full digest grounded in the cloned repo)

1. **Data model — pure Automerge, zero git** ✅ (verified by absence: no `git2`/`libgit2` anywhere in the Rust tree). A project branch = a `GodotProjectDoc` Automerge document holding a `files` map; text/scene files inline as Text CRDTs; each binary asset is its own whole-content Automerge doc. Godot `.tscn`/`.tres` scenes are **parsed into typed node/property trees** and reconciled via `autosurgeon` — so concurrent edits to different nodes/properties merge cleanly **by construction**, and the node-level diff UI falls out of the data model.
2. **The "commit"** ✅ = an Automerge `Change` carrying JSON `CommitMetadata` (username, branch id, merge/revert markers, changed-file list) in the change message. Content-addressing is Automerge's SHA-256 `ChangeHash` DAG; a `HistoryRef` = `{branch: DocumentId, heads: Vec<ChangeHash>}`.
3. **Sync** ✅⚠ — local-first; samod repo dials `ws://`/`tcp://` to an optional relay (`alpha.backstitch.dev:8085`); **granularity is the file save** (100ms-debounced FS watcher, batched into one commit), not the keystroke. Offline works; the relay is availability, not authority.
4. **Branch / merge / revert** ✅⚠ — `merge_branch()` is literally `target_doc.merge(source_doc)` (Automerge structural merge), wrapped in a mandatory **Merge Preview → Confirm** human ceremony; revert is a **forward commit of the inverse diff**, previewed the same way — history is never rewritten. Scalar-level concurrent-write conflicts are deliberately left unresolved rather than auto-picked (*"multiple clients would thrash on it"* ✅ code comment).
5. **Identity** ✅ — a free-text `username: Option<String>` in commit metadata. No keys, no signatures (grep-verified absence), no auth on the relay (*"anyone can access your data, if they guess the Project ID… Look into ZeroTier or Tailscale"* ⚠ their own docs). No AI-agent framing anywhere.
6. **Their candid open problems** ✅⚠: an offline-edit overwrite race they are fixing with a **hash-based local filesystem index**; a checkout-vs-commit race narrated in-source as *"the Old Bad Way"*; the GB memory ceiling (whole binary assets as single docs — no chunking/LFS story).

---

## The question answered: is realtime SCM possible?

**Yes, with four load-bearing qualifications** — each one a design decision we inherit as evidence rather than re-derive:

1. **Realtime SCM is not git-compatible SCM.** Backstitch's own composition with git is *parallel coexistence*: two version-control systems, one working directory, no round-trip — git sees snapshots, backstitch sees changes. ✅⚠ The lesson is not "abandon git"; it is that **the realtime plane and the covenant/history plane are different planes and compose side-by-side rather than one absorbing the other**. That is exactly the composition shape available to us (below, and in the brit-native companion note).
2. **Realtime works where the artifact is structured.** Scenes-as-typed-trees is why their merges converge; text files get Text-CRDT line merging (git-equivalent luck); *semantic* conflict stays human. Our substrate is ahead of their starting line here: EPR atoms, content nodes, and manifests are already structured, canonical, content-addressed dag-cbor — **we are closer to backstitch's sweet spot than a source-code tree is**.
3. **Ceremony survives realtime; it relocates.** Continuous sync inside a branch; a witnessed Preview→Confirm gate at the branch boundary. This is independently convergent with our ratification-at-dev-merge acceptance act (Witness tier, recorded-not-judged) and with brit's "merge is a covenantal joining" framing — a shipped exhibit that the covenant model and realtime collaboration are not in tension.
4. **Identity is the unsolved half, and it is OUR solved half.** Plaintext usernames + VPN-as-auth is the degenerate attribution form this repo just spent a session superseding: session-scoped `ActorClaim`s with steward slots (`epr actor claim`, landed 2026-08-15), reach-filtered projection (`reach_is_distribution_safe`, fail-closed ✅), amber/green authority discipline (*"the converged doc value is unauthenticated peer input… would launder gossip into notarization provenance"* ✅ `projector.rs:470-477`), and a cryptographic ceiling already implemented in brit (`engine/signing.rs` `AgentKey`, ed25519, wired into three attestation writers ✅). **The export direction of this survey is as real as the import direction.**

## Grounded against our own build state (the internal researcher's findings, compressed)

- **We already run backstitch's substrate — as replication, not authoring.** The elohim-storage sync plane (`src/sync/`, 3,012 LOC + wire protocol) is Automerge 0.10 (same generation as backstitch ✅), one doc per content row, sled-backed, projected single-writer-per-fact from SQL, synced over libp2p request-response with a hand-rolled `save_after`/`load_incremental` delta protocol (NOT `automerge::sync` ✅) — and its authority discipline is explicit: CRDT is merge-safe transport; SQL + DHT notarization stay authority; exactly one marked heal channel back (`blobHash` → amber `crdt_converged_at`, never `dht_anchor_hash` ✅). **The gap between us and a backstitch-shaped authoring surface is not the sync engine — it is the branch-as-doc data model and the ceremony UI on top.**
- **The worktree pain this survey was commissioned against is real and quantified** ✅: ~13 recorded concurrent-session collisions in 3 months (index races, a 146-file bulk revert, amend-rewrites-their-commit, 5 CI builds killed by push races, path-limited commits failing at sub-file granularity), a single-integrator push bottleneck, and a five-worktree live tree where other-vendor agents get isolation and Claude sessions share the root by doctrine (operator-visibility). Backstitch's two in-source races (editor-closed overwrite; checkout-vs-commit) are the *same failure class* — filesystem-as-shared-truth with concurrent writers — and their fix direction (all writers through the doc plane; a hash-based FS index reconciles disk against it) is the same trajectory our `.eprfs` content-addressed sidecars already walk.
- **lvi is the eventual home of the dev-loop half, and it is docs-only today** ✅: its spec replaces the source *registry* (GitHub→p2p blob plane) and delegates git/covenant semantics to brit; nothing in it addresses co-editing or worktrees. A realtime dev-workspace plane is unclaimed design space there, not a contradiction of anything written.

## Take / Watch / Leave (seam-routed)

**TAKE** (minted to clusters — see Outputs):

1. **Branch-as-doc + `HistoryRef {branch, heads}` + Merge-Preview ceremony** as the data model for a **collaborative authoring plane over structured content** (lamad content, specs/plans, graphos stories) — riding the existing sync plane, NOT the SQL-authoritative projection path; authority discipline (amber/green, reach filter) carries over unchanged. Route: dataplane borrows cluster; p2p-design-gate mandatory (it births a doc-namespace entity and a ceremony act).
2. **Revert-as-forward-inverse-commit** — confirms our append-only corrections discipline (notes, claims, holds) at a second site; adopt the *name* for any future content-plane revert. (Not worth a cluster row alone; recorded here.)
3. **The FS-index lesson applied to our worktree contention**: the durable answer to concurrent-writers-on-one-tree is writers-through-the-doc-plane with the filesystem as projection — lvi-generation work, brit-`NodeSeed`-sealed, not a near-term mechanism. Near-term, our stigmergic `.epr-meta` damping + path-limited commits remain the standing answer (the context-engineering survey's hub+stigmergy finding stands). Route: harness borrows cluster, as evidence attached to the stigmergy row.

**WATCH:**

- **samod** — the extracted Automerge sync engine under backstitch (server repo public). Our hand-rolled delta protocol carries a named standing red (*"sync-scale-honesty"*: the round opener still re-enumerates the corpus ✅ `p2p/mod.rs:7779`); when that red is next worked, evaluate samod's sync-state machinery as prior art before growing ours. Same automerge generation both sides ✅.
- **Keyhive** (their access-control/E2EE line) — the auth half backstitch punts; sits beside `p2panda-encryption` in our unbuilt confidentiality plane's candidate list. Organizational-lineage only; no code read this survey.

**LEAVE:**

- **Plaintext-username attribution and unauthenticated relay** — the degenerate forms our actor plane and reach/consent planes exist to supersede. Documented here as the field's live exhibit of why an identity floor matters *before* scale.
- **Whole-binary-assets-as-docs** — their GB ceiling is the cost; our blob plane (separate protocols, inventory-vs-bytes decoupling) already refuses this shape.
- **Keystroke-realtime ambitions** — backstitch itself chose save-granularity; nothing in our problem space demands finer.

## The brit composition (settled questions — full evidence in the brit-native note)

The four questions this session's parent work left open are now settled from brit's tree at `40aa3ddc` and recorded in **[brit `docs/research/backstitch-realtime-scm-brit-composition-2026-08-15.md`](../../elohim/brit/docs/research/backstitch-realtime-scm-brit-composition-2026-08-15.md)**: the trailer key registry is two const arrays + an `AppSchema` trait (the schema-driven registry of the design doc is unimplemented; `Co-Authored-By` is explicitly pass-through today ✅); **`AgentKey` is implemented and wired** (the phase-2a plan doc is stale ✅); **commit-lift is not built** (`brit-bridge` designed, deliberately deferred ✅) so epr-cli's `git log` reading remains the interim seam — with the noted risk that the repo now has **two independent trailer readers** (git `%(trailers)` in epr-cli; gitoxide `BodyRef::trailers` in brit-epr) with disjoint key vocabularies, the exact BritCid/BlobCid divergence shape the 2026-07-12 consolidation spec warns about; and the Nexus dependency path for consuming `brit-epr` exists and is documented, with the forked-`gix-object`-at-same-version caveat ✅. The composition verdict in one line: **brit is the covenant plane realtime lacks, backstitch is the collaboration plane brit never claimed — parallel planes, one working surface, ceremony at the boundary.**

## Outputs (mint pass)

- Row **11** minted to [arch-dataplane-borrows-backlog](../data/timeline/backlog/arch-dataplane-borrows-backlog.md) (branch-as-doc authoring plane, p2p-design-gated).
- Row **10** minted to [agentic-harness-borrows-backlog](../data/timeline/backlog/agentic-harness-borrows-backlog.md) (worktree-contention evidence + doc-plane trajectory, attached to the stigmergy row's subject).
- Brit-native companion note committed in the brit repo (`docs/research/`), first file in that directory.
- `research-manifest.json`: `backstitch` repo registered (cloned shallow).
- Takes 2 and the WATCH items die honestly in this prose per the cluster discipline.
