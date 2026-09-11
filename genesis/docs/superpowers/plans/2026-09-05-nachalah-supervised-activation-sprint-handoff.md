---
id: nachalah-supervised-activation-sprint-handoff
title: "Nachalah next sprint — conductor adoption through ark"
status: Draft
class: protocol-canonical
context-tier: disclosed
habits: [nachalah-allotment, runtime-upgrade-propagation, runtime-death-witnessed]
topic: [runtime-upgrade, conductor, ark, recovery, release-channel]
graduation-trigger: >
  The household mesh completes two consecutive conductor upgrade ceremonies and a failed-readiness
  rollback through the existing release channel, with preserved identity, verified running binaries,
  and adoption evidence; the resulting implementation and receipts supersede this handoff.
cites:
  - "nachalah-allotment-epic | The accepted holding epic this sprint enables by bringing conductor adoption forward | sha256:210aba054878c407 | path: genesis/docs/superpowers/specs/2026-09-05-nachalah-allotment-epic-design.md"
  - "runtime-artifacts-elected-content | The existing release election, verification, adoption discipline, and evidence machinery this sprint must extend | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "holochain-evolution-epic | The existing DNA-lineage vehicle preserved while this sprint replaces only the conductor runtime | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
---

# Nachalah next sprint — conductor adoption through ark

Deliver one complete conductor upgrade cycle through the existing elected release ceremony:
publish → peer fetch → local verification → supervised activation → running-identity/readiness
verification → attestation. Prove failed-readiness rollback and then repeat the complete cycle.
This is the next enabling increment for the accepted Nachalah plan: bring conductor adoption
forward before building or actuating earned arcs.

The original scope remains in the Nachalah epic and the acceptance feature. This handoff selects
the next runnable delivery slice; it does not replace the epic or certify scoped holding.

## Starting state: preserve and establish the implementation base

At handoff authoring, the checkout is on `dev`, HEAD
`dbc2f703e83663d73f36f1740cfa2e3de8cc54a3`. The preceding Nachalah work is **uncommitted**.
A fresh worktree from HEAD alone will not contain it. Before implementation, inspect the current
diff, preserve unrelated work, and establish a reviewed base containing the prerequisite changes.
Stage explicit paths only. Do not reset, overwrite, or absorb other sessions' changes.

The prerequisite changes are:

- `elohim/ark/core/src/manifest.rs`: `Probe::ExecutableIdentity`.
- `elohim/ark/supervisor/src/{driver,native,supervisor}.rs`: fail-closed executable observation;
  the native driver hashes `/proc/<pid>/exe`, not a replacement file at the artifact pathname.
- `elohim/ark/core/src/lifecycle.rs`: `ReadinessFailed` preserves the failed rung. A readiness kill
  goes through witness creation and the existing restart governor; it no longer closes as a
  successful intentional stop. A subsequent stop request cannot erase that failure.
- Corresponding real-process tests and core/supervisor seam-registry entries.
- `app/elohim-app/scripts/hc-mesh.sh`: the conductor ladder ends with executable identity.
  Rebuild/bootstrap ark before using this declaration with an older ark binary.
- Package-first updates to the root gospel and `hc-dev-orchestrator`, including generated
  Claude/Codex/Antigravity projections. Preserve their package/projection agreement.
- `genesis/a2o/features/delivery/nachalah-allotment.feature`: 20 scenarios/outlines, all `@wip`.
  The `nachalah-allotment` habit is **unwired**, with no runnable holding/adoption check yet.

Recorded evidence from the preceding pass, not a claim that today's whole working tree was retested:

| Check | Result | Local log, if still available |
|---|---|---|
| `just gate elohim-ark` | Format, clippy, 120 tests passed | `/tmp/nachalah-ark-gate.log` |
| Package projection verifier | 1,772 checks passed, 88 packages | `/tmp/nachalah-packages-verify.log` |
| Gherkin parser | 196 feature files parsed | Re-run the existing parser |
| Mesh declaration fixture | Identity rung present; manifest CID equals berth pin; no conductor launched | Re-run against the rebuilt ark |
| Habit projection and `epr check` | Passed | Re-run after changes |
| Full app gate | Stopped before lint: pnpm could not create `/nix/xdg/cache/pnpm/store/v11` | `/tmp/nachalah-app-gate.log` |

