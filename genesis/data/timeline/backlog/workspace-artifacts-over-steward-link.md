---
id: "backlog-workspace-artifacts-over-steward-link"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Build artifacts shared between the steward's workspaces over the steward link: rakia input key, closures, brit build records"
slug: "workspace-artifacts-over-steward-link"
written: "2026-10-23"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "rakia / brit / eprfs / cargo-bridge"
domain: "D5"
relatedNodeIds:
  - "habit:materialize-what-is-read"
  - "habit:devices-share-one-berth"
tags: [rakia, brit, artifacts, eprfs, steward-channel, second-plan]
---

# Artifacts over the steward link (the second plan)

The design is `workspaces-cooperate-as-devices-design` §8. Plan it once T4 of the channel plan lands
(the tier ALPN with StewardDevicesGate), because it reuses that ALPN and admission unchanged.

- **Input key:** the CID of `{project, recipe, toolchain, rustflags, files: path→bafkrei…}`, built
  from the gate project's `inputs.sources` globs and `Cargo.lock`.
- **Record:** the output closure's root CID, the builder device and session, and the cargo-bridge
  Process. It is a crossing event on the channel allow-list.
- **Fetch:** a view walk of the peer's tree (materialize-what-is-read, scenario 2).
- **Ruling (operator, 2026-10-23):** agents may commit the key and record types to rakia-core and move
  the pin under the pin-attestation gate.
- **Graduation:** shem runs `just mesh preflight` on binaries built on ethosengine, with no cargo
  invocation on shem.
