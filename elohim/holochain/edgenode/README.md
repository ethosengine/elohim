# Elohim Edge Node Configuration and Persistence Reference

Elohim lets people hold and serve identity and shared learning content through
peer-owned infrastructure. In this repository, one **edge node** is one human's
`elohim-storage` service paired with a Holochain conductor. The conductor holds that
human's distributed-hash-table (DHT) identity and executes their Holochain applications
(hApps); storage keeps and projects content. A doorway is the shared web gateway in
front of one or more edge nodes, not part of a peer's identity.

This README serves maintainers who already have an authorized, provisioned Elohim
Eclipse Che workspace and are changing conductor configuration or preserving an
existing household mesh. It is not a workspace or dependency installation tutorial.
For first-time setup, binary preparation, and the full command catalogue, use the
[hc-dev-orchestrator guide](../../../.agents/skills/hc-dev-orchestrator/SKILL.md) and
the [root developer guide](../../../AGENTS.md#build--test-commands). Return here for
configuration authority, status interpretation, and identity-safe persistence.

Inline paths and commands below are relative to the repository root,
`/projects/elohim`; Markdown links resolve from this README. This README is at
`elohim/holochain/edgenode/README.md`.

## Runtime boundary

```text
Browser ──portal/sign-in──> doorway ──HTTP──> elohim-storage
                         │                    └──App WebSocket──> Holochain conductor
                         └── bootstrap endpoint                            ├── DHT via iroh relay
Admin client ───────────────────────────────Admin WebSocket────────────────┤
                                                                           └── local data + Lair keystore
```

The portal is the browser sign-in surface served through the doorway. The doorway
projects the peer-native system to web clients. `elohim-storage` stores content bytes
and calls the conductor's application interface. The conductor executes zome calls
(named application functions compiled into the installed Holochain app),
participates in the DHT, and keeps agent keys in its in-process Lair keystore. Bootstrap
discovers peers. An operator's iroh relay keeps them reachable when direct connections
fail.

The repository compatibility contract is Holochain 0.7.0: the DNA workspaces—the
source trees that compile the app's integrity rules and callable zomes—pin
`hdk = "=0.7.0"`, and the mesh refuses a conductor from a different Holochain line or
a mismatched `holochain`/`hc` pair. This is a source-tree invariant, not an observation
of the running fleet. The supported fleet conductor source is the commit pinned by the
`elohim/holochain-conductor` Git submodule entry; inspect it with:

```bash
git rev-parse HEAD:elohim/holochain-conductor
```

Other fixed boundaries are:

- iroh is the Holochain transport. Bootstrap and relay URLs must come from the same
  operator's declared topology; there is no public-relay fallback.
- The local household smoke homes its three conductors to one local relay. The fleet's
  supported fork also has a separately tested cross-relay path, so this local shape is
  not a universal Holochain constraint.
- The browser target is `@holochain/client ^0.21.0`.
- The canonical local Dowell fixture has three members: Matthew, Jessica, and James.
  Matthew's conductor uses admin/app ports `4444`/`4445`, Jessica uses
  `4454`/`4455`, and James uses `4464`/`4465`. Every assigned admin WebSocket must remain
  unavailable to untrusted networks. App WebSockets carry authenticated zome calls.
- Fleet conductor state lives under `/var/local/lib/holochain`, including the Lair
  keystore under `ks/`. Back up or migrate that directory as one identity-bearing unit.

## Configuration authority

Do not infer authority from proximity to this README:

| Change | Authoritative source | Validation and handoff |
|---|---|---|
| Local household topology, endpoints, or persisted-state behavior | `app/elohim-app/scripts/hc-mesh.sh` | `just mesh preflight`, then the provisioned-mesh checks below |
| Fleet bootstrap and relay assignment | `environmentConfig` in `elohim/holochain/Jenkinsfile` | The edge pipeline renders every conductor manifest and refuses a missing relay |
| Shared fleet conductor interfaces, persistence, or gossip shape | `genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml` | The pipeline rejects unresolved placeholders and runs `scripts/ci/validate-conductor-config.sh` on every rendered manifest |
| Adam's explicit conductor shape | `genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml` | Adam is the remote genesis/bootstrap human. His explicit manifest preserves measured resource and gossip differences; evaluate it alongside the shared template whenever an interface, persistence, or security invariant changes |
| Conductor binary or fork revision | `elohim/holochain-conductor` submodule pin | Follow the [conductor image procedure](../../conductor-image/README.md) |

`elohim/holochain/edgenode/conductor-config.yaml` is a complete standalone example,
not a deployment source. The local mesh generates its own configuration. Fleet
configuration is embedded in the shared template above and rendered by
`elohim/holochain/Jenkinsfile`. The adjacent `Dockerfile`,
`Dockerfile.zombie-fix`, and `docker-compose.yml` describe an obsolete image generation
and are not a supported Holochain 0.7 path.

The standalone file's `advanced.k2Gossip` values show the household slow-link profile:
longer rounds exchanging DHT operations with peers and fewer accepted rounds reduce
pressure on constrained links at the cost of slower propagation. Per-human fleet
overrides live in `genesis/orchestrator/data/deployments.json`. Do not change these
values without propagation and quiescence measurements from the affected topology.
Here quiescence means the bounded probes report no actionable divergence or pending
reconciliation, rather than merely showing that every process is running.

A local mesh or scenario does not prove a fleet rollout. The fleet evidence chain is:
the full conductor-submodule SHA determines the `conductor-<first-12-SHA>` image pin;
the edge build reports that same pin, the intended monorepo revision, and successful
rendered-config validation; then every unsuspended target selected from
`genesis/orchestrator/data/deployments.json` reports completed storage and conductor
StatefulSet rollouts, with the observed conductor image matching the source-derived pin
and either differing from the prior pin or being that human's first rollout. The
conductor image procedure defines the exact log signals and rejection cases.

### Applying a local configuration change

First classify the setting. Resume starts storage, doorway, relay, and portal
processes from the current launcher inputs, but it reuses each existing
`<resolved-mesh-dir>/conductors/<peer>/conductor-config.yaml`. `conductors-restart` also
leaves those files, keys, chains, DHT databases, and caches untouched; it is suitable
for a process or logging change, not a generation-time configuration change. Changes
to sandbox-generation inputs such as `mesh_network_args` or
`patch_mesh_gossip_config` therefore do not reach an existing household automatically.
Such a source change needs its own reviewed, guarded in-place migration before restart.
If none exists, leave the identity-bearing household unchanged rather than using reset
as a configuration-application mechanism.

| Setting kind | Representative inputs | When it is read | Supported application path |
|---|---|---|---|
| Process-launch input | `MESH_TRANSPORT_BACKEND`, doorway ports, relay URL, logging environment | Each process launch or restart | Stop, preflight, start, wait, then verify the value through status or its focused probe |
| Generated conductor input | `mesh_network_args`, `patch_mesh_gossip_config`, sandbox generation flags | Only when `hc sandbox generate` writes `conductor-config.yaml` | Use an existing reviewed in-place migration; otherwise keep the household unchanged |
| Persisted identity or archive state | conductor keystore/database, storage identity keys, Mongo archive posture | Reused on every resume | Restore the coherent fixture backup; never edit or regenerate one part to apply configuration |

For an unfamiliar input, trace its use in `hc-mesh.sh`: a value passed to a process in
`start_*` or `restart_*` is launch-read; a value used only in the cold-generation block
or written into `conductor-config.yaml` is generation-time. If neither route is clear,
do not restart or reset until its owning code and focused probe identify the boundary.

For a launcher input that is read on restart, an ordinary restart preserves identity
when the household is complete. Record `just mesh status`, stop it with
`just mesh stop`, make the source or environment change, then run
`just mesh preflight`, `just mesh start`, `just mesh wait`, and `just mesh status`.
A successful resume records in `<resolved-mesh-dir>/logs/start.log`:
`resuming 3 conductors from complete persisted household state` and
`restarting 3 conductors from EXISTING sandboxes (no generate, keys kept)`. The final
status must show all readiness legs green. Where `status` exposes the changed value,
require the intended value there; otherwise require the setting's focused probe or log
evidence before calling it applied.

## Checking a provisioned household mesh

Run these commands from the repository root. They assume the orchestrator guide's
binary and dependency preparation is already complete and do not install or build
anything. Select an alternate household root before any inspection or lifecycle command,
then keep it exported through `preflight`, `start`, `wait`, `status`, `prologue`, tests,
and `stop`:

```bash
export MESH_DIR=/absolute/path/to/the/household
```

For the implicit default, leave `MESH_DIR` unset: that is what permits a
stopped legacy `/tmp/elohim-local-mesh` or interim `genesis/local-dev/household` to
migrate to `genesis/local-dev/household-dowell` when the destination is absent.
Explicitly exporting even that default path suppresses migration.

The full doorway sequence also requires an executable MongoDB and, for an existing
household, an `archive` posture with its MongoDB data intact. Resolve the state root
without changing it before starting:

```bash
if [ -n "${MESH_DIR:-}" ]; then
  printf 'explicit: %s -> %s\n' "$MESH_DIR" "$(readlink -m "$MESH_DIR")"
else
  for path in "$PWD/genesis/local-dev/household-dowell" \
              "$PWD/genesis/local-dev/household" /tmp/elohim-local-mesh; do
    if [ -e "$path" ] || [ -L "$path" ]; then
      printf 'candidate: %s -> %s\n' "$path" "$(readlink -m "$path")"
    fi
  done
fi
```

An explicit override is the root to inspect. With no override, one existing real
target is unambiguous; inspect its `doorway-archive-mode`. If it is a legacy/interim
root and the persistent destination is absent, keep `MESH_DIR` unset so start can
migrate it. Symlinks resolving to that same target are aliases. More than one distinct
real target is ambiguous: do not start or choose one by deletion; preserve them and
reconcile their lineage first. No candidate means fresh state, for which startup
selects archive posture from the provisioned MongoDB executable. The persistence
section below explains the archive-less boundary.

```bash
just mesh preflight
just mesh start
just mesh wait
just mesh status
```

`preflight` checks binaries, transport capability, toolchain/DNA compatibility,
persisted-state coherence, and ports before launch. `start` launches the owned
processes in a detached session. `wait` succeeds with `ready in <seconds>s` only after
the default relay, both doorways, portal, three conductor admin listeners, and three
storage health endpoints answer. That proves the configured local processes and health
surfaces are reachable; it does not prove seeded fixture data, a scenario, or fleet
artifact identity.

This local fixture has two gateways over the same household peers. Doorway A is owner
`alpha` at `http://localhost:8888`, primarily backed by Matthew; doorway B is owner
`apex` at `http://localhost:8889`, primarily backed by Jessica. Both origins earn or
lose eligibility independently in both public-name membership documents. For example,
`membership/elohim.local.json` lists the doorway origins currently allowed to answer
for `elohim.local`; it never selects application bytes. The separate
channel declaration, `membership/authority.json`, assigns that name's `converged`
channel. On that channel,
each doorway resolves the declared head—the notarized identity of the current
`elohim-host-landing` application version—and serving doorways must agree on it.

For the default Dowell fixture, `status` must show:

- conductor listeners for `matthew` (`4444`/`4445`), `jessica` (`4454`/`4455`), and
  `james` (`4464`/`4465`);
- `storage=UP` for those peers on `8090`, `8091`, and `8092`;
- `doorway :8888 UP`, `doorwayB :8889 UP`,
  `portal :8081 UP (/threshold/login serves through the doorway)`, and
  `relay :3340 UP`;
- with the default `MESH_MEMBERSHIP=1`, both `leg alpha` and `leg apex` reported `up`.
  These beacon processes watch doorway A and doorway B respectively and add or remove
  that doorway's origin from the eligible set for `elohim.local` and
  `alpha.elohim.local`. The default membership documents are
  `genesis/local-dev/household-dowell/membership/elohim.local.json` and
  `genesis/local-dev/household-dowell/membership/alpha.elohim.local.json`. Each JSON
  `members` array must contain both `alpha` at `http://localhost:8888` and `apex` at
  `http://localhost:8889`; `status` renders those as
  `eligible: alpha=http://localhost:8888, apex=http://localhost:8889` (order is not
  significant). With the default three-second probe interval, a healthy origin joins
  after two successful probes (about six seconds). `MESH_MEMBERSHIP_NAME`,
  `MESH_MEMBERSHIP_CANDIDATE_NAME`, and doorway port overrides change these expected
  filenames or origins. If status says
  `membership disabled (MESH_MEMBERSHIP=0 — no household public-name authority)`, this
  readiness leg is deliberately unavailable rather than passed; and
- `mongod :27017 UP (archive-backed doorways)` for the full doorway, portal, prologue,
  and hosted-scenario sequence below. The status line
  `mongod :27017 down (doorways run archive-less: inert warm shell)` is only the
  launcher's label for a closed MongoDB port. It does not read the posture marker and
  does not prove that either doorway process is running. It does not pass this full
  readiness check.

`conductor RUNNING` names the executable and version actually observed. A down
conductor, storage peer, doorway, portal, relay, or enabled membership leg; a missing
or incomplete membership document after the two-probe join interval; or a nonzero
`wait` means this check has not passed.

A refusal is a safety result, not an invitation to delete files. For example, an
incomplete household prints its missing paths and:

```text
REFUSED mesh start: household state is incomplete (conductor=complete storage=partial archive=complete)
  restore the missing state, or deliberately recast the stopped household with MESH_RESET=1 just mesh start
```

Restore the named state when it should survive. The coherent local backup and restore
unit is the resolved mesh state directory: it holds `conductors/` with the `.hc` roster,
each peer's conductor database and Lair keystore, alongside storage identities, content,
archive posture, account archive, membership, and staged fixture state. Preserve and
restore that directory as one lineage while all owned processes are stopped, then run
`just mesh preflight`. The repository has no automatic backup/restore verb; a
partial or mixed-generation restore is expected to refuse. Use the guarded reset only
for a deliberate full recast, as described next.

## Persistent Dowell fixture

The default mesh is an execution of the canonical Dowell household fixture, not a
second household definition. It resolves Matthew, Jessica, and James against
`genesis/data/humans/humans.json`, the checked-in projection generated from
`genesis/data/humans/*.md`, and requires the exact `household-dowell` member set before
migration or launch. The `holochain-seeder` package owns that projection; mesh launch
only reads it and refuses a missing, ambiguous, or mismatched binding. After a human
source change, refresh it from the repository root with
`pnpm --filter holochain-seeder run build:data`. The pre-push human/presence gate
validates the sources, regenerates the artifact, and refuses an unstaged difference.
Inspect the generated diff and commit `humans.json` alongside its Markdown source
change. Resolve that source/projection difference before retrying a binding refusal;
resetting runtime state cannot repair it.

The current binding is Matthew → `human-matthew-manager`, Jessica →
`human-jessica-spouse`, and James → `human-james-son`; all three rows must name
`household-dowell`, and no additional row may name that household.

The fixture's gitignored runtime state lives at
`genesis/local-dev/household-dowell`; a nonempty `MESH_DIR` selects an explicit
alternative.

The launcher classifies state before writing:

| State | Meaning and safe action |
|---|---|
| Fresh | No conductor or storage state exists. A normal start may create the canonical local instance. |
| Complete and stopped | The conductor roster and each peer's config, keystore, conductor database, content database, and storage identity exist. A normal start resumes them. Dual/iroh storage peers must also retain `iroh.key`; a recorded libp2p-only peer need not. |
| Archive-backed | The recorded doorway posture requires both MongoDB data and the MongoDB runtime. Restore either missing part before resuming. |
| Intentionally archive-less | The recorded posture remains archive-less; start does not silently create a new account archive. |
| Partial or ambiguous | Start lists the missing state and refuses. Restore it or choose the stopped-only full recast. Absence never implies permission to mint a replacement identity. |

The authoritative posture record is `<resolved-mesh-dir>/doorway-archive-mode`. Read it
without exporting an implicit default:

```bash
inspect_root=/absolute/path/identified/by/the/read-only-resolution-above
cat "$inspect_root/doorway-archive-mode"
```

Its entire value is `archive` or `archive-less`. A missing file or any other value is
unknown; do not infer the intended posture from whether MongoDB happens to be running.
For an existing household, keep it stopped and restore the marker from the same coherent
fixture backup; if no such evidence exists, preserve the state for investigation rather
than choosing a posture. Fresh state is different: no identity, storage, or archive state
exists yet, and the first successful start writes the marker selected from the available
MongoDB runtime.

Archive-less means conductor and storage state can remain coherent without MongoDB,
and the supported launch shape is `MESH_DOORWAYS=0`: conductors and storage run, while
no doorway or membership-beacon process is started. It cannot complete the doorway
path documented here. With doorways enabled, the launcher declares MongoDB to each
doorway, and the doorway refuses to start when that declared database is absent rather
than downgrading authentication. Portal sign-in, hosted-human registration, the full
prologue, and the example hosted scenario therefore require an archive-backed mesh.

There is no supported in-place archive-less → archive-backed transition. On resume the
launcher honors the recorded `archive-less` value even if a MongoDB executable later
appears; changing the marker or creating an empty MongoDB directory would invent an
account lineage unrelated to the existing conductor cells. If those identities must
survive, stop and preserve the coherent resolved fixture directory and do not convert it.
To obtain an archive-backed recast of the reusable test household, make the provisioned `mongod`
executable discoverable as described by the orchestrator guide, then use the full
recast below with its identity and archive loss accepted. Fresh/reset startup selects
`archive` only when that executable is present. An alternate `MESH_DIR` contains its own
`conductors/` state as well as its storage and archive state.

`MESH_RESET=1 just mesh start` is the stopped-only full recast and does not pause for
confirmation. Inspect the resolved target separately before arming it:

```bash
reset_target="${MESH_DIR:-$PWD/genesis/local-dev/household-dowell}"
printf 'reset mesh state: %s\n' "$reset_target"
printf 'reset conductor state: %s\n' "$reset_target/conductors"
MESH_DIR="$reset_target" just mesh status
cat "$reset_target/doorway-archive-mode"
```

For an intentionally selected legacy root, set `reset_target` to that root's absolute
path. Passing it per command below makes the reset target exact and suppresses default
migration; it does not export `MESH_DIR` into later shell commands.

Once admitted, reset replaces that local fixture's conductor sandboxes and keys,
storage and iroh identities, doorway accounts and archive, membership, and staged
fixture state. It mints new local identities and discards the previous local archive;
it does not touch deployed conductor state. Only after checking the printed path and
deciding that all of those local records may be replaced, stop the owned processes and
invoke the reset directly:

```bash
MESH_DIR="$reset_target" just mesh stop
MESH_DIR="$reset_target" MESH_RESET=1 just mesh start
```

The reset refuses while an owned process or mesh port remains live.

A single stopped prior root—either `/tmp/elohim-local-mesh` or the interim
`genesis/local-dev/household`—moves to the persistent default only when the destination
is absent. Each absent prior path then becomes a compatibility symlink for captured
absolute paths. A complete, stopped Dowell conductor sandbox at the historical shared
`elohim/holochain/local-dev` root moves peer-by-peer into `conductors/`; unrelated
single-peer sandboxes stay in place, and old per-peer paths become compatibility links.
Partial or competing conductor state refuses without overwrite. Explicit `MESH_DIR`
values suppress these default migrations.

## Preparing and remeasuring scenarios

The **prologue** is the write-bearing preparation phase for Act I, the repository's
first household scenario lane. It requires an already-running mesh plus the built
landing browser/server and Lamad learning-application browser distributions; the
orchestrator guide owns those preparation commands.

For an alternate household directory, retain the `MESH_DIR` selected before startup;
every preparation, test, and shutdown command must continue to resolve that same root.

Run it once before the first focused scenario:

```bash
just mesh prologue
```

It seeds the base content and household cast through both doorways, stages the landing
and Lamad bundles, and declares each bundle's current published version (its head).
It writes the scenario fixture manifest to
`<resolved-mesh-dir>/household-fixture.json`, the hosted-human roster to
`<resolved-mesh-dir>/prologue-hosted-humans.json`, membership documents to
`<resolved-mesh-dir>/membership/`, and process logs to `<resolved-mesh-dir>/logs/`.
With no override, the resolved directory is
`genesis/local-dev/household-dowell`. When a built app lacks
`version.json`, it stamps the existing distribution but never rebuilds it. These writes
persist in conductor, storage, and the required doorway archive. Treat every invocation
as a real staging attempt and require the fresh terminal result:

```text
PROLOGUE: complete — all MUST-SUCCEED legs green
```

That marker establishes the prologue's hard preparation legs. Soft seed failures remain
visible earlier in its output and can still deprive a dependent scenario of its fixture.
For this example, the hard `seed-base-corpus-via-A/B` legs supply the landing and
manifesto content; also require the soft `propagate-landing-to-B-after-browser`,
`propagate-landing-to-B-after-server`, and `stamp-server-projection-peers` legs to
succeed before testing cross-doorway head agreement. Each success is the exact line
`EXIT[<leg-name>]=0`; any nonzero value leaves this preparation incomplete even when the
terminal hard-leg marker is green. The scenario's own Cucumber
result remains the terminal proof.

After that one successful prologue, repeated focused scenarios may reuse the same
running household and persisted roster. `just test mesh` refuses and requests a new
prologue when the roster is absent or older than a recreated doorway archive. A full
recast or restored/recreated archive requires a new prologue; an ordinary stop and
complete-state resume preserve the preparation.

For example:

```bash
just test mesh features/dataplane/doorway-failover.feature
```

In ordinary terms, this feature checks that the shared site and replicated public
content remain reachable through the tested doorway pair when one storage peer stalls,
without two serving doorways disagreeing about the current landing page. Specifically,
it checks the current pair has at least one serving doorway, serving siblings agree on
the declared landing-page head, and a replicated manifesto blob still arrives through
`alpha-A`—the scenario name for doorway A at `http://localhost:8888`, backed primarily
by Matthew's storage at `http://localhost:8090`—when that storage peer is deliberately
stalled and restored. It does not induce a doorway outage or prove that the public apex
name changes doorways; that transition belongs to
`features/dataplane/doorway-apex-transition.feature`.

The command establishes that result only when it exits 0 and Cucumber reports every
selected scenario passed. A skipped, pending, undefined, or failed scenario does not
establish the behavior.

Stop only after the scenarios finish:

```bash
just mesh stop
```

A normal stop ends the owned processes while preserving household identities,
databases, archive posture, and staged fixture for a later complete-state resume.
