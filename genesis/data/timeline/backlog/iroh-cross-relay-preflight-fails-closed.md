---
id: "backlog-iroh-cross-relay-preflight-fails-closed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "kitsune2_transport_iroh fails closed on cross-relay peers — the per-doorway relay split partitions alpha's DHT (fork fix committed, ship sequence pending)"
slug: "iroh-cross-relay-preflight-fails-closed"
written: "2026-08-09"
author: "convergence final-seam shift"
status: "wip"
priority: "high"
tags: [iroh, relay, kitsune2, transport, wave2, alpha, partition, conductor-fork]

relatedNodeIds:
  - "backlog-shem-relay-dns-iroh-bypasses-hosts"
  - "backlog-susan-kitsune2-gossip-never-attempts"
  - "backlog-flip-day-relay-sovereignty-probes"
---

# The defect

`kitsune2_transport_iroh-0.4.1/src/lib.rs:762-801`,
`IrohTransport::own_url_for_preflight`. Upstream's final statement is:

```rust
warn!(%peer_url, %peer_relay, "Peer is on unknown relay, failing preflight");
None
```

The caller, `create_connection_and_context` at `src/lib.rs:896-904`, turns that
`None` into:

```rust
warn!(?remote_url, "Outbound connection attempted before relay address is known; relay registration may still be in progress");
Err(K2Error::other("Connection attempted before home relay URL is known"))
```

**The error message names a different condition than the one that fired.** It
says "we have no local URL"; the actual condition was "the peer homes to a
relay we do not home to". One overloaded string covering two unrelated causes
is why this seam survived three investigations at the wrong layer.

# Why the live evidence looked self-contradictory

Alpha, pod `elohim-eve-alpha-0`, incarnation `889aa0e5`, after the
2026-08-09T10:11Z restart that carried the operator's node-local-dns fix:

| Time | Evidence | Reads as |
|---|---|---|
| 10:11:23 | magicsock `actor.rs:994` — `home is now relay https://relay.elohim.host./, was None` | home relay CONFIRMED |
| 10:11:25 | `lib.rs:967` `do_insert_relay: relay added, local URL constructed` ×5, one per DNA space | every per-space relay inserted, `local_url` populated |
| 11:22 | `sys_validation_workflow` … `Connection attempted before home relay URL is known` at ~600/min | home relay UNKNOWN |

All three are true at once. `local_url` was known; `space_relays` held all five
spaces; the **peer** was on the other relay.

# Why alpha has two relays

Wave-2 relay sovereignty D2 (operator ruling, 2026-08-05) homes each conductor
to its **primaryDoorway's** relay:

| Doorway | Relay | Active humans |
|---|---|---|
| A (operations) | `relay.alpha.elohim.host` | matthew, jessica, james |
| B (shem/apex) | `relay.elohim.host` | adam, gertrude, susan, eve |

Rendered by `resolveRelayUrl()` in `elohim/holochain/Jenkinsfile:663-675`, gated
by `scripts/ci/validate-conductor-config.sh:103-127`. So eve (B) can preflight
adam/gertrude/susan and **fails 100% of initiations to matthew/jessica/james**.
Symmetrically for the A side. The active fleet is split 4/3 — the DHT is
partitioned along the relay line.

The design doc's own D2 rationale asserts this is safe:

> a conductor holds exactly one *home* relay …, but peers do NOT need to share
> one — a peer URL embeds *that peer's* home relay, and the dialing side
> connects through **the other peer's** relay. Heterogeneous home relays across
> a fleet are native to the protocol.

That is true of iroh and of the peer-URL encoding. It is **not** true of
`own_url_for_preflight`, which refuses before iroh is ever asked to dial. The
design doc has been corrected in place.

# Relationship to the DNS fix (this is the residual, not a regression of it)

`db5fb585b` + the operator's node-local-dns change (`e4cb2a2`, noted in
`92790a580`) cured the **home-relay** half: before it, shem conductors could not
reach `relay.elohim.host` at all, so `local_url` was genuinely `None` and the
error was literally true — matching the 100%-failure counts recorded in
`susan-kitsune2-gossip-never-attempts` (adam 57/57, eve 531/531, gertrude
538/538). After it, the same string keeps firing for the **cross-relay** half.
Same message, different cause. Expect the rate to have dropped from 100% of
initiations to roughly the cross-doorway fraction, not to zero.

