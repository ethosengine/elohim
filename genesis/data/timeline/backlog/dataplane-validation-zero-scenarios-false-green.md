---
id: "backlog-dataplane-validation-zero-scenarios-false-green"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane Validation reads SUCCESS when zero scenarios ran — support-load crashes are silent false-greens"
slug: "dataplane-validation-zero-scenarios-false-green"
written: "2026-07-31"
author: "claude (saga-final-chapters shift)"
status: "open"
priority: "high"
jobs: [elohim-edge]
tags: [dataplane-validation, a2o, cucumber, false-green, ci, edge]
cites:
  - elohim/holochain/Jenkinsfile
  - genesis/a2o/steps/fixture-devices.steps.ts
---

# Zero-scenario validation runs must not read as green

Edge #1275 ([edge:validate-only]) posted SUCCESS while its Dataplane
Validation ran **zero scenarios**: a `Cannot find module 'chai'` in one step
file killed cucumber's entire support load, the advisory catchError swallowed
it, and the stage printed `Findings: 0 (scenarios: 0)` + `(no @concern:
tagged scenarios ran)` — indistinguishable from a clean pass at the
build-status level. Only a manual log read caught it ("silence is not
success"). The same hole would mask any future support-load regression,
config error, or empty tag-filter.

## Fix shape

In the Dataplane Validation stage (elohim/holochain/Jenkinsfile), after the
cucumber run: if the parsed scenario count is 0, mark the stage UNSTABLE with
an explicit "0 scenarios ran — support load or filter is broken, this is NOT
a pass" message (validate-only runs included). A validate-only build whose
only purpose is measurement must hard-distinguish "measured green" from
"measured nothing".

## Occurrence

- 2026-07-31 edge #1275: chai import in fixture-devices.steps.ts (from the
  device-fixture lint pass 1d45c6428) — fixed by converting to node:assert.
