# Cargo Target Pool — Family-Shared Targets with Roster Presence

**Status:** Design v2 — 2026-05-10
**Author:** Opus (post-disk-fill incident this morning; reframed after user
feedback that exclusive-lease + independent-worktree-cleanup was the wrong shape)
**Scope:** Eclipse Che workspace; the elohim monorepo
**Out-of-scope:** sccache substrate, Garage, Cargo workspace topology, /shift parallelism, replacing `git worktree`

---

## What changed from v1

v1 framed this as "exclusive leases on N anonymous slots, sweep stale
leases by pid liveness, treat worktree cleanup as a sibling problem."
The user's correction:

> Agents need to coordinate which `target/` directories they're going
> to share, and mark something to say "hey I'm still using this." If
> the agent gets interrupted mid-task, and the lock is still there but
> the agent is stopped, an agent wanting to clean up that directory
> must review the worktree merge state or clean it up and update the
> other agent's abandoned lock. `target/`s are shared for worktrees on
> a feature family.

The reframing in three lines:

1. **The unit of sharing is a feature family**, not an anonymous slot.
   `iroh-pkarr`, `iroh-phase12-manifest`, `iroh-recovery-e2e`, etc., all
   share the iroh family's target directory because they're working on
   related code with similar feature flags and source overlap.
2. **The "lock" is a presence marker (a roster), not exclusion.**
   Multiple agents on the same family co-tenant. Cargo's own intra-target
   advisory lock (`.cargo-lock` inside `target/`) handles per-rustc
   serialization at the right granularity. The pool roster is "who is
   currently active in this family," for stewardship visibility.
3. **Cleanup is a stewardship action with worktree-state inspection,
   not a generic sweep.** When an agent finds a stale roster entry, it
   has to investigate the corresponding worktree — merged? abandoned?
   alive-but-crashed? — and decide whether to remove the worktree,
   prune the entry, or leave the worktree alone.

The rest of v1's reasoning (WASM exemption, sccache orthogonality,
wrapper-script integration point, devfile env wiring) carries forward
mostly unchanged.

---

## Why this exists

Six abandoned worktrees this morning, each carrying its own `target/`,
totaled ~90GB and crashed the workspace at 100% disk. The pattern recurs
because:

- Per-worktree `target/` is the Cargo default; a fully-built worktree
  costs ~18GB of disk (storage 5.8G + doorway 7.7G + sweettest 4.7G,
  measured at repo root today). Native crates dominate. WASM target
  is small and lives separately.
- Wave-1 of the agentic shift dispatched 2 parallel worktrees with new
  target dirs each. Plans go to 3-4 wide.
- `git worktree remove` after a merge is not happening reliably. Old
  worktrees rot, holding their `target/` hostage.
- **sccache is live** (Garage S3 bucket `sccache-elohim`,
  `RUSTC_WRAPPER=sccache` confirmed in PID 1 env). It caches *compile
  outputs* across invocations, but does **not** shrink `target/` —
  Cargo still materializes incremental artifacts, dep-info, and final
  binaries locally. sccache and target-dir sharing are **orthogonal**;
  both are wanted.
- A naive global `CARGO_TARGET_DIR` shared across all worktrees fails
  three ways: (1) Cargo's per-target lock serializes builders that
  shouldn't serialize (different families); (2) different RUSTFLAGS or
  feature flags between worktrees corrupt incremental state; (3) any
  worktree running `cargo clean` wipes everyone's cache.
- A per-feature-family target dir solves all three: (1) within-family
  serialization is *desired* (related work benefits from short builds),
  (2) within-family RUSTFLAGS/features are similar by construction,
  (3) `cargo clean` is scoped to the family that consciously asked for
  it.

## Lessons absorbed before designing

Two prior incidents that constrain the design:

1. **DNA build path-fragility** (shift `2026-05-09T16-30-orchestrator-clean-cascade`).
   Setting `CARGO_TARGET_DIR=/cargo-target` in the DNA Jenkinsfile broke
   `hc dna pack` because it `canonicalize`s
   `./target/wasm32-unknown-unknown/...` and post-build steps fail when
   the artifact lives elsewhere. **Decision:** This pool **does not
   manage WASM (DNA) target dirs.** Those stay at `./target` per
   workspace. The pool only manages native builds (elohim-storage,
   doorway, steward/node, sweettest).

2. **Sweettest already separates target dirs.**
   `elohim/holochain/tests/sweettest` builds with
   `CARGO_TARGET_DIR=target/native-tests` precisely because its native
   build conflicts with the parent workspace's WASM build layout. The
   "redirect native target while leaving WASM target alone" pattern is
   precedent.

---

## Mental model: the family pool

A feature **family** is a set of related branches working on the same
problem area. Today: `iroh-*`, `epr-phase-*`, `agent-<hash>`. Each
family has one target directory, located at:

```
$CARGO_TARGET_POOL_ROOT/family/<family-name>/<workspace-rel-path>/<profile>/
```

where:

- `<family-name>` is derived from branch name (rules in §Decisions/Family
  derivation), or explicit via `.family` file or env var.
- `<workspace-rel-path>` is the path of the Cargo workspace inside the
  worktree, with `/` replaced by `__` for filesystem flatness (e.g.,
  `elohim__elohim-storage`, `doorway__doorway-service`).
