---
id: "backlog-ghost-decay-deploy-verification-runbook"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Deploy-verification runbook for the ghost-decay cure: prove bundle freshness, coordinator hot-swap applied, and local-get live — from logs/metrics only"
slug: "ghost-decay-deploy-verification-runbook"
written: "2026-08-10"
author: "batch-3 integration session (uncertainty-reduction dispatch)"
status: "backlog"
priority: "high"
tags: [dataplane, deploy-verification, coordinator-hot-swap, edge-pipeline, read-only-probe, codex-claimable]
cites:
  - elohim/holochain/Jenkinsfile
  - elohim/elohim-storage/src/p2p/view_federation.rs
---

# Verification runbook — turn "probably raced" into a yes/no per leg

READ-ONLY source trace producing a runbook (appended under `## Runbook`
here). The integration wave (46e853521) races the DNA pipeline against the
edge image bake; the zome local-get fix rides the happ bundle. Produce exact,
copy-pasteable checks for each leg:

1. **Bundle freshness in the edge image.** Where exactly does the edge
   Dockerfile/Jenkinsfile fetch `elohim-happ:dev-latest` (stage + line), and
   what digest/stamp does the deploy record (the happ-digest stamp from
   9f9c4aec4)? Give the command/log-line that tells whether edge build #N
   baked the bundle from DNA build #1396 or its predecessor.
2. **Coordinator hot-swap applied.** The exact conductor/storage log line(s)
   `happ_manager::sync_coordinators` emits on an applied update (and on
   skip), and the per-pod Loki query to confirm 7/7 application after the
   deploy. Note ALLOW_COORDINATOR_UPDATE defaults from ALLOW_DNA_REINSTALL
   (alpha=true).
3. **Local-get live signature (the decisive one).** After the fix is live on
   a supplier, its head-record responder degrade cause must FLIP:
   `elohim_content_head_record_degraded_total{cause="budget_elapsed"}` stops
   growing and `{cause="no_record"}` starts, on adam + the shem trio. Give
   the PromQL (rate over 30m, per pod) and the expected before/after shape.
4. **Decay engagement signature.** The ordered metric sequence that proves
   the arm is working end-to-end: contest_skipped{evidence_absent_backoff}
   rising → (>=1h dwell) → elohim_content_ghost_decay_author_total > 0 →
   ghost-witness log `authored > 0` → content_local_anchored rising toward
   ~4467 on matthew → divergent_actionable settling into 0-2.

No code changes; runbook must be executable by any agent with Loki/Prometheus
MCP + curl.

## Runbook

### Source-trace verdict (2026-08-10)

The phrase "baked the bundle" is not literally true for either edge image. The
edge pipeline pulls `elohim-happ:dev-latest` into both Docker build contexts
(`elohim/holochain/Jenkinsfile:1741,1798`), but both Dockerfiles explicitly say
the hApp is **not baked** and neither copies `elohim.happ`
(`elohim/holochain/edgenode/Dockerfile:6-16`,
`elohim/holochain/edgenode/scripts/Dockerfile:4-23`). The runtime path is:

1. DNA build pushes the commit tag and then `dev-latest`
   (`elohim/holochain/dna/Jenkinsfile:891-935`).
2. Edge deploy resolves `dev-latest` and logs
   `Resolved hApp digest for tag 'dev-latest': sha256:...`
   (`elohim/holochain/Jenkinsfile:636-650`).
3. That digest is written to pod-template annotation
   `elohim.host/happ-digest`, making a tag move restart the StatefulSet
   (`elohim/holochain/Jenkinsfile:719,830-834`;
   `_edgenode-consolidated.template.yaml:203-217`).
4. On restart, init container `happ-fetcher` pulls the still-floating
   `dev-latest` tag (`_edgenode-consolidated.template.yaml:270-281`).

Therefore the annotation is a **rollout trigger, not a digest pin on the pull**.
There is a small resolve-to-pull TOCTOU window. The decisive freshness proof is
not the annotation alone: compare the DNA push digest, all seven edge-resolved
digests, and all seven `happ-fetcher` pull digests; then require the coordinator
hot-swap/runtime signatures below.

At `2026-08-10T19:21:57Z`, Jenkins read-only evidence said:

