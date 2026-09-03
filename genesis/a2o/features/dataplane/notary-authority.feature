# RED-FIRST. These scenarios ARE the schedulable red for habit `notary-authority`.
# They assert what "notarized" must come to mean on the live alpha federation, and pass
# unchanged the moment it does.
#
# WHAT THE SUBSTRATE TAUGHT US (the penicillin note — do not lose this):
#   The 2026-07-03 gap was "elohim.host serves the landing at trust=published, never
#   notarized — the anchor didn't propagate." That gap CLOSED. But closing it revealed a
#   deeper truth, observed 2026-07-09:
#     alpha-A      /db/content/elohim-host-landing.trust = "notarized"  dhtAnchorHash = uhCkkmx3Fb8…
#     elohim.host  /db/content/elohim-host-landing.trust = "notarized"  dhtAnchorHash = uhCkkwLFEVDz…  (DIFFERENT)
#     …and their blobHash differs too (each host built + notarized its OWN bundle).
#   Both peers are green — over TWO DIFFERENT HEADS. "Reaches notarized" was never the
#   invariant; it is a FALSE-GREEN. The two independent authorings are not a bug to lock
#   out — they are the natural state of a peer network where many peers legitimately author.
#   The invariant is CONVERGENCE: every federation peer must resolve the SAME canonical head,
#   and canonical status must be an EARNED authority act — never last-writer recency, never a
#   privilege of the peer that ran the deploy.
#
# THE MODEL THIS FEATURE ENCODES (how it releases into the wild):
#   Authority over an EPR is EARNED, socially, through attestations. A version authored
#   without earned attestations for its EPR stays PRIVATE (limited reach); it does not get to
#   move the commons head. Reach is earned up the tiers (amber → green) and, at the commons
#   threshold, a head is PROMOTED to canonical and propagated for every peer to adopt.
#   Elohim agents are the system TENDERS of these socially-derived grants — grantors of reach
#   that help humans help themselves: they witness and actuate the community's grant, they
#   never bestow authority by fiat. There are no standing "granted roles" in the live network,
#   only earned ones.
#
#   The notary is to the CRDT plane as HTTPS is to HTTP: the CRDT converges the *value*, the
#   DHT notary adds the *authority* (the author/community signature over the canonical head).
#   Without it, "whoever wrote last" wins — grandma's caption, a household page, an EPR's blob
#   can be silently overwritten and served as truth. Green = "this is THE canonical head the
#   community's earned-reach grant elected", not "some peer notarized something locally".
#
# SHARPENED 2026-07-10 (replication EXONERATED — the gap is PURELY election):
#   Confirmed live that both blobs are present on both peers — GET the elohim.host bundle on
#   alpha-A and the alpha-A bundle on elohim.host both return 200. So the divergence is NOT a
#   byte-replication gap; the bytes moved. It is purely a HEAD-ELECTION gap: each peer serves its
#   OWN content-row blobHash (declaredHead is null on both), and the serve path does not consult
#   declared_head at all (`read_head_blob_hash` is called only in tests — the C3 read side is
#   dead). Worse, the reconcile heal loop resolves the head from EACH peer's OWN conductor, so it
#   re-stamps each peer's own head — per-node resolution PRESERVES the split. This is why the new
#   landing served once then "regressed": elohim.host was always going to fall back to its own
#   head. The cure is a single canonical head-binding, honored on serve over any node-local
#   authoring — NOT more replication.
#
# DEV-TIME SCAFFOLD (honest, and on the clean-up-toward trajectory — never extend it):
#   We are not participants in the authoritative network; we are devs in god-mode building the
#   model. Until the socially-derived earned-attestation grant path is wired, the promotion
#   decision is ROUGHLY HARD-CODED (genesis/deploy designates the canonical head). That
#   hard-coding is a labeled stand-in for the social-grant process, not a network role and not
#   the goal — it must dissolve into earned-reach promotion, not calcify into first-writer-wins.
#
# Design:  genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
# Plan:    genesis/docs/superpowers/plans/2026-07-01-crdt-content-dataplane-full1c-implementation-plan.md (C1-C5)
# Spine:   genesis/manifests/habits.yaml — node `notary-authority`
@e2e @dataplane @concern:notary-authority @act:i @holochain:0.7
Feature: Notary authority — the federation converges on ONE earned canonical head
  Converged content state (the value) lives on the CRDT plane; the DHT notary witnesses
  authority, provenance, and HEAD-of-DAG selection over it. Many peers may legitimately author
  a version; exactly one head is CANONICAL, and canonical status is EARNED (socially-derived,
  attestation-gated reach promoted to the commons tier), tended by Elohim, never won by last
  writer. This feature asserts the two things that make the notary the notary: that every
  federation peer resolves the SAME canonical head — not merely "some notarized head" per peer —
  and that moving that head requires earned authority, refused for a peer that has not earned it.

  Both invariants are RED today: the landing EPR is green on both peers over DIVERGENT heads
  (convergence unwired), and the earned-authority promotion gate is not yet arbitrated — the
  decision is still a dev-time hard-coded stand-in for the social grant. The failures below name
  exactly what must come true, and pass unchanged the moment it does.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  @requires:multi-node
  Scenario: A federation peer serves the landing EPR at the notarized tier (baseline — green)
    # Green baseline. The landing EPR is served with a notarized head on the author/promoter
    # peer — the padlock is lit where the canonical head has been elected. This confirms the
    # notary path produces green at all; the gap is convergence (next scenario), not reachability.
    Then EPR "elohim-host-landing" trust is "notarized" on peer "alpha-A"

  @requires:multi-node
  Scenario: Every federation peer resolves the SAME canonical head (RED — the convergence gap)
    # RED-FIRST, and the heart of the notary. Today alpha-A and elohim.host each report
    # trust="notarized" but over DIFFERENT canonical heads: divergent dhtAnchorHash (different
    # notary action) and divergent blobHash (each host authored its own bundle). Both green over
    # two truths is a FALSE-GREEN — authority is a privilege of the peer that ran the deploy, not
    # a property of the commons. There is no wired cross-peer head election/adoption: the reconcile
    # heal loop detects divergence but re-affirms each peer's OWN head by design ("never adopt the
    # peer's value"), so the split is a stable no-op, not convergence.
    #
    # This scenario FAILS today with:
    #   EPR "elohim-host-landing": alpha-A dhtAnchorHash != elohim.host dhtAnchorHash
    #   — the federation carries two canonical heads; no earned head has been elected + adopted.
    #
    # When it passes, one canonical head (its notarized anchor) has been elected by earned
    # authority and ADOPTED by every federation peer — verified adoption (author/community
    # signature checked against the notary witness), never a blind copy of a peer's assertion.
    # A reader on elohim.host gets the same canonical answer as one on alpha-A.
    Then EPR "elohim-host-landing" resolves the same canonical head across peers "alpha-A" and "elohim.host"

  Scenario: A peer without earned authority cannot move the canonical head (RED — no earned-grant gate)
    # RED-FIRST. Moving the canonical head must require EARNED authority for that EPR — an
    # attestation-derived grant the community made and Elohim tends — not mere possession of a
    # write path. Today there is no arbitrated earned-grant gate: the served head advances by
    # last-writer recency (VERDICT L2 #3), so any peer on the wire could overwrite it (REQ-N5
    # poisoning vector). A peer that has earned nothing for this EPR is not refused; it simply wins
    # the race. (Until the social-grant path is wired, the promotion decision is a dev-time
    # hard-coded stand-in — this gate is what replaces that scaffold with earned authority.)
    #
    # Read-only-and-safe today: it probes the head-authority surface for a real notarized EPR and,
    # only if that surface exists, attempts an ungranted (unearned) move against a dedicated
    # test-persona id — never production content.
    #
    # This scenario FAILS today with:
    #   head-authority surface /db/content/elohim-host-landing/head on alpha-A does not refuse an
    #   ungranted move — no earned-authority arbitration exists.
    #
    # When it passes, the notary refuses a move by a peer lacking earned authority for the EPR
    # with 401/403 (reach-cohort edit-membership + author/community signature): earned authorship
    # declares, Elohim witnesses the grant, and no unearned peer can move the canonical head.
    Then the HEAD authority surface refuses a non-author move of "elohim-host-landing" on peer "alpha-A"

  # ── EPR upgrade lifecycle (archetype) ────────────────────────────────────────
  # The whole life of a versioned artifact, not just a single-shot convergence. An archetype
  # EPR (the landing is the exemplar) is authored, earns commons reach, becomes canonical, is
  # UPGRADED to a new version that earns canonical in turn, and — the clause that IS the
  # "deployed once, regressed to old" bug — the upgrade STAYS elected: a re-seed / re-projection
  # / restart must never fall the served head back to the superseded version. This is the RED
  # that defines "durable head", and the genesis lifecycle coverage the archetype needs.
  #
  # @wip: the author-v2 / promote-to-canonical / re-seed steps are not yet wired (they are the
  # cure this feature drives). Kept parseable and RED-defining so it names the contract without
  # aborting the run. Steward-declared binding is the near-term promotion mechanism; the long-term
  # trajectory is self-electing supersession-lineage (v2 declares it supersedes v1 → DAG tip),
  # so any peer holding both elects v2 deterministically. See the design + C3 backlog.
  @requires:multi-node @requires:owned-substrate
  Scenario: An archetype EPR upgrade elects the new head everywhere and does not regress (RED — durable upgrade)
    Given EPR "elohim-host-landing" resolves the same canonical head across peers "alpha-A" and "elohim.host"
    When the steward promotes a new version of "elohim-host-landing" to canonical
    Then EPR "elohim-host-landing" resolves the same canonical head across peers "alpha-A" and "elohim.host"
    And the canonical head of "elohim-host-landing" is the newly promoted version on peer "elohim.host"
    # Durability: the head must survive the substrate churn that caused the live regression —
    # a re-seed and a projector re-run must not resurrect the superseded head.
    When the substrate re-seeds and re-projects "elohim-host-landing"
    Then the canonical head of "elohim-host-landing" is the newly promoted version on peer "elohim.host"
    And the canonical head of "elohim-host-landing" is the newly promoted version on peer "alpha-A"

  # ── The upgrade doorbell (operator direction, 2026-07-11) ────────────────────
  # Today a declared canonical head reaches other peers only by the COINCIDENCE of
  # passive op-gossip meeting a heal-tick — convergence has no deadline. The doorbell
  # gives it one: when a head is declared, the declaring peer ANNOUNCES
  # "canonical-head-changed {content_id}" on a working plane (the libp2p storage gossip
  # the CRDT sync already trusts for liveness), and every receiving peer triggers its
  # OWN conductor resolve + verified stamp. The trust rule is absolute (REQ-N5 /
  # REQ-F4): the announcement carries NO authority and NO head value to adopt — it is
  # "go verify against the DHT now", and verification terminates in the receiving
  # peer's own conductor. A forged or stale doorbell ring is harmless: the peer just
  # re-verifies and lands wherever the notary actually points. This is also how the
  # CRDT sync plane and the DHT notary plane pull together instead of waiting on each
  # other — the value plane converges eagerly, the authority plane verifies eagerly.
  #
  # @wip: the doorbell carrier (gossip topic + conductor-resolve trigger) is not yet
  # implemented — dht-unity plan T4. Kept parseable and RED-defining.
  @wip @requires:multi-node
  Scenario: A declared canonical head rings the upgrade doorbell — every peer re-verifies and adopts within one sync window (RED — no doorbell)
    Given EPR "elohim-host-landing" has a declared canonical head on peer "alpha-A"
    When the canonical head declaration is announced to the federation
    Then EPR "elohim-host-landing" resolves the same canonical head across peers "alpha-A" and "elohim.host" within one sync window
