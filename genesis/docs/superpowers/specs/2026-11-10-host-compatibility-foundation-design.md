---
title: Closure, not image — the host-compatibility foundation under the dataplane
id: host-compatibility-foundation-design
status: Draft
class: architecture
serves: [sprint-lands-each-closure-once, measure-runs-on-a-peer, workspace-device-arrival, dataplane-convergence]
date: 2026-11-10
context-tier: disclosed
steward: human:matthew
author: "Claude (Fable 5.2), at the operator's direction; grounded by three read-only seam surveys on 2026-11-10 (envelope/placement, artifact-store keying, hardware/packaging canon)"
tags: [compatibility, requires, provides, admit, variant-set, closure, declared-target, diversity-of-peers, ark, placement, artifact-store, wasm, isa, abi, accelerator]
graduation-trigger: >-
  Two devices on their own native images (x86-64-v3 and x86-64-v2) and one device of a second
  architecture (aarch64) each pin the same variant-set CID for elohim-storage. Each resolves its own
  variant with no human naming an instruction set, runs it, and exchanges the same declared content
  with the other two in both directions. A variant whose requirement a device does not meet is refused
  before launch with a typed reason naming the unmet axis, and a requirement on an axis the device
  does not know is refused as unknown. Receipts attach to sprint-lands-each-closure-once and
  measure-runs-on-a-peer.
cites:
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md
  - "compute-envelope-tevah | manifest/berth split, the reserved closure field, the Wasm process kind and the fingerprint this spec extends | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "what-every-app-inherits-placement-contract-design | rule 14 and section 13 (native variants chosen from device capability) and rule R-P4 (device facts stay local) that this spec implements | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "native-dev-plane-one-derivation-earned-reach-design | derivation keys and the reach ladder; its 'a differing toolchain computes a different key' sentence is kept literally true | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
  - "elohim-seam-map-concern-routing | seams 3.1-3.3 (hardware, OS/packaging, runtime) whose shared compatibility question this spec answers | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "device-footprint-residency-carrying-capacity-design | the diversity-of-peers epic; capacity stays there, compatibility is here | sha256:99ab8f8330cc3f24 | path: genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md"
---

# Closure, not image — the host-compatibility foundation under the dataplane

Status: **Draft for review by all working sessions.** Nothing here is built, so each rule below is a
proposed requirement with no enforcer yet. §12 lists the rulings this
spec asks the steward for, and §13 lists what each working session should check it against.

## 1. Why this exists

The protocol's peers are meant to differ: a watch, a phone, a laptop, a household box, a rack, and
within one class different processors, operating systems and accelerators. Sharing artifacts and
running workloads across them needs one answer to one question: **will these bytes run on this
host?** Today nothing in the protocol can ask it.

What happened when the question went unasked:

- **Edge build 1683** died with `Fatal glibc error: CPU does not support x86-64-v3` on a Sandy
  Bridge host. The fix was a second hand-built image.
- **F2r (2026-11-10)**: two of one steward's devices could not share a build of `elohim-storage`.
  The first fix moved both devices onto the weaker machine's image. The steward rejected it: no
  levelling devices to the lowest common denominator.
- **K1b** then proposed a compatibility check inside the artifact store, specific to ELF, glibc
  and x86.

Each was a local repair of the same missing seam. The 2026-10-22 harvest already named the rule,
"blood-type compatibility for donor and recipient", and the placement contract already says native
binaries "vary by architecture and cargo features, chosen from the device's declared capability".
Neither is built, and neither says what happens for ARM, RISC-V, a GPU or hardware not yet made.

This spec does three things:

1. Defines the compatibility contract once: **requires**, **provides**, **admit** (§3–§5).
2. Replaces the image as the unit of shipping with the **closure**, whose members each carry their
   own requirement, and the **variant set**, which lets one pin span different hosts (§6–§7).
3. Names every consumer of the contract and what changes at each (§10), so the artifact store, ark,
   placement and release adoption stop growing private versions of it.

### 1.1 What it is not

- Not a scheduler. Compatibility answers "can it run here". It does not answer "should it" or "is it allowed".
- Not capacity. Memory, CPU share, disk and VRAM quantities stay in placement's capacity stage.
  The harvest's sentence holds: "Spare RAM cannot satisfy missing instructions. A compatible ELF
  cannot authorize a task."
- Not a new store, registry, DHT entry type or schema plane. Records ride existing ones (§9).
- Not a rule that every device runs the same bytes. Only the portable tier is identical everywhere.

## 2. Current state

| Area | Wired today | Gap |
|---|---|---|
| Host observation | `ark_supervisor::driver::Fingerprint` reports hostname, kernel release, cgroup delegation, enforcement tier | No architecture, instruction-set features, libc, OS family or accelerator. The tevah spec lists "arch" as a passport attribute; the code omits it |
| Artifact requirement | `ark_core::candidate::CandidateRuntime.platform` (an exact target triple) and `capabilities`, with `WrongPlatform` and `MissingCapability` refusals | Only tests call it. It compares strings for equality, so it cannot express that a build for an older instruction-set level also runs on a newer host |
| Pins | `ArtifactRef::Pinned { cid, sha256, bytes }`, release `Artifact { blob_cid, … }`, image tag `conductor-<hc12>` | Each names exactly one set of bytes. A shared `RuntimeManifest` CID cannot span two hardware flavors |
| Placement | `Requirements` and `OfferTerms` cover capacity, isolation, scopes and carriers | `admit` has no compatibility stage and no refusal reason for it |
| Derivation key | `MethodDescriptor` keys rustc version and host triple, cargo, flags, `RUSTFLAGS`, and the path and `--version` text of `cc` and `ld` | It keys `ld`, which the build does not use, and omits the C runtime files the link does use. That is what split F2r |
| Device catalogue | `devices.schema.json` has free-text `cpuClass` and `gpu` | `boot_registration.rs` drops both. They are marketing names, not requirements |
| Producer-side proof | brit builds at an x86-64-v2 ceiling and runs the bytes under `qemu -cpu Nehalem` and `Haswell` | The result is not recorded as a requirement any consumer reads |
| Portable artifacts | wasm zomes and DNA, JS bundles, generated types and seed data are identical on every host | The tevah `Wasm` process kind is declared and returns `UnsupportedKind` |
| Other targets | The node updater matches `(os, arch)` over `x86_64 / aarch64 / armv7`; the conductor fork's flake lists `aarch64-linux` and `aarch64-darwin` | The updater is superseded by the tevah design. Nothing builds or proves a non-x86 target |

