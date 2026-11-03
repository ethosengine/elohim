---
id: "backlog-brit-daily-driver-handoff-2026-10-27"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Next sprint: daily-drive brit — the social and evidence layer around commits, built on the steward-space landing plane"
slug: "brit-daily-driver-handoff-2026-10-27"
written: "2026-10-27"
author: "claude-opus-5-5 (steward-space design session), at the operator's direction"
status: "backlog"
priority: "high"
area: "scm / brit / rakia / steward space"
tags: [brit, scm, handoff, rakia, landing-plane, attestation, change-id, walkthrough, cas]
cites:
  - "steward-space-landing-plane-design | the spec whose slices A6-A9 each win in this handoff serves (change ids, verdict reuse, walkthroughs, proposal notes, CAS heads) | sha256:6ef574355778bebd | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
  - genesis/data/timeline/backlog/gate-merge-gradient-reading-2026-10-25.md
  - genesis/data/timeline/backlog/brit-act-level-attribution-identity-chain.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
---

# Daily-drive brit: the social and evidence layer around commits

> Operator, 2026-10-27: *"I'd really like to daily drive brit and get it ready for those exciting wins."*

This is a handoff for the sprint after the steward-space landing plane
(`2026-10-27-steward-space-landing-plane-design`). It integrates with that spec's Arm A slices. It
does not fork them.

## Why brit, and what brit is for

The 2026-10-27 SCM horizon scans read the field in three camps:
- agent-scale centralisation: Anthropic Scion, GitLab next-gen SCM, Garden, code.storage;
- social objects replicated beside code: Radicle collaborative objects (COBs), git-bug, jj, Willow, Keyhive;
- agent provenance as a git sidecar: Entire, GitLab Governance for Agents.

The full reports are in the operator's plans folder:
- `/projects/.claude-config/plans/sunny-growing-quail-agent-a9e65fb1d1919cc24.md` (named systems);
- `/projects/.claude-config/plans/sunny-growing-quail-agent-ae574755886092133.md` (p2p SCM).

Their consistent message is that brit should not race to replace git transport. Its job is to own the
**social and evidence layer around commits**: proposals, reviews, attestations, walkthroughs.

- **Already built (pin 76ba95d8a):** a signed, content-addressed note store under
  `refs/notes/brit/{build,validate,deploy,reach,covenant,meta}`, which CI already trusts (`brit-attest.mjs`,
  `gate-evidence.mjs` reuse by inputs hash, `delivery-series.mjs`, `just measure`'s validate note).
- **Only planned:** `brit attest`, `Reviewed-By:`, `MergeProposalContentNode`, git over p2p, fork governance.
- **Not specified anywhere:** review, issue and discussion objects.

The SCM-side primitives from the scans mostly did not reach the spec's slices (stable change id, CAS
with expected-prior head, walkthrough records, sparse fetch, derived data). This handoff carries them.

## Operator intent — first-class native blob storage (2026-11-02)

**Open, high priority; capture for the next Brit planning pass.** The operator
clarified during the first Brit dogfooding sprint: Brit is intended to be the
reference SCM tooling for the Elohim-native developer. The blobs it works with
are expected to be stored on the P2P datastore and cached locally in the EPRFS
filesystem projection. This is core product intent, including large-file support,
not an optional future LFS storage bridge. It extends the social/evidence emphasis
above; that emphasis must not exclude native content storage and projection.

Git LFS is an interoperability surface for existing repositories within this
direction. Its pointer format and content hashes must not become a second native
storage authority. Reuse the existing EPRFS addressing, storage and projection
interfaces; establish the current implementation before proposing new ones.

Desired developer outcomes to turn into executable acceptance scenarios:

- A developer checks out a revision and obtains its file bytes through the local
  projection, retrieving missing blobs from the P2P datastore and verifying them.
- Staging changed content enters the native blob-storage lifecycle. Work remains
  possible offline with locally available content, and missing content produces
  an explicit availability result.
- Publication distinguishes a locally recorded commit from the availability of
  its required blobs to the intended peers. Define and test the required
  availability evidence before claiming that publication is complete.
- Existing Git LFS content can cross the compatibility boundary with byte/hash
  verification. Git object IDs, LFS SHA-256 identities and EPRFS CIDs are not
  interchangeable without an explicit, verified mapping.
- Local cache eviction and subsequent retrieval preserve content identity;
  corrupt or unavailable bytes cannot be presented as a successful checkout.

