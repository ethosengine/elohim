---
title: "Tevah — the elohim-native compute envelope: the primitive under the runtime, and the death witness as its first output"
id: compute-envelope-tevah
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: the household mesh runs its conductors under the envelope binary AND a forced conductor death on the mesh produces a death witness whose CID is held by every custodian inside the declared reach, rendered at /epr/{cid} for a household member and refused to a stranger (the @concern:death-witness receipt), AND the operator records acceptance of §12's decision register (a signed-off edit or an epr flow note on this spec)
created: 2026-09-02
domain: D-runtime-operations × seam atlas 3.2 OS/packaging · 3.3 runtime/footprint · 3.15 resource governance — the primitive both 3.2 and 3.3 assume and neither owns
topic: [compute-envelope, tevah, supervisor, death-witness, runtime-manifest, berth, passport, cgroup, quota, compute-rea, kubelet-parity, device-spectrum, generational-shift, self-healing, upgrade-propagation, lvi, steward-node]
informed-by:
  - genesis/data/timeline/backlog/elohim-native-compute-envelope-the-pod-under-the-runtime.md (the envisioned primitive this spec canonizes)
  - genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md (the first output; its P2P design gate is re-run and corrected in §7)
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md (CORRECTED DIAGNOSIS — the ground truth for what the witness must be able to say)
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md (the atlas this spec adds one primitive to)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (release adoption — the loop the envelope is the apply vehicle for; §11.5 is answered here)
  - genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md (the four pillars; the observe pillar the witness inverts)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md (floor/ceiling — the restart verdict is floor, the repair intent is ceiling)
  - elohim/lvi/docs/specs/2026-07-20-elohim-native-devspace-design.md (the six lvi invariants this spec honors; lvi-actuator becomes a consumer)
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md (the six-field envelope contract; ram-guard as the mechanical precedent)
cites:
  - genesis/data/timeline/backlog/elohim-native-compute-envelope-the-pod-under-the-runtime.md
  - genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "runtime-artifacts-elected-content | Runtime Artifacts as Elected Content | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "actuatable-self-healing-control-plane-design | Actuatable Self-Healing Control Plane | sha256:e715507d5700471b | path: genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md"
  - "compute-commitment-substrate-floor-design | Substrate Floor / Elohim Ceiling | sha256:614e30134ee0d7ab | path: genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md"
  - elohim/lvi/docs/specs/2026-07-20-elohim-native-devspace-design.md
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/runtime_passport.rs
  - elohim/elohim-storage/src/services/release_adoption/mod.rs
  - elohim/elohim-compute/src/actuation.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - genesis/agentic/bin/ram-guard
  - steward/node/src/pod/mod.rs
  - genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml
  - app/elohim-app/scripts/hc-mesh.sh
---

# Tevah — the elohim-native compute envelope

**One sentence:** a peer is made of processes, and *tevah* (תֵּבָה, the ark — the vessel that
carries a declared set of living things through a hostile flood; the dove is its witness) is the
one primitive that runs them, quotas them, accounts for them, listens to them, and witnesses them —
the same on a watch, a household box, a rack blade, a Tauri desktop, a devspace, and inside a k8s
pod — so that every power a household borrows from `kubectl describe` and `kubectl logs --previous`
today is the peer's own, and the first thing it says is what the 2026-09-02 crash loop could not:
*"conductor died 2.3 s into boot: no genesis for five cells; the previous boot deleted the source
chains during a reinstall that timed out; DHT data intact."*

**Naming.** The provisional names `elohim-pod` / `epr-pod` are retired for the runtime: `pod`
already names `steward/node/src/pod`, the cluster *operator* (the agent-pod sense), the exact
opposite layer; the atlas's thesis is that misrouting is the dominant failure, and a primitive
named for the layer it is not invites it twice (k8s pod = packaging, seam 3.2). Following the house
precedent (brit · rakia · lvi · eae), the runtime is **tevah**, the plain alias is *the envelope*,
and "epr-pod" survives only as the operator's gloss. The declaration is a **`RuntimeManifest`** — a
Manifest-kind EPR (`EprKind::Manifest`, schema key `runtime-manifest`, the sibling of
`app-manifest`): the manifest of claims about what runs, notarized like every other manifest.
`*Seed` was rejected 2026-09-02 — "seed" already means content seeding in this repo. The per-blade
half is a **`Berth`**: what one blade offers and what it currently holds (register 25).

## 0. The north star this is read through

> Hyperscaler-like capabilities for a p2p community — something no one has really tried. The goal is
> not to recreate the elohim protocol for a hyperscaler; it is to use what we understand about
> hyperscalers today to design compute primitives that make a durable, highly-available, observable
> substrate a possibility over a diversity of peers on a potentially hostile network — an ecology
> that can heal, support, balance, budget, plan, and care for itself. (operator, 2026-09-02)

Every external model surveyed (§13) was read for the primitive it reveals and the assumption it
smuggles: a trusted operator with a shell, a homogeneous fleet, a central control plane, root, a
cloud that outlives the device. The common shape of every mature "last words" system — an
observer that outlives the victim, a small typed reason, a bounded blob, a queue with attempt
counters — assumes an *outside* (kubelet, journald, Android's ActivityManager, Crashpad's
collector). A household peer has no outside. **The envelope is therefore designed so that the
parent role is sufficient** — write-ahead intent before every decision, `rusage` at reap,
unprivileged `/proc` and cgroup reads, a typed reason that never guesses — and so that its only
"outside" is the custody plane it already has: a custodian named by commitment is the ecology's
witness of last resort. That is the inversion the atlas §8 names: parity is the floor; the
social/custody plane is where human scale exceeds the hyperscaler, and here it does so literally,
because nobody counter-signs a *death* — Holo counter-signs service; nobody counter-signs failure.

## 1. What the grounding found (21 briefs, adversarially verified, 2026-09-02)

The envelope atom's "three partial envelopes" table is wrong in every row but one, and the
corrections are load-bearing:

| Claim in the atoms | What the code says (verified) |
|---|---|
| `process_manager` "after 2026-09-02: try_wait, exit status + tail (landed)" | Landed on dev (`264ce8ce4`, `dcf9a16c3`; `happ_manager` migration intent `867e4bf9b`) — the grounding briefs read a stale tree and called it uncommitted; the feasibility review corrected that. What is still true: ring is 200 lines (atom says 400); the give-up path carries no tail; nobody `try_wait`s after readiness; the tail is logged, never written, CID-named, offered, or attested. |
| `lvi-actuator` "owns process supervision + hard cgroup quota" | `elohim/lvi/` has **zero `.rs` files**. Spec-only. Its quota is podman CLI vocabulary in a doc. |
| `steward/node/src/pod` "is a cluster operator with `compute_rea.rs` (compute as REA)" | A monitor→analyze→decide→execute loop whose service health is a constant, whose consensus is a simulation, whose `RestartService` handler fabricates success, and whose `ComputeEvent` is an inference-token record with random ids that is minted and dropped. The binary has never run on the fleet; the alpha container named `elohim-node` runs `elohim-storage`. |
| k8s "the real envelope on alpha" (`_edgenode-consolidated.template.yaml`) | The conductor moved out at rung 2: the conductor runs in `_edgenode-conductor.template.yaml` as **the same storage image in embedded mode with every feature off** — a whole elohim-storage binary whose only job is to spawn holochain. That pod *is* the envelope in disguise, and it is the cheapest extraction path (§11). |
| Witness atom: "custody commitments already name who replicates a node's atoms" | **Wrong.** Custody commitments are per exact blob hash (`custody-blob`, id over provider × receiver × blob marker); a freshly minted witness has no custodian; minting one is conductor-required. |
| Witness atom: "a new `contentType` `runtime:death-witness` … DNA-hash-NEUTRAL" | **Wrong.** `Content::validate` refuses any type not in `ALL_CONTENT_TYPES` (integrity zome). A new content type **moves the DNA hash**. The valve is the one ReleaseManifest uses: an existing type + `metadata_json` kind. |
| Witness atom done-when: "`just mesh recovery cold <peer>` … kills the conductor" | That command kills **storage** and wipes its store; it never touches a conductor. `features/recovery/` does not exist and would collide with `features/auth/recovery/`. |
| Atlas §8: observability = `/metrics` + `/health` + Grafana + self-heal | The observe pillar is request-time projection + an external poller + a developer-side ledger, structurally blind to a child's death ("a dead node cannot self-report"), and the poller was never registered as a hook. |

And the four facts that decide the shape:

1. **Nothing is the parent where it matters.** On the household mesh the conductor is launched by
   `setsid nohup hc sandbox run`, or in `MESH_CONDUCTOR_LAUNCH=direct` mode by `setsid nohup sh -c
   "echo test | holochain --piped …"` — either way deliberately parentless; on `just dev start` storage runs with no
   nohup, no log, no pid; on the Tauri desktop the conductor is *in-process* (`tauri_plugin_holochain`)
   and the storage sidecar's stdio is inherited, not captured; on alpha the only pipe-owning
   supervisor ships inside a container whose image field is deliberately frozen (rung 2). The lane
   that may kill cannot see the supervisor; the lane with the supervisor may not kill.
2. **Liveness after readiness exists nowhere, and cannot be delegated.** `/health` stays 200 by
   design; `/health/serving` is "deliberately NOT wired as any probe"; the conductor pod has no
   bridge. A conductor that dies after readiness is neither witnessed nor **restarted** — by anyone.
3. **The supervisor shares an OOM group with the child it witnesses**, everywhere (kubelet sets
   `memory.oom.group=1` on every container cgroup; the dev pod's group-kill on 2026-08-29 killed 35
   processes). On the death class alpha has actually seen (OOM), the witness cannot be written unless
   the cgroup topology changes.
4. **Everything that makes an artifact someone else's to read is keyed to the conductor's agent
   key** — signing (storage holds no key), custody minting, read gates — and the passport names the
   conductor by `CONDUCTOR_IMAGE_TAG`, the very string the release-manifest schema says never to
   trust. The envelope needs a node-local authoring identity and must name what it runs by content.

## 2. The primitive

