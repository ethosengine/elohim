---
title: App-Interface Zome-Call Deadlines — conductor capability spike (fork patch + upstream PR draft + decision memo)
id: conductor-call-deadline-capability-spike
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: ratified-and-decomposed OR superseded-by-upstream-merge
created: 2026-08-08
topic: [conductor, holochain-fork, upstream-contribution, zome-call, deadline, backpressure, head-plane, admission-control]
cites:
  - head-plane-trust-gradient-program-plan | Parent program: this is its T13 spike; the plan states the uncancellable-call constraint (evidence item 4) that §1-2 here qualify, and owns the elohim_head_batch_queue_wait_ms probe the decision memo depends on | sha256:69f96e0a10dc54dd | path: genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md
  - genesis/data/timeline/backlog/2026-08-04-conductor-fork-rebase-0-6-3.md
  - adam-slow-link-write-guard-saturation | The constraint of record: write-guard saturation is the load under which abandoning a queued zome call actually returns a database permit, which is what makes an app-interface deadline worth anything | sha256:556142ddd510a091 | path: genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
---

# App-Interface Zome-Call Deadlines — conductor capability spike

**T13 of the head-plane trust-gradient program.** Design-bearing spike, two
instruments, both legs operator-gated. Nothing here is deployed, pushed, or
opened as a PR.

## 1. What this spike settles

The program plan records the constraint as evidence item 4: *"`HcClient::call_zome`
has no timeout and no cancellation (`hc_client.rs:405`). A caller timeout leaves
the conductor executing with nobody listening."* The operator directive is that
this scheduling floor — owned by an upstream we do not control — is not something
we design around forever, and that both instruments (our fork, an upstream PR)
are on the table with no ideological ordering.

This spike did the code archaeology, found the constraint is **narrower and more
tractable than the framing implied**, implemented the tractable part on the fork,
and drafted the general capability for upstream. The decision memo (§6) weighs
the two instruments by delivery need and names the falsifiable probe that decides
whether the fork leg is warranted at all.

**The finding that reframes the problem:** the Rust client *already* has a
per-call timeout, and never tells the conductor about it.

```rust
// crates/client/src/app_websocket.rs:547 (holochain 0.6.3)
pub struct CallZomeOptions {
    /// Per-call timeout override.
    pub timeout: Option<Duration>,
}
```

The timeout is enforced entirely on the client's side of the websocket
(`app_websocket_inner.rs:134-143` — `tx.request_timeout(msg, t)`). The conductor
receives an ordinary `AppRequest::CallZome` and has no idea a deadline exists.
So the capability to add is not "invent deadlines for zome calls" — it is
**"carry the deadline the caller already declared across the interface, and let
the conductor act on it."** That is a much easier thing to argue upstream than a
new scheduling concept, because it removes an asymmetry rather than adding a
feature.

## 2. Evidence — what a zome call actually does, and what is cancellable

All references are to the conductor fork at `elohim/holochain-conductor`,
branch `elohim-0.6.3` (upstream tag `holochain-0.6.3`).

| Stage | Where | Cancellable by dropping the request future? |
|---|---|---|
| Websocket receive, concurrency fan-out | `conductor/interface/websocket.rs:33,450` — `for_each_concurrent(CONCURRENCY_COUNT = 128)`, **per connection** | n/a |
| App-request dispatch | `conductor/api/api_external/app_interface.rs:95` | n/a |
| Signature verification | `conductor/conductor.rs:1043-1065` | **yes** |
| Cell lookup, init check, source-chain workspace construction | `conductor/cell.rs:709-755` | **yes** — these `await` on database access |
| Database read/write permit acquisition | `holochain_sqlite/src/db/access.rs:186-201` → `acquire_semaphore_permit(semaphore).await` | **yes** — a plain tokio `Semaphore::acquire`; dropping the future *gives the queue slot back* |
| WASM function body | `core/ribosome/real_ribosome.rs:833` — `tokio::task::spawn_blocking(...)` | **no** — a spawned blocking task runs to completion regardless |
| Post-wasm workflow (validation, publish trigger, commit) | `core/workflow/call_zome_workflow.rs` | **yes**, at each `await` |

Three consequences, and they are the whole design:

1. **A deadline at the app-interface layer is genuinely useful today.** Most of
   what a saturated conductor spends a zome call's wall-clock on is *waiting* —
   for a read permit, for a write permit, for a connection from the pool — and
   every one of those waits is a cancellable `await`. Abandoning the call
   releases the queue slot to somebody else. This is exactly the pressure named
   in the write-guard saturation record.