Planning handoff: ground this in Brit's existing daily-driver habit and the
current EPRFS/storage seams, then apply the P2P and local-first design gates before
choosing entities, routes, retention rules or publication semantics. This entry
records operator intent and desired outcomes, not a completed design or capability.

The current dogfooding fix for `brit diff --cached` refusing on unchanged
`diff=lfs` fixture files remains independently useful and should continue. Its
completion does not fulfil this native-storage requirement. Preserve that sprint's
friction evidence while graduating this intent into a bounded implementation slice.

## Initial daily-driver prerequisites

- **Pin the brit checkout to its pin.** The working tree was 385 commits behind
  (`git submodule update elohim/brit`). Re-read anything cited from the off-pin checkout.
- **Daily-drive means brit is on every device's path.** A pinned `brit` and `brit-build-ref` must resolve
  through the bin store like `epr` (the session pin already reports a brit CID), on both ethosengine and shem.

## Operational agility: brit off GitHub, and auto-update on both devices

> Operator, 2026-10-27: *"resolving brit's gh dependence on releases/artifacts and getting our auto-update action going on for brit in both our workspaces too for that operational agility."*

**Today, as read from code.** The device manifest
(`genesis/orchestrator/manifests/runtime/workspace-device.manifest.json`) declares
`follows: [{channel: bin:brit, mode: apply}]`. brit's source is `kind: gitlink` at `elohim/brit` with
`releasePrefix: brit-build-` and admission attestation `github-checks ethosengine/brit "Tests pass"`.

When the pin moves, the bin store (`elohim/eprfs/device-runtime/src/bin_store/lifecycle.rs:440-510`) gets
the bytes in one of two ways:
1. Build from the exact pin: cargo offline, then anonymous, then authenticated, `--profile release-github`,
   using the Nexus cargo registry (`source.rs:407`).
2. Fall back to `gh release download brit-build-<pin12> --repo ethosengine/brit`.

On ethosengine the brit store holds one release (`current.json`: landed 2026-10-26, `previous: null`), so
following brit has never been exercised here.

**GitHub sits on the path in three places:**
- admission: the GitHub check API;
- bytes: `gh release download`;
- naming: the `release-github` profile and Actions-built release.

**Step 1, now: constant latest snapshot artifacts from the existing pipeline.**

> Operator, 2026-10-27: *"we might reuse.. but just start getting constant latest snapshot artifacts from that pipeline going."*

- Reuse brit's current CI.
- Make it publish a `brit-build-<sha12>` snapshot release (the three bins) for EVERY commit that passes on
  its integration branch, not only for pins.
- With a snapshot always present, the bin store's download leg never strands, and `follows bin:brit apply`
  can run on every pin move on both devices.
- Measure it as "pin moved → both devices follow".

**Step 2, then: the native path, using the landing plane's own primitives:**
- **The device that builds brit at the pin publishes the release into the steward space** as a rakia
  artifact. It is the bin-store release manifest by CID (bins `brit`, `brit-build-ref`, `brit-verify`),
  chunked on the dataplane, and its sibling fetches it over the eprfs tier. The `gh release download`
  fallback becomes "fetch the sibling's release by CID", and GitHub releases become one more source.