# The fix (committed to the conductor fork, NOT pushed)

`elohim/holochain-conductor` @ `e4a1c9bb2` on branch `elohim-0.6.3`.

`patches/kitsune2_transport_iroh/` — the unmodified crates.io 0.4.1 tarball plus
marked `ELOHIM PATCH` hunks, wired through `[patch.crates-io]`. Vendored rather
than submoduled because `elohim/kitsune2` is pinned at v0.3.2, which predates
the `transport_iroh` crate entirely.

The preflight advertises **our own** address so the remote can address us back;
the remote reaches us through **our** home relay, which it learns from exactly
that URL. Whether the remote's own home relay is one of ours is irrelevant to
our addressability. So: fall back to the global home-relay URL instead of
refusing. `None` survives only for the genuinely address-less case — the one
case for which the caller's message is literally true. The message **string is
deliberately unchanged** so existing Loki probes keep matching; two new
`info!`/`warn!` lines discriminate the conditions.

Tests, all foreground, all EXIT echoed:

- `own_url_for_preflight_unknown_relay_returns_none` **inverted** to
  `..._falls_back_to_global`
- `..._without_any_own_url_returns_none` — the surviving `None` contract
- `..._cross_doorway_relay_split` — the live alpha shape (five per-space relays
  on `relay.elohim.host`, peer on `relay.alpha.elohim.host`)
- Both new unit tests verified **FAILING** against the unpatched function
  (2 failed / 4 passed), passing after (50 passed / 0 failed on the full lib
  suite)
- `crates/holochain/tests/iroh_stage0.rs::stage0b_cross_relay_two_doorway_relays`
  — end-to-end: one shared local bootstrap, two conductors homed to *different*
  relays, asserts both relay hosts appear in the peer store and that ops
  converge across the split. The pre-existing `stage0` test homes BOTH
  conductors to the same relay and structurally cannot catch this.

# Ship sequence

The iroh conductor does **not** come from the tx5 edgenode image. It rides in
`elohim-storage-iroh`, whose base is a one-time conductor artifact:

```
ethosengine/holochain @ elohim-0.6.3
  └─ che-devworkspaces/containers/elohim-edgenode/Dockerfile   (git clone --depth 1)
       └─ che-devworkspaces/jenkins/Jenkinsfile-elohim-edgenode
            HC_FEATURES containing `transport-iroh`
            → harbor …/elohim-edgenode-iroh:hc-elohim-0.6.3-iroh   (named anchor)
                 └─ storage-iroh/Dockerfile ARG CONDUCTOR_SOURCE
                      └─ scripts/ci/push-storage-iroh.sh  (edge pipeline, Push stage)
                           → …/elohim-storage-iroh:${STORAGE_TAG}
                                └─ resolveStorageImage() → alpha STS → deploy
```

Ordered:

1. **Push the fork.** `elohim/holochain-conductor` → `ethosengine/holochain`,
   branch `elohim-0.6.3`, commit `e4a1c9bb2`. Nothing downstream can see the fix
   until this lands; the edgenode Dockerfile clones the branch by name.
2. **Run the che-devworkspaces `elohim-edgenode` job** with
   `HC_BRANCH=elohim-0.6.3` and
   `HC_FEATURES=sqlite-encrypted,wasmer_sys,transport-iroh,jemalloc`.
   This is the only producer of `elohim-edgenode-iroh`.
3. **Push any monorepo commit tagged `[build:edge]`.** `push-storage-iroh.sh`
   rebuilds `elohim-storage-iroh:${STORAGE_TAG}` on the new conductor base and
   the deploy stage repoints alpha.
4. **Verify** with the probe below.

No monorepo Dockerfile or manifest edit is required for step 3 —
`storage-iroh/Dockerfile:50` already defaults `CONDUCTOR_SOURCE` to
`elohim-edgenode-iroh:hc-elohim-0.6.3-iroh`, and step 2 moves that tag in place.