2. **It is not full cooperative cancellation, and must not be sold as such.** An
   already-running WASM body keeps running. It is bounded only by the ribosome's
   metering points (`real_ribosome.rs:825,844`). Any response we return on
   deadline must say "I stopped waiting", never "it did not happen".
3. **Upstream already reasons this way, in this codebase, one layer down.**
   `holochain_sqlite/src/db/access.rs:158-160`:

   > *"Once sync code starts in the spawn_blocking it cannot be cancelled BUT if
   > we've run out of threads to execute blocking work on then this timeout
   > should prevent the caller being blocked by this await that may not finish."*

   That is the same idiom, the same caveat, and the same justification — applied
   to a database thread pool instead of to the app interface. The PR is asking
   upstream to extend a pattern they already hold, not to adopt a new one.

Also relevant: `incoming_request_concurrency_limit` (default = `db_max_readers − 3`,
`config/conductor.rs:142-167`) throttles **network** authority responses. There
is no equivalent ceiling for app-interface zome calls; `CONCURRENCY_COUNT = 128`
is a per-connection constant, not a conductor policy, and a client can raise its
own effective limit by opening more websockets.

## 3. Capability scope — the smallest genuinely-useful first slice

**Stage 1 (this spike): bounded, refusable app-interface zome calls.**

- *Deadline propagation.* The caller's declared deadline crosses the interface.
- *Deadline-aware admission.* At a configured concurrent-call ceiling, a call
  that declared a deadline is **refused immediately** rather than queued behind
  work that would consume its whole deadline before it started. A call that
  declared no deadline is still queued — an existing client has no code to
  handle a refusal, so the ceiling must be invisible to it.
- *Bounded response.* The call is abandoned when the deadline elapses and a
  typed response is returned, releasing queued database permits.

**Deliberately out of stage 1, named rather than hand-waved:**

- *Stage 2 — cooperative cancellation inside host calls.* Thread the deadline
  into `CallContext` so `get`, `get_links`, and the cascade can observe it and
  return early. Larger diff, touches the HDK-facing host API, needs its own
  design conversation about what a partially-completed host call returns.
- *Stage 3 — WASM preemption.* Wasmer metering middleware is already wired
  (`reset_metering_points` / `get_used_metering_points`); driving remaining
  points to zero from a watchdog would trap a running body. This is genuine
  preemption and genuinely invasive — it changes what a zome author can assume
  about their function completing.

Stage 1 is the right first slice because it is the only one that is **provably
behaviour-neutral when unconfigured** (§4 test: an unconfigured conductor admits
every call unbounded, exactly as today) and because it captures the largest
share of the observed cost — queueing, not computing.

## 4. Instrument A — the fork patch

**Branch:** `elohim-0.6.3-call-deadline` @ `8ee534862`, one commit off
`elohim-0.6.3` (`6d0814266`), in the `elohim/holochain-conductor` submodule.
Committed in the submodule only; the monorepo gitlink is untouched and the
submodule is left checked out on `elohim-0.6.3`. Not pushed.

### 4.1 What it changes

| File | Change |
|---|---|
| `crates/holochain_conductor_api/src/app_interface.rs` | New `AppRequest::CallZomeWithDeadline { call, deadline_ms }` variant |
| `crates/holochain_conductor_api/src/admin_interface.rs` | New `ExternalApiWireError::ZomeCallRefused` and `::ZomeCallDeadlineExceeded` variants |
| `crates/holochain_conductor_api/src/config/conductor.rs` | `ConductorTuningParams` gains `zome_call_deadline`, `zome_call_deadline_max`, `max_concurrent_zome_calls` + accessors |
| `crates/holochain/src/conductor/api/api_external/app_interface.rs` | `ZomeCallAdmission` policy + pure `decide()` predicate + in-flight slot guard; `CallZome` arm factored into `handle_call_zome`; unit tests |
| `crates/holochain/src/sweettest/sweet_conductor_config.rs` | Exhaustive struct literal updated |
| `crates/client/src/app_websocket.rs` | `CallZomeOptions::declare_deadline` + `with_declared_deadline()`; sends the new variant when opted in |

### 4.2 The decision predicate

The whole policy is one pure function, which is what makes it testable without
standing up a conductor:

```rust
pub fn decide(&self, declared: Option<Duration>, in_flight: usize)
    -> ZomeCallAdmissionDecision
{
    let effective = match declared {
        Some(d) => Some(d.min(self.max_deadline)),
        None => self.default_deadline,
    };
    match (self.max_in_flight, effective) {
        (Some(max), Some(_)) if in_flight >= max =>
            ZomeCallAdmissionDecision::Refuse { in_flight, max_in_flight: max },
        _ => ZomeCallAdmissionDecision::Admit(effective),
    }
}
```

Defaults are `None`/`None`/`None`, so an operator who changes nothing gets
`Admit(None)` for every call at any load — byte-for-byte today's behaviour.
Opting in is an explicit act, and the consequence of opting in (a default
deadline makes the concurrency ceiling start biting calls that declared nothing)
is pinned by its own test rather than left as a surprise.

### 4.3 Two legs, only one of which we can consume today

The patch has a **config leg** (conductor-wide default deadline + ceiling, no
wire change, no client change) and a **per-call leg** (the new `AppRequest`
variant + client support).

**Only the config leg is reachable from elohim-storage.** `elohim-storage`
pins `holochain_client = "=0.9.0-dev.24"` from crates.io
(`elohim/elohim-storage/Cargo.toml:123`), published off upstream's `develop`
lineage. The client crate *inside* our fork at the 0.6.3 tag is version
**0.8.3**. A `[patch.crates-io]` redirect — the mechanism the fork already uses
for tx5 — cannot satisfy an `=0.9.0-dev.24` requirement from a 0.8.3 source. So
the per-call leg lands on the fork as correct, tested code that our own runtime
cannot call until either the client pin moves or we hand-construct the request
variant in elohim-storage.

Nor is moving the pin a free action. The pin carries its own warning in place
(`elohim/elohim-storage/Cargo.toml:121-122`):

> *"Raising this pin re-opens wire-skew risk against the fork; lowering it
> re-breaks the admin seam. Move it only with a fresh cross-version diff."*

This is not a defect in the patch. It is the delivery fact that drives §6.

### 4.4 What compiled

`cargo fmt --check`, `cargo check` on all three touched crates, and
`cargo test -p holochain --lib` all exit 0; the eight unit tests pass. A release
build and the sweettest integration scenarios were **not** run. Full record with
exit codes, and an explicit list of what remains unverified, in §7.

## 5. Instrument B — upstream PR draft

Everything from here to the end of §5 is written as the PR body, in upstream's
voice. It contains no elohim-specific reference by design. Per
`CONTRIBUTING.md`: fork, branch named for the fix, base branch `develop`, merge
not rebase, tests required.

---

### Title

`feat(conductor): honour caller-declared deadlines on app-interface zome calls`

### Problem

A client of the app interface can already give a zome call a timeout —
`CallZomeOptions::timeout` in `holochain_client`, and the equivalent in every
other client. That timeout is enforced entirely on the client side of the
websocket. The conductor is never told about it.

The consequences fall on both sides:

- **The conductor does work nobody is waiting for.** When a client's timeout
  fires it stops reading the response, but the call keeps running: it keeps its
  place in the database read and write permit queues, keeps a source chain
  workspace open, and eventually commits. Under load this is the worst possible
  behaviour, because the calls that time out are exactly the ones queued behind
  a saturated resource, and abandoning them silently would have freed that
  resource for calls that could still succeed.
- **The client cannot distinguish "slow" from "refused".** A timeout is the only
  signal available, and it arrives after the full timeout has elapsed. A
  conductor that already knows it cannot start the call for another thirty
  seconds has no way to say so.

There is no admission control on app-interface zome calls at all.
`incoming_request_concurrency_limit` bounds network authority responses;
`CONCURRENCY_COUNT` in `conductor/interface/websocket.rs` is a per-connection
constant of 128, so a client raises its own effective limit simply by opening
more connections. An operator who wants to bound how much concurrent zome-call
work a conductor accepts currently has no setting to reach for.

### Design

Carry the deadline the caller already has across the interface, and let the
conductor use it for two things.

**1. Admission.** A new optional `tuning_params.max_concurrent_zome_calls`
bounds how many zome calls one app interface runs concurrently. At the limit, a
call that declared a deadline is refused immediately with
`ExternalApiWireError::ZomeCallRefused` — an immediate, actionable refusal is
strictly better for a caller with a deadline than a queue slot it cannot use.
A call that declared **no** deadline is still queued, so the setting never
changes what an existing client sees.