- `<profile>` is `dev`, `release`, or whatever `--profile X` sets.

Multiple worktrees on the same family + same workspace + same profile
**share the same physical directory** as their `CARGO_TARGET_DIR`.

A **roster** at `family/<family>/.roster.json` records every agent
currently active in that family. Roster entries are presence markers,
not exclusion locks. Cargo's own `.cargo-lock` inside the target
directory handles intra-rustc serialization where it must.

**Stewardship cleanup** runs on entry. Before adding itself to the
roster, an arriving agent inspects existing entries; for each entry
that fails liveness, it investigates the entry's worktree state and
takes one of: prune-roster, remove-worktree, or leave-alone. This is
the only place worktree-merge inspection lives.

---

## Decisions

### 1. Slot key and family derivation

**Slot directory key:** `(family, workspace_rel_path, profile)`.

**Why this granularity.** Family is the axis along which feature flags
and RUSTFLAGS are similar by construction. `workspace_rel_path` is
needed because different crates within the same family (storage vs
doorway vs sweettest) have different Cargo manifests and would
otherwise stomp each other. Profile splits debug from release.

**Why drop the absolute path that v1 used.** Sharing across worktrees
on the same family is the whole point. dep-info collisions are
absorbed by sccache (rebuilds become free at the rustc-output layer)
and by Cargo's incremental-state regeneration. Within a family,
source-tree similarity is high → most rebuilds are no-ops or near-
no-ops.

**Family derivation rules** (first match wins):

1. `CARGO_TARGET_POOL_FAMILY` env var, if set non-empty.
2. `.family` file at the worktree root, contents trimmed.
3. Branch-name pattern: take the substring before the first `-` or `/`,
   lowercased.
   - `iroh-pkarr` → `iroh`
   - `iroh-phase12-manifest` → `iroh`
   - `feat/iroh-pkarr` → `iroh` (after stripping `feat/`)
   - `epr-phase-2b-batch-c` → `epr`
   - `agent-a25c4b8b69e4560d4` → `agent`
   - `dev` → `dev` (literal; the main branch builds in its own family)
   - `worktree-iroh-parallel-stack` → `iroh` (after stripping
     `worktree-`)
4. If none of the above yield a non-empty family, fall back to the
   branch name itself sanitized to `[a-z0-9-]`.

**Family override at acquire time.** A wrapper invocation can pass
`--family X` (consumed by `cargo-leased` before forwarding to cargo).
Used when the heuristic gets it wrong — e.g., a temporary spike that
should NOT pollute a real family's cache.

**Why prefix-split, not the full branch name.** The whole point of
family sharing is iroh-pkarr and iroh-phase12-manifest reach the same
slot. Branch-as-family would defeat that.

### 2. Roster, not exclusive lease

