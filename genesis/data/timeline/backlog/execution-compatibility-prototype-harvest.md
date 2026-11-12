---
id: "backlog-execution-compatibility-prototype-harvest"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Harvest from the retired 2026-11-08 execution-compatibility prototype"
slug: "execution-compatibility-prototype-harvest"
written: "2026-11-12"
author: "claude-fable-5-2 (shem, session 3dd97e3d), from a read-only comparison against dev"
status: "open"
priority: "medium"
tags: [host-compatibility, ark, host-contract, qualification, harvest]
---

# Harvest from the retired 2026-11-08 execution-compatibility prototype

The steward retired the prototype on 2026-11-12. It was uncommitted work in shem's shared
checkout (17 files across `elohim/epr`, `elohim/placement/core`, `elohim/ark/cli`, the SDK
schemas, one a2o story, one habit atom and native-dev-plane spec section 2a). The
`host_contract` crate (`crates/elohim-host-contract`) supersedes its CPU, OS and ABI
comparison, its refusal reasons and its rule that observations are never serialized.
Ethosengine (lane K owner) wanted none of it. The bytes are kept in
`genesis/local-dev/stewardship-20261112/shared-checkout-44.tar` on shem (not in the repo).

This entry keeps what dev does not yet have.

## 1. The `ark compatibility` surface, for the A2/A3 ark lane

- Command: `ark compatibility --contract <PATH> --artifact <PATH>`, both required. Read-only:
  it never launches, signs or publishes.
- Bounded reads: the contract is a regular file of at most 64 KiB, parsed as JSON; the
  artifact is a regular file of at most 64 MiB.
- Byte-exact check first: the artifact is hashed and compared with the contract's artifact
  identifier; a mismatch exits 65 with "artifact mismatch" on stderr.
- Output: one JSON object on stdout with the artifact identifier, the identifier of the
  contract bytes, a list of issues (each with a reason and the evaluator or capability), and
  `scope: "local-comparison-only"`.
- Exit codes: 0 for no issues, 1 for issues, 65 for a data failure.
- Over `host_contract`, the command would build `Provides` from the A2 observation adapter and
  call `admit(&Requires, &Provides)`.
- Observation adapter (A2 is open and unassigned): the prototype probed x86 features with
  `is_x86_feature_detected!` and read arch, OS and environment from the build target. Rule to
  keep: a missing observation means unknown; only an explicit false means absent.
- Test vectors: a contract naming an unimplemented ABI evaluator yields exactly one
  `unobserved` issue and exit 1; a contract of an unsupported version yields
  `unsupported-contract`; changed artifact bytes yield exit 65.

## 2. Ideas with no twin on dev

- **Qualification scope.** A qualification is bound to five things together: artifact,
  contract, method, workload and environment, plus an execution mode (native, emulated or
  static). The modes are not comparable with each other, and a change to any of the five
  invalidates the qualification. The sprint's only emulation item is the K6/K10 refusal.
- **Accelerated paths.** An optional map of guarded faster paths that never raises the
  admission floor. The plan defers the accelerator axis (item X2).
- **A versioned envelope and a JSON schema** for the requirement payload. `Requires` on dev has
  neither.

## 3. Story scenarios with no twin on dev

- A C dependency compiled at a v3 default while Rust targets v2 must refuse.
- A broken qualification runner reports "unmeasured", never a verdict on the candidate.
- Admission of a native TLS build keeps the prior install across an illegal-instruction exit
  (132), a timeout and a certificate-setup error.
- Losing a peer preserves the local install.
- The same principal on several peers counts as one for independence.
- A coverage view shows missing coverage as missing, not as passing.

## 4. Wording worth keeping

- Habit guard: "compatibility is not reach, permission, principal independence or public
  graduation."
- Spec section 2a: custody is device-held or network-held and a doorway is not a custodian;
  public graduation has its own prerequisite; the accelerator belongs in the contract after
  CPU, OS and ABI.

## Unverified

- The prototype's habit cited a proof directory
  `genesis/a2o/reports/recovery/brit-cpu-20261108/`; nobody checked that it exists.
- Whether unreadable or oversize inputs to `ark compatibility` also exit 65 was not confirmed.
