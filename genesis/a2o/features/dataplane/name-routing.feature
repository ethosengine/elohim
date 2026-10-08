# A doorway answering for a public name, read on a household that owns its
# doorways and has no ingress in front of them.
#
# Wire detail: steps/dataplane/name-routing.steps.ts owns the reads. The rule
# under test is doorway/doorway-service/src/services/name_routing/standing.rs
# (`served_under_standing`), applied in server/http.rs before dispatch; the
# membership documents are written by relay-addr-beacon's `file` sink, one leg
# per owned doorway, under $MESH_DIR/membership/, and hc-mesh.sh hands every
# household doorway that directory (DOORWAY_MEMBERSHIP_DIR) plus, for alpha and
# apex, the names their beacon legs compete for (DOORWAY_PUBLIC_NAMES).
#
# Campaign story 2.2 (genesis/docs/superpowers/plans/
# 2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md), the
# household precondition for sprint 2.3's two-doorways-behind-one-name fleet
# run. Habit: served-under-standing (routing half); doorway-failover reads it as
# a station. Federation path routing — a doorway relaying a SITE path it holds
# no contract for — is a different file, features/federation/name-routing.feature.
@e2e @dataplane @concern:served-under-standing @act:i @requires:multi-node
Feature: A doorway answers for a public name only when it stands for that name
  A public name, such as elohim.local, is the address a visitor types. It does
  not belong to one machine. The household keeps one membership document per
  public name, listing the doorways currently advertised as able to answer for
  it; a doorway's entry leaves the document while it cannot serve and returns
  when it can. A doorway stands for a name when it is declared one of that
  name's doorways, whether or not the document lists it at this moment. Being
  listed can change from minute to minute; standing for a name does not. On
  this household two doorways, alpha and apex, stand for both
  household names, elohim.local and alpha.elohim.local, and are normally listed
  in both documents. The third doorway, gamma, stands for neither: it reads the
  same membership directory and finds itself in no document. The name
  alpha.elohim.local does not mean "alpha's machine"; it is a second public name
  both doorways carry.

  The holder of a name is the first doorway its document lists. The holder's
  answer is the name's reference answer. Any other listed doorway answers the
  name itself, from its own copy of the site, when it serves the same landing
  bundle for that name as the holder does. Each doorway reports, at its
  coherence endpoint, which bundle it serves at each path; the build stamp is
  the version the bundle's own version file names. The two are checked
  separately: the stamp is a label a build declares, while the bundle is
  addressed by its bytes, so only the bundle proves two doorways serve the
  same files. A doorway that stands
  for a name but is not listed at the moment hands the request to the holder
  rather than refusing it. A doorway that does not stand for the name answers
  HTTP 421 Misdirected Request and names the doorways that are listed.

  A doorway's federation id is the stable identifier it reports for itself at
  its coherence endpoint (/api/v1/federation/coherence); it is not any public
  name. Every answer a doorway gives for a public name carries an
  x-elohim-served-by header holding the federation id of the doorway that
  produced it, so a client outside the household can always tell which doorway
  answered. A doorway that hands a request on to the holder also adds an
  x-elohim-name-route header; an answer without it was produced by the doorway
  that was asked.

  Each request below goes straight to one doorway's own address with the public
  name in the Host header. Nothing sits in front of the doorways, so the answer
  is the doorway's own decision. Public DNS, TLS and reaching a doorway across
  the internet are not tested here; they belong to the fleet run in sprint 2.3.

  Each doorway takes up a newly published landing bundle on its own clock: it
  checks its own storage for a new bundle every 30 seconds, and learns which
  bundle the other doorway serves on its next federation check. For that short
  window after a publish or a restart the two serve different bundles, and a
  listed doorway that is not the holder correctly hands visitors to the holder.
  The scenarios below are about the settled household, so the background first
  waits, up to two minutes, until both doorways report the same bundle at the
  landing mount, the site's root path "/". Not converging within two minutes is
  itself a failure.

  Not staged here: a listed doorway whose declared version differs from the
  holder's, and a doorway that stands for a name while it is not listed. Both
  hand the request to the holder. The household cannot yet put one doorway
  behind the other, or hold one out of the document while it keeps serving, on
  demand, so those rules are covered by the doorway's own tests, not this file.

  Background:
    Given the household's membership documents list doorways "alpha" and "apex" as members of "elohim.local" and "alpha.elohim.local"
    And doorways "alpha", "apex" and "gamma" each read public-name membership from the household's membership directory
    And doorways "alpha" and "apex" have converged on the bundle each serves at the landing mount, the root path "/"

  Scenario: A listed doorway that is not the holder answers the name with the holder's build stamp
    Given doorway "alpha" is the holder of "alpha.elohim.local"
    When a visitor asks doorway "apex" for "alpha.elohim.local" at that doorway's own address
    Then the answer is HTTP 200 and contains the landing page's app-root element, where the site's app starts
    And the answer's x-elohim-served-by header is the federation id doorway "apex" reports for itself
    And the answer carries no x-elohim-name-route header
    And doorway "apex" serves the same build stamp under "alpha.elohim.local" as doorway "alpha" does
    And the landing bundle doorway "apex" serves under "alpha.elohim.local" is the one doorway "alpha" serves for that name

  Scenario: A doorway that does not stand for the name refuses it and names the listed doorways
    Given doorway "gamma" stands for no household name
    When a visitor asks doorway "gamma" for "elohim.local" at that doorway's own address
    Then the answer is HTTP 421 Misdirected Request
    And the refusal names every doorway the membership document lists for "elohim.local"
    And the answer's x-elohim-served-by header is the federation id doorway "gamma" reports for itself

  Scenario: The holder's own answer names the holder too
    Given doorway "alpha" is the holder of "alpha.elohim.local"
    When a visitor asks doorway "alpha" for "alpha.elohim.local" at that doorway's own address
    Then the answer is HTTP 200 and contains the landing page's app-root element, where the site's app starts
    And the answer's x-elohim-served-by header is the federation id doorway "alpha" reports for itself
