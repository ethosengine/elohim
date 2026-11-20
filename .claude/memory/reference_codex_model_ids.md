---
name: reference-codex-model-ids
title: "Codex CLI model ids and exec flags"
id: reference-codex-model-ids
description: "gpt-5.6 only as -luna/-sol/-terra (plain rejected), plus gpt-5.5, gpt-6-astra, gpt-6.1-sol, gpt-6.2-astra (default); review vs write exec flags."
metadata: 
  node_type: memory
  title: Codex CLI model ids and exec flags
  type: reference
  originSessionId: 81f8cba7-09d3-4cc0-83a9-af45293320fd
  modified: 2026-10-25T10:55:53.402Z
cites:
  - .epr-meta/elohim/algorithms/recall-contract.json
---

Codex CLI (`codex exec`, `CODEX_HOME=/projects/.codex-config`, config default `gpt-6.2-astra`, reasoning `medium`):
- Model ids the ChatGPT account accepts (from the CLI's own cache, 2026-09-22): `gpt-5.5`, `gpt-5.6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-6-astra`; the config (2026-11-20) also lists `gpt-6.1-sol` and `gpt-6.2-astra` — re-probe before relying on the list. Plain `-m gpt-5.6` returns `400 … not supported when using Codex with a ChatGPT account`.
- Operator's framing: 5.6 ≈ Opus-class (implementation legs), astra ≈ Fable-class (hard problems, reviews).
- Read-only review: `codex exec -s read-only --ephemeral --color never -c model_reasoning_effort="high" -o <result.md> - < <prompt.md>`.
- Write leg in a directory: add `-s workspace-write -C <dir>`; a non-git dir also needs `--skip-git-repo-check` or it refuses with "Not inside a trusted directory".
- `-o` writes only the last message; when the run errors the file is not created — check the stdout log.
- **Never `--add-dir /tmp`.** The workspace-write sandbox mounts a `.git` guard into each writable root, so `/tmp/.git` appears on the host for the run's lifetime. That makes every path under `/tmp` read as "inside a git repository": on 2026-10-24 it failed epr-cli's device-key tests mid-push. Grant a subdirectory instead (`/tmp/<run>`, `/var/tmp`).
- **Granting a subdirectory is not enough: workspace-write makes `/tmp` writable by default.** On 2026-10-24 an empty root-owned `/tmp/.git` was left behind AFTER a run that never named `/tmp` (only `--add-dir /tmp/eprfs-gate-target`); the next `just gate eprfs` failed `device_key::tests::{env_override_wins, m4_relative_or_in_repo_key_path_refused}`. Cure per run: `-c sandbox_workspace_write.exclude_slash_tmp=true -c sandbox_workspace_write.exclude_tmpdir_env_var=true` and give the leg `TMPDIR=/var/tmp/<run>`. After any write leg, check `ls -d /tmp/.git` and `rmdir` it (only ever empty).
- **A green sandbox run is not a green device.** The sandbox has no Claude ancestor process, no live device key and no live bin store, so session-, device- and pin-dependent code is never exercised the way pre-push exercises it. On 2026-10-25 slice A2 passed four sandboxed full-gate runs and then failed four real pushes on: berth's envelope guard with no sessions dir and a pre-envelope `epr`; recall refusing any non-pinned executor; hook tests resolving the live session through P1. Before pushing a leg's work, run its gates yourself outside the sandbox, and make fixtures isolate `EPR_PROC_ROOT`, `ELOHIM_DEVICE_KEY_FILE` and `ELOHIM_SESSION_ID`.

- **Rust legs must run the gate's clippy, on the gate's toolchain.** The sandbox defaults to `stable`; the gate runs `RUSTUP_TOOLCHAIN=1.100.0 cargo clippy -p <crates> --all-targets -- -D warnings`. On 2026-11-05 legs that ran only `cargo test` cost three repair rounds (large_enum_variant, result_large_err ×55, too_many_arguments). Put that exact command in every Rust brief.
Related: [[feedback_agent_fleet_and_harness]], [[feedback_sealed_decisions_must_not_outrun_evidence]].

Cited because `.epr-meta/elohim/algorithms/recall-contract.json` `lens_table.stated` keys Codex model ids (gpt-5.6-sol); an id the CLI stops accepting drifts that table.