**Decision:** The roster is a multi-tenant registry. Many agents may
be present in the same slot simultaneously. Cargo's intra-target
advisory lock handles per-rustc serialization where two agents
genuinely race (and that serialization is *desired* — A's build
warms B's fingerprints).

Roster file at `family/<family>/.roster.json`:

```json
{
  "schema_version": 1,
  "family": "iroh",
  "entries": [
    {
      "agent_id": "shift-2026-05-10T14-30-iroh-phase12",
      "pid": 4711,
      "start_time_ns": 17829341192,
      "boot_id": "f4c8...e91",
      "hostname": "elohim-devspace-pod-abcd",
      "worktree_path": "/projects/elohim/.claude/worktrees/iroh-phase12-manifest",
      "branch": "iroh-phase12-manifest",
      "joined_at": "2026-05-10T14:32:01Z",
      "ttl_seconds": 14400,
      "command": "cargo build --release",
      "workspace_rel_path": "elohim/elohim-storage",
      "profile": "release",
      "last_heartbeat": "2026-05-10T14:32:01Z"
    },
    { ... }
  ]
}
```

**Roster mutations are atomic.** Read-modify-write under
`flock family/<family>/.roster.lock`. The lock is on the roster file
**only**; it does not block cargo invocations within the slot.
Critical section is small (parse JSON, edit array, write back).

**Liveness criteria** (entry is live if all hold):

- `boot_id` matches the workspace's current boot_id, AND
- `kill -0 <pid>` succeeds, AND
- `/proc/<pid>/stat` field 22 (`starttime`) matches `start_time_ns`,
  AND
- `now() - max(joined_at, last_heartbeat) < ttl_seconds`.

If `/proc` unavailable (non-Linux), drop the start-time check; rely
on TTL + kill-0. (We're in Linux; `/proc` is always there.)

**Heartbeat.** Optional in T1, recommended for any build expected to
exceed `ttl_seconds / 2`. The wrapper script can fork a heartbeat
loop that updates `last_heartbeat` every 5 minutes for the duration
of the cargo invocation. T1 ships without; T2 adds it once we observe
TTL-induced false-stale.

**TTL.** 4 hours default (`CARGO_TARGET_POOL_TTL_SECONDS=14400`).

### 3. Pool sizing — by family, with caps

**Decision:** Slot count is dynamic by family; capped by total active
families and total disk.

- `CARGO_TARGET_POOL_MAX_FAMILIES` (default 6): max distinct families
  that may have allocated slots simultaneously. Beyond cap, acquire
  selects an LRU family for **eviction** (full prune of that family's
  slot tree, after stewardship cleanup confirms no live agents).
- `CARGO_TARGET_POOL_MAX_DISK_GB` (default 80): soft cap on total
  pool footprint; if exceeded, eviction is forced regardless of
  family count.
- `CARGO_TARGET_POOL_ACQUIRE_TIMEOUT` (default 60s): max wait for
  flock during roster mutation. Beyond timeout: fall back per
  `CARGO_TARGET_POOL_FALLBACK`.

Why these specific numbers:

- 6 families: today there's effectively 1 active (iroh) and a few
  trailing (epr, agent-*). Headroom for a few more without thrash.
- 80GB: leaves 40GB headroom in the 120Gi `/projects` PVC for
  source, sccache local fallback dir, etc.
- LRU on family-touch (any acquire bumps `last_touched_at` in a
  small `family/<family>/.meta.json`).

**Within a family, the slot itself doesn't have a size cap.** It's
shared across all worktrees + workspaces + profiles in the family.
Total family disk = sum of (workspace_rel_path × profile) subdirs.

### 4. Hook integration point

**Decision:** Wrapper script `cargo-leased` is the integration point.
Same as v1 — this part didn't change.

`cargo-leased <cargo-args...>` behavior (revised for v2):

1. If `CARGO_TARGET_POOL_DISABLE=1`, `exec command cargo "$@"`.
2. If pre-existing `CARGO_TARGET_DIR` is set in env, honor it; treat
   as opt-out, `exec command cargo "$@"`.
3. WASM-target detection: if the nearest `Cargo.toml` declares
   `crate-type = ["cdylib"]` or there's a `dna.yaml` / `happ.yaml`
   adjacent in the workspace, **short-circuit to plain cargo** (DNA
   build escape hatch — protects against repeating yesterday's
   `hc dna pack` regression).
4. Detect workspace root, profile, family. Compute slot dir path:
   `$POOL_ROOT/family/<family>/<workspace_rel_path>/<profile>`.
5. Acquire roster slot:
   - Stewardship pass: scan existing roster entries; for any failing
     liveness, run `pool_steward_clean <entry>` (see §6).
   - Add own entry to roster.
   - Release roster lock.
6. Set `export CARGO_TARGET_DIR=<slot dir>`. `mkdir -p` the dir if
   absent.
7. Trap `EXIT INT TERM HUP` → remove own entry from roster (under
   roster lock).
8. `exec command cargo "$@"`.

### 5. Slot lifecycle

**Decision:**

- **Family slot creation.** Lazy on first acquire.
  `mkdir -p $POOL_ROOT/family/<family>/<workspace_rel_path>/<profile>`.
  Roster file initialized empty if missing.
- **Persistence.** Family slots persist across all roster transitions
  (warm by default).
- **Cleanup at entry.** Stewardship pass (§6) runs at every acquire
  before the new entry is added.
- **Old-family GC.** When `CARGO_TARGET_POOL_MAX_FAMILIES` is
  exceeded, evict LRU family (full subtree prune) — but only after
  stewardship cleanup confirms no live agents in that family. If
  any are live, eviction picks the next LRU.
- **Per-family disk GC.** A separate `cargo-pool prune --older-than
  DURATION` walks `family/*/` and prunes slot subdirs whose
  `.last-build` stamp file is older than DURATION. Slots survive even
  if their family roster is empty, because warm-cache benefit comes
  from re-entry — keep them until disk pressure forces wipe.
- **Poisoning recovery.** Operator runs `cargo-pool prune family
  <name>` to nuke a family's slot if a build pathology suggests
  cache corruption.

### 6. Stewardship cleanup — the part that matters

**Decision:** When an agent finds a stale roster entry, it does NOT
just delete the entry. It investigates the entry's worktree and acts.

This is the load-bearing piece the user was asking for. It replaces
v1's "independent worktree janitor" decision.

**Algorithm `pool_steward_clean(stale_entry)`:**

1. **Read the stale entry.** Note `worktree_path`, `branch`,
   `joined_at`, the failure reason (pid dead / TTL exceeded / boot_id
   mismatch / etc.). Log the inspection start to `pool.log`.

2. **Does the worktree path still exist?**
   - **No** → worktree was already removed. The roster entry is a
     ghost. Drop it. Done.
   - **Yes** → continue.

3. **Is the worktree's `.git` valid?**
   - Run `git -C <worktree_path> rev-parse HEAD`. If it fails, the
     worktree directory exists but is broken. Log a warning, drop the
     roster entry, **do not** remove the directory (operator can
     decide). Done.
   - If success → continue.

4. **What is the branch's merge state?** Run, in order, against the
   parent repo (`/projects/elohim`):
   - `git -C /projects/elohim branch --merged dev | grep -Fx <branch>`
   - `git -C /projects/elohim ls-remote --heads origin <branch>`
   - `git -C /projects/elohim log --oneline -1 <branch>`

   Classify:

   | Local merged to dev? | Exists on origin? | Action |
   |---|---|---|
   | Yes | (any) | **Cleaned** — branch already integrated; worktree is safe to remove. |
   | No  | No  | **Orphan** — branch was deleted on origin and never merged. Surface to operator; do not auto-remove. |
   | No  | Yes | **Active** — branch is still in flight upstream. Drop the roster entry only. Do not touch the worktree. |
   | (failure) | (failure) | **Unknown** — log + drop roster entry only. Conservative. |

5. **Take action.**
   - **Cleaned**: drop roster entry, `git -C /projects/elohim worktree
     remove --force <worktree_path>` (the `--force` is needed because
     the worktree may have a dirty status from the crashed agent's
     last write; merge already happened so dirty work is reproducible
     from history). Log the removal.
   - **Orphan**: drop roster entry. Append to
     `$POOL_ROOT/orphan-worktrees.tsv`:
     `<timestamp>\t<worktree_path>\t<branch>\t<reason>`.
     Operator periodically reviews this file and decides whether to
     remove or rebuild the branch.
   - **Active**: drop roster entry only. The next live agent on this
     branch will re-add itself.
   - **Unknown**: drop roster entry only. Same as Active.

6. **Log the outcome.** One JSON line in `pool.log`:
   ```json
   {
     "ts": "2026-05-10T14:51:02Z",
     "event": "steward_clean",
     "stale_agent_id": "shift-...",
     "worktree_path": "...",
     "branch": "...",
     "classification": "cleaned" | "orphan" | "active" | "unknown",
     "action": "roster_drop" | "worktree_remove" | "orphan_logged" | "noop",
     "reason": "pid_dead" | "ttl_exceeded" | "boot_id_mismatch" | ...
   }
   ```

**Concurrency of stewardship.** The stewardship pass holds the
roster's `flock` for its entire duration. Two arriving agents
serialize on roster lock; whichever wins runs the stewardship pass,
then the other comes in and sees the cleaned roster. Worktree
operations (`git worktree remove`) take fractions of a second, so
holding the roster lock for that duration is acceptable. If we ever
see lock-hold-time pathology, move worktree removal to a deferred
queue (next acquire processes one entry from the queue).

**What the stewarding agent must NOT do:**

- Never remove a worktree whose branch is still active on origin
  (data-loss risk).
- Never `git push` or otherwise communicate with remotes during
  stewardship (read-only inspection).
- Never modify `dev` or any other branch's state — only operate on
  the worktree directory and the roster.
- Never act on a worktree it cannot definitively classify. "Unknown"
  → roster-only drop, never worktree removal.

**Manual entry point.** Operators can run `cargo-pool steward
[--dry-run]` to invoke the stewardship pass against all families
without acquiring a slot. Useful before manual cleanup or as a
follow-up to a workspace restart.

### 7. Observability

**Decision:** `cargo-pool` CLI for inspection; one append-only log;
roster files self-describing.

- `cargo-pool status` — table per family: family name, slot disk
  usage, roster size, oldest live entry age, time since last build,
  flagged conditions (stale entries, TTL approaching).
- `cargo-pool roster <family>` — full roster dump for one family.
- `cargo-pool steward [--dry-run] [--family <name>]` — run
  stewardship pass, print classifications and actions taken (or that
  would be taken under `--dry-run`).
- `cargo-pool sweep` — alias for `steward` without classification
  detail (keeps muscle memory from v1).
- `cargo-pool prune family <name> [--yes]` — wipe a family slot tree.
- `cargo-pool prune --older-than DURATION [--yes]` — wipe per-slot
  subdirs whose `.last-build` is older than DURATION.
- `cargo-pool log [-n N] [--family <name>]` — tail event log.
- `cargo-pool key` — print the family + slot path for current PWD
  (debugging).
- `cargo-pool orphans` — print contents of
  `$POOL_ROOT/orphan-worktrees.tsv`.

Event log at `$POOL_ROOT/pool.log` — one JSON line per event. Events:
`acquire`, `release`, `steward_clean`, `evict_family`, `prune`,
`fallback`, `error`.

### 8. Interaction with sccache

**Confirmed orthogonal.** sccache caches at the rustc-output level,
keyed on (preprocessed source content + rustc args + env). Family-
shared target dirs do not affect sccache's keys.

**Subtlety.** Family sharing means dep-info will ping-pong between
worktree absolute paths within a family (e.g., iroh-pkarr's source
paths and iroh-phase12-manifest's source paths). Cargo regenerates
dep-info on each invocation, and the rustc invocations themselves
hit sccache for the actual compile work. So the cost is "Cargo
re-runs the rustc dispatch" rather than "we recompile everything"
— in practice, the dispatch itself is fast and sccache short-
circuits the heavy lifting.

If we observe pathological rebuild loops (Cargo invalidating
incremental state too often within a family), revisit by adding a
secondary axis to the slot key, e.g., per-worktree subdirs within
the family slot. Tier 2 only — don't speculate.

### 9. Devfile env wiring

**Decision:** Six env vars and one PATH prefix.

```yaml
- name: CARGO_TARGET_POOL_ROOT
  value: /projects/.cargo-target-pool
- name: CARGO_TARGET_POOL_MAX_FAMILIES
  value: '6'
- name: CARGO_TARGET_POOL_MAX_DISK_GB
  value: '80'
- name: CARGO_TARGET_POOL_TTL_SECONDS
  value: '14400'
- name: CARGO_TARGET_POOL_ACQUIRE_TIMEOUT
  value: '60'
- name: CARGO_TARGET_POOL_FALLBACK
  value: private        # private | fail | wait_forever
- name: PATH
  value: '/projects/elohim/genesis/agentic/bin:/nix/xdg/cache/pnpm:...rest unchanged...'
```

Pool root **must** live on `/projects` (only persistent volume).
Wrapper script lives in repo (`genesis/agentic/bin/cargo-leased`),
not baked into image — repo-versioned, editable without container
rebuild.

`CARGO_TARGET_POOL_FAMILY` and `CARGO_TARGET_POOL_DISABLE` are NOT
set in the devfile — they're per-shell or per-shift opt-outs.

Devfile change costs commit + push + restart-from-local-devfile.
This batch lands all six env additions at once to avoid repeating
the cost.

### 10. Failure modes & recovery

| Failure | Mitigation |
|---|---|
| Acquire crashes during roster write | Roster written via `mv` atomic-rename pattern. Partial state impossible. |
| Two agents race the roster | `flock family/<family>/.roster.lock` makes the read-modify-write critical section atomic. |
| Slot dir corrupts (disk full mid-build) | Build fails, cargo error surfaces. Operator wipes via `cargo-pool prune family <name>`. T2: auto-detect bad CACHEDIR.TAG / missing fingerprint dir. |
| Workspace pod restarts | All in-flight roster entries become stale (boot_id mismatch). First acquire after restart runs stewardship and cleans them per §6. |
| Lease/roster file corrupt JSON | Treat whole roster as empty after warning to stderr; build proceeds. Operator informed via next `cargo-pool status`. |
| `cargo-leased` killed by SIGKILL | Trap can't fire. Stewardship reclaims at next acquire (TTL or kill-0 failure). Worst case: roster shows ghost until next stewardship run. |
| `flock` not available | Hard error. Wrapper refuses to operate without atomic locking. (In rust-nix-dev, `flock` is in util-linux — present.) |
| Stewardship classifies a worktree as Cleaned but the operator was actively using it locally | The operator's local edits would be reflected in `git status` and shouldn't be the cause of a stale roster entry — only a *crashed* agent's entry triggers stewardship. So we'd only be inspecting a worktree whose pid is dead AND whose branch is merged to dev. The worktree being still-edited-by-a-human past merge is unusual. **Mitigation:** stewardship checks `git -C <worktree> status --porcelain`; if non-empty, classify as "active-but-suspicious" and surface to orphan log instead of removing. |
| Family heuristic gets the wrong family | Operator overrides via `.family` file at worktree root or `CARGO_TARGET_POOL_FAMILY` env var. |
| Cross-family branch (e.g., a feature touches both iroh and epr work) | The branch's family is whatever the heuristic says; if wrong, override per above. The pool isn't trying to be smart about cross-family code; it just wants a consistent answer per worktree. |
| LRU eviction picks a family with active agents | Eviction algorithm requires zero-live-roster-entries; passes back to next-LRU until one qualifies, or aborts with operator-visible error if no family qualifies. |
| `git worktree remove --force` fails (e.g., locked file) | Log error to `pool.log`, leave worktree in place, drop roster entry, append to orphan log. Operator decides. |

---

## Interfaces

### Filesystem layout

```
$CARGO_TARGET_POOL_ROOT/                              # /projects/.cargo-target-pool
  pool.log                                            # append-only JSON-line event log
  orphan-worktrees.tsv                                # appendix: branches deleted on origin but never merged
  family/
    iroh/
      .roster.json                                    # presence registry
      .roster.lock                                    # flock target (empty file)
      .meta.json                                      # last_touched_at, family-level metadata
      elohim__elohim-storage/
        dev/
          CACHEDIR.TAG
          .last-build
          debug/  release/  .fingerprint/             # populated by Cargo
        release/
          ...
      doorway__doorway-service/
        dev/   release/
      elohim__holochain__tests__sweettest/
        dev/   release/
    epr/
      .roster.json
      ...
```

### Roster file schema

`family/<family>/.roster.json`:

```json
{
  "schema_version": 1,
  "family": "iroh",
  "entries": [
    {
      "agent_id": "shift-2026-05-10T14-30-iroh-phase12",
      "pid": 4711,
      "start_time_ns": 17829341192,
      "boot_id": "f4c8...e91",
      "hostname": "elohim-devspace-pod-abcd",
      "worktree_path": "/projects/elohim/.claude/worktrees/iroh-phase12-manifest",
      "branch": "iroh-phase12-manifest",
      "joined_at": "2026-05-10T14:32:01Z",
      "ttl_seconds": 14400,
      "command": "cargo build --release",
      "workspace_rel_path": "elohim/elohim-storage",
      "profile": "release",
      "last_heartbeat": "2026-05-10T14:32:01Z"
    }
  ]
}
```

`agent_id` derives from: shift id when running under `/shift`, else
`<branch>-<pid>-<short-uuid>`. Always unique; used as the primary key
when removing the agent's own entry on exit.

### Family meta file

`family/<family>/.meta.json`:

```json
{
  "schema_version": 1,
  "family": "iroh",
  "created_at": "2026-05-10T08:14:33Z",
  "last_touched_at": "2026-05-10T14:32:01Z",
  "total_disk_bytes": 18234567890
}
```

`total_disk_bytes` is opportunistically updated on release; not
authoritative (use `du -sb` for ground truth).

### Env vars

(Same as v1 plus the new ones; see Decision 9.)

### Script signatures

#### `genesis/agentic/bin/cargo-leased`

```
cargo-leased [--family NAME] <cargo-args...>
```

Behavior covered in §Decisions/4.

Exit codes: pass through Cargo's exit code. Wrapper-specific failures
use `exit 78` (EX_CONFIG).

#### `genesis/agentic/bin/cargo-pool`

```
cargo-pool status                              # families table
cargo-pool roster <family>                     # full roster of one family
cargo-pool steward [--dry-run] [--family X]    # run stewardship pass
cargo-pool sweep                               # alias for steward (muscle memory)
cargo-pool prune family <name> [--yes]         # wipe one family
cargo-pool prune --older-than DURATION [--yes] # GC old per-slot subdirs
cargo-pool prune --all [--yes]                 # last resort
cargo-pool log [-n N] [--family X]             # tail event log
cargo-pool orphans                             # list orphan-worktrees.tsv
cargo-pool key                                 # print family/slot for PWD
```

#### `genesis/agentic/lib/pool.sh` (sourced by both)

Functions:

```bash
pool_init                                  # ensure $POOL_ROOT and family/ dirs
pool_family_for_worktree <worktree>        # echo family name (heuristic chain)
pool_slot_path <family> <ws_rel> <profile> # echo absolute slot dir
pool_with_roster_lock <family> <cmd...>    # flock-wrap a command on roster.lock
pool_roster_load <family>                  # cat roster.json (or empty roster JSON)
pool_roster_save <family> <json>           # atomic mv-rename pattern
pool_roster_add <family> <entry-json>      # under lock, append entry
pool_roster_remove <family> <agent-id>     # under lock, remove entry
pool_entry_live_p <entry-json>             # exit 0/1/2 (live/stale/malformed)
pool_steward_clean_stale <family>          # run §6 algorithm, return action log
pool_evict_family <name>                   # full subtree prune after live-check
pool_classify_worktree <worktree> <branch> # echo cleaned|orphan|active|unknown
pool_log_event <json>                      # append one line to pool.log
```

---

## Tier 1 — minimum viable

**Goal:** Land enough that the next agentic shift's parallel waves
share family slots instead of cloning targets per worktree, AND
stewardship cleanup actually removes merged worktrees.

### T1.1: Pool lib

- [ ] Step 1. `genesis/agentic/lib/pool.sh` — implement all functions
      listed in §Interfaces. Use `POOL_ROOT_OVERRIDE` for testability.
- [ ] Step 2. `genesis/agentic/lib/pool.test.mjs` — unit tests using a
      sandbox temp dir as `POOL_ROOT_OVERRIDE`. Coverage must include:
      family derivation (8 cases from Decision 1), roster atomic
      add/remove, liveness (live, dead pid, mismatched start_time,
      mismatched boot_id, TTL exceeded, malformed JSON),
      stewardship classification (4 outcomes: cleaned / orphan /
      active / unknown), worktree-existence-check, race serialization
      via flock, atomic-rename on crash mid-write.

### T1.2: `cargo-leased` wrapper

- [ ] Step 3. `genesis/agentic/bin/cargo-leased` — POSIX-sh-compatible.
- [ ] Step 4. Family detection chain (env → .family file → branch
      prefix → branch name).
- [ ] Step 5. WASM-target detection (cdylib in nearest Cargo.toml or
      adjacent dna.yaml/happ.yaml) → short-circuit to plain cargo.
- [ ] Step 6. Pre-existing CARGO_TARGET_DIR → short-circuit to plain
      cargo.
- [ ] Step 7. Stewardship pass at acquire time (calls
      `pool_steward_clean_stale` on the target family).
- [ ] Step 8. Trap-driven release on EXIT/INT/TERM/HUP.
- [ ] Step 9. `--family X` argument support; consumed before forwarding
      to cargo.

### T1.3: `cargo-pool` CLI

- [ ] Step 10. `genesis/agentic/bin/cargo-pool` with the subcommands
      listed in §Interfaces.
- [ ] Step 11. `status` table format aligned to terminal width; shows
      family, disk usage (du -sh), live/stale roster counts.
- [ ] Step 12. `steward --dry-run` prints the classification + action
      it WOULD take without performing it.
- [ ] Step 13. `prune family` requires `--yes` non-interactively;
      interactive confirm if stdin is tty.

### T1.4: Devfile env

- [ ] Step 14. Edit `devfile.yaml` to add the six env vars and
      `genesis/agentic/bin` PATH prefix.
- [ ] Step 15. `genesis/agentic/bin/README.md` operator quickstart
      including the restart-from-local-devfile note.

### T1.5: Skill integrations

- [ ] Step 16. `.claude/skills/agentic-developer/SKILL.md` — Step 2
      (palette prediction) recommends `cargo-leased` for cargo
      invocations; document family-override in shift Objective when
      the heuristic is wrong for that shift.
- [ ] Step 17. `.claude/settings.json` durable allowlist — add
      `Bash(cargo-leased *)`, `Bash(cargo-pool status)`,
      `Bash(cargo-pool roster *)`, `Bash(cargo-pool steward *)`,
      `Bash(cargo-pool sweep)`, `Bash(cargo-pool log *)`,
      `Bash(cargo-pool orphans)`, `Bash(cargo-pool key)`. Keep
      `Bash(cargo-pool prune *)` on per-use approval (destructive).
- [ ] Step 18. `.claude/skills/superpowers/finishing-a-development-branch/`
      addendum (or matching project skill) — after merge, suggest
      `cargo-pool steward` to clean the just-merged worktree.

### T1.6: Justfile integration (deferred to T2 by default)

- [ ] Step 19 (optional T1). One Justfile swap to demonstrate: replace
      `cargo` with `cargo-leased` in `elohim/elohim-storage/justfile`.
      Validates the wrapper survives pre-push-hook invocation under
      `dash`.

---

## Tier 2 — enhancements

- **Heartbeat thread** during long builds (>TTL/2). Wrapper forks a
  loop that updates `last_heartbeat` every 5 minutes.
- **Auto-prune dirty slots on entry.** Inspect `slot/<workspace>/<profile>`
  before `cargo` runs; if `CACHEDIR.TAG` missing or `.last-build`
  unparseable, wipe before lease.
- **Cron'd stewardship.** `postStart` background loop running
  `cargo-pool steward` every 5 minutes regardless of acquire activity.
- **Cron'd disk GC.** `cargo-pool prune --older-than 7d --yes` once a
  day.
- **Per-worktree subdirs within family slots.** If dep-info ping-pong
  becomes pathological, add the worktree-relative-path as a sub-axis
  inside the slot.
- **Justfile sweep.** Patch all relevant justfiles to call
  `cargo-leased` for native builds (not WASM).
- **Pre-merge hook integration.** When `git merge` succeeds on dev
  (or `gh pr merge` does), trigger `cargo-pool steward` automatically.
- **Operator dashboard.** `cargo-pool status --json` consumed by a
  small status-line widget; family disk usage as part of the agentic
  shift journal.
- **Multi-host awareness.** If we ever run a sidecar container that
  also builds, lease entries already include `hostname` — extend
  liveness to "same hostname OR proxy-check via known mechanism."
- **Family explicit registry.** `genesis/agentic/families.json`
  declares known families with descriptions; the heuristic falls back
  to declared families when prefix is ambiguous.
- **Orphan worktree review tooling.** `cargo-pool orphans review`
  walks `orphan-worktrees.tsv` interactively, asking the operator to
  remove or restore each entry.

---

## Migration path

**Cutover, not coexist.** Pool is opt-in via wrapper. Existing
worktrees with private `target/` dirs keep working until their next
`cargo clean` or `rm -rf target/`. There is **no** auto-migration of
cached artifacts from per-worktree `target/` into family slots —
dep-info absolute paths would not match the new slot location and
would force rebuild anyway.

**Operator playbook for adopting the pool:**

1. Pull latest dev. `cargo-leased` is on PATH; env vars are set.
2. In each existing worktree, `rm -rf target/` once. (Local cache
   loss; sccache restores most of it on next build.)
3. Run builds via `cargo-leased` from then on.
4. Run `cargo-pool steward` once after first successful family build
   to verify stewardship works end-to-end on a known-good worktree.

**Roll-back:**

- `CARGO_TARGET_POOL_DISABLE=1` in shell or devfile → wrapper becomes
  passthrough.
- Or: revert the devfile PATH change → plain `cargo` resolves; wrapper
  stays in repo, harmless.

---

## Verification

### Smoke test (single worktree, family-of-one)

```bash
cd /projects/elohim/.claude/worktrees/iroh-pkarr/elohim/elohim-storage
cargo-leased build
cargo-pool status
# Expected: family=iroh, 1 active roster entry, slot dir created
cargo-pool roster iroh
# Expected: one entry with this PID, branch=iroh-pkarr, status=live
```

### Family sharing test

```bash
# Terminal A
cd /projects/elohim/.claude/worktrees/iroh-pkarr/elohim/elohim-storage
cargo-leased build &
PID_A=$!

# Terminal B (within 5s)
cd /projects/elohim/.claude/worktrees/iroh-phase12-manifest/elohim/elohim-storage
cargo-leased build &
PID_B=$!

# Terminal C
cargo-pool roster iroh
# Expected: TWO entries, both with status=live, sharing the same slot dir
```

After both finish:

```bash
cargo-pool roster iroh
# Expected: 0 entries (both released their slots cleanly)

du -sh /projects/.cargo-target-pool/family/iroh/elohim__elohim-storage
# Expected: ~6GB (one shared target, not 12GB)
```

### Stewardship test — cleaned branch

```bash
# Set up: a worktree on a branch already merged to dev
git checkout dev
git merge --no-ff some-already-cherry-picked-branch
# Spawn a build that crashes:
cd /projects/elohim/.claude/worktrees/some-merged-branch/elohim/elohim-storage
cargo-leased build &
PID=$!
sleep 2
kill -9 $PID

# Roster has stale entry
cargo-pool roster <family>
# Expected: one entry, status=stale (pid_dead)

# Run stewardship
cargo-pool steward --dry-run
# Expected output classifying this entry as 'cleaned' (branch is merged
# to dev), proposed action 'roster_drop + worktree_remove --force'

# Real run
cargo-pool steward
# Expected: roster empty, worktree directory gone, pool.log shows
# steward_clean event
```

### Stewardship test — active branch

Same setup but with a branch that is NOT merged and exists on origin.
Expected stewardship output: classification 'active', action
'roster_drop' only. Worktree directory and branch remain.

### Stewardship test — orphan branch

Same setup but branch is deleted on origin AND not merged. Expected:
classification 'orphan', roster_drop, append to orphan-worktrees.tsv,
worktree directory remains.

### Concurrent stewardship test

Two terminals both run `cargo-pool steward` simultaneously. Roster
flock serializes them. Both succeed; only one runs the actual
classifications; the other sees a clean roster. Idempotent.

### Realistic load test

Next agentic shift's wave-1 dispatch with 4 parallel worktrees on the
iroh family + 1 on an unrelated family. Observe:

- `cargo-pool status` shows 2 families (iroh, other).
- iroh family slot dir disk usage peaks <12GB (one target shared).
- `df -h /projects` peaks <70%.
- `cargo-pool log` shows acquire/release events; no surprise
  steward_clean during the run.

### Long-haul test

Persist pool across two shifts ~12h apart. Second shift's first build
on a known iroh worktree:

- Should reacquire the same family slot (warm).
- Build should be near-no-op if no source changed.
- `cargo-pool log` should show clean event sequence.

After workspace restart-from-local-devfile (boot_id changes):

- First acquire after restart should run stewardship and clean every
  pre-restart roster entry.
- `cargo-pool log` should show steward_clean events with reason
  `boot_id_mismatch`.

---

## Honest engineering — known weaknesses

1. **Stewardship's "merged to dev" check is local-only.** If `dev` on
   origin has commits the local `dev` doesn't, a branch merged to
   *origin/dev* would classify as 'unknown' (or 'active' if still on
   origin). Mitigation: stewardship runs `git fetch origin dev`
   *non-destructively* before classification. Add `Bash(git fetch
   origin dev)` to the agentic palette.

2. **`git worktree remove --force` is destructive.** A crashed agent
   may have unstaged work that's not in any commit. Eligible for
   removal only when branch is merged to dev — meaning every
   meaningful change should already be in history. But: the agent's
   *uncommitted scratch* would be lost. Mitigation: stewardship
   appends a `git stash list` + `git status --porcelain` snapshot to
   the steward_clean log event before removal, so post-mortem can
   recover if needed. Optional T2: snapshot the diff into
   `pool/recovered/<branch>-<timestamp>.patch` before removal.

3. **Family heuristic for ambiguous branches.** `feat/iroh-pkarr`
   strips `feat/` and gets `iroh`. But `feature-x-iroh-update` would
   yield `feature`. Operator override exists; surface heuristic
   result to the operator at first acquire of an unfamiliar branch
   (one-line stderr note from the wrapper).

4. **Pool root on `/projects` couples to that volume's health.**
   Already noted; visibility via `cargo-pool status` is the early-
   warning.

5. **No multi-host story.** Hostname is in the lease; cross-host
   liveness check is deferred. Acceptable until we actually run
   multi-pod builds.

6. **Wrapper is bash, not Rust.** Same trade-off as v1. Rust-binary
   in T2 if bash grows past ~400 lines or develops parsing bugs.

7. **What if a family explodes in count?** Operator runs `cargo-pool
   prune --older-than 1d` to evict cold sub-slots. Or raises
   `CARGO_TARGET_POOL_MAX_FAMILIES`. Or the LRU eviction handles it.

8. **Stewardship picks a victim the operator was about to use.**
   Eviction only fires under disk OR family-count pressure, and
   evicts LRU. The operator's about-to-use family was presumably the
   most recent acquire — would be MRU, not LRU. Low risk.

---

## Could the whole approach be wrong?

Three remaining alternatives:

- **Don't share at all; just GC private target dirs more aggressively.**
  Worktree janitor runs daily, removes merged worktrees, their target
  dirs go with them. No cross-worktree sharing. Simpler. Loses warm-
  cache reuse across worktrees in the same family — but sccache
  restores most of it. **The reason this still loses:** in-flight
  parallel builds on related branches each pay their full ~18GB.
  Wave-of-4 still hits 72GB peak. Family sharing brings that to ~18GB
  peak for the whole family.
- **Per-worktree target dirs but on a shared-cache filesystem layer
  (overlay/CoW).** Theoretically attractive; in practice Eclipse Che
  doesn't expose CoW filesystems to user containers, and any solution
  that needs root or kernel features is out of scope. Would also
  break Cargo's mtime-based invalidation in subtle ways.
- **Trust sccache fully; use plain cargo.** Disk usage stays
  unbounded per worktree; today's incident recurs. Doesn't address
  the disk pressure.

The family-shared model with stewardship cleanup is the most aligned
with how the team actually works (related branches → related caches)
and with the user's stated mental model.

---

## One-paragraph summary

A per-feature-family target directory under
`/projects/.cargo-target-pool/family/<family>/`, shared across all
worktrees, workspaces, and profiles in that family. Multiple agents
co-tenant the same target via a roster file (presence registry) under
`flock`, with Cargo's intra-target lock serializing rustc invocations
naturally. Stewardship cleanup runs on every acquire: stale roster
entries are inspected against their worktree's git state and
classified as cleaned (worktree-removable), orphan (branch dropped
upstream — surface for operator), active (still in flight — drop
roster entry only), or unknown (conservative — drop entry only).
WASM (DNA) builds bypass the pool entirely. sccache stays orthogonal.
Tier 1 lands as a pool lib, two wrapper scripts, devfile env
additions, and an agentic-developer skill patch. Worktree cleanup
and pool GC are coupled — a single stewardship action cleans both.