Standing rules this spec keeps:

- Footprint tuning is native-binary-only. wasm and DNA bytes are not varied per device (placement contract rule 14).
- A manifest is shared and input-addressed; a berth is one device's instance of it (tevah §3).
- A differing toolchain computes a different key, so nothing is falsely shared (native-dev-plane §4).
- Device facts do not leave the device; only their effect does (placement rule R-P4).

## 3. The contract

Three parts, all pure data and one pure function.

```
Requires  — what an artifact needs of a host   (travels with the artifact)
Provides  — what a host can run                (observed on the host, stays on the host)
admit(Requires, Provides) -> Admitted | Refused(reason)
```

The rule is directional. An artifact's requirement must fit inside what the host provides. A host
that provides more than is required is compatible; the reverse is not.

### 3.1 Axes

A requirement is a set of clauses. Each clause names an **axis** and a predicate on it. A host's
`Provides` is a set of observed values, one per axis it knows how to observe.

An axis is defined by an id and one of four comparison kinds. The kinds are the closed part of the
design; the axes are open.

| Kind | Satisfied when | Used for |
|---|---|---|
| `equal` | the host's value equals the required value | architecture family, OS family, byte order |
| `subset` | every required feature is in the host's feature set | instruction-set extensions, engine features |
| `at-least` | the host's version is not lower than the required version, within one named line | kernel ABI, libc symbol version, OS deployment target, API level |
| `present` | every required name is resolvable on the host | loader path, shared libraries, accelerator APIs |

Initial axes. Only the x86 rows are proven; every other row is a named, unproven extension point.

| Axis id | Kind | Example requirement | Proof state |
|---|---|---|---|
| `arch` | equal | `x86_64`, `aarch64`, `riscv64` (a wasm module carries no `arch` clause; its gate is `engine.wasm`) | x86_64 observed |
| `os` | equal | `linux`, `android`, `darwin`, `ios`, `windows`, `browser` | linux observed |
| `isa.x86` | subset | features of `x86-64-v2` | v2 and v3 observed (builds 1683, 1685; F2r) |
| `isa.arm` | subset | `neon`, `sve`, `crypto` | unproven |
| `isa.riscv` | subset | extension letters, or a ratified profile | unproven |
| `kernel.linux` | at-least | minimum kernel ABI | unproven |
| `abi.libc` | at-least | `glibc ≥ 2.34`, `musl ≥ 1.2`, `bionic ≥ API 30` | glibc observed |
| `abi.loader` | present | `/lib64/ld-linux-x86-64.so.2` | observed |
| `lib` | present | `libgcc_s.so.1` | observed |
| `accel.<api>` | at-least | `vulkan ≥ 1.3`, `cuda ≥ 12`, `metal ≥ 3`, `webgpu` | unproven |
| `engine.wasm` | subset | `simd128`, `threads`, component-model features | zomes run under the conductor's engine; no node-level host |

Rules for axes:

1. **Open set, namespaced ids.** New hardware adds an axis or a value. It must not change the shape
   of `Requires`, `Provides` or `admit`.
2. **Named bundles are content-addressed.** `x86-64-v3` is an alias for a fixed feature list. The
   list is a small document with a CID, so the name cannot drift between peers or over time.
3. **Unknown means refuse.** A clause on an axis the host does not know is refused as
   `unknown-axis`. A clause on an axis the host knows but could not observe is refused as
   `unobserved`. Neither is guessed.
4. **An absent clause requires nothing.** A wasm module carries no `isa.*` clause. A static binary
   carries no `abi.libc` clause. Requirements shrink as artifacts become more portable.
5. **Quantities are not axes.** How much VRAM or memory a workload needs is capacity and belongs
   to placement. Which accelerator API it calls is compatibility and belongs here.

### 3.2 Where each clause comes from

A requirement read only from an artifact's bytes is not sound. A Rust crate built with no
`target-cpu` can still contain C code compiled at the build host's default `-march`, and the ELF
note that would reveal it is often absent. So each clause records its source:

| Source | Meaning | Example |
|---|---|---|
| `declared` | The build named it as an input (§6.3) | the declared target says ISA level v2 and glibc floor 2.34 |
| `extracted` | Read from the artifact's bytes | ELF machine, interpreter, needed libraries, highest symbol version |
| `proven` | The exact bytes ran on a host or emulator of that class | brit's qemu Nehalem and Haswell runs |

Clauses from different sources are merged by taking the stricter one, meaning the one fewer hosts
satisfy: the higher version, or the larger feature set. A disagreement between
`declared` and `extracted` (the build said glibc 2.34, the bytes need 2.39) is recorded and the
stricter value wins.

