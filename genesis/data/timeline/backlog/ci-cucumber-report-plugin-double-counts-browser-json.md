---
id: backlog-ci-cucumber-report-plugin-double-counts-browser-json
kind: backlog
title: "CucumberReport plugin double-counts the browser report in elohim-genesis"
created: 2026-07-21
status: OPEN
domain: D-ci
source: shift 2026-07-21T19-50-land-presence-bootstrap-pipeline (ci-investigator, builds #1340/#1342)
severity: low
tags: [ci-hygiene, genesis-pipeline, cucumber-report]
---

The genesis pipeline's CucumberReport plugin ingests THREE cache files per build:
`cucumber-report-api.json`, `cucumber-report-browser.json`, and `cucumber-report.json` —
and `cucumber-report.json` is byte-identical to `cucumber-report-browser.json`
(verified: both 345046 bytes in #1342, both 344650 in #1340). The browser report is
therefore counted twice: #1342's "52 failed steps" = API 16 + browser 18 + duplicate 18;
#1340's "51" = 17+17+17. Effect: the plugin's failed-step headline overstates the real
failure surface by the browser count, and step-count deltas across builds are noisier
than the true scenario sets. Fix shape: exclude the duplicate glob (either stop emitting
`cucumber-report.json` alongside the browser file, or narrow the plugin's file mask in
genesis/Jenkinsfile). Evidence quoted from builds #1340/#1342 artifacts.