The temporary logs may disappear. Reproduce required checks at the chosen base. No new live mesh
upgrade, alpha adoption, reconstruction, or replication-saving proof was produced by that pass.

## Kickoff evidence — 2026-09-06

The ark prerequisites were independently reviewed and committed as `7fb0e9d0c`.
Fresh `just gate elohim-ark` returned exit 0: format, clippy and 120 tests
(`/tmp/nachalah-sprint-ark-base.log`). Package projection verification returned
exit 0 with 1,772 checks (`/tmp/nachalah-sprint-packages.log`). The mesh declaration,
package/gospel projections and original habit edits remain uncommitted; the ark
commit does not absorb them or unrelated sessions' work. Git's local `core.bare`
was corrected to `false`; `epr doctor` now reports a coherent governance floor.

The host already runs an ark-supervised household using the 0.7 tools at
`/projects/.claude-config/tools/hc-0.7/` and relay at
`/projects/.claude-config/tools/iroh-relay-1.0.3/bin/`. Sandbox process listings
see a private PID namespace and cannot establish host mesh absence. Host inspection
also found another session building in `.claude/worktrees/integ-push`; inspect
both berth leases and host activity before using shared capacity. No running
household process was replaced or stopped in this kickoff.

Station 1 has two explicit prerequisite nodes, projected into the existing
gap-item/valueflow machinery:

- **#1a, signed candidate authentication and compatibility:** an independently
  pinned issuer authenticates exact candidate/target/predecessor content; refusal
  tests cover tampering and incompatible installed state. This output is not an
  activation grant. **Complete:** owning ark gate EXIT=0, 126 tests, independent
  review approved. Brief: `task-nachalah-candidate-contract-brief.md`; evidence:
  `task-nachalah-candidate-contract-report.md`. Test-only prerequisite repair
  `c2fcc6383` removes CID-order assumptions while preserving witness/grace checks.
- **#1b, acting-node delegation proof:** between elected-release verification and
  ark activation, verify the notarized `delegates-compute` action's author,
  recipient, scope, validity and revocation. Probe: malicious/unknown/revoked grants
  leave the incumbent PID unchanged. Current `get_commitment` omits signed author
  evidence; payload provider/recipient and lineage-path quorum cannot establish
  that authority. Bind both grant ActionHash and EntryHash: different authors can
  create identical entry bytes. Non-lineage revocations currently lack authenticated
  lifecycle discovery; resolving a positive revocation record is also distinct from
  proving fresh non-revocation. Define and test the grant-status freshness rule at
  the stop boundary, refusing unavailable evidence. This node remains open, as does
  parent station 1.

These observations do not fulfil any household ceremony or rollback acceptance.
Earned-arc actuation remains disabled.

## Delegation proof continuation — 2026-09-06

Base `7f95ac47f` contains the signed candidate increment. Station #1b decomposes
into two measured prerequisites, leaving its original acceptance intact:

- **#1b-a, exact signed grant record:** resolve the exact Commitment Create
  ActionHash, preserve its signed action, and independently verify author,
  ActionHash, EntryHash and entry definition at the acting node. Probe: two
  authors create identical entry bytes; only the independently pinned exact
  record succeeds, and altered actions, entries and signatures are refused.
  Implementation brief: `task-nachalah-grant-record-brief.md`; gate and review
  evidence: `task-nachalah-grant-record-report.md`. The report bounds completion
  to this non-actuating substation. The legacy entry-hash read's assumption that
  immutable entries have only one authoring action is false. Holochain signs serialized Action bytes,
  not the ActionHash or a JSON re-encoding; signed fixtures preserve that
  constraint independently of the verifier.
