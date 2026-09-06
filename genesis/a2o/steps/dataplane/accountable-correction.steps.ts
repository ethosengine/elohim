/**
 * Step skeletons for `features/dataplane/accountable-correction.feature`
 * (`@concern:accountable-correction`).
 *
 * Contract: genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md
 * (revision 3). Every step body below is a stub — the coordinator surface it needs
 * does not exist yet (see the feature file's header comment for the full gap list).
 * Each stub names the exact contract section and Rust surface (extern / HTTP route /
 * migration) it is waiting on, then returns `'pending'` so cucumber-js reports it as
 * pending rather than passing or crashing. DO NOT implement a body without reading
 * the contract section it names first — several of these (§6 same-root lineage, §8
 * two-phase submission) have failure-mode subtlety the contract spells out in detail.
 *
 * Household-mesh rails a real implementation should reuse rather than reinvent (kept
 * OUT of this file's imports for now — an unused import is dead weight until a body
 * actually calls it; add each back as its first step is implemented):
 *   - `connectConductor` / `meshConductorPorts` / `meshStorageUrl` / `MESH_PEER_ORDER` /
 *     `CONTENT_STORE_ZOME` from `src/framework/dataplane/carried-election.ts` — the
 *     direct callZome rail proven by the 2026-08-31 carried-election mesh proof and
 *     reused by `steps/dataplane/federation-deploy.steps.ts`. This concern's zome
 *     externs (`get_content_lineage`, `amend_content`, `create_vouch`,
 *     `create_feedback_signal`, `get_feedback_signal_record`) all live in the same
 *     `content_store` coordinator zome, so the same rail applies unchanged.
 *   - `getRaw` / `postRaw` / `resolvePeerUrl` from `src/framework/dataplane/surfaces.ts`
 *     — the HTTP surfaces this concern needs: `POST /api/v1/feedback/operations`,
 *     `GET /api/v1/feedback/operations/{id}`, `GET /api/v1/standing/{agent_cid}`
 *     (contract §7-§9; route builders are one-liners over these two peer URLs).
 *   - `hc-mesh.sh storage-restart <peer>` via `spawnSync`, gated by
 *     `destructiveAllowed()` from `src/framework/fixtures/substrate-scope.ts` — the
 *     ONLY sanctioned way a step here moves a peer's storage process (station 4).
 *     Pattern: `steps/delivery/acquisition-pins.steps.ts` (destructiveGate),
 *     `steps/mesh/peer-conductor-resilience.steps.ts` (restartMeshConductors).
 *   - A per-world context (`WeakMap<E2EWorld, ...>`, the `ctx()` pattern used
 *     throughout `steps/mesh/peer-conductor-resilience.steps.ts`) to carry the action
 *     hashes minted across a scenario's steps: Jessica's root Create, James's
 *     correction(s), Jessica's acceptance vouch, her successor(s), and James's §8
 *     operation id. Add it when the first step needs to pass state to a later one.
 *
 * Roles fixed for this whole file (README-accountable-correction.md has the "why"):
 *   Jessica = root author of the corrected record. James = correction author, and
 *   the one first-party filer in station 8. Matthew = durable-discovery witness only
 *   — never a root author, correction author, or acceptor in any station here.
 */

