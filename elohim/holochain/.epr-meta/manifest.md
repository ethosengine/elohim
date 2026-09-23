---
epr-meta-version: 1
id: elohim-holochain-governance
covers: subtree
purpose: >
  The DHT plane: the DNAs (lamad, imagodei, infrastructure, node-registry, mishpat), the edge
  deploy pipeline, and the sweettest harness. This manifest hosts the habit atom for notary
  authority — the promise that converged state can be NOTARIZED, and that authority answers come
  from the notary rather than from last-write-wins order. It carries no author-time rule: the
  drift this tree actually suffers is deploy-shaped (a DNA-content change not reaching running
  conductors; a coordinator-only change that never moves the hash), and those are gated by the
  pipeline and documented in the root CLAUDE.md, not by an edit-time predicate.
---
# elohim/holochain — governance package

One DNA pipeline builds every DNA (`dna/Jenkinsfile`); a DNA subdirectory holding only
`dna.yaml` + `zomes/` + a `justfile` is normal and fully covered. Do not read the absence of a
per-DNA Jenkinsfile as missing CI.

The habit atom here is projected into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.

`covers: subtree` claims the whole DHT plane: `dna/` (the DNA sources; `dna/.epr-meta` adds the
hash-neutrality signal and `dna/imagodei/` hosts its own habit), `rna/` (the migration toolkit that
carries data across DNA versions), `tools/` (`hc-dbtool`), `tests/` (`sweettest/` carries its own
manifest; `manifest-hygiene/`), and `edgenode/`, `elohim-wasm/` and `docs/`. Where a deeper
manifest exists its rules still cascade; this claim only ends the coverage walk here.