The source of a clause matters because of how far the artifact travels. `admit` itself only
compares; which sources are good enough at which reach is decided by the existing reach ladder:

- A device may run its own build on `declared` and `extracted` clauses.
- Wider reach requires `proven` clauses for the host classes it is offered to.

Evidence can also be withdrawn:

- A crash that contradicts a requirement (an illegal-instruction death, a loader failure) is
  counter-evidence. ark's death witness already records the death; the witness names the variant,
  and the claim is withdrawn for that host class. Counter-evidence is floor-protected.

### 3.3 Refusals

`admit` returns one of:

| Verdict | Carries |
|---|---|
| `Admitted` | the clauses checked and the source of each |
| `Refused(unmet)` | the axis, the required value and the host's value |
| `Refused(unknown-axis)` | the axis id |
| `Refused(unobserved)` | the axis id |

A refusal happens before launch and before fetch of the bytes where the requirement is known
first. It must not fabricate a completion or fall back to a guess.

## 4. Provides: observed, private

A host builds its `Provides` at startup by observation: CPU feature flags, kernel release, the
libc it loads, the loader paths that resolve, the accelerator APIs that initialise. It must not be
read from a manifest, a catalogue row or a device-class label. It is observed again when the
envelope starts and when the kernel, a driver or an attached accelerator changes.

`Provides` does not leave the device. Hardware detail identifies a machine, and the standing rule
is that device facts stay local and only their effect crosses. Every use of the contract is
therefore evaluated **on the host that would run the bytes**:

- **Fetch**: the fetching device reads the variant set and admits locally.
- **Placement**: the requester sends the variant set with the task. The provider answers which
  variant it will take, or refuses. The offer does not state what the provider is.
- **Building for another device**: the device that wants the build names the declared target it is
  asking for. That is its own choice to disclose, scoped to the request.

What an outside observer can still learn: a peer that serves variants sees which variant a device
fetched, which reveals a coarse hardware class. Fetch history carries the reach of the artifact
and must not be public metadata, as the harvest already requires.

## 5. Three tiers of portability

Variety is absorbed at three levels, and an artifact declares which it uses simply by which
clauses it carries.

| Tier | What it is | Requires | Bytes across devices |
|---|---|---|---|
| **Portable** | wasm, JS, data, configuration | an engine and its features | identical everywhere |
| **Floor-carried native** | a native binary that brings its own C runtime: statically linked, or shipped with its loader and libraries as closure members | `arch`, `isa.*`, `os`, `kernel.*` | one variant per architecture and ISA level |
| **Host-bound native** | a native binary that uses the host's libc and libraries | the above plus `abi.*` and `lib` | one variant per host ABI line |

How the tiers are used:

- **Portable is the universal floor.** A new instruction set joins the network when the node
  runtime and a wasm engine run on it. Everything in the portable tier works that day. Native
  variants for the new hardware are an optimisation that arrives afterwards.
- **Floor-carried is the default for native artifacts that are shared**: releases, placed tasks
  and tool binaries. Its requirement reduces to what the hardware and kernel truly own.
- **Host-bound is the default for a device's own development builds**, where the host's toolchain
  is the point. Host-bound builds are shareable through the same contract; they simply match
  fewer hosts.

This is the answer to "level the floor without levelling the devices". No device changes its
image. The floor is a property of the artifact.

An artifact whose `Requires` is empty (an image, a text, a data file) is **universal**: every host
admits it, it has one form, and a pin names its CID directly with no variant set. Variants exist
only where `Requires` names a host axis.

### 5.1 What must agree, and what may differ

The tiers say how far bytes travel. They do not say which code belongs in which tier. That line is
drawn by whether peers must compute the same result.

| Kind of code | Examples | Form |
|---|---|---|
| **Must agree** | validation, content addressing, `admit`, elections | one deterministic form shared by all peers. Portable is right here because two peers must reach the same answer, not because of build cost |
| **May differ** | how bytes move, how storage is laid out, how an accelerator is used, how the kernel is driven | a population of native variants, each fitted to its hardware and judged only at the contract (§7.1) |

The portable tier is not a virtual machine laid over the devices to make them alike. Hardware
variety is treated as an ecology: many variants, selected by evidence. Three things are durable and
are designed with care, because many peers depend on them and no single peer can regenerate them:

- the **contract**: the axis vocabulary and the interfaces a native member is judged at;
- the **conformance suite** (§7.1), which is the selection pressure;
- the **attestation** of who built a variant, from what, and where it passed.

Implementations are the disposable side. A native variant may be generated or adapted for one
device and replaced freely, provided it passes. The cost that remains is proving, and proving is
parallel by construction: each device proves its own variant on its own hardware.

## 6. Closure, not image

### 6.1 What is wrong with the image as the unit

A container image fuses five things that have different identities and change at different rates:
the userspace base, the toolchain, the application binary, configuration, and the process model.
Its layers are ordered differences, so a change at the bottom changes every digest above it. One
difference in the base, such as an instruction-set level, therefore produces a whole second image,
a second tag, a second devfile, and a second set of pins. That is the ripple seen in the devfile
split and in the v2 edge image.

The lvi design already states the direction: "Mount, don't ship — kill the image as a unit." The
tevah process spec already reserves a `closure` field, and the placement contract defers "shared
runtime closures" to a later design. This section is that design.

### 6.2 The closure

A **closure** is an unordered set of content-addressed members. Each member has its own CID and
its own `Requires`. A berth mounts the members it needs; it does not unpack layers.