- `elohim-holochain/dev#1396` was still building, checked out at merge
  `46e853521fe9fa91c8bb061df9c0440db444adb3`. Both cure commits
  `a9f9d781b` and `6368847e3` are ancestors of that merge.
- Its changeset contains both cure commits, but a running build is **not proof
  that the Push to Harbor stage completed**.
- `elohim-edge/dev#1339` (started `14:20:45Z`, before either cure commit) was
  still the last edge build; edge was not queued. Thus the cure was not yet
  deploy-proven at this snapshot.

Sources: Jenkins MCP `getBuild`, `getBuildScm`, `getBuildChangeSets`, and
`getJob`; local ancestry checks via `git merge-base --is-ancestor`.

### 1. Prove the DNA artifact was published, then prove what edge deployed

Set the edge build that actually performs the post-#1396 deploy:

```bash
JENKINS_BASE=https://jenkins.ethosengine.com
DNA_BUILD=1396
EDGE_BUILD=N
```

First prove build identity. These artifacts are archived during Setup Version,
so they are readable even before the full build finishes:

```bash
curl -fsS "$JENKINS_BASE/job/elohim-holochain/job/dev/$DNA_BUILD/artifact/build.env"
curl -fsS "$JENKINS_BASE/job/elohim-edge/job/dev/$EDGE_BUILD/artifact/build.env"
```

For this wave, both must show `GIT_COMMIT_HASH=46e85352`; the DNA artifact tag
must be `1.0.0-dev-46e85352`, and the edge `STORAGE_TAG` must end in
`-dev-46e85352`. That proves the storage image includes the decay logic and the
DNA build checked out the coordinator fix. It still does not prove Harbor push.

Extract the digest that DNA #1396 actually pushed to the floating tag:

```bash
DNA_DIGEST="$(
  curl -fsS "$JENKINS_BASE/job/elohim-holochain/job/dev/$DNA_BUILD/consoleText" |
  awk '
    /Pushing hApp floating tag: dev-latest/ { capture=1; next }
    /Pushing WASM cache core/ { capture=0 }
    capture && match($0, /sha256:[0-9a-f]{64}/) {
      print substr($0, RSTART, RLENGTH); exit
    }
  '
)"
printf 'DNA_DIGEST=%s\n' "$DNA_DIGEST"
```

An empty value means #1396 did not publish `dev-latest`; stop. A build result of
SUCCESS without this line is also not sufficient.

Now extract what the edge deploy resolved. `resolveHappDigest` runs once per
active human, so a clean seven-peer alpha deploy produces seven identical
lines:

```bash
curl -fsS "$JENKINS_BASE/job/elohim-edge/job/dev/$EDGE_BUILD/consoleText" |
  sed -nE "s/.*Resolved hApp digest for tag 'dev-latest': (sha256:[0-9a-f]{64}).*/\1/p" |
  sort | uniq -c
```

Pass shape: exactly one row, count `7`, digest equal to `$DNA_DIGEST`. Any
`unresolved-...` warning is a fail for freshness even though the fail-safe still
forces a restart. More than one digest means `dev-latest` moved while the seven
parallel manifests were rendered.

Finally, query the actual init-container pulls over the edge deploy window. This
closes the resolve-to-pull TOCTOU gap because `oras pull` prints its fetched OCI
digest:

```logql
{namespace="elohim-alpha",
 pod=~"elohim-(adam|matthew|jessica|james|eve|gertrude|susan)-alpha-0",
 container="happ-fetcher"}
  |= "Digest: sha256:"
  | regexp `Digest: (?P<happ_digest>sha256:[0-9a-f]{64})`
```

Pass only when there is a post-deploy line for every one of the seven pods and
every `happ_digest` equals `$DNA_DIGEST`. A missing pod is unmeasured; mixed
digests mean a mixed rollout. Do not substitute the edge build-time
`Fetching hApp from Harbor...` lines: those pulls gate legacy image builds but
their bytes are intentionally not copied into either image.

### 2. Prove coordinator hot-swap on 7/7 pods

`happ_manager` computes coordinator hash drift after it has ruled out integrity
DNA drift. `ALLOW_COORDINATOR_UPDATE` defaults to `ALLOW_DNA_REINSTALL`
(`elohim/elohim-storage/src/happ_manager.rs:118-129`). Edge renders
`ALLOW_DNA_REINSTALL=true` for non-prod, including alpha, and Adam's hand-rendered
manifest also sets it true. Thus an unset explicit coordinator flag is enabled
on all seven alpha pods.