**2. Bounded response.** The call is wrapped in `tokio::time::timeout` at the
app-interface boundary. On expiry the call future is dropped and
`ExternalApiWireError::ZomeCallDeadlineExceeded` is returned.

Dropping the future cancels every `await` the call was parked on. In practice
that means the database read and write permit acquisitions in
`holochain_sqlite::db::access` — plain `Semaphore::acquire` calls — release their
queue positions back to the conductor. That is where the value is: under load,
most of a slow zome call's wall clock is spent waiting for a permit, not
computing.

It does **not** interrupt a WASM function body that has already begun
executing, because that runs on `spawn_blocking` (`real_ribosome.rs`) and a
spawned blocking task cannot be cancelled. That work stays bounded by the
existing metering points. The error text says so explicitly, and the doc
comments repeat it, because a caller that reads `DeadlineExceeded` as "the call
did not happen" and retries could double-commit. The semantics are
**"I stopped waiting"**, not **"it did not run"**.

This is the same trade-off, with the same caveat, that this codebase already
accepts one layer down — see the comment above the `tokio::time::timeout` in
`DbRead::read_async`. This change applies the pattern at the interface boundary
where the caller's own deadline is known.

**Why the deadline is not covered by the call signature.** `ZomeCallParams` is
signed and hashed; adding a field there would change the signing input for every
client. The deadline is carried unsigned, alongside the signed params. It is
scheduling metadata about the caller's own patience, not an authorization claim:
the only party a forged deadline can disadvantage is the caller that appears to
have declared it. Neither refusal nor abandonment can grant capability, mutate
another agent's state, or bypass validation.

### API surface

**`holochain_conductor_api`**

```rust
pub enum AppRequest {
    // ... existing variants unchanged ...
    CallZomeWithDeadline {
        call: Box<ZomeCallParamsSigned>,
        deadline_ms: u32,
    },
}

pub enum ExternalApiWireError {
    // ... existing variants unchanged ...
    ZomeCallRefused(String),
    ZomeCallDeadlineExceeded(String),
}
```

Both are additive variants appended to their enums. `AppRequest` is externally
tagged (`#[serde(tag = "type", content = "value")]`), so an existing client that
never sends the new variant is byte-identical on the wire, and an existing
client that never receives the new errors is unaffected.

**`ConductorTuningParams`** — three new optional fields, all defaulting to the
current behaviour:

| Field | Default | Effect when unset |
|---|---|---|
| `zome_call_deadline: Option<Duration>` | `None` | Calls that declare no deadline are unbounded, as today |
| `zome_call_deadline_max: Option<Duration>` | 5 minutes | Clamps client-declared deadlines so a client cannot pin resources indefinitely |
| `max_concurrent_zome_calls: Option<usize>` | `None` | No admission limit, as today |

`ConductorTuningParams` is `#[serde(deny_unknown_fields)]`, so these are
additive-only: an existing config file still parses, and a config file using the
new keys against an older conductor fails loudly rather than silently ignoring
them — which is the right failure for a resource-governance setting.

**`holochain_client`** — `CallZomeOptions` gains `declare_deadline: bool`
(default `false`) and a `with_declared_deadline(timeout)` builder. When set
*and* a timeout is present, the client sends `CallZomeWithDeadline` instead of
`CallZome`. It defaults to `false` because sending the new variant to a
conductor that predates it produces a deserialization failure; the default can
flip once the supported-conductor floor moves. This is the only behavioural
opt-in a client needs, and the timeout value it sends is the one it was already
enforcing locally.

### Test plan

Unit (pure, no conductor — included in this PR):

- An unconfigured conductor returns `Admit(None)` for every call at any
  in-flight count. This is the behaviour-neutrality guarantee and is the most
  important test in the change.
- A declared deadline is honoured, and clamped to `zome_call_deadline_max`.
- The default maximum is 5 minutes when unset.
- `zome_call_deadline` applies when the caller declares nothing; an explicit
  declaration always wins.
- At `max_concurrent_zome_calls`, a call with a deadline is refused, a call
  without one is admitted.
- Setting `zome_call_deadline` makes the concurrency ceiling bite calls that
  declared nothing — the documented consequence of opting in, pinned so it
  cannot regress into a surprise.
- In-flight slots are released on drop, however the call ended.

Integration (`sweettest`, to add — see "Status" below):

