---
id: artifact-store-brief-common
status: active
---

# Common rules for every artifact-store brief (read before your own brief)

- **Worktree:** your brief names it: `/projects/elohim/.worktrees/as-<track>` on branch `artifact-store/<track>`, `.family` = `sprint`. Never run git in `/projects/elohim` (the shared main checkout). Never `cd` into a worktree from the main shell; use `git -C <wt>` / `brit` with `-C`, or a subshell. One committer per worktree; commit by pathspec.
- **Envelope (Claude legs that commit):** `epr session open --model <your model id>` → `epr actor claim --as agent:<track>@<model id> --session <S>` → `berth moor --session <S> --runtime claude-code --model <id> --lab anthropic --task artifact-store-<track>`. A commit that prints `UNMEASURED` means fix the envelope before the next commit; never rewrite history.
- **Codex legs:** never run cargo or git; `TMPDIR=/var/tmp/as-<track>`; `-c sandbox_workspace_write.exclude_slash_tmp=true -c sandbox_workspace_write.exclude_tmpdir_env_var=true`; absolute `-o`; your receipt goes in your final message only, never appended to a document; never claim a review verdict.
- **Cargo:** `RUSTUP_TOOLCHAIN=1.96.1`; **`RUSTFLAGS` as the gate declares it** — the shell exports the WASM `--cfg getrandom_backend="custom"`, which fails native eprfs/doorway links with `undefined symbol: __getrandom_v03_custom` (eprfs: `RUSTFLAGS=""`; elohim-storage keeps the custom flag); prefer `just gate`, which sets it; set `CARGO_TARGET_DIR` from `cargo-pool key` in your worktree; never point two worktrees at one target dir; judge a run by `EXIT=$?` on its own line, never by piped output. One storage-class build on this device at a time; the household mesh stays down while elohim-storage builds.
- **Gate:** `just gate <project>` from your worktree, log to `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/gate-<track>-<project>.log` with `EXIT=` (path is in the main checkout's hub; write it with an absolute path).
- **KEEP (never touch):** `refs/notes/brit/**`; `/nix/xdg/config/elohim/device/`; `/nix/xdg/config/epr/peer/`; `.eprfs/status/{index,identity-reserve,recall}`; cite fingerprints; DNA zomes; Jenkinsfiles; `scripts/ci/*edge*`.
- **Never:** a dual-read, a prefix validator, a persistent iroh copy, a replica count at `self` reach, a query plane, a second key derivation for the same closure.
- **Report:** `genesis/docs/superpowers/plans/task-as-<track>-report.md` with frontmatter `status: claimed-not-verified`, `landed_commit:`, `verified_by:` (blank; the reviewer fills it), `concerns:`, `actor:`; body = what changed, tests by name with their result, gate log path and `EXIT=`, and any deviation from the brief with its reason.
- **Design record:** `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/design.md` governs over `plan.md` §2.1 where they differ.
