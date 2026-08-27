---
epr-meta-version: 1
id: elohim-holochain-governance
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