Exact log vocabulary:

- drift seen: `Coordinator-zome drift (DNA hash unchanged — integrity-only hashing cannot see this)`
- applied per role: `Coordinator zomes hot-swapped to bundle version`
- summary: `Coordinator-zome drift handled` with `drifted_roles` and
  `applied=true`
- disabled skip: `NOT hot-swapping — set ALLOW_COORDINATOR_UPDATE=true (or ALLOW_DNA_REINSTALL=true) to apply; the conductor keeps serving the OLDER coordinator wasm until then`
- neutral/ambiguous: `No coordinator-zome drift`
- failures: `coordinator drift check FAILED — coordinator-only changes will NOT deploy until resolved`, `get_dna_definition failed — skipping coordinator drift check for role`, `failed to build coordinator bundle — skipping role`, or `update_coordinators failed — role keeps old coordinators`

These are emitted at `happ_manager.rs:130-140,438-489`. For this change the
drifted role is `lamad`.

Fleet query over a window beginning just before the rollout:

```logql
sum by (pod) (
  count_over_time(
    {namespace="elohim-alpha",
     pod=~"elohim-(adam|matthew|jessica|james|eve|gertrude|susan)-alpha-0",
     container="elohim-node"}
      |= "Coordinator zomes hot-swapped to bundle version"
      | json
      | fields_role="lamad"
    [30m]
  )
)
```

Pass shape: seven pod series, each `>= 1`. Search the same window for every
skip/error branch:

```logql
{namespace="elohim-alpha",
 pod=~"elohim-(adam|matthew|jessica|james|eve|gertrude|susan)-alpha-0",
 container="elohim-node"}
  |~ "NOT hot-swapping|No coordinator-zome drift|coordinator drift check FAILED|get_dna_definition failed|failed to build coordinator bundle|update_coordinators failed"
```

Interpret `No coordinator-zome drift` carefully: paired with the matching
`happ-fetcher` digest it can mean the correct coordinator was already installed;
alone it can equally mean the pod received the predecessor bundle. It is never
a freshness proof by itself. A genuinely fresh install can also bypass
`sync_coordinators`; in that case require its matching pull digest plus the
local-get metric signature below rather than inventing a missing hot-swap.

### 3. Prove the local-get coordinator behavior is live

The code change is exactly `GetOptions::default()` to `GetOptions::local()` at
`content_store/src/lib.rs:5202-5218`. A missing record should now return promptly
and increment `cause="no_record"`; the old network search exhausted the
responder budget and incremented `cause="budget_elapsed"`. Query Adam plus the
Shem trio (Eve, Gertrude, Susan) over the same 30-minute windows immediately
before and after rollout:

```promql
sum by (pod, cause) (
  rate(elohim_content_head_record_degraded_total{
    namespace="elohim-alpha",
    pod=~"elohim-(adam|eve|gertrude|susan)-alpha-0",
    cause=~"budget_elapsed|no_record"
  }[30m])
)
```

Expected shape under continuing head-record traffic:

- before: `budget_elapsed > 0`, `no_record == 0` (the diagnosed Adam sample was
  202/202 budget elapsed);
- after: `budget_elapsed` stops growing (rate approximately zero) and
  `no_record > 0` begins on the suppliers actually asked.

For sparse traffic, inspect integer deltas as the sanity view:

```promql
sum by (pod, cause) (
  increase(elohim_content_head_record_degraded_total{
    namespace="elohim-alpha",
    pod=~"elohim-(adam|eve|gertrude|susan)-alpha-0",
    cause=~"budget_elapsed|no_record"
  }[30m])
)
```

Both causes flat at zero is **inconclusive**, not a pass. Confirm requests were
still being made from the requester side:

```promql
sum by (pod, state) (
  increase(elohim_content_adopt_evidence_total{
    namespace="elohim-alpha",
    state=~"carried|no_record|budget_elapsed|conductor_error|unknown"
  }[30m])
)
```

If request traffic is nonzero but all four supplier pods remain
`budget_elapsed`-dominated, the coordinator fix is not live regardless of the
build bars. If requester evidence becomes `no_record`-dominated while responder
`no_record` rises, the behavior is live.

