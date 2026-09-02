@e2e @lamad @epr-decomposition @native-content-graph @regression @requires:doorway @requires:seeded-content @act:i
Feature: Shared exploration sidebar surfaces authored and discovered neighbors
  A learner reading a doctrinal page sees, beside the content, the SAME "Explore"
  sidebar whether they opened it standalone at its universal EPR address or met it
  inside a learning path. The sidebar shows two kinds of neighbor, honestly
  distinguished: AUTHORED related concepts (the explicit relatedNodeIds the author
  curated) and a DISCOVERED — "you might also explore" — section of computed tag
  neighbors the resolver inferred (computed=true). Discovery is offered as
  exploration, never dressed up as authored truth: its cards carry a
  non-"explicit" inference source.

  This is the native content-graph seam slice. It is STORY-FIRST: the resolver
  (computed=true) and the shared sidebar are not yet on any live stack (alpha runs
  the old storage), so these scenarios go green once the branch is built and
  deployed. The witness corpus is the doctrinal docs, seeded as commons markdown
  EPR nodes: "confession" carries explicit relatedNodeIds to
  "constitution"/"manifesto"/"theology" AND shares the "stewardship" tag with at
  least one node NOT in its relatedNodeIds, which the resolver surfaces as a
  discovered (inferenceSource:tag) neighbor. The doctrinal corpus renders via the
  markdown renderer — independent of the absent epr-composite / PathViewer
  keystone — so the experience is provable on the household floor.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the doctrinal docs "manifesto","constitution","confession","theology" are seeded as commons markdown EPR nodes
    And "confession" has authored relationships to "constitution","manifesto","theology"
    And the backend computes at least one discovered (tag) neighbor for "confession" not in its relatedNodeIds

  @browser-only
  Scenario: Standalone viewer shows the See-also sidebar with both edge kinds
    # Runs against the legacy standalone /resource/{id} viewer (lamad's
    # ContentViewerComponent) — the shared exploration sidebar is a lamad
    # component and cannot cross into the shell-owned atom home at /epr/{id}
    # (spec: 2026-09-02-epr-atom-home-shell-component-design.md §4.2); a
    # standalone see-also affordance ON the atom home is filed against the
    # commons plan (spec §8), not built here.
    # Authored edges fill the See-also sections; the resolver's computed tag
    # edges fill the "Discovered — you might also explore" section.
    When the learner opens the EPR address for "confession"
    Then the exploration sidebar is visible
    And it lists "constitution", "manifesto" and "theology" as authored related concepts
    And it shows at least one discovered concept card
    And the discovered card's inference source is not "explicit"

  @browser-only
  Scenario: The same sidebar appears inside the path-step lesson view
    # The path-coupled lesson-view renders the SAME app-exploration-sidebar (the
    # whole point of the shared component): a learner who reaches the MANIFESTO as
    # a path step gets the identical authored + discovered neighborhood.
    #
    # Retargeted from "confession" to "manifesto" (resolving the captured C2 seed
    # gap the @wip documented): the doctrinal trio stays deliberately
    # un-path-attached (slice plan C1 Step 3), but "manifesto" is the markdown
    # node a seeded path ALREADY steps through (love-map-matthew-jessica step 2,
    # resourceId: "manifesto") — exactly the "retarget at the node a seeded path
    # already steps through" branch the gap note named. Manifesto's authored
    # cards are its cites-mesh relatedNodeIds (constitution/confession/theology);
    # its discovered card comes from the resolver's tag pass (external ≥1-shared-
    # tag neighbors exist beyond the explicit-precedence-excluded trio). This is
    # the operator-named delivery surface: graph-native discovery of the
    # theology/confessional beside the Elohim Protocol manifesto lesson.
    Given the learner is on a path step rendering "manifesto"
    Then the exploration sidebar is visible with authored and discovered neighbors

  @browser-only
  Scenario: Doctrinal markdown renders independently of the path keystone
    # Runs against the legacy standalone /resource/{id} viewer — the shared
    # exploration sidebar is a lamad component and cannot cross into the
    # shell-owned atom home at /epr/{id} (spec: 2026-09-02-epr-atom-home-
    # shell-component-design.md §4.2); a standalone see-also affordance ON
    # the atom home is filed against the commons plan (spec §8).
    # "theology" is a commons markdown EPR node. Its body renders via the markdown
    # renderer with no dependency on the epr-composite / PathViewer keystone, and
    # the shared sidebar populates beside it — the doctrinal reading experience
    # stands on its own.
    When the learner opens the EPR address for "theology"
    Then the markdown content body is rendered
    And the exploration sidebar is populated