## Operator ruling needed — the rollback anchor (design doc D9)

Step 2 **overwrites** `elohim-edgenode-iroh:hc-elohim-0.6.3-iroh`, which D9
names a rollback anchor that stays live for the wave. Two ways to keep the
contract; this is a decision, not a default:

- **(a) Harbor re-tag first.** Copy the current anchor to
  `hc-elohim-0.6.3-iroh-prepatch` before re-running, then proceed as above. No
  repo change; rollback = repoint `IROH_CONDUCTOR_SOURCE` at the preserved tag.
- **(b) New branch.** Push the same commit as `elohim-0.6.3-crossrelay`, run the
  edgenode job against it (tag becomes `hc-elohim-0.6.3-crossrelay-iroh`), then
  set `IROH_CONDUCTOR_SOURCE` (or `storage-iroh/Dockerfile`'s ARG default) to
  the new anchor. Anchor untouched; costs a branch and a monorepo ref edit that
  MUST NOT land before the edgenode job has produced the tag, or the edge Push
  stage fails.

(a) is the smaller move. Recorded here rather than chosen unilaterally because
D9 is a ratified design decision.

# Verification probe

The discriminating Loki line, `container=elohim-node`:

```
Peer homes to a relay we do not; advertising our own home-relay URL for preflight
```

Its presence means the patched conductor is live AND cross-relay peers are being
reached. Alongside it, `Connection attempted before home relay URL is known`
should fall to zero on any conductor whose home relay is confirmed — and any
residual occurrence now genuinely means "no local URL", which is what it says.

Pre-deploy, the same discrimination is available on the *unpatched* fleet: grep
for upstream's `Peer is on unknown relay, failing preflight` (`lib.rs:795`). It
sits immediately before every misleading error and names the real cause. It was
never in any probe table — added to the design doc's §5.3 table now.

# Not verified tonight (ceiling-classed)

- **Live Loki confirmation.** No observability tool surface reachable from the
  dev container this shift; the `Peer is on unknown relay` counts per pod are the
  one piece of direct live evidence still missing. Everything above is
  source-plus-render-map, which is decisive for the mechanism but does not
  measure the residual rate.
- **`stage0b` execution.** Needs both relays reachable and a full conductor test
  build (well past the shift's foreground budget). It type-checks clean
  (`cargo check -p holochain --test iroh_stage0 --features test_utils`, EXIT=0).
- **Whether the A-side shows the mirror symptom.** Expected by symmetry
  (matthew/jessica/james failing to adam/gertrude/susan/eve); unmeasured.

# Candidates ruled out at source, not by pattern-match

- **relay-less addr update clobbering `local_url` back to `None`** —
  `spawn_watch_addr_task` (`lib.rs:632-644`) writes only inside
  `if let Some(url) = get_url_with_first_relay(&addr)`. There is no `else`
  branch and no write of `None`. Refuted.
- **Watcher missing the initial value** — `create()` deliberately spawns the
  watcher *before* `insert_relay` (`lib.rs:555-584`, with the comment saying so).
  Refuted for the create-time path.
- **Write landing on a different instance than the reader** — `local_url` is one
  `Arc<RwLock<…>>` cloned into the watch task, the accept task, and every
  connection context. Refuted.
- **Trailing-dot host mismatch** — iroh's `RelayUrl` canonicalizes *both* sides
  to FQDN form, and the `url` crate strips default ports, so
  `relay_url_from_peer_url(peer)` compares equal to our own parsed relay. This
  is a real probe hazard (already documented at design doc §5.3 caveat 1) but
  not this defect.
- **Relay allowlist refusing cross-relay dialers** — both relay ConfigMaps set
  `access = "everyone"` (`manifests/doorway/alpha-b.yaml`, `alpha.yaml`), so
  nothing gates a doorway-A endpoint from connecting to doorway-B's relay. The
  fallback is reachable, not theoretical.

# Upstream

Upstream-candidate against `holochain/kitsune2`. The vendored copy keeps the
integration-test sources verbatim and marks every hunk `ELOHIM PATCH` so the
diff lifts cleanly into a PR.
