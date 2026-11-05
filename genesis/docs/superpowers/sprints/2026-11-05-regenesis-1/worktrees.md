# re-genesis 1 worktrees

Base: local dev 9c103746e (origin/dev is 426b0bcb7 and has diverged; the instructed command used local `dev`). Brit fallbacks: rev-parse reads, and `submodule update --init` (brit submodule only has `list`). Logs: /projects/elohim/genesis/local-dev/regenesis-1/logs/

| worktree | path | branch | HEAD | .family | submodules | pnpm | last line of output |
|---|---|---|---|---|---|---|---|
| rg1-kernel | /projects/elohim/.worktrees/rg1-kernel | regenesis/kernel | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-eprfs | /projects/elohim/.worktrees/rg1-eprfs | regenesis/eprfs | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-brit | /projects/elohim/.worktrees/rg1-brit | regenesis/brit | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-schemas | /projects/elohim/.worktrees/rg1-schemas | regenesis/schemas | 9c103746e | sprint | y | y | pnpm: Done in 12.3s using pnpm v10.30.3 sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-dna | /projects/elohim/.worktrees/rg1-dna | regenesis/dna | 9c103746e | sprint | n | n |  |
| rg1-storage | /projects/elohim/.worktrees/rg1-storage | regenesis/storage | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-doorway | /projects/elohim/.worktrees/rg1-doorway | regenesis/doorway | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-app | /projects/elohim/.worktrees/rg1-app | regenesis/app | 9c103746e | sprint | n | y | pnpm: Done in 12.2s using pnpm v10.30.3 |
| rg1-seeder | /projects/elohim/.worktrees/rg1-seeder | regenesis/seeder | 9c103746e | sprint | n | y | pnpm: Done in 9.4s using pnpm v10.30.3 |
| rg1-a2o | /projects/elohim/.worktrees/rg1-a2o | regenesis/a2o | 9c103746e | sprint | n | y | pnpm: Done in 9.3s using pnpm v10.30.3 |
| rg1-steward | /projects/elohim/.worktrees/rg1-steward | regenesis/steward | 9c103746e | sprint | y | n |  sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |
| rg1-integrate | /projects/elohim/.worktrees/rg1-integrate | regenesis/integrate | 9c103746e | sprint | y | y | pnpm: Done in 7.8s using pnpm v10.30.3 sub: Submodule path 'sophia': checked out '631f7f4b8d76e5eba2d10db5bbf73006300dea0d' |

All steps exited 0; no failures.

## Follow-up: rebase onto origin/dev (NOT applied)

Commits in 9c103746e..origin/dev (merge-base of the two is 7cc777e63):
426b0bcb7 fix(eprfs-core): give derive_or_reuse's lookup helper the same reasoned too_many_arguments allow
51b580f75 chore(eprfs): eprfs-core 0.3.4 and eprfs-meta 0.3.5 — Track A and the integrity lint changed the published packages
c1846d5b9 backlog(native-dev-plane): rows 5-8 — Track A's contract delta, the integrity-marker parity gap, and the gate and brit defects met while landing
74ebf6d34 feat(eprfs-meta): an integrity-class rule declared below `ask` fails validation
5aa7edbda feat(eprfs-core): slice 4 — walk_spine, project_view, derive_or_reuse over an injected RecordWitness
b05a282be Merge current dev with verified Brit collision-safe rebase pin
26701ea3a fix(brit): advance collision-safe native rebase to 0.1.11
66c09cf26 fix(bridges): cargo and jenkins lock the extracted rust-style scanner, not the whole checker
dc01152d8 governance(rust-style): the checker stops compiling the tree it polices — one-time bootstrap

elohim/brit gitlink moved: 9fd93a2bd8b71ddc5301d6bad4fa286f9ea405cd -> c2f8193105f5af57f6c5b9bfcf5ccd980e72ea8c

Rebase result: `brit rebase origin/dev` failed in every worktree ("shallow repositories require Git"; no state changed). Git fallback `git rebase origin/dev` stopped at da1726e1a in every worktree with a content conflict in genesis/data/timeline/backlog/arch-native-dev-plane-backlog.md: origin/dev's c1846d5b9 added rows 5-8 at the same spot where da1726e1a adds its own rows 5-7 (radicle custody/BlobStore/byte store). Not a pure replay. All 12 rebases were aborted; every worktree is back at 9c103746e, clean apart from .family. Submodules were not re-inited (no rebase landed).

## Final state (resolved rebase)

Git fallback: rebase — brit rebase refuses shallow repositories. Rebased in rg1-integrate onto origin/dev (426b0bcb7). Conflicts resolved per ruling: origin rows 5-8 kept verbatim; radicle rows renumbered 5-7 -> 9-11 (anchors row-9..row-11, amended text from 9c103746e kept); reference replacements applied in the radicle survey, the native-dev-tooling plan, arch-dataplane-borrows row 23, and CLUSTERS (Rows 9-11, count 11). Other 11 worktrees: `git reset --hard` to rg1-integrate HEAD (Git fallback: reset — brit has no branch reset verb). All 12 HEAD = 1051d0957585b9c302cedf8e05bcd13ccd46b9c7. Rust worktrees (kernel, eprfs, brit, schemas, storage, doorway, steward, integrate): elohim/brit HEAD = c2f8193105f5af57f6c5b9bfcf5ccd980e72ea8c.