- A zome function that sleeps past its deadline returns
  `ZomeCallDeadlineExceeded`, and the interface accepts further calls
  immediately afterwards (the slot was released).
- With `max_concurrent_zome_calls = 1`, a second concurrent deadlined call is
  refused with `ZomeCallRefused` while the first is still running, and is
  admitted once it finishes.
- With `max_concurrent_zome_calls = 1`, a second concurrent call with **no**
  deadline is queued and eventually succeeds.
- A call abandoned on deadline while queued for a database write permit does
  not leave the permit held: a subsequent call acquires it without waiting the
  abandoned call's remaining time.
- Wire compatibility: an `AppRequest::CallZome` encoded by the previous release
  decodes unchanged on the new conductor, and a `CallZomeWithDeadline` encoded
  by the new client fails cleanly (not ambiguously) against the old conductor.

### Migration and compatibility

- **Existing clients:** no change. They send `CallZome`, the conductor applies
  `zome_call_deadline` (unset by default), and nothing they can observe differs.
- **Existing conductors, new client:** only if the client opts in with
  `declare_deadline`. Otherwise byte-identical. Opting in against an old
  conductor produces a deserialization error, which is why the default is off.
- **Existing configs:** parse unchanged; all three keys are optional.
- **Operators:** the recommended adoption order is to set
  `zome_call_deadline_max` first (a clamp only), then observe, then set
  `max_concurrent_zome_calls`, and only then set `zome_call_deadline`, which is
  the one setting that changes behaviour for clients that asked for nothing.
- **New failure mode to document in the changelog:** a call may be abandoned
  after its WASM body has begun and therefore may still commit. Clients must
  not treat `ZomeCallDeadlineExceeded` as proof the call did not run. This is
  the single most important line in the release note.

### Patch series

1. `holochain_conductor_api`: add the `AppRequest` variant and the two
   `ExternalApiWireError` variants, fully documented. No behaviour.
2. `holochain_conductor_api`: add the three `ConductorTuningParams` fields and
   accessors, all defaulting to current behaviour. No behaviour.
3. `holochain`: `ZomeCallAdmission` + the pure `decide` predicate + unit tests.
   No wiring.
4. `holochain`: wire admission and the timeout into `AppInterfaceApi`, factoring
   the existing `CallZome` arm into a shared `handle_call_zome`.
5. `holochain`: sweettest integration scenarios.
6. `holochain_client`: `CallZomeOptions::declare_deadline` and the builder.

Commits 1–4 are the capability; 5 is the evidence; 6 is optional and could be
split into a follow-up if the maintainers prefer to move the client
independently.

### Status of this draft

Unit tests are written. **The sweettest integration scenarios in the test plan
are specified but not yet written**, and `CONTRIBUTING.md` is explicit that a
PR changing functionality needs them. This draft is not submittable until they
exist. It is recorded here in full so that the remaining work is scoped rather
than rediscovered.

The draft is written against the `holochain-0.6.3` tag. Upstream's base branch
is `develop` (`holochain` 0.8.0-dev.0, `holochain_client` 0.10.0-dev.0 as of
2026-08-08). **The port is mechanical**: every anchor point this patch touches
is byte-identical between `holochain-0.6.3` and `upstream/develop` —
`AppRequest::CallZome`'s handler arm is still at `app_interface.rs:95` with the
same body, `ExternalApiWireError` and `ConductorTuningParams` are unchanged in
shape, and `CallZomeOptions` still has exactly the one `timeout` field. The
change applies cleanly; only the tests need re-running there.

---

## 6. Decision memo — which instrument, by delivery need

One criterion, per the operator directive: *what delivers the vision.* No
ideological ordering between forking and contributing; both are instruments.

### 6.1 Timeline

| | Fork, config leg | Fork, per-call leg | Upstream PR |
|---|---|---|---|
| Work remaining | sweettest scenarios; conductor image build | + resolve the client version skew (§4.3) | + sweettests; + review cycles (the port itself is mechanical) |
| Gated on | operator image build + a config key per env | a decision about the `holochain_client` pin | upstream maintainers |
| Earliest effect | next conductor image build | not this sprint | **not on our lineage at all** |