- **Admission is a brit attestation of that build** (A8's shape: subject = pin closure, output = release CID):
  - On the real network of one, a sibling's attestation is enough to *adopt* a release on another of the
    steward's devices. It is labelled `independence: sibling`, and the trust is the steward's own.
  - The upstream GitHub check stays as one witness. It becomes evidence, no longer the gate.
  - Promotion beyond the grant (S2+) still needs independent attestation.
- **Auto-update on both devices** is the existing `follows bin:brit apply`, driven by pin moves on dev.
  Exercise it for real:
  - a pin bump lands on dev;
  - each device's bin store follows within one session start;
  - `just status device` reads `brit binary … (current)` on ethosengine and shem;
  - the predecessor stays available for revert.

  Apply the 2026-10-27 lessons from the epr follow:
  - prove the new release's exec floor before switching;
  - keep a fallback to the last admissible release;
  - land a release only after its declared bins are complete (the `bootstrap admissible()` stranding).

**Done when:**
- **Step 1:** every passing brit commit has a snapshot release, and a brit pin bump on dev reaches both
  devices' bin stores, with either device able to revert to the predecessor.
- **Step 2:** the same holds with no `gh` call and no GitHub check in the admission path.

## The wins, in order (each small, each on the landing plane's critical path)

| # | Win | Integrates with | Borrowed from | Done when |
|---|---|---|---|---|
| 1 | **Landing intents as brit notes**: `refs/notes/brit/proposal/<intent-cid>` carrying {base, tip oid, closure keys, evidence, actor, lane, state}. A working first form of `MergeProposalContentNode`: a pull request with no GitHub, written by agents and invisible to Jenkins | spec A9 (landing-intent queue), D3 (no forge branches) | Radicle COBs, git-appraise | two devices queue, see and land each other's intents with no forge PR; `brit` lists them |
| 2 | **Sibling verdict reuse as the build cache**: a green gate is a brit attestation whose subject is the closure key and whose `output_cid` is the artifact CID (rakia R12 c). A `binds-signer` roster row admits the sibling's signer for reuse, labelled `independence: sibling` and never counted in k-of-n | spec A8, A0 probe 1 (closure keys must match at 100 %) | in-toto Statements, Turborepo cache semantics | a closure gated on one device is reused on the other with a named attestation, not re-run |
| 3 | **Stable change id trailer** (`Change-Id:` or a brit-native key): claims, intents and attestations key on it, so a rebase no longer orphans them | spec A6, A9; `brit verify` trailer validation | jj change ids, Gerrit | an intent and its attestations survive a rebase onto a moved dev |
| 4 | **Walkthrough record per handoff**: {claims, proof kind + proof (attestation CID or build number), `unproven`, unaccounted residue} as a brit note. The fruit that crosses, never the transcript | spec A7 (offer), §5.7, "thoughts private, fruit judged" | Garden SCM walkthroughs, Anthropic's review tool Antique | the next device-to-device code handoff is checkable from its walkthrough alone |
| 5 | **CAS with expected-prior head on every shared head**: lease, lane, rakia channel head and the dev ref move each cite the head they expect; a stale writer is refused and named | spec A3, A9; the 2026-10-27 launcher stranding (two readers of "verified" disagreed about one head) | Anthropic Scion ref moves, code.storage `expectedHeadSha` | a stale writer is refused with the head it missed, never silently wins |

Each win also emits a ValueFlows event beside its record (produce, use, deliver-service), accounted to
the principal with agent provenance per the spec's A5. Each artifact carries its upstream contribution
edges (spec §5.7 guidestar).

## Native storage: source to explicit execution (2026-11-02)

This section owns the native-storage progression retained in the operator's
[source-loop plan](../../../docs/superpowers/plans/2026-11-02-brit-native-source-loop.md).
It complements the social/evidence work above. A local filesystem working copy
remains editable; an explicitly selected candidate has a canonical Tree v1 CID
and complete private custody. Capturing or restoring it publishes nothing and
executes no hooks. An explicit `brit build run` delegates the fixed whole tree and
original task specification to Ark's existing admission and supervisor. Its local
outcome is neither an acceptance decision nor a reusable build attestation.

The captured source, output and executable boundaries retain the 64 MiB object
and 1 GiB expanded closure caps. EPRFS owns filesystem capture/restore and canonical
storage; Brit consumes released 0.3.3 registry interfaces; Ark owns execution.
No sibling dependency override or competing collaboration model is introduced.

Measured evidence and frontier:

- Capture `a64aa39627170cc7a91de3207ebc3bed30b34da8` and durability
  `34fb6a57db18c36d23c3fbde5d7e1244b731dd04` are committed. The continuation
  release gate and publication succeeded before consumer work. Retained capture,
  restore and storage tests exercise representation, observed races, bounded
  objects, sync failures, process interruption and residue; they do not claim an
  atomic snapshot of an independently changing source.
- Native-task parent `76554ccafc510e2a63226733fe2a0eacdc6f94e9` binds Brit
  `4730ba4b6b29365e3e9161853d85c0f45994e572`. Current review identified one
  late-interruption coverage gap; the repaired Ark CLI suite passed 21 unit,
  15 smoke and 8 native integration tests in the retained continuation evidence.
- The frozen primary oracle at 20:54Z returned **0**: installed Brit ignored
  `ARK_BIN`. Acceptance-story repairs that transport choice and adds an empty-PATH
  regression, local Cucumber stations and independent declaration-identity checks.
  The fixed oracle binary remains stale until the host's post-story build pair;
  a focused scratch binary does not establish a frozen oracle success.
- Acceptance-story measured **6 local scenarios / 28 steps**, **8 ordinary Brit
  journeys plus 1 explicit real-Ark journey**, and **45 Ark CLI tests** (21 unit,
  15 smoke, 9 native), all passing. New-step ESLint and strict TypeScript, scoped
  Brit Clippy and Rust suppression policy pass. An earlier full a2o typecheck
  failed on installed storage-client declarations; the verification addendum below
  supersedes that blocker. No whole-owner green is inferred. Exact commands,
  binary/source hashes and retained failed attempts are
  indexed in `acceptance-story-evidence.json` in the current shift directory.
- [The owning sprint result](../../../docs/superpowers/sprints/2026-11-02-brit-native-source-loop-result.md)
  carries exact runnable commands, current focused results and the fresh-runtime
  successor journey. Raw current logs live under
  `.claude/shifts/brit-c1-20261102/`; predecessor evidence remains immutable in
  `.claude/shifts/brit-native-loop-20261102/`.
- Broader `shared-working-versions.feature` stories remain `@wip`. Local proof
  establishes no Avodah editor, peer freshness, shared-edit authority, publication
  or independent acceptance. Final owner gates, stable frozen measurements and
  appointed acceptance remain pending. Upstream licensing remains blocked by the
  operator's decision. The daily-driver habit stays red.

Verification addendum from the continuation's independent acceptance-story review:
the host's ignored dependency artifact rebuild passed in 17.517 seconds, followed
by full `pnpm run typecheck` in `genesis/a2o`, exit 0 in 40.877 seconds
(`a2o-storage-client-rebuild-20261102T210627Z.json`). No source, configuration or
lockfile repair was needed. Preserve the earlier failed logs; final owner gates
remain required. The same review identifies an unresolved Windows regression in
published EPRFS 0.3.3 `DirectoryStorage`: directory synchronization opens an
unsupported handle, and deduplicated writes sync a read-only handle. Existing
Windows `snapshot seal` fails with Access Denied before Linux-only restore
(`windows-storage-diagnosis-20261102T205356Z.json`, CI job 125528586847).
The required station between durable custody and cross-platform snapshot
compatibility is a verified Windows namespace-durability implementation plus
passing create/new/repeated/concurrent-put and existing shared-tree tests, followed
by a new immutable release and consumer upgrade. It remains unresolved; Linux
success cannot clear it, and the separate licensing refusal remains in force.

Successor stations are explicit and ordered:

1. **Root-pinned peer hydration:** fetch the complete requested closure through an
   explicitly selected serving store, never advertise the whole private store.
2. **Large-file custody and cache lifecycle:** account for retention and eviction
   without raising the existing 16 MiB peer frame size.
3. **Native working contexts/history:** use existing EPR/REA records and keep
   Automerge as the shared working-version direction.
4. **Publication and availability:** require actual standing, publication authority
   and peer-held bytes rather than infer them from a successful local capture.
5. **Git/LFS interoperability and reusable builds:** bridge the same source identity
   and complete execution closures before allowing evidence reuse.

## Later social/evidence work

- **Review and discussion objects** as op DAGs (Radicle COBs) at S2, held by the collective's members, never on the lamad `Content` valve.
- **Derived data as pure functions of CIDs** (Scion): blame, attribution and folds that any peer computes and verifies.
- **Want-based sparse fetch for agent sessions** (Scion, GitLab, Garden): the device keeps the full replica; sessions read through it.
- **Git over p2p** (brit phase 3): bundles over the eprfs tier (spec A7b) are the stepping stone.
- **Merge tallies and fork governance in brit:** wait for qahal's tally home and the mishpat `CouncilRoster`. brit reads consent requirements and never owns governance (brit spec, composition.md:66).

## Not building

- Session or transcript capture beside commits (Entire). A trailer may carry a hash commitment to a private session record; never the transcript.
- A brit-owned governance or tally engine.
- A forge-hosted queue, PR bot, or Jenkins branch per intent.

## Open question for the sprint's first decision agent

Should the proposal namespace live in brit (`refs/notes/brit/proposal`) or as typed epr records on the
steward space, with brit notes as the degraded transport? The scans favour one replication substrate,
social objects next to code. Decide at the sprint's first milestone (operator ruling 2026-10-27:
resolve forks in flight, behind a swappable interface).

