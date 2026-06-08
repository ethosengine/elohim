@e2e @lamad @epr-decomposition @native-content-graph @regression @requires:doorway @requires:seeded-content
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
    # The standalone /epr/{id} content-viewer renders the shared sidebar as a
    # pinned rail. Authored edges fill the See-also sections; the resolver's
    # computed tag edges fill the "Discovered — you might also explore" section.
    When the learner opens the EPR address for "confession"
    Then the exploration sidebar is visible
    And it lists "constitution", "manifesto" and "theology" as authored related concepts
    And it shows at least one discovered concept card
    And the discovered card's inference source is not "explicit"

  @browser-only @wip
  Scenario: The same sidebar appears inside the path-step lesson view
    # The path-coupled lesson-view renders the SAME app-exploration-sidebar (the
    # whole point of the shared component): a learner who reaches "confession" as a
    # path step gets the identical authored + discovered neighborhood.
    #
    # @wip — SEED GAP (captured for C2, not a fake pass): the witness corpus
    # deliberately leaves confession/theology/constitution NOT path-attached
    # (slice plan Task C1 Step 3: "Do NOT add any of these ids to paths/*.json").
    # So no seeded path steps through "confession" today, and the path-step step
    # below resolves its pathId/stepIndex from a runtime lookup that will find
    # none. Un-@wip once a path step renders a doctrinal markdown node (either
    # attach confession to a path step, OR retarget this scenario at the markdown
    # node a seeded path already steps through). The shared-sidebar contract it
    # guards — lesson-view renders the identical authored+discovered neighborhood
    # — is real and proven at the component layer (lesson-view spec); this is the
    # end-to-end witness waiting on a path-attached markdown step.
    Given the learner is on a path step rendering "confession"
    Then the exploration sidebar is visible with authored and discovered neighbors

  @browser-only
  Scenario: Doctrinal markdown renders independently of the path keystone
    # "theology" is a commons markdown EPR node. Its body renders via the markdown
    # renderer with no dependency on the epr-composite / PathViewer keystone, and
    # the shared sidebar populates beside it — the doctrinal reading experience
    # stands on its own.
    When the learner opens the EPR address for "theology"
    Then the markdown content body is rendered
    And the exploration sidebar is populated