The decisive timeline fact is the last cell, and it is *not* about porting
difficulty. Verified against `upstream/develop` on 2026-08-08: every anchor
point the patch touches is byte-identical to `holochain-0.6.3`, so the change
applies cleanly there. The obstacle is lineage, not drift — `develop` is
`holochain` 0.8.0-dev, our fleet runs 0.6.3. Even an instant, friendly merge
does not reach a running elohim conductor until we rebase the fork onto that
later lineage, which is a Wave-2-scale piece of work in its own right per the
convergence campaign. **The upstream PR cannot be the delivery instrument for
this sprint under any review pace.** That is not an argument against it; it is
an argument about which job it is for. It also means the usual reason to delay
an upstream contribution — "we'd have to port it" — does not apply here.

### 6.2 Maintenance cost

The fork currently carries four commits. This would be a fifth, touching six
files across three crates. The rebase conflict surface is small and
well-shaped — one match arm, one struct field list, one exhaustive test literal
— but `app_interface.rs`'s request-dispatch match is a file upstream does churn,
and the sweettest config literal will conflict on every upstream field addition.
Call it a modest but permanent tax, paid at every rebase.

A merged upstream capability retires that tax entirely. This is the honest
self-interested case for the PR and it is a real one — but it is a cost we stop
paying *later*, not a capability we gain *sooner*. Note the asymmetry in how the
tax accrues: the patch is cheap to carry *now* precisely because 0.6.3 and
`develop` have not diverged at these anchor points, and it gets more expensive
exactly as upstream moves. Contributing early is the cheapest moment to
contribute.

What neither instrument retires: the **in-wasm batch deadline in T1**. That is a
different layer — the zome bounding its own work from inside — and it stays
warranted regardless of what the conductor learns to do at the interface. The
two compose; one does not substitute for the other. Any framing that treats the
conductor patch as making T1 unnecessary is wrong.

### 6.3 Risk

**Fork, config leg.** The sharp edge is the abandoned-but-still-committing call
(§2 consequence 2). A conductor-wide `zome_call_deadline` applied to a fleet
would eventually abandon a slow-but-legitimate call — a first-run init, a large
commit, a cold cache — and a caller that retries on `DeadlineExceeded` could
double-commit. This is the same failure family as the `GapTracker` trap the
program plan already names for T3 (`unattempted` must go back to `pending`,
never `mark_failed`). Mitigation is a discipline, not a setting: the elohim
caller must classify `DeadlineExceeded` as **unknown outcome**, never as
failure. The admission ceiling has no such edge — a refused call demonstrably
did not run — which argues for adopting the ceiling before the default deadline.

**Fork, per-call leg.** No runtime risk; it is unreachable until the client pin
question is answered, and answering it (moving elohim-storage off
`=0.9.0-dev.24`) is a large, cross-family change with its own risk that has
nothing to do with deadlines.

**Upstream PR.** No runtime risk to us. The only cost is reviewer goodwill,
which is a resource worth spending carefully and worth not spending on a draft
whose integration tests do not exist yet.

### 6.4 The probe that decides whether the fork patch stays warranted

`elohim_head_batch_queue_wait_ms`, the histogram T3 adds, where
`queue_wait = observed RTT − extern-reported in-wasm elapsed_ms`. It isolates
conductor-side queueing from in-wasm work, which is exactly the quantity this
patch acts on.

Read it over a full quiesce window on both adam (WAN) and matthew (LAN) after
T3 and T5 have landed on alpha. Three outcomes, with the decision attached:

- **p95 `queue_wait` collapses below ~1s and the AIMD batch size converges
  high.** Batching alone removed the queueing pressure. The conductor was never
  the binding constraint; it was the round-trip count. **Do not deploy the fork
  patch.** Keep the branch, keep the upstream PR (still pro-social, still
  retires nothing we are carrying), and do not add a fifth fork commit for a
  problem the storage layer already solved.
- **p95 `queue_wait` stays above the extern budget (4s), or the AIMD converges
  to its floor (8) on adam.** Round-trips fell but the conductor is still the
  bottleneck. **Deploy the fork patch's config leg**, ceiling first, then
  default deadline, per §6.3.
- **`unattempted_total` is high while `queue_wait` is low.** The bound is
  in-wasm; the extern is running out of budget before it runs out of patience.
  Neither instrument helps. That is a T1 budget-tuning finding, and reading it
  as a conductor problem would be the misrouting the seam map warns about.

The probe must also confirm `PTxnGuard` rate stays flat — the program plan's
existing falsifiability condition. A `queue_wait` improvement bought by raising
conductor pressure is not an improvement.

### 6.5 Recommendation

