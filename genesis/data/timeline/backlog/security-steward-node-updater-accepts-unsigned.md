---
id: "backlog-security-steward-node-updater-accepts-unsigned"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "steward/node's self-updater accepts unsigned downloads — retire it onto the exec floor"
slug: "security-steward-node-updater-accepts-unsigned"
written: "2026-10-24"
author: "Claude (orchestrator, Opus 5.5)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "genesis/docs/superpowers/specs/2026-10-23-household-follows-latest-design.md"
  - "genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
tags: [security, auto-update, steward, exec-floor, retire]
---

**Defect.** `steward/node/src/update/download.rs` `verify_signature` logs
"Signature verification not yet implemented" and returns `Ok(())`. A download is
therefore accepted on a sha256 match against a manifest fetched from the same
doorway that served the bytes. That proves integrity in transit, not authenticity.
The updater is wired: `dashboard/setup.rs` builds it, and `apply.rs` renames the
download over `current_exe`.

**Disposition: retire, not repair.** The tevah spec ("Delivery of bytes") already
names this updater as superseded by the exec floor:

- spawn only from a `VerifiedRelease`;
- re-hash the bytes at spawn;
- record the CID that ran;
- keep the stakes gate.

`household-follows-latest` §7 lists "extending `steward/node/src/update/` instead
of retiring it" as an anti-goal. Implementing Ed25519 here would harden a second
updater beside `release_adoption`.

**Probe.** `grep -n "Ok(())" steward/node/src/update/download.rs` inside
`verify_signature`. A steward build that reaches the update path with a manifest
whose `signature` is garbage still applies.

**Current state.** Open. The cure lands when steward/node's process is launched by
`ark` from a verified release, at the household-follows-latest slice 3 rung or
later. Delete `steward/node/src/update/` and its `dashboard/setup.rs` wiring in the
same change.
