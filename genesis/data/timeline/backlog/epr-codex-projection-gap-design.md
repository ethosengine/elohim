---
id: "backlog-epr-codex-projection-gap-design"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Codex hook + agent projections absent from epr packages — needs a package-first design claim"
slug: "epr-codex-projection-gap-design"
written: "2026-08-06"
author: "agentic-developer"
status: "open"
priority: "medium"
area: "epr-meta"
domain: "design"
tags: [epr-meta, projections, codex, package-first, design-domain, needs-brainstorm]
---

# Codex hook/agent projections gap — design first

`epr doctor` reports Codex hook and agent projections are still absent from the
`.epr-meta/elohim/packages` layer (skills/commands/agentdocs project to both runtimes; hooks
and agents currently project to Claude only). Per the plant-eprfs family discipline this is a
package-first DESIGN question — what a Codex-side hook/agent projection even is (execution
contract, registration surface, fidelity gate), not an opportunistic edit to the projection
script. Route through the package-authoring skill + a design pass before any implementation.
