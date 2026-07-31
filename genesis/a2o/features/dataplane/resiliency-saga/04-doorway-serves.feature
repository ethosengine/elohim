# Chapter 4 of the resiliency-saga: matthew hosts a doorway. A human hitting "/" on
# his doorway must get the actual landing SPA shell back, not a 404 or an error page
# — hosting is only real when it is human-visible.
#
# Proof signal: GET / -> HTTP 200 with the rendered SPA shell (<app-root>) present in
# the response body. The structural <app-root> tag is used rather than the page
# <title> text so this check survives copy/branding changes.
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): a small raw-body query pair
# ("... expecting raw text" / "the raw response status is ..." / "the raw response
# body contains ...") — the existing generic "When I query" step's captured body is
# private to dataplane.steps.ts and has no body-substring assertion, and
# delivery.steps.ts's "the response is HTML containing ..." reads a DIFFERENT
# capture store populated only by its own "When I request/fetch" steps. The raw
# query step polls the peer's /health until ready (bounded) before issuing the
# GET, absorbing the doorway-pod-restart window that immediately follows an edge
# deploy without masking a genuine outage — see waitForDoorwayReady in
# src/framework/dataplane/surfaces.ts.
#
# Status: see the chapter table in README.md (this directory) — the README, not this header, is the authority (headers here went stale mid-2026-07).
@e2e @dataplane @concern:saga-04-doorway-serves
Feature: Chapter 4 — the doorway serves
  Hosting a doorway means a human's browser gets a real page back. This chapter
  proves the root path answers 200 with the actual SPA shell rendered — the visible,
  human-facing proof that matthew's doorway is up and serving.

  Background:
    Given peer "elohim.host" at "elohim.host"

  Scenario: The root path serves the landing SPA shell
    When I query "/" on peer "elohim.host" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"
