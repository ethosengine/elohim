# Chapter 5 of the resiliency-saga: adam co-stewards via a rea-agreement. matthew's
# content needs more than one steward to survive a single device loss — adam's
# co-stewardship is a Mishpat-notarized Commitment (action="replicates-commons" or
# "replicates-dwelling", per mishpat_projection.rs parse_replicates_commons /
# parse_replicates_dwelling), projected into elohim-storage's rea_commitments table
# as an active row (household_resilience.rs filters exactly these two action values
# for its commitment-backed collectives count).
#
# Proof signal: GET /api/v1/commitments?action=replicates-commons&state=active
# (elohim-storage/src/http.rs handle_db_... via db::rea_commitments::list_commitments,
# which filters on rea_commitments.action.eq(action) AND rea_commitments.state.eq(state)
# — an exact, already-wired HTTP surface, proxied by the doorway's generic "/api/"
# service-path prefix) reports at least one row within 60 seconds.
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): a polling commitment-count
# step against this exact surface (no existing step polls /api/v1/commitments with
# a retry loop — resilience.steps.ts's "I list active {string} commitments" reads
# once, with no retry budget).
#
# Status today: BORN RED — no such commitment has been notarized and projected on
# alpha yet; this chapter is the loop's work queue entry for wiring the co-steward
# agreement flow end to end. Do not weaken this assertion to make it pass.
@e2e @dataplane @concern:saga-05-co-steward-agreement
Feature: Chapter 5 — adam co-stewards via a rea-agreement
  matthew's content needs more than one steward to survive a single device loss.
  adam's co-stewardship is a Mishpat-notarized Commitment, projected into
  elohim-storage's rea_commitments table as an active row. This chapter is born red:
  the agreement has not yet been notarized and projected on alpha.

  Background:
    Given peer "alpha-A" at "alpha-A"

  # ---------------------------------------------------------------------------
  # STATIONS (added 2026-07-26, story-harvest of the overnight cure sprint).
  #
  # The 2026-07-26 cure sprint proved this chapter is not one node but a pipeline:
  # explicit pin (consent) -> provide tick -> mishpat notarization -> bounds-validated
  # ProvideAnnounce -> graduation -> mishpat->rea mirror -> the rea projection the
  # final scenario below reads. EIGHT stacked defects sat along it, each invisible
  # until the one above was cured, and the only way to see which link was live was
  # log archaeology. These stations make the earned links directly measurable, so a
  # future red names WHICH link broke instead of re-deriving the whole chain.
  #
  # The chapter's finish line is unchanged: the final scenario still reads the rea
  # projection and is still born red. A station passing is NOT the chapter passing.
  #
  # Parameter-bearing constraints discovered on this chain (all live-verified
  # 2026-07-26 ~12:30 UTC on alpha-A):
  #   * Poll budget vs cucumber's step timer: cucumber's DEFAULT step timeout is
  #     30s, shorter than a 60s poll — a "within 60 seconds" step MUST declare
  #     { timeout: 75_000 } or cucumber kills it at 30s before the poll's own
  #     governor ever fires (edge #1233: "function timed out ... 30000 ms").
  #     retry()'s own maxAttempts default (10) also exhausts a 60s budget in ~21s.
  #   * Route cache: the commitments routes are cached for 30s, so a freshly
  #     notarized row can lag a read by up to one cache window. A station read is a
  #     steady-state measurement, never a just-authored one.
  #   * Namespace rule: pin head_refs are `epr:`-prefixed ("epr:elohim-host-landing")
  #     while content ids on the pull route are BARE ("elohim-host-landing"). The
  #     provide tick matched prefixed head_refs against bare content ids, so the
  #     desired set was EMPTY for every pin that ever existed. Station A asserts
  #     both spellings deliberately — that mismatch is the defect it guards.
  # ---------------------------------------------------------------------------

  # Station A — consent is declared, and its bytes have actually landed.
  # Live on alpha-A: pin #433 (household-dowell), headRef "epr:elohim-host-landing",
  # kind=item, status=active; pull rollup {total:1,fetched:1,pending:0,failed:0,
  # caughtUp:true}. Both routes are doorway-proxied since edge #1238.
  # Honesty note: the /api/v1/pins list wire does NOT project a provide flag — this
  # station proves the pin exists, is active, is item-kind, and its bytes arrived.
  # Provide INTENT is proven downstream by station B: only the provide tick can
  # author that commitment, so a green station B is the provide leg's real witness.
  Scenario: A stewardship pin with provide intent is active and caught up
    Then doorway "alpha-A" has an active item pin whose head references "elohim-host-landing"
    And within 60 seconds doorway "alpha-A" reports the pull for "elohim-host-landing" is caught up

  # Station B — the notarize -> announce -> graduate leg, read at its own source.
  # This is the station that proves the governance path works even while the
  # mishpat->rea mirror (the final scenario's surface) is still dark: the commitment
  # is live in the mishpat ledger and never reached rea_commitments. Live on alpha-A:
  # cid uhCEkd2ZZTOht... action=replicates-commons provider=uhCAkYi1... state=active
  # — the first co-steward agreement ever notarised on the fabric (2026-07-26).
  # Reading /commitments/facing/rea (mishpat-sourced) rather than /commitments
  # (rea-projection-sourced) is the whole point: same word, two homes, one mirror
  # between them. A green B + red final assertion localises the break to the mirror.
  Scenario: The co-steward agreement is notarised and active in the governance ledger
    Then within 60 seconds doorway "alpha-A" has at least one active "replicates-commons" commitment in the rea-facing ledger

  # THE CHAPTER'S FINISH LINE — unchanged, still born red. Reads the rea projection
  # (rea_commitments), which the mishpat->rea mirror has never populated.
  Scenario: An active replicates-commons commitment names a co-steward
    Then within 60 seconds doorway "alpha-A" has at least one active "replicates-commons" commitment
