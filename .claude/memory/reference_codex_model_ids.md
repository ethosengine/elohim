---
name: reference-codex-model-ids
title: "Codex CLI model ids and exec flags"
description: "gpt-5.6 only as -luna/-sol/-terra (plain rejected), plus gpt-5.5, gpt-6-astra; review vs write exec flags."
metadata: 
  node_type: memory
  title: Codex CLI model ids and exec flags
  type: reference
  originSessionId: 81f8cba7-09d3-4cc0-83a9-af45293320fd
  modified: 2026-09-22T02:39:16.214Z
---

Codex CLI (`codex exec`, `CODEX_HOME=/projects/.codex-config`, config default `gpt-6-astra`, reasoning `medium`):
- Model ids the ChatGPT account accepts (from the CLI's own cache, 2026-09-22): `gpt-5.5`, `gpt-5.6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-6-astra`. Plain `-m gpt-5.6` returns `400 … not supported when using Codex with a ChatGPT account`.
- Operator's framing: 5.6 ≈ Opus-class (implementation legs), astra ≈ Fable-class (hard problems, reviews).
- Read-only review: `codex exec -s read-only --ephemeral --color never -c model_reasoning_effort="high" -o <result.md> - < <prompt.md>`.
- Write leg in a directory: add `-s workspace-write -C <dir>`; a non-git dir also needs `--skip-git-repo-check` or it refuses with "Not inside a trusted directory".
- `-o` writes only the last message; when the run errors the file is not created — check the stdout log.

Related: [[feedback_agent_fleet_and_harness]], [[feedback_sealed_decisions_must_not_outrun_evidence]].