**Land the fork patch as committed code. Do not deploy it yet. Finish and open
the upstream PR on its own clock.**

The two instruments are doing different jobs and neither ordering is
ideological. The upstream PR is the only path that ever retires the fork tax, it
costs nothing to have outstanding, and its merge timeline is irrelevant to this
sprint — so start it now precisely *because* it is slow. The fork patch is the
only path that could affect alpha this quarter, but its deployment decision is
cheap to defer by exactly one measurement window and expensive to reverse if it
starts abandoning legitimate calls. So it exists as a branch, ready, and the
`elohim_head_batch_queue_wait_ms` histogram decides whether it ships.

The thing this spike actually bought, independent of either instrument, is the
reframing in §1 and §2: the conductor's uncancellable call is not a wall, it is
a **missing message**. The caller already knows its deadline. Most of what a
slow call spends is cancellable waiting. That is a solvable problem at either
end, and the program plan's evidence item 4 can be updated to say so.

## 7. Verification record

Run 2026-08-08 from the submodule with `RUSTFLAGS=""` and `CARGO_TARGET_DIR` at
the cargo-pool slot for this worktree, from a cold slot. Exit codes echoed on
their own line from a redirect, never a pipe.

| Command | Exit |
|---|---|
| `cargo fmt --check -p holochain_conductor_api -p holochain -p holochain_client` | **0** |
| `cargo check -p holochain_conductor_api` | **0** |
| `cargo check -p holochain_client` | **0** |
| `cargo check -p holochain --lib --features test_utils` | **0** |
| `cargo test -p holochain --lib --features test_utils app_interface::tests` | **0** — 8 passed, 0 failed, 351 filtered out |

The eight unit tests are the ones listed in §5.4 under "Unit". The `cargo test`
run linked the full `holochain` lib test binary, so the crate compiles for real
and not only under `check`.

**What was not run, and why it matters:**

- **No `cargo build --release`.** Hours from a cold slot; out of scope for a
  spike. The release profile is not expected to differ, but that is an
  expectation, not evidence.
- **No sweettest / integration run.** The scenarios in §5.4 are *specified but
  not written*. Every runtime claim in this document — that abandoning a call
  releases a database permit, that a refusal arrives immediately, that the
  interface accepts work again after a deadline fires — rests on reading the
  code, not on observing it. They are stated as design intent and must not be
  cited as measured behaviour.
- **`cargo check -p holochain_conductor_api --all-features` fails**, at
  `tx5-connection/src/config.rs:18`. This is **pre-existing and unrelated**: the
  tx5 backend features are mutually exclusive and `--all-features` enables
  them together, against the local `[patch.crates-io]` tx5 checkout. Production
  builds use `--no-default-features --features
  sqlite-encrypted,wasmer_sys,transport-tx5-backend-go-pion,jemalloc`
  (`elohim/holochain/edgenode/Dockerfile.zombie-fix:37,63`).
- **The checks above ran under the crate's *default* features, which at 0.6.3
  means `transport-iroh`, not the tx5 set production uses.** The patch is
  entirely at the app-interface layer and touches no transport code, so this is
  not expected to matter — but the production feature combination has not been
  compiled with this patch, and the conductor image build is where that gets
  proven.

## 8. Requires an operator decision

1. **Open the upstream PR?** Requires an EthosEngine or personal GitHub
   identity, a fork of `holochain/holochain`, and the sweettest scenarios in
   §5.4 written against `develop`. Recommended yes, on its own clock.
2. **Include the `holochain_client` commit in the upstream PR, or split it?**
   Adding a public field to `CallZomeOptions` breaks direct struct-literal
   construction. Semver-minor for a 0.8.x crate, but maintainers may prefer the
   client to move separately.
3. **Deploy gate for the fork config leg** — confirm the §6.4 probe is the
   decision rule, and that the adoption order is ceiling-then-deadline.
4. **The `holochain_client` version skew** (§4.3) is a standing delivery
   constraint well beyond this patch: our runtime pins a crates.io client from
   upstream's `develop` lineage while our conductor runs a 0.6.3 fork. Anything
   we add to the app-interface protocol on the fork is unreachable from our own
   runtime until that is reconciled. Worth a backlog row of its own.
5. **Program-plan amendment.** Evidence item 4 currently reads as a hard
   constraint. §1–2 of this spike qualify it. Recommend amending it to name the
   cancellable/uncancellable split rather than leaving "no cancellation"
   standing unqualified.
