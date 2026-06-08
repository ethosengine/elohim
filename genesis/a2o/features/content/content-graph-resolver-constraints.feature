@e2e @lamad @epr-decomposition @native-content-graph @content-graph-resolver @requires:doorway @requires:seeded-content
Feature: Native content-graph resolver — discovered constraints
  # Harvested 2026-06-08 from the native-content-graph-seam slice (story-harvest).
  # These lock CONSTRAINTS surfaced DURING implementation that the happy-path a2o
  # (content/exploration-sidebar.feature) does not cover. All @wip until step defs
  # are wired AND the storage build carrying the resolver is deployed (alpha currently
  # runs the pre-resolver storage). Personas + doorway background per a2o convention.

  Background:
    Given the doorway gateway is serving the lamad content API
    And the elohim-storage native content-graph resolver is deployed

  @wip @regression @browser-only
  Scenario: Incoming explicit edges survive the resolver graph source
    # CONSTRAINT (load-bearing): the resolver graph endpoint is OUTGOING-traversal only.
    # The panel MUST MERGE the list-both source (explicit edges, BOTH directions) with the
    # graph endpoint's computed edges. Replacing the source with the graph endpoint ALONE
    # (the first B2b cut) silently drops INCOMING explicit edges — the parents / Part-Of
    # bucket goes empty for a node reachable only by an incoming edge.
    # Regression anchor: if this risk stops reproducing, the resolver may now support
    # direction=both natively (the filed follow-up that would retire the Angular merge).
    Given content node "child-concept" has an incoming "CONTAINS" relationship from "parent-concept"
    And "child-concept" has no outgoing relationship to "parent-concept"
    When the learner opens the EPR address for "child-concept"
    Then the exploration sidebar lists "parent-concept" as an authored related concept

  @wip @browser-only
  Scenario: The panel degrades to explicit-only when the resolver graph endpoint fails
    # CAPABILITY: getContentGraph and getRelationships each catchError to empty at the
    # service layer, so a graph-endpoint failure (404 / 5xx / undeployed) degrades the panel
    # to explicit-only rather than blanking it — a forkJoin errors as a whole otherwise.
    Given content node "confession" has authored relationships to "theology" and "manifesto"
    And the content-graph resolver endpoint for "confession" returns an error
    When the learner opens the EPR address for "confession"
    Then the exploration sidebar still lists "theology" and "manifesto" as authored related concepts
    And the discovered section is empty
    And the exploration sidebar is not blank

  @wip @regression
  Scenario: A content-rooted graph never leaks contributor or identity nodes
    # CONSTRAINT (privacy / domain boundary): graph.json's traversal whitelist is filtered
    # to EprHead-to-EprHead (content-to-content) edges only. MASTERY_OF
    # (ContributorDID-to-EprHead) is EXCLUDED so a content-rooted traversal cannot follow
    # mastery links out to learner-identity nodes.
    # Regression anchor: the whitelist initially included MASTERY_OF (caught in A5 review).
    Given content node "confession" exists
    And a contributor has a "MASTERY_OF" relationship pointing at "confession"
    When the content-graph for "confession" is resolved
    Then every returned node is a content node
    And no returned node is a contributor or learner-identity node

  @wip
  Scenario: The resolver clamps unbounded graph-query parameters at the HTTP boundary
    # CONSTRAINT (security / resource bound): the route u32-parses then clamps each param,
    # closing a usize-as-i64 to SQL LIMIT -1 (unbounded scan) truncation and bounding cost.
    # Operational parameters (defense-in-depth: route clamp AND resolver cap):
    #   depth         default 1, HARD CAP 3         (bounds BFS connection-holding + fan-out)
    #   maxComputed   default 25, HARD CEILING 100  (bounds tag-discovery fan-out)
    #   minSharedTags default 1, floor 1
    # Informs: content-graph resolver / NodeCapabilities presets + operator docs.
    # Review after: major releases, storage container sizing changes.
    When a graph query for "confession" requests depth 99 and maxComputed 9999
    Then the resolver traverses at most depth 3
    And it returns at most 100 computed edges
    And no database query is issued with a negative row limit

  @wip
  Scenario: Computed discovery edges are never persisted (Category C)
    # CONSTRAINT (substrate): computed (inferenceSource=tag) edges are recompute-on-read,
    # NEVER written to the relationships table or the DHT — the resolver trait has no write
    # method (structural enforcement). Two peers with the same content compute the same tag
    # edges with no consensus (offline-correct).
    Given content nodes "confession" and "concept-indigenous-wisdom" share the "stewardship" tag
    And they have no authored relationship between them
    When the content-graph for "confession" is resolved with computed edges enabled
    Then the response includes a discovered edge to "concept-indigenous-wisdom" with inference source "tag"
    And no new relationship row is persisted for that discovered edge
