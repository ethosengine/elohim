---
name: project_dna_hash_depends_on_build_path
title: DNA hash depends on the absolute build path, not only source
id: project-dna-hash-depends-on-build-path
description: "Cargo metadata includes package paths: identical source in different worktrees changes DNA hashes — check before hash comparisons or coordinator hot-swap."
metadata:
  type: project
---

**Verified 2026-09-06 (slice 1 of accountable-correction):** identical integrity source, identical `dna.yaml`, identical
`Cargo.lock`, same rustc 1.98.0 — `lamad.dna` packed in `.claude/worktrees/accountable-correction` hashed
`uhC0ku0iA72…` while the main-tree pack hashed `uhC0kbYC3xV…`. The integrity WASMs differ by 38 bytes and the
string diff is only the v0 crate disambiguator in mangled symbols (`Cs16yqjtqznoq_` vs `Cs4vmAsx1PpWD_`): Cargo
derives `-C metadata` for a path package from its source path, so a different absolute workspace path yields
different symbol names → different WASM bytes → different DNA hash.

**Consequences:**
- A local "DNA hash unchanged" check is only meaningful when both packs come from the SAME absolute path. Compare
  integrity source + `dna.yaml` + lockfile by git instead, or pack both at one path.
- The fleet's hash comes from CI (`$WORKSPACE/elohim/holochain/dna/elohim`, `dna/Jenkinsfile:151,492`), with no
  `--remap-path-prefix`/metadata pinning — "coordinator-only changes keep the DNA hash" holds only while the Jenkins
  workspace path is identical build to build (a `workspace@2` concurrent checkout would move the hash with zero
  source change and partition the fleet). File under the Evolution epic: pack DNAs at a canonical fixed path.
- Sweettests in a worktree use a self-consistent local pack; they never prove fleet hash identity.

See [[project_pipeline_dispatch_ordering]], [[project_holochain_evolution_epic]].
