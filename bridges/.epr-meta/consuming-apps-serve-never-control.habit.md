---
epr-habit-version: 1
id: consuming-apps-serve-never-control
invariant: >
  A consuming app (a legacy service doing work beside the network, Jenkins first) is served by the
  network and never controls it. Every record a consuming-app bridge emits is an observation-plane
  external-claim observation with the app as provenance only. Every gift sits under a governed
  offer that is active only on a verdict from a Steward who is not its author, creates no claims,
  and discloses both sides of the edge. A fixture approval never reads as network validation.
status: green
active: false
checks:
  - "cd bridges/jenkins && just gate: the translator's output admits only Process/FlowEvent; observe refuses without mutation on malformed input and on a Proposed or withdrawn offer; re-observing is idempotent (the `jenkins-bridge` gate project)"
  - "cd elohim/eprfs && just gate eprfs: `service:` is provenance-only; it cannot claim, author, attest or approve, and a Steward can't approve an offer they authored"
  - "jenkins-bridge drift over the latest observed edge build prints no `undisclosed capability` against the ACTIVE offer (the declared-vs-observed gap; checked only once the offer is active)"
refs:
  - "canon: bridges/CLAUDE.md §Consuming apps: served, never in control (five rules)"
  - "atlas: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md §3.6 (consuming apps)"
  - "offer: bridges/.epr-meta/offers/jenkins-edge-pipeline.offer.md"
  - "precedent: records-lifecycle §D.8 (vendor bridges enter as observations), generalized from money to work"
  - "stewardship: .eprfs/status/affiliations.jsonl (plural Stewards; human:adam is a fixture)"
guard: >
  Regression risks: (1) a later rung (the plan as a gift, reuse evidence) that lets a consumer's
  report skip or decide native work without a human-rooted grant — control arriving in a gift's
  costume; (2) a second consumer copying the Jenkins shapes instead of lifting the offer and claim
  envelope to a shared home; (3) green on a fixture approval alone — bootstrap validation proves
  the primitive runs, not that the network has two real Stewards; (4) drift counting stages wfapi
  reports as FAILED after an earlier failure (~55 ms, never ran) as having exercised their
  capabilities, which inflates the observed side of the gap.
retire-when: >
  when CI stages run as native peer-executed steps and every consuming app is served only gifts,
  with its observations graduated by peer evaluation rather than read by the bridge that wrote them.
---
DELTA 2026-09-25: born RED. Slice 1 landed the first consuming-app bridge:
- the `edge-pipeline` recipe;
- `bridges/jenkins` (translate/observe/drift/card), which can only express observations;
- `service:` as a provenance-only participant kind;
- the governed, two-sided-disclosure Jenkins offer, which starts Proposed.

Receipt over the real archives (orchestrator #1903→edge #1483 SUCCESS, #1906→edge #1484 FAILURE):
- 18 and 20 records appended; re-observing appended 0;
- #1484's failed stage reads `dismiss`;
- drift is clean on both builds against a fixture-repo offer.

Checks 1 and 2 are green by their gates. Check 3 waits on the operator's approving verdict, which
is not minted by any agent. Status stays red until the offer is active and drift runs against it.

DELTA 2026-09-25: guard risk (2) closed before a second consumer arrived. The external-claim
envelope and the offer declaration now live in `elohim_epr_rea::external` (pure; offer standing
stays in epr-cli), and `jenkins-bridge` and `epr flow`'s offer standing import them with no copy
left. `Process` gained `classified_as` (skipped when empty; golden
`an_unclassified_process_keeps_its_pre_classified_cid` holds the pre-change CID), and each observed
build's Process carries its envelope and build slots. Receipt over #1483/#1484 (graph basis, plus
#1484 wfapi-only): all 17+19+19 event records are byte-identical to before; only the 3 Process
CIDs moved, because they now carry slots.
Status stays red on check 3.

DELTA 2026-09-26 (FLIPPED GREEN; every check green by its own command):
1. `just gate jenkins-bridge` EXIT=0 at 2800ba8e0, 14 + 5 pass. The translator admits only
   Process/FlowEvent; observe refuses without mutation and is idempotent; a real graph with a
   still-queued sibling (orchestrator #1910) now translates.
2. `just gate eprfs` EXIT=0 on a clean dev checkout. `service:` can't claim, author, attest or
   approve, and an author can't approve their own offer.
3. The operator (human:matthew, Steward, via a device-signed steward-of-record claim) approved
   the offer. The approval was re-recorded after the offer text was corrected, so the offer
   was re-addressed, and it is now active as bafyreibkpfcowvdxtxzpyomqfeqawbo4dh4uhjsmpo5q7vpnvl3xvvwsxe.
   `jenkins-bridge observe` of the latest completed edge build, #1485 (orchestrator #1910),
   appended 18 records as process bafyreihzojv3fydxljwjoyrn5lgtljzaaujijqzqzpcw5trixyxpvw5xz4
   in scope of that offer. `jenkins-bridge drift` reads: clean — no recipe drift, no
   undisclosed capability. Informational only: Staging and Prod deploy not executed; the
   credential store and the GitHub webhook disclosed but not exercised.

Guard (4) is closed too: a FAILED stage that started after an earlier failure ended is a
cascade skip that exercises nothing. The invariant holds at Bootstrap stakes for the one
consumer the collective owns (crossesNetwork: false).