| Member kind | Examples | Tier |
|---|---|---|
| Portable member | DNA, hApp, wasm guest, app bundle, seed data, configuration | portable |
| Native member | `elohim-storage`, `doorway`, `ark`, the conductor, `epr`, `brit` | floor-carried or host-bound |
| Floor member | a C runtime, loader and library set that floor-carried members link against or load | carries only `arch`, `isa.*`, `kernel.*` |
| Toolchain member | compiler, linker, sysroot (build closures only) | native |
| Conformance member | the suite a variant set names: harness, vectors and expected outputs (§7.1) | portable |

Two closures use the same machinery:

- A **run closure** is what a berth mounts to run a workload. It is named by the
  `RuntimeManifest`.
- A **build closure** is what a derivation mounts to build: toolchain members, a sysroot and the
  source view. It is the `DevspaceSeed` projection's "toolchain marker blobs", which that spec left
  as its first open design task.

Because members are independent, a difference in one does not move the others. A v2 and a v3 peer
share every portable member and differ in the native members only.

### 6.3 The declared target

A native build names its **declared target** as a build input:

```
declared target = { arch, isa bundle, os, floor: sysroot CID, C compiler flags }
```

- The **sysroot** is a floor member: the startup objects, libc stubs and compiler support
  libraries the link uses. It is content-addressed and fetched from the artifact store like any
  other member. The linker uses it instead of whatever the build host's OS provides.
- The **C compiler flags** fix the instruction-set level for C dependencies, so the requirement
  no longer depends on the build host's compiler defaults.

Consequences:

1. Two devices on different native images that build the same source for the same declared target
   use identical link inputs. Their derivation keys match and they share an identity hit. This
   closes F2r for the common case without levelling either device.
2. The artifact's `declared` clauses are known before the build runs: the ISA bundle and the
   libc floor the sysroot implies.
3. A device may also build a second variant tuned to its own hardware. That is another member of
   the same variant set (§7), not a replacement.

4. A declared target adds sharing; it is not a condition for building. A device that does not hold
   the sysroot, and cannot reach a peer that does, still builds with its own host toolchain. That
   build is host-bound, keyed truthfully, and runs on the device that made it.

The declared target is part of the method, so it is part of the key. The standing sentence stays
literally true: a differing toolchain computes a different key.

How the floor is realised for a given binary is a measured choice between three mechanisms, and
this spec does not pick one for all binaries:

| Mechanism | Result | Known cost |
|---|---|---|
| Pinned sysroot at an old libc floor | host-bound with a low, declared `abi.libc` clause | still needs a compatible host libc |
| Static link (musl) | floor-carried | allocator and resolver behaviour differ; `steward/node` already ships this way |
| Carried loader and libraries | floor-carried | mixing a carried libc with host libraries is unsafe; the Tauri pipeline hit this as stack smashing and patches the interpreter today |

### 6.4 What becomes of images, Nix and devfiles

| Thing | Role under this design |
|---|---|
| OCI image | A **carrier**: one way to deliver a closure to a host that only accepts images, such as a k8s node. The k8s bridge renders it from the closure. Its digest is delivery evidence and is not the workload's identity |
| Workspace image and devfile | The carrier of a device's bootstrap and its native toolchain. Each device keeps the image native to its hardware. Shared build inputs come from the build closure, not from the image |
| Nix | Its derivation math is kept, as lvi and tevah already rule. Its store and language are not adopted. A Nix-built toolchain or sysroot is a legitimate way to produce a closure member |
| Image flavor tags (`:latest`, `:v2-latest`) | Stay as carriers. Nothing pins them as identity. A member's CID is the identity |

## 7. Variant sets

One logical artifact has several variants. A **variant set** is a small content-addressed document:

```
VariantSet {
  family:      key of the derivation with the declared target erased
  conformance: CID of the suite every variant must pass (§7.1)
  variants:    [ { requires: Requires, artifact: CID, key: derivation key, proofs: [attestation CID] } ]
}
```

- Each variant is an ordinary derivation with its own key and its own record. Selecting a variant
  is an identity hit on that variant's key. Nothing is admitted under a key it was not built under.
- A derivation key is the CID of a method descriptor (recipe, tools, flags, declared target) paired
  with an input root. The **family key** is the same descriptor hashed with its declared-target
  fields left out. A device that can compute its own key can therefore compute the family key
  locally, with or without a manifest.
- Each variant's record is indexed under both its own key and its family key. A fetch that misses
  on its own key reads the records under the family key. Lookup stays by key or CID; no query
  plane is added.
- A variant set is an open population, not a list one party closes. Whoever publishes a release
  assembles the first set from the variants it holds proofs for. Any device may contribute a
  variant by appending an entry with its proof (§7.1). A device may hold a wider set at its
  steward's reach than the one a release elected.
- A pin names the **variant set CID**. Peers on different hardware therefore share one pin, and one
  `RuntimeManifest` CID, which restores "peers on one declaration share one manifest CID".
- The berth records which variant its device resolved. That is a per-device fact, which is what a
  berth is for.

Selection rule, evaluated on the host:

1. Drop every variant `admit` refuses.
2. Among the rest, prefer the variant whose requirement is the most specific the host satisfies
   (a v3 host prefers a v3 variant over a v2 one).
3. If none remains, refuse with the reasons for each. Build locally where a recipe exists;
   otherwise report the unmet axis to the person in plain words.

**What already runs keeps running.** Adopting a new variant set does not remove the variant a
device is running until its replacement is held, verified, admitted and started. A device that
cannot be admitted to any variant of a new set stays on what it has and is told why. Losing a peer,
a pool or a doorway removes no variant a device already holds.