## Reconciliation, 2026-10-29

The ngit survey's challenge pass (`genesis/research/ngit-nostr-git-forge-cross-pollination-2026-10-29.md`) re-read this handoff against source and against the closure-keyed landing-plane design (Draft, another session's worktree). What changed:

- **The daily driver was blocked one layer below these wins.** brit cannot open /projects/elohim (gix fixes the Reduced-trust 32-slot default before `safe.directory` upgrades trust; the repo has 41 packs), `brit add` refuses everywhere because LFS drivers are configured globally, and `build find/get` admit notes without verifying their signatures. The sprint `genesis/docs/superpowers/plans/2026-10-29-brit-daily-driver-evidence-layer-sprint.md` (brit branch `feat/daily-driver-evidence-layer`) takes those first.
- **Win #1 (landing intents as brit notes)** is superseded: the closure-keyed design makes the landing intent an `Intent` atom in the device's flow store (S1b). The open question at the end of this handoff is answered there. NIP-34's lifecycle is review input to that owner; status must be a derived fold, never an authored record.
- **Win #2 (sibling verdict reuse)** is closure-keyed S2a; note that decision D5 rules out a `binds-signer` row at S1, which this handoff still names. Reuse is only trustworthy once `build find` verifies (sprint G2).
- **Win #3 (Change-Id)** is closure-keyed S1b.
- **Win #5 (CAS)**: the `dev-integration` window is closure-keyed S1b; brit's own note and ref writes get an expected prior in sprint G3.
- **Operational agility, Step 1**: the producer already exists as brit commit `2d4acccb08` on `feat/build-artifacts-per-commit` (pushed, unmerged); it publishes on main pushes only, not every passing integration commit. Step 2 (brit off GitHub) is closure-keyed plan row 11; the missing node is a brit `ValidationAttestation` for brit's suite at the pin closure, produced off GitHub.
- **Stale citations**: the `gh release download` leg is now `bin_store/lifecycle.rs:463-497`; the registry credential is `source.rs:384`.

