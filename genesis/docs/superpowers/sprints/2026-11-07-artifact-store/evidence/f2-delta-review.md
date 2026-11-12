---
id: f2-orchestrator-delta-review
cites: []
---
# F2 delta review of 5c53da02 (orchestrator, Claude Opus 5.5, 2026-11-08)

Scope: the four fixes ruled in f2-ruling.md (items 3, 9, 11, 12), diff e31083b7..5c53da02. Read, not re-run; gates on e626043f: eprfs EXIT=0 (evidence recorded), orchestrator EXIT=0 (evidence recorded).

Verdict: **APPROVE** for the ruled scope.

- **Item 3.** `tool_lines` (artifact.rs) keys `cc`, `ld` and `$CC` resolved under the recorded fixed PATH: canonical path plus the CID of `--version` output, `-` when absent. hc-mesh.sh sets the same PATH in the env -i allowlist and records it with `describe`. The test moves the key on a version change and on a path change. Gcc's internal `collect2`/`ld` prefix may bypass PATH, but `cc --version` covers the gcc release and `/usr/bin/ld` is the usual resolution. Accepted.
  **Consequence for F2r:** a canonical tool path (a nix store path, an image's gcc) now enters the key. Shem and ethosengine reuse each other's builds only when their resolved cc/ld match. Compare the two devices' `tool:` key lines before the 454 MB receipt; a mismatch is a correct miss, not a fetch failure.
- **Item 9.** `served_store_matches` refuses `unrecorded:` when the lock is held and the marker is missing, unreadable, empty or names another store. The test covers all four plus the passing case. Accepted.
- **Item 11.** `Bound { wait, by_deadline }` is decided once when the bound is computed (remaining ≤ own bound), and carried through the dial, manifest and chunk paths. No tolerance window remains. A refused dial is now distinct from an elapsed one. Accepted.
- **Item 12.** Staging directories are named `.fetch-<dest>.<32 hex>` and hold an flock on their `lock` file for their life. The sweep removes only matching, non-symlink directories whose lock it can take, or lockless ones past 60 s. The kernel releases flock on SIGKILL, so a killed run's directory is reclaimable. The creation-to-lock race is named in the report and fails safe (LocalStore, local build). Accepted.

Residuals carried as ruled (f2-ruling.md): 1, 2, 4, 5, 6 (same-account), 10 (F5/I6), follow-ups 7, 8, item 13 to F2r.
