---
name: elohim-package-authoring
description: Author and maintain Elohim-native skills and agents from .epr-meta/elohim/packages, treating Claude and Codex files as generated projections.
metadata:
  author: elohim-protocol
  version: 1.0.0
  sourceRuntime: elohim-agent
  packageKind: SkillPackage
---

# Elohim Package Authoring

Use `.epr-meta/elohim/packages` as the canonical authoring surface for Elohim-native skills and agents.

## Rules

- Edit package JSON first.
- Treat `.claude/*` and `.codex/*` as runtime projections.
- Run the package projection check after changing packages.
- Do not require a Claude source file for packages whose `metadata.sourceRuntime` is `elohim-agent`.

## Workflow

1. Author or edit the package under `.epr-meta/elohim/packages`.
2. Regenerate projection fixtures with the package projection CLI.
3. Generate runtime surfaces only when the repo intentionally wants local Claude/Codex files refreshed.
4. Verify projection drift before committing.