### Handoffs from the 2026-10-29 sprint (brit branch `feat/daily-driver-evidence-layer`)

Each item belongs to another owner; none was changed by the sprint.

- **Closure-keyed owner (pre-push Stage 1a):** once brit opens the monorepo, the brit advisory leg runs a real `brit verify`, measured at 17.4 s wall / 68 s CPU on /projects/elohim. Bound or retire the leg before the pin moves.
- **Closure-keyed owner (readers):** `gate-evidence.mjs:264` and `ark/cli/src/brit_index.rs:225` discard stderr on success, so they never see `unverifiable:` lines; `delivery-series.mjs:118-153` and `epr walk` read notes raw and bypass verification. Route them through `build find/get`. Move the seed readers (`gate-evidence.mjs` selfAgentId, `household-attestation.ts` workspaceAgentId) to `brit-build-ref signer --json`, which returns the same agentId without opening the seed.
- **S2d' owner:** after G2 a tampered or unsigned note reads as absent through `build get` (the miss line, exit 0), so `brit-attest.mjs` will rewrite it; new write refusals surface as `Error: REFUSED concurrent update: …` (anyhow prefix). With 8 concurrent writers × 25 puts on one notes ref, 96/200 succeeded and the rest refused after three lost publications — fail-closed by design; the S2d' flock keeps its value for that contention.
- **brit, next sprint:** a build attestation does not cite its subject inside its signed bytes, and a validation attestation signs no step, so its `<step>` ref segment is unauthenticated (both are signed-shape changes needing a schema version); `rev-parse` of a user rev should use `--verify --end-of-options` and peel `^{commit}`; a valid cache-tree parent never compares a child's id with its tree entry (an upstream candidate); `brit status`/`diff` may still run filter drivers through racy-clean comparison.
- **Covenant owners (brit `covenant.rs`, `epr covenant`):** `epr covenant` honours clauses without verifying their signatures (`covenant.rs:527-533`), and a purpose clause does not cite the affirmation it is honoured under.
- **Harness:** the covenant-recording PreToolUse guard blocked source edits and a `--help` probe whose text merely mentioned covenant functions; narrow it to the recording act.
- **Superseded:** the unmerged brit branch `feat/build-put-success-value` (`06f9f234a4`, ReachLevel as a view over protocol Reach) is superseded by the deletion of unsigned `reach`, unless rebuilt over verified reads.
- **Devspace environment:** the workspace exports `JENKINS_URL`, so every tool that sniffs for CI (the `is_ci` crate and its kin) treats a developer's devspace as a CI runner. gix-testtools therefore never writes fixture archives locally and changes its git-version skip logic. Found when a new gix fixture's archive was produced only on GitHub (brit run 40935986385). Scope what reads it, or stop exporting it into interactive shells.
- **brit security:** `cargo-deny-advisories` is red on brit main at `76ba95d8a0` (RUSTSEC-2026-0418 libz-sys, -0414 wikilinks, -0346 to -0349 rustls). "Tests pass" does not require that job, so it does not block landing, but it is open dependency/security work.