- **#1b-b, authority and fresh status at stop:** bind provider/recipient, exact
  candidate target scope and validity, then require authenticated fresh status
  before process mutation. Probe: absent, stale, forged or revoked evidence
  leaves the incumbent PID unchanged. Empty DHT lifecycle results are not proof
  of non-revocation: gossip may omit a valid revocation, and existing lifecycle
  tags carry discoverability rather than authority. An issuer-signed status
  snapshot still has a revocation race; any bounded lease must declare its
  outstanding-lease semantics rather than silently delay immediate revocation.

The decomposition ruling is recorded on #1b with `epr flow note`. Neither new
substation is a new release controller or permission to activate earned arcs.
No household processes have been changed during this continuation.

Tooling prerequisite reproduced and locally resolved: the globally selected pnpm
attempts to create `/nix/xdg/cache/pnpm/store/v11` and fails. Corepack with
`COREPACK_HOME=/tmp/nachalah-corepack` runs the repository's pinned pnpm `10.30.3`;
`/tmp/nachalah-bin/pnpm` is a session-local wrapper for gate execution. This is
tool availability evidence only, not an app gate pass.

Preservation checks during this continuation: package projections passed all
1,772 checks across 88 packages (`/tmp/nachalah-continuation-packages.log`), habit
projection `--check` passed, `epr check` returned exit 0 with local advisories,
the mesh script passed `bash -n`, and all 20 existing Nachalah scenarios/outlines
parsed. The scenarios were not executed and their `@wip` tags remain intact.

## Current implementation boundaries

### Authority/status audit — 2026-09-06 continuation

Read-only implementation audit at `8c32aa11e` confirms two missing nodes inside
#1b-b. These are source findings, not passing runtime probes. The exact-record
implementation and its reported gate/review evidence remain preserved and
uncommitted; no household process was changed in this audit.

- **#1b-b-identity**, between exact-record authentication and authority evaluation:
  authenticate the grant author's binding to the provider identity, the acting
  peer's binding to the recipient, and the candidate signer's delegated standing.
  Probe: each missing or mismatched binding leaves the incumbent PID and source
  chains unchanged. Current state: **unimplemented**. `CandidatePin::signer` is
  an Agent EPR CID; `AuthenticatedCommitmentRecord::author` is a Holochain key.
  `p2p/binding_proof_wire.rs::classify_binding_signature` verifies transport/key
  control but explicitly cannot classify Agent EPR CIDs. Neither raw identifier
  equality nor a projected identity row supplies the missing authority/lineage.
- **#1b-b-ordering**, between authenticated authority/status and process stop:
  establish an explicit rule ordering revocation against the first destructive
  process action. Probe: revoke after an authentic active-status response but
  before stop, with gossip delayed; a response alone leaves the incumbent PID
  unchanged. Current state: **unimplemented; semantics unresolved**. Immediate
  revocation needs a shared ordering boundary; bounded outstanding permits change
  revocation semantics and require an explicit decision. This audit authorizes
  neither a permit nor an activation path.

### Ruling — the stop-boundary contract (2026-09-06, orchestrator)

**Revocation dominates. Permits are not bearer instruments.** A `delegates-compute`
grant is a bounded, revocable commitment; a permit, lease, or positive status
response is a *claim against* that commitment, never a second commitment, so it
inherits the grant's liveness. Expiry is the ceiling on a claim's life and
revocation is its floor. Therefore:

1. **Revocation cancels every not-yet-started activation.** There are no
   outstanding-lease semantics: nothing issued before a revocation stays usable
   after it. (An outstanding lease makes revocation's effective time
   `max(remaining lease)`, which this audit already names as silently delaying
   immediate revocation. In this protocol the grantor holds the arc; the acting
   node's authority is the bounded thing.)
2. **The stop boundary is the first destructive process action against the
   incumbent.** The ordering rule: authenticated fresh grant status is the LAST
   check inside that boundary — after identity bindings, compatibility and
   staging, immediately before the stop signal. Fresh means an issuer-signed
   positive assertion ("active at T") with T later than the activation's own
   decision time and later than every revocation this peer has seen. An empty
   search, an unsigned or aged assertion, or a forged one is `unavailable`:
   refuse, incumbent PID and chains unchanged (the outline rows stand as written).
3. **The residual race is closed by reversibility, not by a permit.** Revoke
   after a fresh positive status but before the stop, with gossip delayed, is
   irreducible in a distributed system. The ark's activation witness records the
   grant ActionHash and the exact status assertion it acted on; an authenticated
   revocation that arrives later with a notarized time before the recorded stop
   is a *late revocation* and MUST trigger local rollback to the retained
   previous runtime (Station 3 machinery), attested through the evidence path.
   The window shrinks to gossip delay, the exercise is reverted, and a record
   remains — an authority leak becomes a witnessed, reverted act.
4. **An activation whose destructive action has begun runs to the next safe
   stop** (never a half-killed incumbent), then the same rule applies.

Consequence for the story: "A signed status response cannot conceal a revocation
before stop" currently expects refusal because no ordering rule exists. Under this
ruling a fresh positive status inside the window DOES authorize the stop, and the
late revocation produces rollback plus attestation. Reshape that scenario so its
finish line is "James's conductor runs the previous release again, and both the
late revocation and the status it superseded are attested"; the refusal outline
keeps its five rows. Route the `#1b-b-identity` bindings through the
`identity-cross-signed` habit's home (imagodei device/agent attestation), not a
new binding type in ark or storage.

Concrete reuse limits: `services/commitment_fetcher.rs` reads `revoked_at` from
the original immutable payload in its conductor path, or from mutable SQLite in
its projection path. Revocations are separate `revokes-commitment` entries.
`mishpat/src/commitments.rs` validates non-lineage revocation target/time without
binding its author to the grant provider and emits no lifecycle-discovery links
for that class. Existing lifecycle tags are discoverability hints, not authority.
An authenticated positive revocation can justify refusal; absent search results
cannot justify permission. `services/bounds_validator.rs` also does not compare
its event performer to the fetched recipient; it cannot alone discharge the
recipient-binding prerequisite. Preserve the shared bounds-validator home when
addressing that gap rather than duplicating its scope/reach/rate policy.

Three new `@wip` scenarios/outlines in `nachalah-allotment.feature` preserve these
refusal requirements before the existing household finish-line scenario. They
are specifications only; identity, authority, freshness, activation, rollback,
and both household ceremony runs remain open. Earned-arc actuation stays disabled.

| Concern | Existing home and what to reuse | Missing work |
|---|---|---|
| Release identity, requirements, and class | `elohim/elohim-storage/src/services/release_adoption/mod.rs`; schema `elohim/rakia/schemas/v1/release-manifest.schema.json`; `genesis/a2o/scripts/epr-release-package.ts` | There is no `conductor-binary` enum/class yet. Extend the authoritative schema, packager, and consumers together. Inspect rakia's repository boundary before editing it. |
| Election, fetch, local verification, discipline | `release_adoption/{watch,artifact_pull,verify}.rs`; `release-ceremony.ts` | Bind runtime compatibility and activation authorization to the elected candidate; do not add another release controller. |
| Apply dispatch | `ApplyVehicle`, `ApplyRegistry`, `release_adoption/apply.rs`, and registration in storage `main.rs` | Add the conductor vehicle and its authenticated handoff to ark. `StorageBinaryVehicle` currently stages only and accepts declared Simulacra stakes. |
| Child supervision | `elohim/ark/{core,supervisor,cli}` | Targeted activation, durable progress, previous-runtime retention, and local rollback. The current readiness-failure governor retries the declared child; it does **not** select the previous binary. |
| Adoption reporting | `release_adoption/{state,apply,path_evidence}.rs` and `/admin/adoption` | Join stage/activation/rollback outcomes to their release and channel. `pending_restart` is currently sticky via OR in `record_applied`; a later sweep cannot establish that the staged executable is running. |
| Storage reconnection | `elohim/elohim-storage/src/hc_client_registry.rs` | Exercise the existing whole-client reconstruction and fresh app-authentication token path after conductor replacement. Do not invent a second reconnect loop. |
| Lineage | `release_adoption/{carry,revert,readopt}.rs`, `happ-lineage-migration.feature` | Preserve this existing vehicle. Selective carriage and scoped successors belong to the subsequent holding sprint. |

Ark core owns pure decisions. Ark supervisor owns process/filesystem effects and currently has a
no-network boundary. Storage resolves and verifies elected content and publishes outcome evidence.
Choose the smallest authenticated local handoff that preserves those responsibilities; do not move
DHT election, HTTP fetching, or global trust scoring into ark.

## Ordered delivery stations

These are sprint work boundaries, not claims that implementation has already been assigned.
At kickoff, resolve/project the corresponding valueflow work and claim each commitment before
dispatching an implementer. The implementer brief must name the reviewed base SHA and applicable
rulings. Keep the existing two-active-habit fence; do not activate a third habit to start this work.

### 1. Establish the activation contract and its refusal tests

Invoke the P2P design gate before adding persistent or wire types. Reuse release EPRs,
commitments, and adoption evidence. Local operational progress is not a new notarized head per
poll. Identify which existing authority permits activation, how the acting peer verifies it,
and what evidence binds the candidate to that authority. A supplied file path or caller's
assertion that bytes were verified is insufficient authorization.

Define the candidate's platform, installed-version binding, database read/write compatibility,
and required protocol capabilities in the release contract. The first candidate must preserve
executable rollback compatibility. Refuse an unsupported downgrade before stopping the incumbent.
Do not infer compatibility from a filename, matching version string, or successful process spawn.

**Exit proof:** incompatible, stale, unverified, and unauthorized candidates leave the incumbent
running and produce explicit refusals; a compatible verified candidate can enter durable staging.

### 2. Implement durable staging and a targeted ark transition

Extend the existing vehicle dispatch with `conductor-binary`. Stage durably and retain the
previous executable and runtime manifest. Bind progress to the release/channel, target child,
expected predecessor, and manifest identities. Serialize competing transitions and reject stale
predecessors. Replaying a settled request must not restart the child again.

Persist the recovery information before the first destructive process action. Restart only the
named child and preserve databases, agent keys, berth identity, signing credentials, environment,
and required configuration. An ark/process crash must not leave two conductors owning one data root.
On recovery, derive what actually runs before marking anything adopted.

**Exit proof:** real child-process tests cover crash/restart at staging and activation boundaries,
idempotent replay, conflicting requests, unchanged sibling PIDs, and preserved data-root contents.

### 3. Prove readiness failure causes local rollback

After replacement, use the service readiness ladder and compare the actual executable with the
candidate pin. Keep staged, activating, running, and failed outcomes distinguishable. Restore the
previous executable and manifest on failed readiness without requiring a functioning conductor
to approve this immediate recovery. Verify the restored process too; rollback failure must remain
explicit, with bounded attempts and durable evidence.

Exercise absent identity, wrong identity, candidate early exit, readiness timeout, and ark restart
while activation is unresolved. Preserve the existing write-ahead witness discipline and never
turn a failure into a successful stop. Durable recovery must choose the retained fallback rather
than repeatedly executing a failed candidate through the ordinary restart governor.

**Exit proof:** an unusable candidate restores a verified working predecessor while retaining
the same keys/data/berth. Unsupported database downgrade is refused rather than attempted.

### 4. Rejoin the shared evidence path after service returns

Let storage recover its conductor clients through the existing fresh-token path. Record the
actual running release and publish the adoption or rollback outcome through existing channel
evidence and attestation discipline. Survive a storage restart between child recovery and evidence
publication; replay must not duplicate the material outcome or restart the child again.

The shared activation mechanism should be usable for storage when storage is supervised, but
this sprint's required vertical proof is **conductor replacement**. The old storage-binary
staging vehicle is not evidence of storage self-activation or fleet binary rollout.

**Exit proof:** after conductor replacement, a real authenticated zome call succeeds and peers
can resolve the adoption/rollback evidence tied to the exact release and running binary.

### 5. Rehearse the complete household ceremony twice

Wire the existing Nachalah scenarios “A staged executable is not reported as the running release”
and “The household adopts and recovers through the shared upgrade ceremony.” Keep the concern tag.
Split stations further if needed to expose intermediate truths while retaining the finish line.
Remove `@wip` only from scenarios that are implemented and actually pass.

Bootstrap the adopter once through the existing deployment/local-stack mechanism if necessary.
After that bootstrap, use the existing packager and release ceremony to publish a real compatible
conductor candidate; the peer fetches, verifies, activates, and attests it. A directly copied binary,
manual restart, or a new release controller does not prove this path. Use distinct executable
identities for the successful replacement proof; a new release label over the same bytes does not
exercise binary replacement. Test doubles remain useful for failure injection, not as the only proof.

Run successful adoption, a failing candidate with automatic local rollback, and a second complete
successful ceremony. Preserve the recovery-capable release channel throughout. Do not migrate its
authority space during this runtime replacement.

**Exit proof:** durable run evidence records candidate/release/channel identifiers, expected and
observed executable digests, before/after child and sibling PIDs, agent/cell identity continuity,
readiness and reconnect results, failure/rollback outcome, and peer-resolvable attestation.
Exclude credential material from reports. Reuse the a2o run/check witness format.

## Verification and closeout

- Claim the shared mesh/cargo capacity through the existing berth workflow before using it;
  inspect ownership rather than stopping another session's mesh. Use `hc-dev-orchestrator` for
  stack operation and the current native cargo pool/RUSTFLAGS rules.
- Run owning gates for each changed tree: ark, storage, a2o, and schema/package projections as
  applicable. Repair or report the pnpm cache/toolchain problem; it is not a waived gate.
- Run the wired ceremony scenarios on the household mesh through `just test mesh` with their
  file/tag scope. Record command, revision, environment, direct exit status, and receipt location.
- Update the existing habit atoms with one-line evidence deltas and re-project
  `genesis/manifests/habits.yaml`. A conductor ceremony receipt expands upgrade evidence only as
  far as measured. Nachalah holding/recovery/earned-arc acceptance remains unfulfilled.
- Record every discovered missing prerequisite as a station with an assertion and probe. Avoid
  a new parallel backlog or release ledger. Preserve the resulting evidence in the existing homes.

Sprint completion requires the real repeated ceremony and failed-adoption rollback proofs above.
A green unit suite, a parsed manifest, staged bytes, or a readiness rung alone is insufficient.

## Subsequent sprint boundary and inherited constraints

After this delivery path is proven, proceed to the exhaustive record policy census and verified
membership/scoped-successor work, then the exact-stock-0.7 fork with actuation disabled, observation,
one non-gold mesh space, and finally an alpha canary through the same ceremony. Do not treat this
handoff as authorization to bypass those dependencies or weaken any acceptance floor.

Trust selects fewer authorities; each assigned authority validates fully. Household affirmation
establishes belonging. Reading permission, canonical-version authority, DHT holding, and encrypted
custody stay distinct. Wider reach requires witnessed promotion; custody does not grant readability.
Preserve one-device-loss household recovery, three evidenced household holding domains for deeds,
and seven admitted diverse full-covering hubs for gold. Logical households on one host cannot
certify physical diversity. Alpha, arc actuation, selective migration, and performance savings
remain separate required delivery proofs beyond this sprint.

## Kickoff prompt

> Read this handoff and its cited sources, inspect the shared working tree, and establish a reviewed
> base that includes the uncommitted ark prerequisites. Deliver the conductor-binary vertical path
> through the existing elected release ceremony and ark, including authenticated activation,
> durable recovery, actual running identity, failed-readiness rollback, fresh-token reconnect,
> and peer-visible evidence. Finish with two successful household ceremony runs and the failing
> candidate recovery run. Keep earned-arc actuation disabled. Use the existing valueflow claim,
> review, and evidence workflow, and report unmet proofs explicitly.