**Definition.** A *tevah* is a supervisor that is the **parent** of a declared set of processes,
that **outlives** every one of them and **carries state across its own restarts**, and that owns,
for each child: lifecycle (spawn · readiness · liveness · a restart policy whose every decision is a
recorded verdict), quota (declared vs effective, honestly), accounting (compute as REA at three
cadences), listening (every child's stdout/stderr, ring-buffered, structured-parsed), and describe
(a content-addressed passport). Its first-class output is the **death witness**: the terminal
receipt of a child's lifetime, harvested by the parent that already held its pipes, written to the
node's own disk before any effect is taken, offered to the node's custodians, attested when a
conductor is next available.

**What it is not.** Not an image, a registry, a pull path, or a layer store (eprfs "mount, don't
ship" owns delivery). Not an OCI runtime or a sandbox (isolation is a *driver*, §4). Not a
scheduler (placement is a market, never master-electing; a conductor with a source chain is not a
fungible replica). Not a controller over other peers (that is `steward/node`'s operator, which sits
*above* the envelope and today fabricates the observations the envelope will make true).

**The unit and the tree.** One root envelope per device — the peer runtime: conductor, storage,
doorway, sidecars, each a child. Admitted guests (an lvi devspace, a rakia build step, a delegated
compute job) run as **sub-envelopes** with quota carved from the parent's — the cgroup hierarchy *is*
this shape, and co-resident safety falls out by construction: the protected set is the parent's own
children; a guest dies whole and alone (`memory.oom.group=1` on the guest leaf, `0` at the root —
the virtual-peer contract's inversion, now a per-child field rather than a heuristic).

**The five verbs are not a sixth grammar.** Each is an instance of the atlas §6 cohesion grammar:

| Envelope verb | Grammar verb | The instance |
|---|---|---|
| lifecycle | `runGovernor` | `RestartGovernor: elohim_compute::Governor` — Request = spawn child N again; Grant = the manifest's restart budget + quota (a self-contract for own children, `delegates-compute` for guests); Context = the incident state; Effect = a spawn plan; every give-up a `Refusal` naming its `LimitOwner` |
| quota | `bindCapability` · `commit(face)` | bounds ride the commitment's `bounds` (extra keys, `additionalProperties: true`) — `Operator` for own children, `Commitment` for guests |
| accounting | `authorAtom` → `rollupCoverage` | `economic_events` rows `bounded_by` the commitment, `substrate_signal ∈ compute·storage·bandwidth·energy·time`, rolled up over the recursion seam |
| listening → witness | `authorAtom` | the reach-declared atom the witness gate already chose |
| describe | `authorAtom` | the passport atom at `/epr/{cid}` |

**Verdicts are receipts.** The one decision that reshaped the design: today the most destructive
actuation in the tree — `clear_conductor_state`, which re-keys — is taken by an env-gated `if` with a
log line as its only trace, while a *human's* restart of the same conductor is already a
commitment-gated, emit-then-act REA action (`operator-runtime-surface`, green). The envelope makes
its own verdicts the same record class: scoped to a **self-contract** `delegates-compute`
commitment the runtime mints for itself at first readiness (today only an admin seed route mints
one), emit-then-act, `bounded_by` it. The witness atom *is* the verdict's payload. A restart the
node gives itself is then auditable by a custodian exactly like a restart the operator gave it.

## 3. The declaration — `RuntimeManifest` and `Berth`

Nix's derivation/instantiation split, applied: the **manifest** is input-addressed and *shared* (peers
on one declaration share one manifest CID — a family's three boxes, a fleet's seven humans); the
**berth** is per blade (ports, data dir, the agent placed there, effective quota). Guix's
"declared OS → generated supervisor" is copied as *the manifest generates the supervision*; its
Scheme, its global store, and "upgrade next time it stops" are rejected.

```
RuntimeManifest {                                   // content-addressed, DAG-CBOR (bafyrei…); kind "runtime-manifest"
  schema:        1,
  supersedes:    Option<Cid>,                   // the lineage — a manifest is the sixth declared-head-over-lineage instance
  reach:         Reach,                         // household tier (vocabulary: unresolved — declared drift; §7)
  archetype:     DeviceArchetype,               // the budget floor this manifest is sized for (archetype-resource-budgets.json)
  envelope: {                                   // the six-field virtual-peer contract, made a per-manifest value
    bound:       ResourceQuota { memory_bytes, cpu_millis, pids, disk_bytes },
    measure:     Committed,                     // committed-not-gross (anon+kernel+shmem+unevictable; page cache never counts)
    protected:   [ProcessName],                 // never shed
    shed_order:  [ProcessName],                 // tier ladder, newest-first within a tier
    graded:      { soft, high, hard },          // 70/80/88 %-class bands; hard = the kernel's, high = throttle+reclaim
    reciprocity: Ledger,                        // where breach events land (§5.3)
  },
  processes: [ ProcessSpec {
    name:        "conductor" | "storage" | "doorway" | …,
    kind:        Native | InProcess | Wasm | Delegated,       // the driver class (§4)
    artifact:    ArtifactRef::Channel{channel_id} | ArtifactRef::Pinned{cid, bytes, sha256},
                                                // Channel = auto-adoption by election (runtime-artifacts §4); Pinned = a lockfile
    closure:     [Cid],                         // the runtime closure — declared, never scan-discovered (eprfs materializes, verifies by hash)
    argv:        [Template], env: [Template],   // templates resolve against the Berth, never against ambient env
    env_scrub:   true,                          // the child inherits nothing it was not given
    imports:     [ Import ],                    // wasmCloud links as capability grants: {kind: AdminWs|DataDir|Socket|Fd, from, to}
    readiness:   [ Probe ],                     // a LADDER: NotifyFd(sd_notify) → StdoutLine("Conductor ready.") → AdminWs(port) → AppInstalled → CellsGenesised; per-rung patience
    liveness:    Probe,                         // pid + AdminWs ping; never a sibling's /health
    policy:      ChildPolicy {                  // OTP child spec + systemd result vocabulary
      restart:   Permanent | Transient | Temporary,
      strategy:  OneForOne | RestForOne,        // RestForOne for conductor → storage → doorway
      shutdown:  { signal: SIGINT, grace_ms, then: SIGKILL },   // the conductor speaks only SIGINT
      intensity: { max_deaths, window_s },      // s6 death tally + OTP window; readiness RESETS the window
      backoff:   { min_s, max_s, steps },
      same_cause_limit: 3,                      // three identical (class, first structured line, uptime<5 s) → GiveUp
      critical_sections: [ Declared | InferredFromAdminCall ],  // uninstall_app: restart/kill REFUSED inside
    },
    quota:       ProcessQuota { share, oom_group: bool, oom_score_adj, cpu_weight },
    listen:      { ring_lines: 200, tail_lines: 40, parsers: [DbPoolSaturation, FatalPanic, OutOfThreads, AdminListenerFinished, StartupMilestone] },
    bounded_by:  BoundedBy::SelfContract | BoundedBy::Commitment(cid),   // guests carry lvi's triple {quota, ttl, bounded_by}
    ttl:         Option<Duration>,              // guests only; revocation = lease expiry, never interrupt
    update:      DownloadThenKill | HandOver,   // balena strategies; the conductor cannot hand over
  } ],
  repair: {                                    // what today is GENESIS_SELF_HEAL_IDENTITY / ALLOW_DNA_REINSTALL — declared, never env
    identity:    Reseedable | LineageBearing,   // re-key allowed only when Reseedable, only on a genuine child death, only once per incident
    drift:       RequireMigrationIntent,        // DNA-hash-moving reinstall needs a per-roll intent naming every drifted role
  },
}

Berth {                               // agent-scoped composite (blade agent × manifest cid); kind "berth"
  manifest:      Cid,
  node:          AgentCid,                      // uhCAk… — the canonical join key; transport ids resolve TO it
  self_contract: Cid,                           // the delegates-compute commitment the verdicts are bounded_by
  custody_spool: Cid,                           // the standing commitment naming who custodies this node's witnesses (§6)
  data_root:     { path, passphrase_source: Empty | Piped | Keystore },   // the passphrase seals lair AND db.key; alpha's is empty
  ports:         { admin_ws, app_ws, http, … },
  effective:     [ EffectiveQuota { process, tier: Enforced(cgroup) | Bounded(rlimit+nice) | Delegated(k8s) | Intrinsic(wasm) | None } ],
  applied:       { manifest_cid, passport_cid, at },   // balena "applied": passport hashes == manifest closure hashes
}
```

**Who may change it — three authorities, three gates, unchanged from what the code already has.**
Above the DNA line the process set is the output of an **election** on a followed channel: an
`ArtifactRef::Channel` moves by re-election and the envelope's restart policy *is* the apply
vehicle for the binary class (this answers runtime-artifacts §11.5: the envelope owns the process,
storage attests; the `pendingRestart` the storage-binary vehicle leaves today finally has a
consumer). Consent is a **pause, not a veto**: a child holds an update lock for a bounded, declared
reason (a quiesce in flight, a chain write) — never an operator flag standing forever, which is
exactly what `ALLOW_DNA_REINSTALL=true` on fourteen StatefulSets was. At the DNA line only a per-roll
`DNA_MIGRATION_INTENT` naming every drifted role's bundle hash may move the manifest (a new manifest with
`supersedes`). Re-key is the node's own policy, gated on a genuine child death, once. The passport
names which gate last acted and on what evidence.

**Composition with what is wired.** The manifest is the *target state*; the passport (§5.5) is the
*current state*; the envelope is the diff-and-apply loop with `applied` as its exit condition. The
release-adoption controller (`services/release_adoption/{watch,verify,state,apply}.rs`, 7.4 k lines,
mesh-proven) keeps its role — resolve the channel head through this node's own conductor, fetch,
verify locally, hand a `VerifiedRelease` to a vehicle — and the envelope becomes the vehicle for the
binary class. The manifest cannot be *read* from the DHT at boot (the conductor that serves the DHT is
itself a child): it is pinned on the node's disk, content-addressed, and the DHT is where the *next*
manifest is discovered and the *applied* fact is attested — the witness's amber→green rule applied to
the declaration side.

### 3.1 The tiers — hardware is cattle, the household is the pet (2026-09-02 thread)

"Cattle, not pets" is violated on purpose at exactly one tier and honored at the others.

| Tier | What it is | Replaceable? | Its witness | Its record |
|---|---|---|---|---|
| **Hardware + kernel** | a blade, its PSU / SSD / RAM / GPU / fan; one kernel | **cattle** — slot one in, pull one out | the kernel's own, read **once** by the ark at fingerprint time and on a probe cadence (dmesg, SMART, sensors, the OOM killer); nothing above the ark re-reads it | `attestation:device-health` |
| **Berth + ark** | one blade's runtime: the ark is the blade's parent process; the berth is what the blade offers and holds | dies with the blade, and that is fine — the ark has no identity beyond its blade | the ark, for its children | the **berth passport** (`kind: runtime-passport`) — the passport is the berth's, never "the node's" |
| **Household footprint** | agent keys, source chains, cells, custody, the standing commitments that say what must be held by how many | **the pet** — lives across berths, never on one; at N−1 when a blade dies, not gone | absence: declared heads stop moving, presence lapses, custody goes unmet at its next check — the gap record at household scale | the commitments themselves |
| **Network** | households under reach-scoped commitments with each other | a household is a pet to its people and cattle to the network | the same absence, one grain up | quilt RS(N,K) + social-recovery quorum |

**Slotting a blade in is a negotiation.** The new blade boots, the ark fingerprints it, and the
berth publishes a **berth offer** — a REA *intent* carrying the fingerprint and the effective quota
tier, never a registration. The household hub matches it against every standing commitment that is
under-held (custody at N−1, `delegates-compute` waiting for a berth, a guest refused elsewhere for
lack of an `Enforced` leaf); new commitments are counter-signed; a new balance holds. Reach decides
how far the offer is visible, so the rebalance may cross households. No scheduler: every party to
the rebalance is a party to a commitment. *Missing node, minted here:* chain blade-boot →
commitments-rebalanced / node **berth-offer** / probe: a new blade's offer appears in the hub's
ledger within N seconds and at least one under-held custody commitment is re-signed against it.

**Loss recurses.** A whole house can be wiped out. Nobody held its pipes, so no death witness is
written; the network witnesses by absence, the under-held commitments rebalance onto neighbours'
berth offers, the people are recovered by the social-recovery quorum (the only step with no
hardware analog), and when the household re-berths the footprint flows back from custodians —
re-held, never restored from a backup. Blade → household → neighbourhood → commons is one
intent–match–commit flow at whichever grain the wound is: the VSM recursion the Weave epic asks
for, and the three test acts read as one healing story.

**Maintenance sits on tier one.** An elohim reads the blade's device-health record and the berth
passport, reaches a verdict ("NVMe wear 94 %, replace within a week"), and mints a work commitment
for the human; whole-blade replacement is the same flow with the footprint re-berthed first. The
human's act is physical; everything before and after it is witnessed.

**What a container's power becomes here.** Hermetic filesystem → the closure (have). Security
boundary → split: isolation is the *floor*, graded per driver and reported honestly; the *ceiling*
is a `delegates-compute` commitment (what it may do) plus reach (what it may read), both revocable
on chain — isolation enforces, the commitment authorizes. Composability → **a manifest may include
a manifest**: a guest is a sub-manifest by CID with its own berth, commitment, quota leaf, and
witness (lvi's `{quota, ttl, bounded_by}` triple). Three classes of "application on the dataplane":
peer-native processes (conductor, storage, doorway — process specs); applications that ride the
dataplane as *content* (elohim-app, lamad, sophia, storybook — elected bundles served by the
doorway's EPR router, composed by an `app-manifest`, the Moss "Tool" analog, and where local-first
lives); and developer/operator tooling (jenkins, grafana, sonarqube, mempalace, MCP servers — guests
under lvi, today operator infra out of seam). **Named gap:** bundles are served, not sandboxed —
Moss's iframe + capability-scoped runtime API has no counterpart yet; the `app-manifest` is where
that capability contract belongs.

## 4. Delivery across the device spectrum

Supervision and listening are the mandatory, privilege-free core (fork/exec, pipes, `waitid`,
`/proc`). Isolation and quota are **drivers** with a witnessed *effective* value — the honesty
Nomad's `raw_exec` lacks (it accepts `resources{}` and enforces nothing). The interface, modeled on
Nomad's task driver and cut to five methods:

```
trait Driver {
  fn fingerprint(&self) -> Fingerprint;                          // isolation tier, quota tier, signals, arch — a PASSPORT attribute, not a scheduler input
  fn start(&mut self, spec: &ProcessSpec, inst: &Berth) -> Result<Handle>;   // Handle is serializable (Nomad TaskHandle)
  fn recover(&mut self, handle: &Handle) -> Result<()>;          // re-attach after the ENVELOPE restarts — children survive it
  async fn wait(&mut self, id) -> ExitResult { code, signal, core_dumped, oom_killed: Evidence, rusage };
  async fn stop(&mut self, id, signal, grace) -> Result<()>;     // SIGINT, then SIGKILL at grace
  fn stats(&self, id) -> ResourceSample;                         // cpu_ns, rss anon/file, fds, threads, io — pulled on the accounting interval
  fn stdio(&mut self, id) -> (Reader, Reader);                   // the envelope owns the rings
}
```

| Rung | How the envelope gets there | Who is PID 1 / parent | Driver | Quota tier | Honest limits |
|---|---|---|---|---|---|
| **k8s pod (alpha)** | replaces the conductor container's entrypoint (the storage-in-embedded-mode pod *is* the envelope today) | tevah is PID 1: subreaper, SIGTERM→per-child SIGINT with grace inside `terminationGracePeriodSeconds`, `/dev/termination-log` ≤4 KiB with witness CID + exit class, `FallbackToLogsOnError` | Delegated (kubelet owns the container cgroup) | `Delegated`; per-child leaves only if `cgroup.subtree_control` is delegated (uid 1000, root-owned cgroup: **not today**) → `Bounded` (rlimit + nice) | an OOM of the container kills PID 1 with the child (`oom.group=1`); the witness for that class is the gap record (§6) until the pod topology gives the conductor its own leaf |
| **household Linux box** | `systemctl --user` unit with `Delegate=yes`, lingering; or plain shell | tevah under systemd (systemd is its heart; `ExecStopPost` files the parent's verdict) | Native | `Enforced` for `memory`+`pids` without root; `cpu`/`io` after a one-time root drop-in (`user@.service.d/delegate.conf`) | the only rung where the quota verb is drawable at all; **guests are refused (fail-closed) on any rung below `Enforced`** — `RLIMIT_NPROC`/`NOFILE` are per-uid, so a same-uid guest under `Bounded` exhausts the parent's own fork and fd budget |
| **household mesh (a2o)** | `hc-mesh.sh` execs `ark run <manifest>` per peer instead of `setsid nohup hc sandbox run` | tevah, subreaper; the mesh's pid registry records the envelope, not the children | Native | `Bounded` inside the dev pod (root-owned cgroup) | the flip authority for every household-lane scenario |
| **Tauri desktop** | the sidecar *is* the envelope: storage becomes its child; the conductor is `InProcess` (tauri_plugin_holochain) | the app | InProcess + Native | `Bounded` on Linux; `None` on macOS/Windows (declared `unwired`, never pretended) | `InProcess` children get the same verdict + witness shape with no pipes: a supervised task with a readiness event and a panic hook |
| **Tauri desktop, macOS / Windows** | the sidecar is the envelope | the app | Native (macOS: `kqueue NOTE_EXIT` + `libproc`; Windows: job objects + `TerminateProcess`, no signals) | `None`, declared `unwired` per verb | no `/proc`, no pidfd, no prctl on macOS; the SIGINT-then-SIGKILL contract is Linux/macOS only and Windows needs a job-object stop; launchd and the Service Control Manager are outer supervisors with no `$SERVICE_RESULT` analog — their verdict is `Lost` |
| **phone (Android)** | in-app | the app | InProcess (+ Wasm for guests) | `None` (app-controllable cgroups unverified) | the OS keeps the death record: `ApplicationExitInfo` (`REASON_*`) read at next launch is the parent-verdict source, and the ≤128-byte `setProcessStateSummary` carries the intent-log pointer pre-death |
| **watch / iOS / browser** | **envelope-absent** | the app / the page | InProcess + Wasm guests (interpreter: Pulley / WAMR — no JIT) | `Intrinsic` for wasm guests (StoreLimits: memory hard; fuel/epoch: CPU ceiling, not share; no threads, no spawn) | iOS cannot spawn; jetsam kills witness and victim as one process; a browser has no fsync-grade write-ahead. Four of the five verbs have no subject here — the rung is *next-launch reconciliation* (MetricKit / a stored intent pointer), not a driver row, and the "same on a watch" claim is scoped to that |
| **Che / lvi devspace** | `lvi-actuator` is a **consumer**: a guest sub-envelope with lvi's `{quota, ttl, bounded_by}` triple | tevah root of the devspace host, guest leaf | Native → Sandbox (cgroup subtree + bubblewrap namespaces; `libcontainer` only if a full OCI bundle is ever needed) | `Enforced` on the guest leaf | lvi's HYDRATING/WARM machine is moved by the envelope's verdicts (`ChildExited`, `GiveUp`, quota-kill); lvi grows no CRASHED state |

**Delivery of bytes.** `ArtifactRef` → eprfs `LocalMaterializer` (sparse, hydrate-on-touch) with two
fixes it needs regardless: verify-at-materialize (`materialize` never calls `verifies()` today) and
the codec bug in `verify.rs` (recomputes dag-cbor for raw blobs — every raw entry reads `Dirty`
forever); plus one adapter, `EprfsStorage` over `BlobStore` (~100 lines by the `MemoryStorage`
template) — today `FetchMissing` has no network implementation. **The exec floor is a type
obligation**: the envelope spawns only from a `VerifiedRelease`-class value (bytes re-hashed at the
moment of spawn; length and digest as separate refusals), records the CID it actually exec'd in the
passport and the witness, and keeps the stakes gate on which classes it may self-adopt (storage-binary
= Simulacra only until soak). It supersedes `steward/node/src/update/` — a wired, version-addressed
self-updater that renames a download over `current_exe` with a checksum, no CID, and no stakes gate.

## 5. The five verbs

### 5.1 Lifecycle — witnessed verdicts

**Readiness is a ladder, not a window.** For the conductor: process alive → builder → **admin
socket bound (before cells exist — which is why a genesis-less cell "briefly readies, then dies")**
→ `Conductor ready.` on stdout and `sd_notify(READY=1)`, one rung, emitted at the same instant
after cells are created (strictly better than the admin socket; ignored by everyone today; the
conductor speaks nothing else of the sd_notify vocabulary) → app installed → cells genesised → gossip joined. The
manifest declares the rungs and their patience (a **declared cold-compile budget** for the
single-threaded wasm compile replaces the fixed 60 × 2 s; the conductor cannot ask for more, since it
speaks only `READY=1`); the envelope exposes the **sd_notify wire protocol** on a **per-child**
`NOTIFY_SOCKET` (`READY=1 STATUS= EXTEND_TIMEOUT_USEC= WATCHDOG=1 STOPPING=1`) with `SO_PASSCRED`
pid-matching (no child can forge a sibling's readiness), a hard per-phase patience ceiling that no
extension may exceed (an immortal child via endless extensions inside a critical section is refused
by construction), and extensions disabled where peer credentials are unavailable, with
`StdoutLine` and `Poll` adapters in declining trust, each recorded in the verdict as the readiness
source. The extended vocabulary (`STATUS=`, `EXTEND_TIMEOUT_USEC=`, `WATCHDOG=1`, `STOPPING=1`) is
for children that can speak it — storage and doorway are ours to teach; the conductor gets the
stdout-line adapter and the admin-socket poll. The five readiness dialects that exist today
(admin-ws connect, `ss -tln`, grepping a PTY log, `/health` curl, a Tauri event) collapse to one
verdict shape.

**Liveness is the envelope's own reaper.** `pidfd`/`waitid` on every child, always, plus the
declared liveness probe; never `/health` (which stays 200 by design). A miss is `Unresponsive`, never
"unknown". This closes the finding that a post-readiness conductor death is today neither witnessed
nor restarted.

**`ExitClass` — the merged vocabulary** (Android `ApplicationExitInfo` × systemd `$SERVICE_RESULT` ×
k8s reasons × OTP), one detection rule each, from the parent's own observation only:
`Completed` · `ExitedError{code}` · `Panicked{code|sig}` (a panic marker in the last 40 lines within
Δt of exit — the `CellWithoutGenesis` case) · `Signaled{sig}` · `CoreDumped{sig}` ·
`OomKilled{scope}` (SIGKILL **and** the child leaf's `memory.events oom_kill` delta > 0, or a
readable kmsg line; SIGKILL without counter evidence stays `Signaled` — never guess) ·
`SpawnFailed{errno}` · `Unresponsive{phase, alive: true}` · `QuotaKilled{dim}` ·
`StoppedByPolicy{intent}` (the supervisor's own intent record precedes the signal — the drift-
reinstall decision *is* the intent) · `DependencyDied{who}` · `Lost` (the envelope itself died or
restarted and cannot say — honest absence, filed by the next boot from the tally). Incident-level:
`RestartLimitHit{n, window}`.

**Three records, never one** (the OTP/systemd separation that today's code collapses into a log line
and a `return Err`): the *fact* (`ExitClass` + `rusage` at reap + the pre-death `/proc` sample), the
*policy* (`ChildPolicy` in the manifest), and the *verdict* — one atom per death, chained by `cause` into
an incident root: `{incident, child, death_n, exit, uptime, readiness_attempts, first_structured_line,
resource_snapshot, passport_cid, preceding_verdict_cids, tally_window, decision: Restart{after} |
GiveUp{rearm: PassportChange | RepairAction | WindowExpiry | OperatorReset} | KeepWaiting | Escalate,
reason: the rule that fired}`. Persist the **death tally on disk before any restart** (it is what makes
`Lost` and cross-boot incidents computable — "the previous boot's torn uninstall is this boot's
context"). Readiness resets the window, under a **non-resettable deaths-per-rolling-hour ceiling** (a readiness
flap must not reset forever — alpha's class); the **same-cause rule** — three consecutive deaths whose
cause hash matches — yields `GiveUp{rearm: …}` at death three, not at kubelet's sixty-fourth, and the
household sentence is written within ten seconds. The cause is `hash(class, exit code, resource
envelope)`, **never log text** (attacker-controllable; the structured line is carried in the witness,
not in the rule). `GiveUp` has a fifth re-arm, **`RevertToPreviousClosure`**: the prior
`VerifiedRelease` of every process is pinned on disk, and failed readiness inside a declared soak
window after an apply reverts to it **without the network** — a bad storage or envelope release must
not be able to remove the only path that would revert it. This is a safety property, not a consent
mechanism; canon's "no per-node veto" is untouched. **A give-up is a state with a named re-arm
condition, never an exit code.** The parent's restart of the *envelope* (kubelet's backoff, systemd's
`StartLimit`) is the outer ring, recorded as `Lost`, never a second opinion.

**What the supervisor must never do to the conductor** (the child contract, verified): kill or
restart inside `uninstall_app` (a non-atomic critical section across two stores; a 30 s busy timeout
was enough to tear it) — critical sections are declared or inferred from the envelope's own admin
calls and *refuse* restart/kill while open, extending every clock across them; restart during the
post-restart integration storm (read-pool saturation and "out of threads" are load, not death; the
120 s give-up was self-inflicted restart pressure); treat drift as migration intent; wipe `databases/`
while keeping `ks/` (`db.key` lives in `databases/`); run two conductors on one data root; change the
passphrase convention between supervisors. Orderly stop is **SIGINT-with-deadline then SIGKILL** —
the conductor handles only SIGINT; today every path is SIGKILL-first and every kubelet stop is thirty
seconds of ignored SIGTERM then a namespace teardown. `kill_on_drop(true)` becomes a per-driver
policy: right for Delegated (the pod dies as a unit), wrong on a household box where the envelope
must update itself under a running conductor — which is what `recover()` exists for.

### 5.2 Quota — topology first, then ram-guard lifted

Nothing in the monorepo writes a resource limit on a peer process; the only scheduler write is a
best-effort `nice`, disabled on alpha. The verb is built in this order:

1. **Topology is the floor.** The supervisor never shares an OOM group with a child it witnesses:
   root leaf `memory.oom.group=0`, per-child leaf `=1`; `memory.high` band before `max` (the
   conductor's SQLite page cache is reclaimable file memory — a hard max with no band turns cache
   pressure into thrash-then-kill); `oom_score_adj` from the declared shed order. Works inside a
   *delegated* cgroup-v2 subtree without root; declares `unwired` honestly where there is no cgroup.
   Leaves only help when a **leaf** limit binds first: the kernel walks victim → oom domain, so an
   ancestor breach under kubelet's `oom.group=1` still group-kills the envelope. Hence the invariant
   **Σ child `memory.max` + envelope headroom < root limit**, a manifest violating it is refused, and the
   OOM class is declared `unwitnessable` where the root's `oom.group` is not writable (alpha today).
   Above `high`, escalation reads PSI dwell time, not just bytes, and `io.max` is assigned where the
   controller is delegated — parking a guest at `memory.high` burns uncharged reclaim otherwise.
2. **ram-guard's shape, lifted**: committed-not-gross, protected set, typed tier ladder,
   never-shed-unknown, re-measure between kills, ≤ N kills per tick, **one witnessed event per kill**
   in the reciprocity ledger — with the process-name heuristics replaced by the manifest's `protected` and
   `shed_order`. Its own `RETIRE_WHEN` names this verb as the platform that retires it.
3. **CPU throttle is a lifecycle signal**, not a passive cap: a CFS-throttled conductor converts
   scheduling delay into `DatabaseError(Timeout)` — the mechanism that tore the uninstall. The
   envelope reads `cpu.stat nr_throttled` and refuses destructive operations while saturated (the
   `happ_manager` preflight generalized). Quota the burst (post-restart integration), not the mean;
   `nice` becomes `cpu.weight` once leaves exist.
4. Seam 3.15's home is misrouted for this concern: `bounds_validator` bounds *authority*, not
   processes. The quota's *declaration* rides `delegates-compute` bounds as extra keys (the arc
   actuator's precedent); its *enforcement* is this new mechanical floor.

### 5.3 Accounting — one record type, three cadences

The ledger already exists with every column and no producer: `economic_events.bounded_by` joined to
`mishpat_commitments.cid` (= entry hash), `substrate_signal` validated by the DNA against
`compute·storage·bandwidth·energy·time`, the 60-minute rate window live in `bounds_validator`. What
the envelope emits, per child, in the shape IPVM's receipt and Golem's debit note agree on:

- **Commitment (before)** — the manifest is the `ran` target; the self-contract (own children) or the
  guest's `delegates-compute` is what every row is `bounded_by`. `action ∈ REA_ACTIONS` (`use` for own
  consumption; `produce`/`transfer` for capacity lent) — never the ad-hoc `compute-fulfilled`, which
  is outside the integrity list and forgeable by its own documentation.
- **Interval rows (during)** — per process, per interval, **cumulative monotone counters since spawn**
  (cpu-seconds, byte-seconds, bytes-egress, restarts, readiness attempts), `provider` = the node's
  `agent_cid`, `receiver` = the child's owning agent, `in_scope_of` = the manifest CID. Golem's trick: a
  lost interval loses nothing; the next row still totals. **Category-C amber rows** (`dht_anchor_hash`
  NULL, exactly as both existing writers) — the head-plane forbids notarizing ~500 k rows/node/year.
- **Breach events + a period composite (attested)** — a shed is a row `bounded_by` the guest's
  commitment (the reciprocity ledger); one composite per period and one per incident are the only
  heads.
- **Terminal receipt (after)** — the death witness *is* the invoice: final cumulative counters, the
  tail, the snapshot, `out: {error: exit-class}`, and the next verdict in `fx`.

Needs one SDK addition: a compute `ResourceSpecification` vocabulary (`cpu-seconds`, `byte-seconds`,
`bytes-egress`) — none exists. Trust on a hostile network comes from the custodian's receipt-of-
receipt, never the supervisor's self-report; the security hold on `record_compute_fulfilled_event`
applies verbatim to any economic consequence of a guest's rows.

### 5.4 Listening

Two rings per child (200 lines), a 40-line stderr-first tail in the summary, and structured parsers
declared per process — for the conductor: `Database read connection is saturated` (INFO; the manifest
pins the `RUST_LOG` filter so it cannot be silenced), `FATAL PANIC:` / `Payload:` / `Location:`,
`Failed to claim a thread … out of threads`, `Admin listener finished` (the socket died while the
process lives), the `Conductor startup:` milestones, `Conductor ready.`. Children's stdio is still
teed to the envelope's fd 1/2 so `kubectl logs` and Loki keep working; the ring is the envelope's
copy. `InProcess` children feed the same ring through a tracing layer.

**Verbosity is a berth dial, not a manifest value** (operator, 2026-09-02): the ark's own log level
lives on the `Berth` (per blade; `ARK_LOG` and `--log-level` override it), the child's level rides
`ChildSpec.env` as a `{log_level}` template resolved from the berth, and a runtime re-dial without
restart (SIGUSR1/SIGUSR2, then the admin socket) arrives in S2. Debugging one box never moves the
shared manifest's CID; the witness's evidence volume (`ring_lines`/`tail_lines`) is a manifest value
because it is the size of a death's evidence, not chatter.

### 5.5 Describe — the passport, by content, persisted

`RuntimePassport` (`GET /version`) is the origin of this verb and stays the live projection. The
passport becomes an **atom** (kind `runtime-passport`, riding `node-context`), re-minted on every
spawn/apply/restart, and its rule is: **it describes only processes the envelope supervises, from
artifacts it hashed itself** — sha256 of `/proc/<pid>/exe` at spawn, never `CONDUCTOR_IMAGE_TAG` (a
mutable name; the `4a81a749` vs `7513654f` drift is a tag inequality nobody compares — with closure
CIDs it is a CID inequality at declaration time). Per process: exe hash, `BuildInfo` where the child
self-reports, argv/env fingerprint, pid, uptime, restart count, last verdict, effective quota tier.
For the conductor additionally, via the admin socket: per-role DNA hash, coordinator wasm hashes,
cell ids, **agent pub key** (the feature file already claims it and the struct lacks it), bundle-on-
disk vs installed (both computed in `happ_manager`, today only in logs). Plus: the last boot decision
(`ReinstallFlags` as read, `DriftAction` as chosen, `CoordinatorSyncReport`), the wire epochs spoken
(the compatibility envelope's missing passport side), followed channels with mode / `appliedRelease`
/ `pendingRestart`, the data-root hash with its `passphrase_source`, and which of the three
authorities last acted. The passport is the **declared-vs-observed(-vs-attested) diff** over the
manifest's closure CIDs; "conductor and node containers on different builds" becomes a sentence it
writes. Until the envelope is PID 1 everywhere, a node passport must *read* the attached conductor's
own `/version` rather than echo its own pin.

## 6. The death witness — end to end

1. **Intent, write-ahead.** Before every decision (spawn, restart, reinstall, re-key, shed) the
   envelope appends `{ts, kind, detail, passport_cid}` to an fsynced intent log under its *own* data
   dir (Urbit's record-before-effect at the supervisor's grain; issue #1250 is the household
   power-cut case). This is what "would have joined the two boots" on 2026-09-02.
2. **Capture, at the parent's grain, no daemon, no root.** `waitid(WNOWAIT)` status → the last
   `/proc/<pid>` sample while the zombie is still readable → `wait4` for `rusage` (race-free
   post-mortem) — which is a **Driver contract**: supervised children are spawned with
   `std::process::Command` and reaped by the envelope's own reaper thread, never `tokio::process`,
   whose internal reaper consumes the status and loses the `rusage`; the last pre-death `/proc/<pid>/{status,stat,limits,fd,io}` sample, the
   child leaf's `memory.events`/`memory.peak`/`cpu.stat` (reads are unprivileged), the tails, the
   structured samples, readiness attempts, the last eight intents, the passport, `stat` of the named
   data-root files (the five 139 264-byte authored DBs). Budget: private atom ≤ 96 KiB; summary ≤ 4 KiB
   so the same bytes fit `/dev/termination-log`. kmsg needs `CAP_SYSLOG` under `dmesg_restrict=1` →
   recorded as absent-with-reason; core dumps are host-optional (`core_pattern` is root's).
3. **Spool — `amber-local`.** DAG-CBOR via `elohim/epr` (the canonical codec; never re-derived),
   CID-named, fsynced under `<envelope-data>/witness/<incident>/`, signed with the **node-local
   authoring identity** — the libp2p transport keypair, the only key that exists outside the
   conductor. The signature is a **detached proof in `metadata_json`** (the envelope convention: the
   canonical hash excludes the proof), verified by a custodian against the node's transport half of
   its `AgentPeerBinding` (`peer_identity_bindings`) — the one place a transport key is already bound
   to an agent, so this is the two-stage identity the gate allows, never a raw cross-namespace
   compare. Namespace discipline, from review: `author_id` is always the **agent** (`Berth.
   node`, known from the last passport); the proof carries `key_namespace: transport`, the transport
   id, and the `AgentPeerBinding` CID as its link; a witness whose binding cannot be resolved is
   minted **`unbound`** and is never promoted to green — storage verifies the transport signature and
   the `node` match *before* re-authoring, so an unauthenticated spool file is never laundered into an
   agent-key claim. The spool directory is `0700` and never inside any guest's `DataDir` import. **Row
   before blob**: the content row is written first, then the pantry file, so there is no row-less
   window in which the blob plane would serve the witness to anyone. The atom is born
   **content-row-shaped** (`CreateContentInput`: `id: <atom cid>`, `content_type: issue-report`,
   `metadata_json.kind: death-witness`, `reach: private`, `blob_cid`, `content_hash`)
   with its bytes in the pantry — so the existing inventory tick advertises it, the existing
   amber→green ladder (`reanchor_backfill`, which walks `content` rows only, and only under the lamad
   app id — the elohim DNA's app, so the witness row is in its partition) can lift it, and the atom
   home (`/epr/{id}` → `/db/content/…`) reaches it. (An EPR atom in `epr_atoms` has no anchor column
   and never enters the pantry: the two stores are disjoint; the `CreateContentInput` path has no
   author-side reach gate and no signature field of its own — hence the detached proof.) Rendering:
   the atom home dispatches on `contentFormat` through the lamad manifest, so S0 renders the raw-JSON
   fallback and S1 declares a `death-witness` format with a renderer — a manifest change, never a DNA
   change.
4. **Offer — `amber-offered`.** The envelope has no swarm and must not borrow storage's (storage may
   be the child that died). The offer is **disk-first and deferred**: whichever storage child is alive
   next — the survivor now, or the next boot — reads the spool and offers along the custody plane.
   Custody needs a shape that does not exist: `custody-blob` commitments are per exact hash, so a
   fresh witness has no custodian. The envelope mints, at first readiness alongside the self-contract,
   a standing **`custody-spool`** commitment whose resource is the node's witness spool (kind + agent,
   not a hash) — a recovery partner like adam is simply a custodian inside the reach declared by that
   commitment; nothing in the witness knows about pairs. Two gate changes follow: a **custody-scoped
   read gate** (a custodian named by commitment may pull and hold what it custodies, independent of
   human relationships — today pull is author-only for every non-commons tier and `DirectOnly` fanout
   has no implementation), and the blob plane's "no content row → serve to anyone" must not apply to
   spool blobs (they carry their content row from birth).
5. **Green.** When a conductor is next available the storage child authors the row through
   `content_store::create_content` under the **agent key**, at the reach the custody-spool set
   resolves to (§6.7) — the anchored content row *is* green for the witness, by the standard
   `dht_anchor_hash` derivation. The **incident attestation** (one head per incident, never per
   restart) is **deferred to the constitutional DNA change** batched with runtime-artifacts §11.1:
   `attestation:death-witness` is absent from `ATTESTATION_KINDS` (floor F1 refuses unknown kinds), and
   the nearest existing kind, `attestation:device-health`, has a closed schema (`additionalProperties:
   false`, a five-value `health_metric`) already ridden by release attestations — riding it would be a
   schema-first extension and a collision, not a free valve. Until the batch, the incident root is the
   anchored witness row of the incident's first death plus the chained verdicts. Custodians attest
   *receipt* the same deferred way; both agent-scoped.
6. **The witness's own death — honest absence.** `PR_SET_PDEATHSIG` is for **guest leaves only**: it
   fires on spawning-*thread* death, which includes the envelope's own runtime shutting down during a
   self-update, and would kill the conductor `recover()` exists to re-attach — containerd shims
   survive their manager precisely by not doing this. Own children get `PR_SET_CHILD_SUBREAPER` when
   the envelope is not PID 1 plus an **exclusive lock on the data root** so a second envelope can never
   double-run a conductor. On the next boot an intent
   with no sealed report becomes a **gap record** `{class: Lost, previous boot_id, intent,
   parent_verdict: kubelet lastState | systemd $SERVICE_RESULT | none, cgroup: recreated}` — systemd's
   `COREFILE=missing` made first-class. The custodian files the gap from its side: a node reappearing
   with a new boot id and no death record for the prior incarnation is a gap, never healthy. Inside a
   pod the envelope writes its intent pointer to `/dev/termination-log` on every decision change, so a
   SIGKILLed supervisor still leaves ≤ 4 KiB behind.
7. **Readers are the reach.** `LocalRelationship` is floor-protected and has three divergent concrete
   representations today (Private placeholder · Trusted|Familiar · standing), and the eight-value
   `Reach` enum has **no household variant** (Private · Self · Intimate · Trusted · Familiar ·
   Community · Public · Commons). The spec does **not** canonize a tier: the witness is born
   `private` (the only tier a transport-key author is admitted at), its readers are bound
   mechanically to *the custody-spool commitment's set*, and the tier the agent-key re-authoring
   lands on is left to the ontology resolution (`unresolved — reach vocabulary in declared drift`;
   `Trusted` is the candidate the reach-earning floor already maps to LocalRelationship). Every
   "household tier" in this document reads that way.

### 6.5 Corrections from station 2 on the mesh (2026-09-03)

- **Custody rides the elohim DNA's `Commitment` through `content_store::create_rea_commitment`, which has no
  action whitelist; its integrity zome has no commitment validation.** `custody-spool` therefore needed no zome
  edit at all — not merely hash-neutral. The mishpat coordinator's exhaustive action match (§7 M5) governs a
  different, three-field homonym on a different DHT; M5 is withdrawn for this path.
- **The custodian's counter-signature already exists as authorship.** A custody-spool row authored on the
  custodian's own conductor binds the custodian by its own pen; `attestation:custodian-commitment` is reserved
  for an explicit second signature later, not required for station 2.
- **The custodian learns of a ward's witness through the shard replication plane** (`run_replication_cycle` →
  `ShardRequest::ListContent{reach_filter: None}`), which carries unanchored, `private` content rows verbatim
  between household peers. No inventory hint was needed. The same plane is the named site of the missing
  custody-scoped read gate (M9): it replicates private rows and their blobs to every household peer with no reach
  check — station 3b's substrate move.
- **One digest, two renderings, at every join.** Replicated rows arrive with `blob_hash` in the CID rendering
  (`bafkrei…`) while inventory rows are `sha256-<hex>`; a join by string equality is silently empty. Join by digest.
- **Measured budget:** custody rows on both custodians ~75 s after the kill on a loaded host (ingest ≤5 s; the
  replication cycle pages peers so its effective cadence is ~60–70 s regardless of the tick dial; custody sweep
  5 s). The story's station-2 budget is two minutes.
- **Missing substrate nodes minted:** M7 the iroh fetch path emits no `serve-blob` receipt; M8 `serve-blob.output_of`
  never names the commitment it discharges; M9 as above; M10 `peer_blob_inventory` persists no kind/type (hints are
  scored and dropped); M11 the commitments HTTP view renders a bare-string `resource_classified_as` as null.
- **Corrections from station 3b (2026-09-03, plan `2026-09-03-ark-s1-station3b-custody-read-gate-plan`).** The HTTP
  path already refuses an anonymous caller on both the row (`GET /db/content/{id}` → 403 + `requiredReach`) and the
  bytes (`GET /blob/{hash}` → `blob_serve_verdict` → 403); the honest gap was the shard replication plane, where the
  same `ShardService` served every `private` row and blob to any PeerId and receivers persisted them verbatim. The
  gate is one pure predicate, `private_serve_verdict`, whose standing facts are the counter-signed custody
  commitments — consent (C12) is the commitment, never the requester's claim — with the requester resolved through
  the existing identity maps (a routing cut at Stage 1; bindings are self-asserted, so the gate carries no economic
  weight until `identity-cross-signed` lands). Only `private` changes behaviour; other reaches are a later station.
  **Posture decided:** 403 acknowledges existence and names the reach required; 404-hides-existence was rejected
  because inventory gossip already advertises the hash (M13) and the household's own non-vacuity control must tell
  "refused" from "missing". **Missing nodes minted:** M12 the iroh byte plane (`iroh_blobs` ALPN) has no reach at
  all — any NodeId pulls any hash; M13 inventory gossip advertises `private` blob hashes to every peer while serve
  now refuses (advertise/serve asymmetry, C7 partial) — custodians need the advert, strangers learn only that a hash
  exists.

## 7. P2P design gate

**Entity: RuntimeManifest** — Notarized (A), reusing the `Content` entry type with `metadata_json.kind =
runtime-manifest` riding `node-context`; DNA-hash-NEUTRAL (a first-class type is hash-moving; batched).
Head-plane: one declared head per manifest lineage; a handful per household, tens per fleet; versions
under it. Address: content-derived CID (`bafyrei…`), input-addressed and shared; **which manifest applies
is a declared head, never recency** — the instance pins it like a lockfile. Stakes: all four; artifact
verification floor-protected (never stage-priced). Integrity zome `content_store_integrity` (packed
from `dna/elohim/`), untouched. Coordinator: existing `content_store::create_content` +
`declare_canonical_content_head` → EntryHash (cid), action hash as `dht_anchor_hash` only.
Projections: `content` row (anchor yes); Automerge: no (below broadcast). Route: none new — `/epr/{cid}`;
the envelope's own admin surface (`ark describe`, a node-local socket) is excluded from
`build_manifest()` exactly as `POST /admin/coordinators/sync` is. Anti-patterns: not modeled in the
k8s plane (the pod spec is one *packaging* of the manifest); no random ids; no "latest".

**Entity: Berth** — Private (B) with an attested effect: agent-scoped composite (node
agent × manifest cid); the *applied* fact is what peers need to verify → the passport atom carries it.
Private source chain / local disk; no shared table; no route.

**Entity: Incident / DeathWitness** — **Notarized (A) at incident grain** (the design-gate review
corrected the witness atom's B2: a `Content` entry is a public notary record and every pantry file is
advertised, so there is no private-chain raw to attest *from*). The **incident** is one content row
riding `issue-report` + `kind: death-witness`, a **composite root whose head moves as deaths are
appended** (`content_head` over the incident's lineage — the same declared-head shape); the per-death
verdicts, the tails, the snapshots, and the passports-at-death are **bytes inside the incident blob**,
never rows and never heads. The DHT entry carries only the proof (hash, kind, agent, incident number,
exit class); the bytes live on the blob plane behind the content row's reach; `metadata_json` never
carries a tail. `Content.id` **is the incident's atom CID** (`bafyrei…`) — the atom home resolves by
`Content.id`, not by entry hash, so declaring one identity kills the third address form; EntryHash is
`cid`, ActionHash is only `dht_anchor_hash`. Head-plane: < 100 incidents/yr across a 7-peer fleet;
zero heads per restart. Transport `auto`; **no conductor call on the death path**. Source of truth:
the node's disk at birth, replicas at custodians, the anchored row the public proof. Readers: the
custody-spool set. The custodied copy is a **redacted summary** (typed class, counters, hashes;
argv/env hashed; fds classed, not pathed); the raw ≤ 96 KiB stays local behind a per-incident grant.

**Entity: RuntimePassport** — Ephemeral (C) as the live `/version` projection, plus **one Notarized
(A) atom per *applied* transition** (a manifest applied, a release adopted, a re-key) riding
`node-context` + `kind: runtime-passport` — never per spawn or per restart (that would be a head per
restart). Its effect — "node X runs closure Y, applied manifest Z at T" — is what the adoption controller,
Station 8's matrix, and custodians diffing peers need. The passport-at-death rides inside the incident
blob. It carries a monotone **`incarnation`** counter so a custodian can detect an absent death record
(a node reappearing at incarnation n+1 with no incident for n is a gap, never healthy).

**Entity: compute accounting rows** — Ephemeral (C) interval samples in a **separate
`compute_samples` table** carrying the Category-C comment (a NULL-anchor row in `economic_events` is
the amber-forever `stewardship_allocations` shape, not C), keyed `(node, child, incarnation, seq)` and
consumed max-by-seq (cumulative-since-spawn without a spawn epoch collides and replays) + Notarized
(A) **breach events and daily composites** via the existing `EconomicEvent` entry, `bounded_by` in
`metadata_json` on the DHT (there is no `bounded_by` field on the entry) and a real column in storage;
a corpus digest per node-month. Requires the compute `ResourceSpecification` vocabulary in
`elohim/sdk/schemas` declared as **app-manifest vocabulary, not a `_dna` enum** — the codegen emits
every `_dna` enum into the integrity zome, which would be a hidden hash move. Head-plane: ~365
composites/node/yr + breaches; ~500 k raw samples/node/yr stay C. Rows naming another agent as
receiver (a guest's use) are economically inert until counter-signed by that receiver; unsigned
interval rows never carry economic consequence.

**Entity: self-contract + custody-spool commitments** — Notarized (A) on the existing
`Mishpat::Commitment` entry (`action = delegates-compute`, and a `custody-spool` action whose
resource marker is a spool kind rather than a hash). **DNA-hash-NEUTRAL, verified**: the mishpat
integrity zome's `commitment_action_requirements` ends in `_ => None` (unknown actions validate);
the only blocker is the mishpat *coordinator*'s exhaustive match ("unhandled action"), a
coordinator-only change healed by hot-swap. Storage's `CONDUCTOR_SOFT_ACTIONS` lists only
`custody-blob` today; the self-contract needs the mishpat cell resolved, which `hc_client` already
does. **Two corrections from review:** (a) the self-contract is minted at first readiness, but the
motivating death happened 2.3 s into boot — pre-readiness verdicts are `bounded_by:
SeedPolicy(seed_cid)`, and a re-key re-mints both commitments and revokes the old (the old provider
is the old agent); (b) a self-authored grant (`provider == recipient`) passes integrity and must be
**enumerated as such** with `epr_scope` scoped to `runtime:<manifest cid>` so the operation-authorization
path never reads it as a content-op grant. The **custody-spool** commitment is the node's *offer*,
not the custodian's *consent*: it becomes binding only when a custodian outside the node's control
counter-signs it (`attestation:custodian-commitment` exists with zero producers — this is its first),
which is also the capture guard: a captured steward naming its own second box as sole custodian holds
no counter-signature, and its incidents stay `unbound`. The commitment carries `bounds{max_bytes,
atoms_per_hour, retention}` validated like `rate_per_hour`; the local spool is a ring with `pruned`
stubs and a typed `Refusal` on breach. Identity `cid = entry_hash`
(never the action hash). Minted through the conductor at first readiness — the runtime authors a
commitment for itself, which no runtime does today (only the dev seed route, which synthesizes a
`dht_anchor_hash` that passes the notarization gate — so the envelope's amber→green cannot use that
column alone as proof; it carries explicit conductor evidence).

**Concern canon (Step 4) for the `RestartGovernor` predicate and the witness.** C0 plane: the
runtime seam's own crate, bridging to the p2p plane through storage — answered. C1 anti-self-election:
the envelope never elects heads; it obeys the channel's election — answered. C2 monotonic authority:
`GiveUp` is a state with a named re-arm; the three authorities are ordered — answered. C3 liveness:
the reaper loop is the envelope's own; the witness is written within a bounded budget before any
effect — answered by construction. C4 honest absence: `Lost`, the gap record, the custodian's gap —
answered. C5 evidence-not-authority: the passport is a diff, the pin tag is never trusted — answered.
C6a bounded work / C6b idempotent effect: rings, budgets, `same_cause_limit`; restart is idempotent
under `recover()` — answered. C7 advertise/serve symmetry: readiness gates advertisement (the DNS
A-record analog is the inventory offer) — answered. C8 observability-per-decision: the verdict *is*
the record; every refusal carries `LimitOwner` + `ReasonLabel` — answered. C9 identity lineage: re-key
only on `Reseedable`, once, witnessed; the passport carries the agent key — answered. C10 contract
evolution: the manifest's `supersedes` + wire epochs in the passport — answered. C11 backpressure: CPU
throttle is a read signal; the harvest adds no load to a dying child — answered. C12 consent: the
self-contract, the guest commitment, the update lock as a pause — answered. C13 graduated authority:
`LimitOwner ∈ SelfLimit·Commitment·Operator·Faith` on every refusal — answered. C14 witnessed residual:
the incident root carries the restart count and the custodians' receipts — answered. Registration: a
`seam-registry.yaml` in the new crate at birth (§8).

**Back-fill check.** (1) The coordinator returns the manifest's / witness's EntryHash; `/epr/{cid}`
accepts exactly that. (2) `content_store_integrity`, untouched — DNA-hash-NEUTRAL by riding existing
types; the first-class types are declared hash-moving and batched. (3) At one year: manifests — tens;
incidents — < 100; composites — ~ 1 per node-period; no measurable quiesce delta.

## 8. Crate boundaries

Workspace members under `elohim/` (`elohim-compute` and `elohim-peer-fabric` are the member
precedents; `eprfs`, `rakia`, and `elohim-storage` are `exclude`d workspaces of their own, and
storage consumes shared crates as path deps — `elohim_compute` is the precedent):

| Crate | Owns | Depends on | Purity |
|---|---|---|---|
| **`ark-core`** | `RuntimeManifest`, `Berth`, `ProcessSpec`, `ChildPolicy`, `ExitClass`, `Verdict`, `Witness`, `Passport`, `ProcessSample` (a NEW per-process cpu/mem/fds/io type — `elohim_compute::ResourceSnapshot` is request-level and serialized to live consumers; it is not extended), the lifecycle state machine (spawn → ready → live → dying → dead), the intensity/same-cause rules, `RestartGovernor: Governor`, the tally | `epr` (codec + CID — never re-derived; the `elohim/.epr-meta` interface-first rule), `elohim-compute` (`Governor`/`Refusal`/`LimitOwner`, `BuildInfo`), `seam-contracts` (`Answer<T>`, `ReasonLabel` — a path dep; no `elohim/` workspace member consumes it yet, so tevah is the precedent) | the peer-fabric shape: no tokio, no diesel, no libp2p/iroh; `Clock`, `ProcessHandle`, `ResourceProbe`, `WitnessSink` traits; a boundary test asserts the dependency tree |
| **`ark-supervisor`** | tokio: spawn, pipes, rings, parsers, `pidfd`/`waitid` reaper, `rusage`, `/proc` + cgroup readers (lifted from `system_metrics.rs`), the intent log, the spool writer, the `Driver` trait + `Native` and `InProcess` drivers; `Cgroup` and `Sandbox` drivers behind features; `Wasm` driver deferred | `ark-core`, `nix`, `procfs` | I/O, no network |
| **`ark`** (binary) | `run <manifest>` (PID-1 mode: subreaper, signal forwarding, termination-log), `describe`, `witness ls|show`, `notify` (the sd_notify socket) | the two above | the launchable unit every context execs |

**Consumers and what changes.** `elohim-storage`: `process_manager.rs` becomes a thin consumer of
`ark-supervisor` (readiness returns a *verdict*; the `AdminWebsocket` is constructed by storage
from it; the DB-pool metrics hook becomes a parser callback), then storage itself becomes a child on
the fleet; storage keeps custody/offer/attest (it owns the swarm and the conductor). `steward/node`:
the executor's `RestartService` calls the envelope's lifecycle verb; its `ServiceStatus{running,
healthy, uptime_secs, restart_count}` becomes true; the `pod` module is renamed at the Hub refactor
(hub-boundaries already plans to fold it into `HouseholdHub`). `lvi-actuator`: a driver profile and
a consumer of verdicts, never a second supervisor. `hc-mesh.sh`: `direct` mode becomes `ark run`.
The conductor template: the `elohim-conductor` container's entrypoint becomes `ark`. Tauri: the
sidecar becomes `ark` with storage as its child. **Deleted or demoted:** `steward/node/src/update/`
(superseded by the exec floor), `compute_rea.rs` (wrong grain, random ids — subsumed by §5.3),
`GENESIS_SELF_HEAL_IDENTITY` / `ALLOW_DNA_REINSTALL` as *behaviour* (they become manifest fields;
`DNA_MIGRATION_INTENT` stays as the per-roll intent input), ram-guard's process-name heuristics.

Each crate carries its own `.epr-meta/` (the habit home, §10) and `seam-registry.yaml` (the birth
rule).

## 9. "Do we write our own docker?" — the answer

Docker is three things, and the envelope's relationship to each is different:

| Layer | What it is | Elohim's answer |
|---|---|---|
| **Images / pull / registry** | content-addressed manifests + layers, digest-verified fetch, overlay unpack | **Already replaced.** eprfs `ProjectionManifest` + `LocalMaterializer` ("mount, don't ship"); artifacts by CID on the blob plane; the release manifest is the executable-artifact manifest. The envelope grows no pull path. |
| **Isolation** | namespaces, seccomp, capabilities, cgroup *limits* | **Consumed, never written.** cgroup-v2 subtree writes (~200 lines) where delegated; bubblewrap for unprivileged namespaces (isolation, zero quota; its `--json-status-fd` is a ready-made witness channel); youki's `libcontainer` only if a full OCI bundle is ever needed for a guest (Linux-only, expects systemd, fail-closed on controllers); wasmtime for guest components. An OCI-compliant runtime is a second project — years, four crates, 14 % documented — and buys nothing for the peer's own trusted children. |
| **Supervision** | spawn, reap, restart, logs, exit status, describe | **Ours, and new.** Nobody has a p2p-native, REA-accounted, witness-first, reach-declared supervisor whose parent role is sufficient without an outside. This is the wheel worth re-inventing, and it is small: the POSIX core `process_manager.rs` already manifests. |

An OCI `config.json` is kept as a **derived, host-local driver input for guests** (typed for free
via `oci-spec`; `annotations` carry the manifest and process CIDs; one projection reaches crun, youki,
runwasi) — never as the declaration (it carries host paths, has no restart policy, no readiness, no
log ownership, no passport, and is fail-closed where the envelope needs declared-vs-effective).

## 10. Evidence

**Habit.** A new atom, born **`red`** (the census refuses `unwired` once a runnable check is
declared, and two are: the a2o concern with every station @wip, and the cargo classifier leg — the
invariant is measured not held), declared where the only supervisor lives today —
`elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md` — with
`retire-when: when the supervisor moves into the envelope crate and this atom moves with it, and a
household operator reads a peer's death from the peer itself with no developer tool in the path`.
The envelope's own habit (the five verbs) is born in `elohim/ark/.epr-meta/` when that crate
exists; do not pre-declare it in storage. **The WIP fence is the operator's call**: both slots are
held; `operator-runtime-surface` is green — finished for fence purposes — and the honest move is
`active: false` there with the slot to the witness. This spec does not flip it.

**Scenario.** `genesis/a2o/features/resilience/death-witness.feature` beside `chaos-peer-churn`
(which already carries the SIGKILL drills), tags `@e2e @resilience @concern:death-witness @act:i
@requires:owned-substrate`, four stations, finish line untouched: (1) the death — three peers each
running its conductor under the envelope; Jessica's conductor SIGKILLed; within N seconds her peer
lists a witness naming the signal, the uptime, the last stderr lines; (2) custody — every custodian
in the spool set holds the CID within the bounded budget, with a receipt event; (3) render/refuse —
Matthew's atom home renders it; an anonymous fetch on Jessica is refused; (4) attestation — Jessica's
conductor restarted; the incident anchored amber → green. Reusable steps exist for the mesh fixture,
custody assertions, the atom home, and reach refusals; the new steps are the launcher precondition,
a per-conductor kill (today only `conductors-restart` all at once), and the witness query.
**Lifecycle-as-fixture precedes lifecycle-as-feature**: until the mesh launches conductors under the
envelope, every household-lane assertion is vacuous by construction.

## 11. Sequencing — slices with receipts

- **S0 — the launcher on the mesh.** `ark-core` + `ark-supervisor` (Native driver) + the `ark`
  binary; `hc-mesh.sh`'s `direct` launch mode (`echo test | holochain --piped --structured=Log
  --config-path …` under `setsid nohup`) becomes `ark run` — a short argv port, since the direct
  parent, the piped passphrase, and per-conductor logs already exist; `hc sandbox generate` keeps
  owning config creation and app install, and `assert_toolchain_parity` still refuses a skewed
  hc/conductor pair. Witness to spool (`amber-local`); the intent log; `ExitClass` + tally +
  same-cause `GiveUp`. Receipt: station 1 of the scenario on the household mesh; the ring, the
  classifier, and their two tests (`264ce8ce4`) lift from `process_manager.rs` into `ark-core`.
  The manifest's artifact reference is born in the closure-CID / channel-head shape with a
  pinned-local-path resolver only (register 24) — S0 never touches the network.
- **S1 — storage consumes; custody and attestation.** `process_manager` delegates to the library;
  the self-contract and custody-spool commitments minted at first readiness; the custody-scoped read
  gate; `amber-offered` → `green`; the passport as an atom. Receipt: stations 2–4; `epr-atom-home`
  renders a witness. Alpha *confirms* by shipping `ark` **inside the edge storage image** as its
  new entrypoint and taking a deliberate `[conductor-roll]` — the conductor container sets no
  `command:` and its image field holds the running image absent a roll, so a template edit alone
  changes nothing (k8s becomes one packaging of the envelope; the split-image drift is a passport
  sentence).
- **S2 — PID 1 with a real signal contract.** SIGTERM → per-child SIGINT with grace; termination-log;
  honest probe endpoints (startup = declared set ready; readiness = conductor serving, flipping false
  on child death; liveness = the envelope itself); the gap record. Receipt: a forced conductor death on
  alpha is served as a witness CID from the node's own surface within N seconds (the termination
  message is harvested only when the *container* exits — it is the receipt for the envelope's own
  death, never for a child death the envelope survives).
- **S3 — quota.** The cgroup driver on the household box (`Delegate=yes`), the leaf topology, the
  ram-guard ladder lifted, effective-tier honesty in the passport, accounting rows and the
  `ResourceSpecification` vocabulary. Receipt: a memory-hog guest is quota-killed with one witnessed
  event while the conductor stays healthy — lvi Slice 1 lands on this.
- **S4 — the binary class adopts by election.** The envelope as the apply vehicle for storage-binary
  releases on the mesh (Simulacra stakes), `pendingRestart` consumed, revert by re-election. Receipt:
  runtime-upgrade-propagation station 7 with a binary. `InProcess` driver for the Tauri desktop.
- **Deferred, shaped so nothing precludes it:** the Wasm driver for guests; sub-envelopes for rakia
  build steps; the first-class `runtime-*` content type and `attestation:death-witness` kind (batched
  into the constitutional DNA change); a minimal transport of the envelope's own (rejected for v1:
  disk-first is honest and cheaper).

## 12. Decision register (sealed by this spec; the operator's acceptance is the graduation trigger)

1. **The envelope is its own crate family and launchable unit, below storage** — never a feature of
   `elohim-storage`'s binary, never grown out of `steward/node::pod`, never `eae`. *Rejected:* growing
   the witness inside `process_manager` (ships only where `kubectl logs --previous` already exists).
2. **Name: tevah; declaration: `RuntimeManifest`; "pod" retired for the runtime.** *Rejected:*
   `elohim-pod` (live collision with the operator module; k8s-plane connotation). Refined by 20.
3. **The parent role is sufficient.** Supervision and listening require no privilege; isolation and
   quota are drivers with a witnessed effective tier. *Rejected:* an OCI runtime; podman/docker as the
   primitive; "cgroups everywhere" (no watch) and "wasm everywhere" (no witness subject).
4. **Verdicts are REA actuation receipts** bounded by a runtime-minted self-contract; the witness is
   the verdict's payload; give-up is a state with a re-arm condition. *Rejected:* a log line + exit
   code; delegating restart policy to kubelet/systemd.
5. **Liveness is the envelope's own reaper**, never a probe on a sibling's `/health`.
6. **Topology before thresholds**: the supervisor never shares an OOM group with a witnessed child.
7. **The manifest names artifacts by content** (closure CIDs; channel heads for auto-adoption; pinned
   CIDs as a lockfile); the passport hashes what it runs. *Rejected:* `CONDUCTOR_IMAGE_TAG`-class
   names; standing env flags as behaviour.
8. **Three authorities, three gates** (election above the DNA line — the envelope is the apply
   vehicle; per-roll intent at the DNA line; self-policy for re-key). Consent is a pause, not a veto.
9. **The witness rides existing content types** (`issue-report` / `node-context` + kind); first-class
   types are batched into the constitutional DNA change. *Corrects* the witness atom.
10. **Custody of witnesses is a standing `custody-spool` commitment**, minted while the conductor is
    alive; readers are that set; a custody-scoped read gate is required. *Corrects* the witness atom.
11. **Disk-first, deferred offer; the envelope has no swarm in v1.**
12. **Node-local authoring identity = the transport keypair** (amber, private) → agent key (green,
    household tier) through the conductor.
13. **Accounting is one record type at three cadences**; interval rows are Category-C; breach events
    and composites are the only heads; the ledger is `economic_events`, not `elohim-compute`.
14. **Evidence home**: habit `runtime-death-witnessed` (born red → green on stations) in storage's
    `.epr-meta` until the crate exists; scenario in `features/resilience/`; the fence flip is the
    operator's.
15. **S0 is the launcher on the mesh**, not a storage feature; alpha confirms.
16. **A custodian's counter-signature makes custody binding** (`attestation:custodian-commitment`,
    its first producer); a self-named sole custodian holds nothing. *From review.*
17. **The witness is Notarized at incident grain**; verdicts and passports are bytes inside the
    incident blob; the passport atom is per applied transition. *From review; corrects §7's first
    draft and the witness atom's B2.*
18. **The envelope's own binary is pinned-only in v1** (never channel-auto), and every process keeps
    a pinned previous closure for `RevertToPreviousClosure` — the network-independent safety brake.
    *From review.*
19. **Guests fail closed below `Enforced`**; supervised children are reaped by the envelope's own
    reaper, never `tokio::process`; `PR_SET_PDEATHSIG` is for guest leaves only. *From review.*
20. **Branding split — tevah is the name, `ark` is the identifier** (operator, 2026-09-02). Prose,
    titles, the habit ledger, and the story say tevah (the ark, תֵּבָה); crates, binaries, paths,
    and type prefixes say `ark`: `ark-core`, `ark-supervisor`, the `ark` binary, `elohim/ark/`.
    One name for people, one for the toolchain — never mixed inside a single surface.
21. **The `epr-pvc` bridge is the guide-star at the end of the valueflow chain, not a slice**
    (operator, 2026-09-02). A bridge-seam crate through which the network of peers offers external
    actors a collective agreement for persistent volumes that are actually backed by this runtime.
    Nothing is minted; `Berth.data_root` is reserved as a volume head with no semantics
    in v1; each slice is checked against one question — *does this preclude the guide-star?*
22. **WIP fence: `runtime-death-witnessed` takes `operator-runtime-surface`'s slot.** The surface
    habit is green with wired checks; a green habit holds no attention. Actives are
    `dataplane-convergence` and `runtime-death-witnessed`.
23. **The constitutional DNA batch waits for S1's measured shape.** First-class `runtime-*` types
    and `attestation:death-witness` are scheduled only after a witness has been anchored on
    existing content types on the mesh; moving the DNA hash for an unmeasured shape is the wrong
    order (smallest slice that flips a named red).
24. **The update-propagation loop is trusted enough to dogfood from S0's shape.** The manifest's
    artifact reference takes the runtime-artifacts closure-CID / channel-head form from the first
    commit; S0 resolves it with a pinned-local-path resolver (no network); S1's first station
    resolves a channel head through the adoption path `runtime-upgrade-propagation` already
    proves (stations 1–8 on the household mesh). The envelope is delivered by the loop it applies:
    accelerate the cycle → new habit → dogfood/refine → master → new habit.
25. **`RuntimeManifest`, not `RuntimeSeed`; `Berth`, not `RuntimeInstance`** (operator, 2026-09-02
    thread). The declaration is a Manifest-kind EPR (`runtime-manifest`, sibling of `app-manifest`);
    "seed" is content seeding here. The berth is per blade. **A manifest may include a manifest**
    (guest recursion, §3.1).
26. **Hardware is cattle; the household is the pet** (§3.1). The ark is the blade's parent and dies
    with it; the passport is the berth's; the household footprint lives across berths under
    commitments and is at N−1, not gone, when a blade dies. *Corrects* this spec's earlier "the
    passport is the node's" reading.
27. **A berth offer is a REA intent, and slotting a blade is a negotiation** across every under-held
    standing commitment; reach bounds how far it travels; no scheduler. Missing node minted:
    blade-boot → **berth-offer** → commitments-rebalanced.
28. **Loss is witnessed by absence and healed by the same flow at every grain** — blade, household,
    neighbourhood, commons; the social-recovery quorum is the only step with no hardware analog;
    re-held, never restored.
29. **Container power, split**: closure = hermetic FS; isolation = floor, commitment + reach =
    ceiling; composability = manifest recursion. Named gap: content bundles are served, not
    sandboxed (Moss iframe + capability API); the `app-manifest` owns that contract when it comes.
30. **k8s is a lockfile-render of the manifest, declared not derived** (2026-09-05, operator-steered
    after the integration push was refused by a stale capacity envelope). `RuntimeManifest.envelope.bound`
    is the one declaration of a peer's compute envelope; `deployments.json`'s `edgenode*` fields stay
    in place for every consumer but become a verified render of a pinned manifest CID (the pin is the
    declared head at the k8s packaging; DHT anchoring stays S1 station 4); the cluster envelope is
    observed by a command, never hand-promoted; ratification is a typed record both validators read,
    never a push-time flag. Sequencing: declaration-only, compatible with quota at S3. **Guide-star
    check (register 21): passes** — the `k8s-bridge` crate renders resources only, `Berth.data_root`
    stays a reserved volume head with no semantics, and the epr-pvc arm lands in the same seam later.
    Spec: `genesis/docs/superpowers/specs/2026-09-05-k8s-bridge-runtime-envelope-render-design.md`.

## 13. What was rejected, per external model

k8s — the kubelet's *powers* are copied (eleven, enumerated in the grounding); its assumption of an
observer outside the container is not (the custody plane is our outside). OCI — the lifecycle and
`poststop` hook idea; not the runtime. Nix/Guix — derivation/instantiation, the declared closure
(assertion form, never NAR-scan discovery), generations with `supersedes`, Shepherd's respawn-limit-
as-state; not the store, the DSL, or "upgrade when it next stops". systemd — the verdict vocabulary
and the sd_notify wire protocol verbatim; `Delegate=` as the household's quota key; not children-as-
units, portable services, or oomd. s6/OTP — the parent rule, the death tally, the three-record
split, intensity with a reset; not a VM. Nomad — the client/driver split and `RecoverTask`; not the
central constraint solver or unenforced `raw_exec` quotas. balena — target/current state with
`applied`, offline-by-default, the update lock as a pause; not the single cloud author. Fly — the
`guest_exit_code` vs `exit_code` split, `requested_stop`, `restarting`; not the host-side control
plane. wasmCloud — links as capability grants, host-owned stderr, per-host ceilings; not OAM, NATS as
the control plane, or fungible instances. Holo — the counter-signed receipt and agreement-before-
events; not the matchmaker. Sandstorm — rootless containment is cheap; not identity by publisher key
or restart policy outside the supervisor. Urbit — record-before-effect at the supervisor's grain, the
per-epoch version stamp, the bounded snapshot; not the unbounded log. IPVM — the receipt shape
(`ran`, `cause`, `out`, `fx`, signature); not "work is a function call".

## 14. Risks, and the one thing that would make this wrong

- **Per-child cgroup leaves inside a k8s pod** depend on `cgroup.subtree_control` delegation the
  container does not have today (uid 1000, root-owned cgroup). Until the pod spec changes, the OOM
  class on alpha is witnessed only as a gap record. Honest, and named; not fixed here.
- **The `custody-spool` action** is coordinator-only (verified); the coordinator match must be
  extended and hot-swapped before S1 mints one.
- **The transport key as author** is admitted only at `private`; if the reach ontology resolution
  binds the household tier to human relationships rather than custody commitments, the custody-scoped
  read gate on the blob plane is the design's load-bearing addition and must land with S1.
- **The incident attestation is deferred** to the constitutional DNA change; until then "green" is
  the anchored witness row, and a custodian's receipt is a content row of its own, not an attestation.
- **iOS/watch** remain in-process only; the "same on a watch" claim holds for verbs over tasks, not
  processes, and is unproven until an `InProcess` driver exists (S4).
- **The one thing that would make this wrong:** if the parent that holds the conductor's pipes cannot
  also own its **data root and config** — `HOLOCHAIN_DATA_DIR` is read nowhere in the fork, the data
  root comes only from the YAML that `hc sandbox generate` writes, and the piped passphrase seals
  both lair and `db.key` — then the envelope supervises a process whose identity it does not
  control, and re-key, migration intent, and the passport's data-root facts are someone else's. S0
  exists to find out whether tevah can template the conductor YAML and own the passphrase source
  before anything is built on the assumption that it can.

## 15. Open questions for the operator — resolved 2026-09-02

1. Names: tevah for people, `ark` for code (register 20).
2. WIP fence: the slot moves from `operator-runtime-surface` to `runtime-death-witnessed`
   (register 22).
3. The constitutional DNA batch waits for S1's measured shape (register 23).
4. `data_root` may name a volume head; the `epr-pvc` bridge stays a floated guide-star, the end
   of this valueflow chain, unminted (register 21).
5. Delivery: working code first; the envelope is built and shipped through the update-propagation
   loop it applies (register 24).
6. Names refined in the 2026-09-02 clarification thread: `RuntimeManifest` / `Berth`; tiers,
   berth offer, loss recursion, and the container split captured in §3.1 (register 25–29).

## 16. Adversarial review disposition (2026-09-02)

Three independent reviews ran against the first draft — feasibility on this codebase
(rust-architect), device-spectrum and hostile-network (red-team), and the P2P design gate plus
concern canon. Every finding and its disposition; the body above already carries the accepted
changes.

**Accepted and folded (the design changed):**

- Feasibility 1 — the supervisor work is on dev (`264ce8ce4`, `dcf9a16c3`, `867e4bf9b`); the
  grounding briefs' "uncommitted" was stale → §1, §11, §15 corrected.
- Feasibility 2 — `hc-mesh.sh` already has a direct parent launch mode → S0 is an argv port; the
  §14 "one thing" is restated as data-root/config/passphrase ownership.
- Feasibility 3 + gate 6 + red-team 16 — the content-row path has no author-side reach gate or
  signature field; the transport key must not become `author_id` → detached proof in
  `metadata_json` with a declared key namespace and the `AgentPeerBinding` CID; `author_id` is the
  agent; `unbound` until the binding resolves; storage verifies before re-authoring; spool `0700`.
- Feasibility 4 — `DirectOnly` atom fanout is unimplemented → the offer rides the custody/blob plane
  via `reconcile_pass`, not atom fanout.
- Feasibility 5, 13 — `reanchor_backfill` walks the lamad app partition; the atom home dispatches on
  `contentFormat` → both named; S1 declares a `death-witness` format.
- Feasibility 6 + gate 1 — no household reach variant → the witness is born `private`; the green
  tier is `unresolved`, candidate `trusted`; nothing canonizes a tier.
- Feasibility 7 — `attestation:device-health` is a closed schema already ridden → the incident
  attestation is deferred to the constitutional batch; green = the anchored row.
- Feasibility 8 + gate 8 — `custody-spool` is coordinator-only → risk downgraded, coordinator match
  named as S1 work.
- Feasibility 9 — the conductor container sets no `command:` and holds its image → tevah ships inside
  the storage image with a deliberate conductor roll.
- Feasibility 10 + red-team 7 — `Conductor ready.` and `sd_notify(Ready)` are one instant; the
  conductor speaks no extensions → one rung; a declared cold-compile budget.
- Feasibility 11, 12 — `ResourceSnapshot` is request-level; `eprfs` is excluded from the workspace →
  a new `ProcessSample` type; crate placement wording corrected.
- Red-team 1 + gate 5 (C12) — a self-minted custody-spool is not custody → custodian counter-signature
  (`attestation:custodian-commitment`), monotone `incarnation`, gap check as a custodian duty.
- Red-team 2 — `PR_SET_PDEATHSIG` contradicts `recover()` → guest leaves only; own children get
  subreaper + exclusive data-root lock.
- Red-team 3 — `tokio::process` consumes the reap → Driver contract: `std::process::Command` +
  `waitid(WNOWAIT)` → `/proc` → `wait4`.
- Red-team 4 — ancestor OOM still group-kills the envelope → the Σ-children invariant, manifest refusal,
  and `unwitnessable` where the root's `oom.group` is not writable.
- Red-team 5, 20 — revert-by-re-election is not a safety brake when storage is the child being
  replaced → `RevertToPreviousClosure` with a pinned previous closure and a soak window; the
  envelope's own binary is pinned-only in v1.
- Red-team 6 — an unauthenticated shared notify socket forges readiness and immortal children →
  per-child socket, `SO_PASSCRED`, hard per-phase ceiling.
- Red-team 8 — the spool is an unbounded obligation → `bounds{max_bytes, atoms_per_hour, retention}`
  on the commitment; local ring with `pruned` stubs.
- Red-team 9 — the custodied payload is a reconnaissance package → redacted summary to custodians;
  raw stays local behind a per-incident grant.
- Red-team 10 — same-cause keyed on log text is evadable; readiness resets forever → cause hash of
  (class, exit code, resource envelope); a non-resettable rolling-hour ceiling.
- Red-team 11, 12, 13 — macOS/Windows unaddressed; Android has an OS death record; iOS/watch/browser
  are envelope-absent → the rung table gained honest rows.
- Red-team 14, 18 — per-uid rlimits make `Bounded` unsafe for guests; user delegation is
  memory+pids → guests fail closed below `Enforced`; wording fixed.
- Red-team 15 (part) + gate 2 — pre-readiness deaths have no self-contract; re-key orphans it →
  `BoundedBy::SeedPolicy` pre-readiness; re-mint and revoke on re-key; receiver-named rows inert until
  counter-signed; `(node, child, incarnation, seq)` keys.
- Red-team 17 — the termination message is harvested only at container exit → S2's receipt restated.
- Red-team 19 — the admin socket needs access control → `0600`, peer-uid check, every mutating verb
  commitment-bounded (added to §8's binary).
- Gate 3 — B2 was a misclassification → Notarized (A) at incident grain; composite root; passport
  atom per applied transition.
- Gate 4 — three address forms → `Content.id` is the atom CID.
- Gate 5 — C0–C14 cannot be `answered` with no crate and no contract tests → every class is
  **`partial`** until `ark-core`'s `seam-registry.yaml` cites its tests (an S0 obligation); C1
  enumerates the self-grant; C7 writes the row before the blob.
- Gate 7 — `validate_economic_event` never checks `REA_ACTIONS`; `use` over `compute-fulfilled` is
  hygiene, not integrity → wording.
- Gate 9 — a `_dna` `ResourceSpecification` enum would move the hash → declared app-manifest
  vocabulary.
- Gate 10, 12 — the period was undefined and NULL-anchor rows are the amber-forever shape → daily
  composites, a node-month digest, a separate `compute_samples` table.
- Gate 11 — two truths of one artifact identity → the `bafkrei…` CID is the identity; sha256 is only
  the spawn-time byte check.

**Weighed and held (the design did not change):**

- Red-team 15 (the rest) — `DNA_MIGRATION_INTENT` as an env string is "a standing flag in a costume".
  Partly true: it is per-roll and names hashes, which is what distinguishes it from
  `ALLOW_DNA_REINSTALL`; making it a signed atom verified against observed hashes is the right end
  state and is recorded as S1+ work, but the env form is the landed floor and this spec does not
  re-open the crash-loop atom's decision 2.
- The self-contract "is an audit scope, not a bound". Correct, and intended: a node cannot be bound by
  itself; what the self-contract buys is that the node's own verdicts are the same record class as an
  operator's, auditable by a custodian. Acts touching another agent are bounded by that agent's
  commitment (§5.3), which is where the bound lives.

**Still open after review** (carried into §15 and the S0/S1 receipts): whether the envelope can own
the conductor YAML and passphrase source (S0 finds out); the custody-scoped read gate on the blob
plane (S1); the coordinator match for `custody-spool` (S1); contract tests for every concern class
(S0/S1); the constitutional batch for first-class types and attestation kinds.