import { Given, When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../../src/framework/world.js';

// ===========================================================================
// Background
// ===========================================================================

Given(
  'Jessica has authored a content record visible to all three peers',
  function (this: E2EWorld) {
    // Pending (contract §1, §2): author a content record on Jessica's peer via the
    // ordinary content-create path, then resolve its root Create action hash — either
    // directly from the create response, or via `get_content_lineage` once it exists
    // (§6). Carry it in a per-world context for later steps (see this file's header).
    return 'pending';
  }
);

// ===========================================================================
// Station 1 — durable discovery, no notification; late arrival applies once (§3)
// ===========================================================================

Given("James's peer has peer-to-peer feedback notifications disabled", function (this: E2EWorld) {
  // Pending (README-accountable-correction.md): set ELOHIM_FEEDBACK_NOTIFY=0 in
  // James's peer's process environment before the next filing call. The env var
  // does not exist on the storage process yet — this step is the a2o side of that
  // contract; the Rust side is owed at the point p2p/mod.rs would send the
  // feedback-signal notification (§4).
  return 'pending';
});

Given("James's peer has peer-to-peer feedback notifications enabled", function (this: E2EWorld) {
  // Pending: the default state (ELOHIM_FEEDBACK_NOTIFY unset or =1) — explicit for
  // readability against station 1's disabled case. No env mutation needed once the
  // disabled step above unsets it per-scenario, but this step should assert that.
  return 'pending';
});

When(
  "James files a correction against Jessica's record with no notification sent",
  function (this: E2EWorld) {
    // Pending (contract §1, §4): create_feedback_signal on James's peer against
    // Jessica's root action hash, with notifications suppressed (see the Given
    // above). Record the resulting correction action hash for later steps.
    return 'pending';
  }
);

Then(
  "Matthew's peer discovers James's correction within its next discovery scan, unnotified",
  function (this: E2EWorld) {
    // Pending (contract §3): poll Matthew's peer's per-act application read surface
    // (once one exists) until it shows the correction action filed above, bounded
    // by the discovery-scan interval (mirror services/release_adoption/watch.rs
    // cadence). Assert this happens WITHOUT Matthew's peer ever having received a
    // p2p notification (§4 path untouched).
    return 'pending';
  }
);

Given(
  'James has filed a second, differently-timestamped correction that predates the first by action timestamp',
  function (this: E2EWorld) {
    // Pending (contract §3, §6): file a second create_feedback_signal whose action
    // timestamp sorts BEFORE the first correction's — ordering is by action
    // timestamp, not filing order, so this may need an explicit backdating fixture
    // hook. Track its action hash alongside the first.
    return 'pending';
  }
);

Given(
  "Matthew's peer has already applied the first correction it discovered",
  function (this: E2EWorld) {
    // Pending (contract §7): poll Matthew's peer's application read surface until the
    // FIRST correction filed above shows status=applied.
    return 'pending';
  }
);

When(
  "the older correction becomes discoverable to Matthew's peer on a later scan",
  function (this: E2EWorld) {
    // Pending (contract §3): the fair rotation across scans (a persisted rotating
    // position over the subscription set) guarantees this within a bounded number of
    // scans — poll rather than sleep a fixed duration.
    return 'pending';
  }
);

Then("Matthew's peer applies the older correction exactly once", function (this: E2EWorld) {
  // Pending (contract §7): the operation-group unit of application means the older
  // correction becomes a group member of the SAME operation group as the first,
  // since both name the same evidence action per §8's group-key rule, OR is an
  // independent group depending on how the fixture files it — read the per-act
  // application surface and assert exactly one applied row for its action hash.
  return 'pending';
});

Then(
  "Matthew's peer still shows the first correction applied exactly once, undisturbed by the late arrival",
  function (this: E2EWorld) {
    // Pending (contract §7): anti-regression — applying the late arrival must not
    // have re-applied or duplicated the FIRST correction's row.
    return 'pending';
  }
);

// ===========================================================================
// Station 2 — notification accelerates, never substitutes; foreign-DNA refused (§4)
// ===========================================================================

When(
  "James files a correction against Jessica's record with a notification sent",
  function (this: E2EWorld) {
    // Pending (contract §1, §4): create_feedback_signal on James's peer with
    // notifications enabled — the direct p2p feedback-signal message, additively
    // carrying {origin_dna_hash, action_hash, routing_key}.
    return 'pending';
  }
);

Then(
  "Matthew's peer applies the correction before its next scheduled discovery scan would otherwise have found it",
  function (this: E2EWorld) {
    // Pending (contract §4): assert the applied timestamp precedes Matthew's
    // peer's next scheduled scan tick (computed from its known interval and the
    // last observed tick) — the acceleration claim stated as a concrete event,
    // not a vague "well inside" margin.
    return 'pending';
  }
);

Then(
  "Matthew's peer's applied correction matches the fetched, verified record James actually authored",
  function (this: E2EWorld) {
    // Pending (contract §1, §4): the receiver must fetch and verify the record
    // (action hash match, entry type, entry hash, entry bytes present) rather than
    // trusting the envelope's semantic payload as evidence on its own.
    return 'pending';
  }
);

When(
  "a feedback notification arrives at Matthew's peer naming a foreign origin DNA hash",
  function (this: E2EWorld) {
    // Pending (contract §1): craft (or have a fixture helper craft) a feedback-signal
    // envelope whose origin_dna_hash does not match Matthew's peer's own content cell
    // DNA hash, and deliver it to the receiver path (epr_atom_service.rs).
    return 'pending';
  }
);

Then(
  "Matthew's peer rejects the foreign-DNA notification without applying anything from it",
  function (this: E2EWorld) {
    // Pending (contract §1): assert no new application row and no standing change
    // resulted from the foreign-DNA envelope.
    return 'pending';
  }
);

Then(
  "Matthew's peer's own content cell DNA hash is unchanged by the rejected notification",
  function (this: E2EWorld) {
    // Pending: a rejected envelope must be inert — this is the anti-regression half.
    return 'pending';
  }
);

// ===========================================================================
// Station 3 — root-author-only settlement (§5.2, §5.3, §6)
// ===========================================================================

Given("James has filed a correction against Jessica's record", function (this: E2EWorld) {
  // Pending (contract §1): create_feedback_signal on James's peer against
  // Jessica's root action hash. Shared precondition for stations 3-7.
  return 'pending';
});

When(
  "Jessica accepts James's correction with an accept-correction vouch",
  function (this: E2EWorld) {
    // Pending (contract §5.2): create_vouch on Jessica's peer, signal_kind=vouch,
    // vouch_kind=accept-correction, target_cid = James's correction action hash,
    // evidence_cid=None. Record the resulting acceptance action hash.
    return 'pending';
  }
);

Given(
  "Jessica has accepted James's correction with an accept-correction vouch",
  function (this: E2EWorld) {
    // Pending: same call as the When above, phrased as a precondition for stations
    // 4/6/7 that start from "already accepted".
    return 'pending';
  }
);

Then(
  "Jessica's acceptance vouch is a distinct, visible record from James's correction",
  function (this: E2EWorld) {
    // Pending (contract §5, §9): two distinct action hashes, both independently
    // resolvable — the "three acts, never collapsed" invariant.
    return 'pending';
  }
);

When(
  "Jessica publishes the amended content naming the corrected record's exact predecessor action",
  function (this: E2EWorld) {
    // Pending (contract §5.3): the owed coordinator extern
    // `amend_content { predecessor_action_hash, content }`, verifying Jessica
    // equals the exact root Create author of the corrected record and writing the
    // Update against that action rather than reselecting the latest ID link.
    // Record the resulting successor action hash.
    return 'pending';
  }
);

Then(
  "Jessica's successor is a third record, distinct from both the correction and the acceptance",
  function (this: E2EWorld) {
    // Pending (contract §5): three acts, three action hashes.
    return 'pending';
  }
);

Then(
  "all three of Matthew's, Jessica's, and James's peers adopt Jessica's successor as the served head",
  function (this: E2EWorld) {
    // Pending: poll each peer's served-head surface (mirror
    // probeServedBundleHead / probeDeclaredHead in
    // src/framework/dataplane/surfaces.ts) until all three agree on the successor
    // action hash minted above.
    return 'pending';
  }
);

Then(
  "a dependent view reading the record re-renders the successor's content",
  function (this: E2EWorld) {
    // Pending: read the record through an ordinary content-read surface and assert
    // its body matches the successor's amended content, not the original.
    return 'pending';
  }
);

When(
  'James attempts to accept his own correction with an accept-correction vouch',
  function (this: E2EWorld) {
    // Pending (contract §5.2): create_vouch on JAMES's peer targeting his own
    // correction action — must be refused because he is not the record's root
    // author (and same-cell self-acceptance is explicitly unsupported in slice 1).
    return 'pending';
  }
);

Then(
  "James's acceptance attempt is refused because he is not the record's root author",
  function (this: E2EWorld) {
    // Pending (contract §5.2): assert the coordinator/storage refusal, and that no
    // acceptance record was minted for James's attempt.
    return 'pending';
  }
);

Then(
  "Jessica's earlier acceptance and successor are unaffected by James's refused attempt",
  function (this: E2EWorld) {
    // Pending: anti-regression — a refused non-author attempt must not perturb the
    // already-settled state from earlier in the scenario.
    return 'pending';
  }
);

// ===========================================================================
// Station 4 — crash windows never double-apply, never lose the correction (§7)
// ===========================================================================

When(
  "Jessica's peer's storage is restarted after the correction is marked applied but before her standing tally reflects it",
  function (this: E2EWorld) {
    // Pending (contract §7): this precise interleaving needs a fault-injection
    // hook inside the ONE diesel transaction that commits the application row and
    // standing tally together — today's code has no such hook. Until it exists,
    // this step cannot induce the exact window the scenario names; a
    // bracket-timed `hc-mesh.sh storage-restart jessica` around discovery is the
    // nearest approximation and should be named as such if used before the hook
    // lands.
    return 'pending';
  }
);

Then(
  "Jessica's peer's projection shows neither a correction marked applied with no matching tally change, nor a tally change with no correction marked applied",
  function (this: E2EWorld) {
    // Pending (contract §7): read the feedback_application row and standing_view
    // row directly (or via GET /api/v1/standing/{agent_cid}) and assert they
    // agree — both reflect the pre-restart state, or both reflect the
    // post-commit state, never a mix.
    return 'pending';
  }
);

When("Jessica's peer's discovery resumes after the restart", function (this: E2EWorld) {
  // Pending: no explicit action needed once the peer is back up — discovery is a
  // background interval. This step polls for the next scan to have run.
  return 'pending';
});

Then(
  "the accepted correction is applied exactly once to Jessica's standing tally",
  function (this: E2EWorld) {
    // Pending (contract §7): via GET /api/v1/standing/{agent_cid} (Jessica's agent
    // cid) — assert the contribution count reflects exactly one application of the
    // accepted correction.
    return 'pending';
  }
);

When(
  "the same accepted correction is replayed against Jessica's peer after it was already settled",
  function (this: E2EWorld) {
    // Pending (contract §7): re-deliver the same correction action (re-run
    // discovery, or re-post the same notification) after the application row
    // already shows status=applied — must be a no-op by that row.
    return 'pending';
  }
);

Then("Jessica's standing tally is unchanged by the replay", function (this: E2EWorld) {
  // Pending: assert the standing read is byte-identical to the pre-replay read.
  return 'pending';
});

// ===========================================================================
// Station 5 — an allegation costs nobody; only acceptance moves standing (§7)
// ===========================================================================

Then("James's own standing is unaffected by the correction he filed", function (this: E2EWorld) {
  // Pending (contract §7): GET /api/v1/standing/{james's agent cid} — filing is
  // never a debit against the FILER; this asserts James's own row, if it exists
  // at all, has not changed because of this filing.
  return 'pending';
});

Then(
  "Jessica's standing reads as Unknown, with no row for her at all, while the correction is unaccepted",
  function (this: E2EWorld) {
    // Pending (contract §7): GET /api/v1/standing/{jessica's agent cid} before any
    // acceptance — assert the row is ABSENT (services/standing.rs Unknown), never
    // present with a zero value. A present zero row would be the old
    // immediate-debit behaviour this station specifically forbids.
    return 'pending';
  }
);

Then(
  "Jessica's standing now exists and reflects exactly one contribution",
  function (this: E2EWorld) {
    // Pending (contract §7): after acceptance, the row must now exist with exactly
    // one counted contribution.
    return 'pending';
  }
);

When(
  "Jessica accepts James's same correction with a second, redundant accept-correction vouch",
  function (this: E2EWorld) {
    // Pending (contract §7): a second create_vouch accept-correction targeting the
    // SAME correction action — repeated acceptances of one correction count once.
    return 'pending';
  }
);

Then("Jessica's standing still reflects exactly one contribution", function (this: E2EWorld) {
  // Pending: the redundant acceptance must not double the contribution count.
  return 'pending';
});

// ===========================================================================
// Station 6 — same-root conflict is named, not silently resolved away (§6)
// ===========================================================================

When(
  'Jessica publishes two amended successors that each name the same predecessor action',
  function (this: E2EWorld) {
    // Pending (contract §6): two amend_content calls from Jessica's peer, BOTH
    // naming the same predecessor action hash. Track both successor action hashes.
    return 'pending';
  }
);

Then(
  "Jessica's peer marks the record contested, visibly listing both branches",
  function (this: E2EWorld) {
    // Pending (contract §6): the owed `get_content_lineage` extern (or its storage
    // projection) must report BOTH candidates sharing one predecessor, and the
    // record's contested marker must be set and readable by an ordinary consumer,
    // not just inferable from raw lineage data.
    return 'pending';
  }
);

When(
  "the deterministic pick resolves one of Jessica's two branches as the served head",
  function (this: E2EWorld) {
    // Pending (contract §6): the ordinary resolver's newest-timestamp +
    // action-hash-tiebreak pick — poll the served-head surface until it names one
    // of the two successor action hashes above.
    return 'pending';
  }
);

Then(
  'the record is still marked contested after the deterministic pick',
  function (this: E2EWorld) {
    // Pending (contract §6): anti-regression — a served pick must NOT clear the
    // contested marker; the fork stays named as an observed-fork record.
    return 'pending';
  }
);

When(
  'Jessica publishes a further amendment naming her already-resolved served head as its predecessor',
  function (this: E2EWorld) {
    // Pending (contract §6): a SEQUENTIAL amend_content — predecessor is the
    // currently-served head, not a race against it.
    return 'pending';
  }
);

Then('this sequential amendment is not marked contested', function (this: E2EWorld) {
  // Pending (contract §6): sequential amendments (each naming the prior head) are
  // ordinary succession and must never be marked contested.
  return 'pending';
});

// ===========================================================================
// Station 7 — rebuild reproduces the live generation, row for row (§7)
// ===========================================================================

When(
  'a fresh standing generation is built from scratch from every retained correction and acceptance, then published',
  function (this: E2EWorld) {
    // Pending (contract §7): trigger the owed rebuild path — a fresh generation
    // (evaluator, pinned policy bytes by CID) built to completion then atomically
    // published. No such trigger exists yet; this step is waiting on the rebuild
    // operator surface named in §7.
    return 'pending';
  }
);

Then(
  "the fresh generation's tallies are identical to the live generation's, excluding storage layout and operational timestamps",
  function (this: E2EWorld) {
    // Pending (contract §7): compare canonical logical rows (policy CID,
    // acceptance dependencies, contributions) between the two generations.
    return 'pending';
  }
);

Then(
  'readers of the fresh generation see it as still rebuilding until every retained correction and acceptance has replayed, never a partial tally',
  function (this: E2EWorld) {
    // Pending (contract §7): during the rebuild window, GET /api/v1/standing/{cid}
    // against the fresh generation must read `rebuilding`, never a half-applied
    // aggregate.
    return 'pending';
  }
);

// ===========================================================================
// Station 8 — one intended act survives a lost response (§8)
// ===========================================================================

Given(
  "James's first-party view files a correction against Jessica's record under one operation id it minted",
  function (this: E2EWorld) {
    // Pending (contract §8): POST /api/v1/feedback/operations on James's peer's
    // storage, with an operation id James's own view generates client-side before
    // the call (never server-assigned). Two-phase: author the Correction EPR (id
    // `correction:<operation_id>`) then create_feedback_signal naming its action
    // hash as evidence_action_hash. Record the operation id for later steps.
    return 'pending';
  }
);

When(
  "the response to James's filing is lost before his view receives it",
  function (this: E2EWorld) {
    // Pending (contract §8): simulate response loss — e.g. the fixture drops the
    // HTTP response body/connection AFTER the server-side commit has already
    // happened, rather than before it. The commit's success and the response's
    // delivery are independent facts; this step only breaks the second.
    return 'pending';
  }
);

When(
  "James's view reloads and retries the filing with the same operation id",
  function (this: E2EWorld) {
    // Pending (contract §8): re-POST /api/v1/feedback/operations with the SAME
    // operation id and the SAME immutable request bytes — recovery enumerates
    // list_feedback_signals_by_signer and matches the tuple; several matches are
    // one group.
    return 'pending';
  }
);

Then(
  "James's peer reports the operation as resolved to exactly one correction, or unresolved, but never two",
  function (this: E2EWorld) {
    // Pending (contract §8): GET /api/v1/feedback/operations/{operationId} —
    // status must be a single resolved action or `unresolved`, never two
    // distinct correction actions attributed to one operation id.
    return 'pending';
  }
);

Then(
  "the retried filing produces at most one contribution toward Jessica's eventual standing change",
  function (this: E2EWorld) {
    // Pending (contract §7, §8): once Jessica accepts, standing must reflect at
    // most one contribution from this operation's group, regardless of how many
    // times the client retried.
    return 'pending';
  }
);

Then(
  "a second presentation reading the same subject agrees with James's view about what was filed",
  function (this: E2EWorld) {
    // Pending (contract §8, and the app-host gap named in contract §10): read the
    // same correction through an ordinary content/feedback read surface and
    // assert it agrees with what GET /api/v1/feedback/operations/{operationId}
    // reports to James's own view.
    return 'pending';
  }
);