A variant set grows by appending. A device with new hardware builds its variant, proves it, and
adds it. No existing pin or peer is disturbed until a new variant-set CID is elected through the
same channel that elects any release.

Different builders of the same variant key may produce different bytes, because development
builds are not deterministic. That is the existing `Mismatch` case in the artifact store and is
unchanged here. It is one more reason shared variants should be built from a declared target with
a release profile.

### 7.1 The conformance suite: how different variants agree

Variants differ in bytes by design, so agreement cannot be a byte comparison. It is a behavioural
one. Each variant set names one **conformance suite** by CID. A variant belongs to the set because
it passes that suite, and for no other reason.

- The suite is a portable closure member: a harness, input vectors and expected outputs. It drives
  a variant only through the interface the contract names (its command line, its wire protocol,
  its files) and compares observable behaviour in a canonical form. It does not compare bytes,
  timing or layout.
- Expected outputs are minted by something other than the variant under test: the portable
  reference where one exists, otherwise a different variant. A suite whose expectations came from
  the variant it judges proves only self-consistency.
- A run produces a **conformance attestation**: variant CID, suite CID, the bundle names of the
  host class it ran on, the verdict and the signer. This is the `proven` clause source of §3.2 and
  the build-provenance attestation named as a missing node in §9.
- A proof is valid for the host class it ran on. A pass on one class does not transfer to another.
- Changing the suite makes a new variant set. Proofs against the old suite do not carry over.

Two kinds of variant join the same set:

| Kind | How it is made | Key | How it is found |
|---|---|---|---|
| Same recipe, different declared target | the release recipe built for another target | its own key, sharing the family key | under the family key, or in the set |
| Adapted | source or recipe changed for one device or class, by a person or an agent | its own key, outside the family | only through its entry in the set |

What a device needs before it runs a variant it did not build:

| Relationship | Evidence that suffices |
|---|---|
| The steward's own devices | a sibling's attestation for the same host class |
| Wider reach | the device runs the suite itself before first adoption, or holds converging attestations from independent provers, as the reach ladder requires |

Limits stated plainly:

- A suite proves what it tests. Code where peers must reach the same answer does not rest on a
  suite; it has one form (§5.1).
- Conformance says behaviour agrees. It does not say a variant is harmless. Native code runs with
  the authority its envelope gives it, and today the envelope enforces nothing
  (`EffectiveTier::None`). Provenance, reach and envelope confinement govern that risk, and an
  adapted variant widens it.
