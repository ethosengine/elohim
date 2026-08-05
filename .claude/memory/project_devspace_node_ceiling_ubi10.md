---
name: project_devspace_node_ceiling_ubi10
title: Devspace node is capped at 22.x by UBI10; CI runs 24
description: "udi-plus lineage gets node from UBI10 appstream (only 22.23.1, no 24 stream) — CI's ci-builder is node:24-bookworm, so dev/CI skew is structural, not drift."
metadata: 
  node_type: memory
  type: project
  originSessionId: 8c38d17a-1437-45fd-93ff-ae4300ae7bb1
  modified: 2026-07-30T14:54:32.031Z
---

`che-devworkspaces/containers/udi-plus/Dockerfile` provisions node with an
**unpinned** `dnf install -y nodejs npm` on `base-developer-image:ubi10-latest`.
Verified on the live base (2026-07-30): UBI10 appstream offers **only**
`nodejs 22.23.1-4.el10_2`, and `dnf module list nodejs` returns *no matching
modules* — there is no node-24 stream to enable. Every devspace image inherits
this: udi-plus-mem, udi-plus-mem-rust-nix, udi-plus-angular, rust-nix-dev.

CI is on a different line entirely: `containers/ci-builder/Dockerfile` is
`FROM node:24-bookworm` (since 814b01c, 2026-07-30), and ci-builder-nix +
ci-playwright inherit it. **So dev/CI node skew is structural, not drift** —
you cannot close it by rebuilding the devspace images.

Two consequences:

1. The monorepo root `package.json` declares `engines.node: ">=24.15"`. No
   devspace image can satisfy it; only CI can. pnpm's `engine-strict` is unset
   (default false), so this warns instead of failing — which is why it went
   unnoticed. Closing it means either fetching node 24 into udi-plus from a
   version-pinned vendor tarball (matching how that Dockerfile already installs
   kubectl/nerdctl/buildctl) or relaxing the declared floor to Angular's own
   range.
2. Because the install is unpinned, the devspace node floor tracks RHEL's
   default stream. Angular 22.1.0 requires `^22.22.3 || ^24.15.0 || >=26.0.0`;
   UBI10 was shipping **22.22.2** — one patch *below* that floor — until
   22.23.1 landed. A RHEL stream decision can silently drop devspaces under an
   Angular requirement.

Related: [[project_angular22_node24_campaign]], [[project_ci_playwright_image]].
