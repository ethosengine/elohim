---
index: false
id: project-ci-storage-topology
name: CI build storage topology — migrated openebs-jiva → openebs-hostpath; hostpath needs deterministic node pinning
title: CI build storage — hostpath PVCs in jenkins ns, node-pinned
description: "CI cache PVCs (nix/cargo/sweettest-target) are openebs-hostpath in jenkins ns; pin with kubernetes.io/hostname nodeAffinity or pods thrash on volume binding."
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
cites:
  - genesis/manifests/nix-cache-pvc.yaml
  - elohim/holochain/Jenkinsfile
  - elohim/holochain/dna/Jenkinsfile
  - steward/device/Jenkinsfile
---

Fixture snapshot of this entry's frontmatter; the body is deliberately not copied.