- A variant that fails is refused and the device keeps what it runs (§7, "What already runs keeps
  running").

### 7.2 Finding a variant, and who holds it

Two questions, answered by two existing mechanisms. No index of variants by hardware is added.

1. **Which bytes?** The variant set answers. A device reads the set it pins, admits locally, and
   has a CID.
2. **From whom?** A variant is an ordinary content-addressed blob. Its holders are resolved the way
   any blob's are, by CID. Universal artifacts and variants share one store and one lookup.

What content addressing buys the network:

- Universal and portable members are held once and shared by every host class. Only the native
  members differ between classes, so a new class adds its native variants and nothing else.
- Each host class carries its own variants. A device does not hold variants it cannot run unless
  it chooses to serve them for others.
- An identical variant built by two devices is one CID and one copy.

Open: custody per variant. A variant for uncommon hardware has few holders (missing node, §9).

### 7.3 Building a variant: the reverse reading

Selection asks which existing variant a host can run. Build planning asks the reverse: which host
can make a variant that does not exist yet. It is the same contract applied to the build closure.

- A build closure's toolchain members carry their own `Requires`. A cross-compiler requires an
  x86 host and produces aarch64 output. "Can this device build for target T" is
  `admit(build closure for T, Provides)`, evaluated on the device that would build.
- A build planner (rakia) therefore sends the job with the build closure's variant set, and the
  provider answers which toolchain variant it will take, or refuses. The planner does not hold a
  table of what each device is.
- **Building and proving are separate jobs.** A variant may be built on any host that admits the
  build closure. It is proven only on a host that admits the output. A proof under emulation is
  recorded as such and counts for the build and run rungs only (ruling H6).
- Cross-compilation from a declared target is preferred to emulated building, which is slower and
  proves nothing more.
- A request for a missing variant names a declared target. That is the requester's own disclosure,
  scoped to the request (§4).

The job is complete when the variant's entry, with its attestation against the set's suite, is
appended. A build with no proof is a candidate and is not offered at wider reach.

## 8. Local-first reading

Read from the person's side: what works on one device with nothing else reachable.

- **Alone, key on the device:** the device observes its own `Provides`, admits against every
  variant set it holds, and runs each admitted variant whose bytes it holds, verified by CID. With
  a recipe and its own toolchain it builds a variant for itself, records the requirement and signs
  the record; peers learn of it when one is reachable. A device with no admitted variant and no
  recipe is told which axis is unmet. Nothing it already ran stops.
- **Alone, key held by a doorway:** a hosted person's device runs the portable tier in the
  browser. The browser observes its own engine features and admits locally; the doorway does not
  select for it. Publishing a build record waits for the doorway to sign, and the person is told it
  is not yet published. The doorway's own node admits its native variants like any other host.
- **Alone, no key:** a visitor runs the portable tier as served. Nothing is kept past the visit.
- **Working version:** none. Variant sets, variants and requirements are fixed and addressed in
  one act. A device's unshared development build is a published thing at private reach, not a
  draft.
- **Each kind of state:** `Provides` and the admission verdict are held on this device and not
  sent. Variant sets, variants, requirements and proofs are published versions.
- **Arrivals:**

  | What arrives | Adds | Learns | When it is gone |
  |---|---|---|---|
  | The person's other device | variants it built, including one built for a declared target this device asked for; bytes; proofs | which variant was fetched, and any declared target requested. It is inside the steward's circle | held variants keep running; this device builds for itself where it can |
  | Another person's peer | bytes and proofs for more host classes | the variant fetched, a coarse hardware class, within the artifact's reach | nothing running stops |
  | An always-on copy or pool | availability, so no device holds every variant | the fetch pattern across devices. Pool membership is public, so this is for public artifacts only | held variants keep running |
  | A doorway | delivery of the portable tier to browsers | what it already sees of a hosted person | native selection on a person's own node is unaffected |
  | A forge or CI | prebuilt variants for classes no household device can build | which variants are requested | devices with a recipe build their own |

  One of the person's own devices can do everything this design asks of a host: serve variants,
  build for a weaker sibling, and prove a host class.
- **Shown to the person:** in plain words and without instruction-set names unless asked: this
  device runs it; this device cannot run it yet and what is missing; a version for this device is
  being fetched or built, and from where; a newer version exists that this device cannot run, so it
  stays on the current one.
- **Refusals:** three places the first draft made an arrival into a condition, each changed.
  A declared target must not be required to build (§6.3 item 4). Adopting a new variant set must
  not evict a running variant (§7). A device that cannot build, such as a phone or a watch, depends
  on others for native variants, so the portable tier is its floor and its last working variant is
  kept.
- **Stands as is:** observation and admission are local and pure. Their home is the device's own
  envelope.

Back-fill check. With every doorway deleted, a person's own node still observes, admits, runs and
builds. A laptop closed mid-adoption and reopened offline runs the variant it ran before; a
half-fetched variant is not adopted. The first step that waits on someone else is obtaining a
variant for hardware this device cannot build for. The bytes do not exist locally, so only the
request, naming a declared target, can be prepared and kept.

## 9. P2P design gate

No new DHT entry type, table or route is proposed.

### Entity: Requires
- **Classification**: Linked (A2) where the artifact is notarized (it is an attribute of a release's
  artifact row); otherwise it rides the artifact's `DerivationRecord` on the steward channel.
- **Head-plane cost**: none. It is a field of the variant set, which hangs off an existing head.
- **Address**: content-derived, as part of the variant set document.
- **Source of truth**: the variant set document. Storage holds a projection.
- **Network stakes**: all four stages. Counter-evidence against a requirement is floor-protected.
- **SDO/RWA test**: a holder sees what an artifact needs, which is public knowledge about public
  bytes. Nothing about any device.

### Entity: Provides
- **Classification**: Ephemeral (C). Rebuilt by observation at every start.
- **Source of truth**: the running host. Not serialized off the device, by the same rule as
  placement's `DeviceFacts`.
- **SDO/RWA test**: the worst holder is the device itself. A serving peer learns only which variant
  was fetched (§4), bounded by the artifact's reach.

### Entity: VariantSet
- **Classification**: content-addressed document, referenced from a release or a `RuntimeManifest`.
  It is a composite root: many variants under one existing head.
- **Head-plane cost**: none added. Item count at one year is the number of released artifacts
  times the number of proven host classes, tens, all under existing release heads.
- **Address**: content-derived (dag-cbor CID). Which set applies is a declared head, elected by the
  existing release channel and not resolved by recency.
- **Transport affinity**: `auto`. The set is small; the variants are ordinary blobs.
- **Integrity zome and DNA-hash class**: none touched. If a release row later needs a new field,
  that change must pass this gate again and state its DNA-hash class.

### Entity: admission verdict
- **Classification**: Ephemeral (C). Recorded locally in the berth and passport, and as the
  existing `Use` flow event on fetch.

### Entity: compatibility proof
- **Classification**: Attested-Private (B2). The raw run stays on the proving device; the outcome
  rides brit's existing build attestation, whose `hardware_profile` slot exists and is unused. Only
  a coarse host class is published, as the runtime-artifacts spec already limits.

### Shape per plane (gradient reading)

Where the cost is paid. Proof is paid once per variant and host class by whoever proves it, and
carried. Admission is a few set comparisons paid once per start, in the background, on the device
that would run the bytes. Nothing in this contract sits on a request path.

| Object | Reach | Custody | Freshness | Linkability | Cost bearer |
|---|---|---|---|---|---|
| Variant set document | the release's reach; a pin in a private manifest is a private reference to it | everyone who pins it, since it is needed to resolve | a declared head; a stale read runs an older variant, which is safe. A withdrawal must reach every holder | none: every holder has the whole set | the publisher |
| Variant bytes | the release's reach | a subset: devices of that host class and always-on copies. Not held by everyone | pinned by CID | fetching or serving one reveals a coarse host class | the devices that run that class, and any pool that chooses to carry it |
| `Provides` | private to the device | none; rebuilt by observation | re-observed at start and on hardware change | the highest in this design, which is why it does not leave | the device |
| Floor and toolchain members | public bytes | builders only | pinned by CID | reveals that a device builds for that target | builders |
| Compatibility proof | follows the artifact | with the variant set | valid until contradicted by a witnessed death | names a coarse host class of the prover | the prover; every device of that class benefits |

Relationship and evidence:

| Between | Relationship | Evidence that suffices |
|---|---|---|
| One steward's own devices | trusted by standing | `declared` and `extracted` clauses; no re-proof per device |
| Peers at wider reach | earned | `proven` clauses, and converging independent builds as the reach ladder requires |

Prices paid on purpose, stated beside the gain:

- Provider-side evaluation costs one exchange carrying the variant set, so that no host publishes
  its hardware.
- A variant for uncommon hardware has few holders. Its custody is thin until others choose to
  carry it, and its recovery path is a rebuild from the recipe and build closure.

Missing nodes, in mintable shape:

- `variant custody / between "variant published" → "variant recoverable after a holder is lost" /
  missing node: a declared holder need per variant, with a probe that counts independent holders /
  current state: custody is declared per blob with no notion of host class`.
- `compatibility proof / between "bytes ran on a host class" → "reach widens for that class" /
  missing node: a build-provenance attestation carrying the proven clauses / current state: brit's
  hardware_profile slot exists and is unfilled; the attestation kind is not minted`.
- `floor redistribution / between "sysroot built" → "sysroot shared at commons reach" / missing
  node: a licence reading of each floor and toolchain member / current state: unread`.

### Decision predicate: `admit`
A pure decision predicate, to be registered in its crate's `seam-registry.yaml` at implementation
with all sixteen concern answers. The answers that shape the design now:

| Concern | Answer |
|---|---|
| C4 honest absence | unknown and unobserved axes refuse; nothing is guessed |
| C5 evidence, not authority | admission says the bytes can run; grant, consent and capacity are checked separately |
| C7 advertise/serve symmetry | a peer that offers a variant must hold its bytes; a variant set names CIDs only |
| C8 observability per decision | every verdict carries the clauses checked and the reason |
| C10 contract-evolution honesty | axes and bundles are versioned documents; a new axis is refused by old peers, not misread |
| C12 consent | compatibility does not widen a grant or a reach |
| C14 witnessed residual | a death that contradicts a requirement is witnessed and withdraws the claim |

## 10. Where the contract is consumed

One implementation, pure, with no I/O, used by every row. The recommended home is a small crate
under `crates/` beside `elohim-address`, so that ark, placement, the artifact store and storage can
all depend on it without depending on each other (ruling H5, §12).

| Consumer | Today | With the contract |
|---|---|---|
| ark driver `Fingerprint` | hostname and kernel | observes `Provides` at start; not serialized off the device |
| ark `authenticate_candidate` | exact triple string equality | calls `admit`; `WrongPlatform` carries the unmet axis |
| `ArtifactRef` and `RuntimeManifest` | one digest per child | may name a variant set; the berth records the resolved variant |
| Artifact store `put` | records key and output | also records `Requires` and adds the variant to its family |
| Artifact store `fetch` | exact key or nothing | on a miss for its own key, reads the family, admits, fetches the chosen variant under that variant's key |
| Bin store adoption (`epr`, `brit`, `ark`) | one CID per release | releases carry a variant set; adoption admits before landing |
| Release adoption in storage | `Artifact { blob_cid }` | native artifacts carry a variant set; portable ones are unchanged |
| Placement `admit` | consent, gate, capacity | a compatibility stage before capacity, evaluated by the provider over the task's variant set, with its own refusal reason |
| `just measure --on` and `ark task run` | the operator names the provider; one pinned artifact | the provider selects a variant or refuses before launch |
| k8s bridge render | image tag chosen by hand per node | renders the carrier image for a variant chosen from the operator's declared node class, as node affinity is declared today. The envelope inside the carrier observes and admits at start, and refuses a wrong choice before launching children. Whether the bridge can learn the admitted variant without the operator declaring it is open |
| Derivation key (K1) | keys a linker the build does not use | keys the declared target and the real link inputs |
| `just mesh preflight` | freshness and pin checks | adds one line: each binary admitted on this host, or the refusal |

Order of checks wherever several apply, each separately measurable as the harvest requires:
compatibility, then capacity, then authority and consent, then protocol contract.

## 11. Proof ladder

Each rung is a receipt, not a design claim. A rung does not start until the one before it is green.

| Rung | Proves | Receipt |
|---|---|---|
| H0 | The vocabulary and `admit` with vectors: v2 on v3 admitted, v3 on v2 refused, glibc floor both ways, unknown axis refused. Each vector is a requirement, a synthetic `Provides` and the expected verdict. Some requirements are extracted from binaries built on the other device; no device's real `Provides` is exported | unit and vector tests committed with the crate |
| H1 | A device observes its own `Provides`; the two devices' observations differ where the hardware does | ark fingerprint output on both devices |
| H2 | The store records `Requires` at `put`; `fetch` selects by family. F2r closes in both directions, including the unsafe one: a variant containing v3 C code is refused on the v2 device, shown under emulation | evidence files in the form the artifact-store sprint already keeps |
| H3 | A declared target with a pinned sysroot gives the two native images the same key and one identity hit. The sysroot is itself a recorded derivation with a key and a requirement, published by the device that built it | F2r receipt with equal keys, naming the sysroot CID |
| H4 | A variant-set pin: one `RuntimeManifest` CID resolves to different variants on the two devices | berth records on both |
| H5 | Placement: a provider refuses an incompatible variant before launch and runs the compatible one. These are the harvest's four stations | the `measure-runs-on-a-peer` scenarios |
| H6 | A second architecture (aarch64): build, run, and exchange declared content with x86 peers both ways | first non-x86 receipt |
| H7 | The portable tier beyond zomes: one wasm guest under the envelope, identical bytes on two architectures | tevah `Wasm` driver receipt |
| H8 | One accelerator axis: an inference workload admitted on a host with the API and refused on one without | first `accel.*` receipt |
| H9 | One conformance suite judges two variants of one binary on the two devices, with expected outputs minted by the other device's variant. A deliberately wrong variant is refused | two attestations naming one suite CID, and one refusal |
| H10 | The reverse reading: a planner asks for a missing variant, a device that admits the build closure builds it by cross-compilation, a device that admits the output proves it, and the entry is appended | the appended variant-set CID with its attestation |

RISC-V and every other target enter at H6's shape: an observed build, run and mixed-peer exchange.
Until then they are named axes with no supported claim.

## 12. Rulings asked of the steward

| Id | Question | Recommendation |
|---|---|---|
| H1 | Does K1 stay a key-honesty fix only, with no cross-key admission in it? | Yes. K1 keys the linker and link inputs the build really uses |
| H2 | Is K1b withdrawn in favour of this contract? | Yes. Its requirement record and typed refusals move here; its "substitute under a foreign key" idea is replaced by variant selection, which is an identity hit on the variant's own key |
| H3 | May a device run a variant built under link inputs it does not have itself? | Yes, subject to `admit` and the reach ladder, as for any bytes it did not build |
| H4 | Is the declared target (pinned sysroot and C flags, formerly K1c) adopted as the default for shared native builds? | Yes, with the mechanism per binary chosen by measurement (§6.3) |
| H5 | Where does the contract's code live? | A small pure crate under `crates/`, registered in the seam registry |
| H6 | Which second architecture proves H6, and on what hardware? | aarch64 on a household single-board computer; emulation is acceptable for the build and run rungs, not for the mixed-peer exchange |
| H7 | May a coarse host class be published in a build attestation? | Yes, limited to the bundle names of the clauses proven, not raw `Provides` |
| H8 | Is the line between portable and native drawn by "must agree" against "may differ" (§5.1)? | Yes. Agreed in conversation by the steward on 2026-11-10; recorded here for the reviewing sessions |
| H9 | Are variant sets open, with membership decided by a named conformance suite and not by who built the variant (§7.1)? | Yes, same agreement. Adapted variants join by attestation; wider reach needs the device's own run or converging independent proofs |
| H10 | Does rakia plan native builds by asking providers to admit the build closure (§7.3), with building and proving as separate jobs? | Yes, pending the rakia session's check against how rakia plans jobs today, which this spec has not read |

## 13. Review asks, by session

- **Artifact-store sessions (shem and ethosengine)**: check §7 against `resolve_holders` and the
  `Produce` and `Use` events. Does a family key computed by erasing the target fields of
  `MethodDescriptor` work with its `deny_unknown_fields` and epoch rules, or does it need a new
  epoch? Check §6.3 against what `artifact.rs` keys today.
- **ark and placement sessions**: check §10 against `Fingerprint`, `CandidateRuntime`, `ArtifactRef`
  and `placement_core::admit`. Is provider-side evaluation (§4) consistent with R-P4 and the offer
  shape?
- **Edge and k8s-bridge sessions**: check §6.4. Can the carrier image be rendered per node from a
  variant, replacing hand-kept flavor tags?
- **lvi**: check §6.2 against the `DevspaceSeed` projection. Is the build closure the missing
  toolchain-marker encoding?
- **rakia session**: check §7.3. Does rakia's job planning have a place for a provider-answered
  build-closure admission, and for a proving job separate from the build job? This spec states the
  requirement without having read rakia's planner.
- **Artifact-store sessions, second ask**: check §7.2. Is resolving a variant's holders by CID
  enough, or does an adapted variant outside the family key need anything the store lacks?
- **All sessions**: name any place this spec levels devices, publishes host facts, or adds a store.
  Each would be a defect.

## 14. Prior art, and why none is adopted whole

| System | What it has | Why it is not enough here |
|---|---|---|
| OCI image index | one tag resolving to per-platform images by `os`, `architecture`, `variant` | a fixed set of fields; no libc, no ISA level beyond a coarse variant string, no accelerator; resolved by a registry, and the unit is still the fused image |
| Python wheel platform tags | a declared libc floor in the artifact name (`manylinux_2_34`) | the closest precedent for a declared floor; one ecosystem, name-encoded |
| Conda virtual packages | host facts such as libc and GPU driver exposed as constraints a solver reads | the right idea of observed host facts as constraints; centralised channels |
| Nix | input-addressed derivations, a `system` string, required system features | derivation math kept; store, language and the single coarse `system` string not adopted |
| Debian multiarch, Android ABI splits, Apple universal binaries | several native variants behind one name | each is fixed to one vendor's list of architectures |

The common limit is a closed list of platforms decided by a central party. The design here keeps
the list open, the observation local, and the proof earned per host class.

## 15. Vocabulary

| Term | Meaning |
|---|---|
| Requires | What an artifact needs of a host: a set of clauses on axes |
| Provides | What a host observed it can run; kept on the host |
| admit | The pure function that compares them |
| Axis | One named dimension of compatibility with a comparison kind |
| Bundle | A content-addressed alias for a fixed feature list, such as `x86-64-v3` |
| Declared target | The architecture, ISA bundle, OS, floor and C flags a native build names as inputs |
| Floor | The C runtime a native artifact links against or carries |
| Closure | An unordered set of content-addressed members, each with its own requirement |
| Variant | One build of a logical artifact for one declared target |
| Variant set | The content-addressed list of an artifact's variants; what a pin names |
| Universal artifact | One with an empty `Requires`; a single form, pinned by its own CID |
| Conformance suite | The content-addressed harness and vectors every variant in a set must pass |
| Conformance attestation | A signed record that one variant passed one suite on one host class |
| Adapted variant | A variant made by changing source or recipe for a device or class; it joins a set by attestation |
| Carrier | A delivery format, such as an OCI image, rendered from a closure |