### 4. Prove decay engagement end to end

Use the first post-deploy `no_record`/evidence-absent observation as `t0`. The
repo does not override `ELOHIM_GHOST_DECAY_MIN_DWELL_SECS`, so the code default
is 3600 seconds; do not expect authoring before `t0 + 1h`. The evidence-absent
backoff itself is six hours in the alpha manifest, so the one-hour dwell remains
inside its active window.

1. Evidence-absent ledger is active and saving repeated contest work:

   ```promql
   sum by (pod) (
     increase(elohim_content_contest_skipped_total{
       namespace="elohim-alpha",reason="evidence_absent_backoff"
     }[30m])
   )
   ```

   Rising after `no_record` proves classification/backoff engagement. It does
   not by itself prove the one-hour age requirement.

2. At or after `t0 + 1h`, decay releases at least one Hold/Contest decision:

   ```promql
   sum by (pod) (
     increase(elohim_content_ghost_decay_author_total{
       namespace="elohim-alpha"
     }[30m])
   )
   ```

   Require a positive delta on at least the peer processing the phantom slice.

3. The ghost-witness author call actually lands (`fields_authored > 0`):

   ```logql
   {namespace="elohim-alpha",container="elohim-node"}
     |= "projection-reconcile[ghost-witness]: authored local heads for rows whose claimed dht_anchor_hash"
     | json
     | fields_authored > 0
   ```

   The exact summary also carries `candidates`, `skipped`, `failed`, `adopted`,
   and `held` (`projection_reconcile.rs:2235-2248`). `ghost_decay_author > 0`
   with `authored == 0` means the decision arm engaged but the conductor write
   did not land; inspect `failed`/`held` and the adjacent re-author failure logs.

4. Matthew's anchored content population rises toward the diagnosed target
   (~4467). The metric corresponding to the log field
   `content_local_anchored` is `local_total{stream="content"}`:

   ```promql
   elohim_projection_reconcile_local_total{
     namespace="elohim-alpha",
     pod="elohim-matthew-alpha-0",
     stream="content"
   }
   ```

   It updates on measured content discovery sweeps; pair it with
   `elohim_projection_reconcile_measured{stream="content"} == 1` so a stale
   gauge is not mistaken for progress.

5. The bank-facing residue settles inside tolerance:

   ```promql
   max_over_time(
     elohim_projection_reconcile_divergent_actionable{
       namespace="elohim-alpha",
       pod="elohim-matthew-alpha-0"
     }[30m]
   )
   ```

   Pass shape: `<= 2` for the full observation window (not one lucky sample).
   This atomic gauge is the correct gate input; do not reconstruct it by
   subtracting `divergent_refused` from `divergent`, because those gauges are
   published at different moments.

### Decision table

| Evidence | Verdict |
|---|---|
| #1396 lacks a floating-tag digest | Desired hApp was not published. |
| Edge's seven resolved/pulled digests equal one another but not #1396 | The race landed the predecessor bundle; build green is irrelevant. |
| Resolved digests match but `happ-fetcher` digests differ | Tag moved in the resolve-to-pull window; rollout is mixed. |
| Pull digest matches, but a pod logs a disabled/failure hot-swap branch | Bundle arrived but coordinator was not applied on that pod. |
| Pull digest matches and hot-swap/fresh-install is proven, but `budget_elapsed` persists under traffic | Local-get behavior is not live; treat deployment as failed. |
| `no_record` + evidence-absent skips rise, but decay stays flat before one hour | Expected dwell, not failure. |
| Same shape after one hour, with live hints continuing | A decay predicate other than deploy freshness is false; inspect local absence/election/live-hint evidence. |
| Decay rises but witness `authored` stays zero | Author path engaged but did not land. |
| Witness `authored > 0`, local total rises, but actionable divergence does not settle | Deployment cure is live; residual is the post-author adjudication cascade (Task 1), not this deploy leg. |

This runbook is source-verified. Live Loki/Prometheus execution was not possible
in the claiming Codex runtime because the observability MCP was not registered;
the Jenkins snapshot above was executed read-only, while the LogQL/PromQL blocks
are exact copy-paste probes for an observability-equipped agent.
