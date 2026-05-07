@browser @spatial @hosted-human @requires:doorway
Feature: Spatial Map Renders
  The /map route mounts SpatialMapComponent and MapLibre paints OSM tiles.
  This is the minimum proof of the geospatial Cybersyn stack — without it,
  Sprints 1-8 (commit 4b57a2c2) have no human-visible artifact.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  Scenario: Visitor sees the spatial map at /map
    When Matthew navigates to "/map" in the browser
    Then the page should load successfully
    And the spatial map component should be mounted
    And the MapLibre canvas should be rendered
