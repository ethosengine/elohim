---
title: "Plane separation — who writes on a person's chain, and where everything else goes"
id: plane-separation-design
status: Ruled
class: architecture
serves: zome-call-cost-bounded
date: 2026-10-03
context-tier: disclosed
steward: agent:integrator@claude-opus-5-5
graduation-trigger: each ruling in §3–§8 becomes a plan under plans/ when its work starts; this note retires to history with the one-line lesson per plane once every ruling is either shipped with its falsifier read, or overturned by one
cites:
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/performance-concern-index.md
  - genesis/data/timeline/backlog/conductor-cap-grant-scan-per-zome-call.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - "shem-performance-deep-dive-report-20261002 | 2026-10-02-shem-performance-deep-dive-report | sha256:df11f4c346d960e2 | path: genesis/docs/superpowers/sprints/2026-10-02-shem-performance-deep-dive-report.md"
  - "campaign-1-4-leg2-device-authorization-20261003 | Campaign 1.4 leg 2 | sha256:a8b6df7ced5151b6 | path: genesis/docs/superpowers/sprints/2026-10-03-campaign-1.4-leg2-device-authorization.md"
  - "trust-as-efficiency-signal | Trust is an Efficiency Signal | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "observation-event-layer-design | Observation/Event Layer | sha256:2b8d214094200bb6 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - elohim/holochain/.epr-meta/zome-call-cost-bounded.habit.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# Plane separation — who writes on a person's chain, and where everything else goes

**Ruled 2026-10-03.** The operator delegated the five decisions of the draft to the integrating
session with the instruction: no shortcuts, settle each design question by hypothesis and
adversarial challenge, and hold the result to the protocol's vision. This document is the result.
The draft it replaces is in git (`8d3d78785`). Two adversarial reviews overturned or reshaped every
draft ruling; where a ruling below differs from the draft, the draft was wrong and §9 says how.

Evidence classes used below: **fleet** = read on alpha on 2026-10-03 through Prometheus, Loki and
Jenkins; **code** = read in this tree or the conductor fork at `adfe3c08b`; **mesh** = the earlier
disposable three-peer measurement, whose actors are test fixtures. Nothing is derived; every figure
is copied from its source.

## 1. The rule

**A person's source chain speaks in the person's voice. It records what that person, or a delegate
under their scoped grant, stands behind. What a machine observes, repairs, obeys or nominates on its
own account does not speak there.**

The draft said the *party bound* by a machine write is the node. That is the wrong cut. A steward
answers for their node; accountability does not move. What separates is **voice, cost and
collision**: a doorway probe writes "Matthew attests doorway X is unreachable" 288 times a day, a
statement the person never made (code: the issuer is the person's agent, the doorway id is only
metadata). Three consequences were measured on the mesh — growth (72 actions an hour on lamad and
120 on infrastructure with nobody using it), price, and collision (14 of 4,000 writes lost) — and
the fleet reading in §2 shows what they cost in production.

## 2. What the fleet showed

The trigger was hosted registration returning 503 in elohim-genesis #1597, #1600 and #1603–#1608.

**Root cause: the conductors are starved by costs that grow with history, and registration fails
when its first write lands on a conductor in that state.**

| Cost | Reading | Class |
|---|---|---|
| Grant lookup on every non-author zome call | The statement still logs as slow thousands of times per 3 h per conductor after the `ab31ecf2c` roll; `rows_returned` 9,316 (eve) to 20,298 (matthew); about 2.8 s typical, 68.7 s at worst | fleet + code |
| The same lookup, across agents | `CapGrant` has no author column and its only index is on the access class, so on a per-DNA database every agent's call walks every agent's grants of that class | code |
| Storage background calls | `record_peer_status` about 50 h of conductor call time per day fleet-wide and the top source of call timeouts; `get_latest_peer_status_for_agent` and `resolve_content_head` next. Each pays the grant lookup | fleet |
| Latest-status read | `get_latest_peer_status_for_agent` fetches every status link the agent ever wrote and sorts them; the heartbeat adds 1,440 a day | code |
| Publish pass | On eve one agent's pass scans 24,264–66,387 rows in 5–19 s about once a minute | fleet |
| Starvation | eve and gertrude (1.5 CPU, 1.875 GiB) 96–100% CPU-throttled and at 95% of memory during the failures; statements returning 4–42 rows take 1–16 s | fleet |

**How starvation becomes a 503 (code, with one fleet instance).** The doorway provisions a five-cell
app, then mints a signing credential for every role cell under the steady-state 10 s per-call
deadline. The conductor's `grant_zome_call_capability` runs zome `init` and commits an
`InitZomesComplete` action before it commits the grant: two write transactions on a database shared
by every hosted agent of that DNA, whose busy timeout is 15 s. The doorway therefore gives up before
the conductor does. In #1608 the registration was provisioned on eve at 14:43:29Z and timed out at
14:43:39Z; eve published two more ops for that agent four seconds later. The keypair and secret died
with the abandoned future, so that grant is unusable; the 503 path does not deprovision, so an
enabled five-cell app is left with no account.

**What was ruled out.**

- *Accumulated hosted agents.* eve carries about 10 live hosted apps and gertrude about 13; net
  accumulation over three days was about 10 apps, with at most 8 failed registrations.
- *786 MB of conductor heap per hosted human.* Not reproduced on the fleet: eve at about 10 apps
  sits at about 1.7 GiB in total. The figure comes from an active local lane and its composition was
  never measured. It must not size any decision until §8's measurement is taken.
- *Live grant growth.* Row counts moved by 2 in 15 h. The fleet mints about 145–190 grants a day,
  about five per new registration; reuse outnumbers minting 40–100 to one. The pile is history.
- *The exporter pin.* `2dd40a9c5` was committed after #1608 ran.
- *OOM as the cause of eve's 102 restarts on Oct 1.* They were `Failed to spawn Lair keystore` two
  seconds after boot, exit 101; only the last was an OOM kill. Cause not established (§8).

**Not established.** Which step inside the 10 s overran (no log line names init or the commit);
whether the publish pass is a republish of already-published ops; any before/after comparison of
the `ab31ecf2c` roll (conductor logs were readable for Oct 3 only); why the dominant
`role 'infrastructure'` timeouts (200–520 a day, Sep 30–Oct 2) stopped at about 22:30Z on Oct 2.

## 3. Authority plane — one trust act per relationship, carried by an O(1) proof

**Gradient reading.** `trustful-self` and `trustful-declared` paying `friction-verify` per call,
`unit-history`. The compression the canon names is one trust act per relationship plus a carried
proof. That is a grant, kept, with a lookup that does not grow.

**Rejected: zero grants for first-party processes.** The hypothesis was that storage and the doorway
already hold the admin socket, so a grant adds no boundary and their calls should be authorised as
the author. It is wrong. The DNA already separates the person from a process under grant:
`content_store/src/invocation.rs` and `mishpat/src/invocation.rs` authorise the chain author acting
as themselves, or a remote agent under an assigned, function-listed, mandate-tagged grant, and
refuse everything else. Storage and the doorway hold all-function grants and are refused at those
gates today. On the author path they would pass every one silently. The admin socket's breadth is
itself a defect (below), not a licence to delete the layer beneath it. This would have skipped a
ceremony, which the gradient method forbids.

**Rulings.**

1. **The lookup is keyed by what is being looked for (fork, owed).** `CapGrant` gains `author` and
   a hash of `author ‖ secret`, indexed together with the access class. A secret-bearing call reads
   the matching grants and nothing else. Rows not yet backfilled take today's path, so correctness
   never depends on backfill completion — a lookup that matched only filled rows would deny every
   existing grant on the roll, and both first-party callers heal a rejection by minting. Every
   matching live grant is returned, because two grants may share a secret with different function
   sets. Both author-scoped revocation probes, the private-entry-only read and the decode-and-compare
   check are kept. One property changes and is accepted: an unrelated corrupt grant no longer denies
   every call. No DNA hash moves. Upstream looked grants up by secret before its storage rewrite;
   this is offered back so the fork does not carry it alone. Branch `perf/cap-grant-secret-key`.
2. **Grants stay, and first-party grants become least-privilege.** Storage and the doorway hold
   assigned grants with a listed function set, not all functions. This is the first real boundary
   these processes have had.
3. **Issuance is idempotent in the conductor.** A grant entry's hash is derived from its content, so
   `grant_zome_call_capability` returns the existing live grant for an identical entry instead of
   committing a second. A check-then-act from the client is racy by the fleet's own evidence (the
   grant landed four seconds after the timeout). Fork change.
4. **The doorway derives its credential; it does not store or re-roll it.** The signing keypair and
   secret are derived from a doorway root secret and the (conductor, cell) pair. A retry after a
   timeout, a restart and a second replica all present the same credential, so rulings 3 and 4
   together make a timed-out mint harmless. Open: which root secret, and its rotation.
5. **Revocation on security events only** — sign-out, device removal, credential heal — never on a
   timer. A grant has no expiry field, so timed rotation would only be more minting. After ruling 1
   a dead grant costs nothing per call, which removes the reason revocation was rejected on
   2026-09-20.
6. **The existing piles are retired at the recast, not pruned.** Twenty thousand deletes would be
   twenty thousand more permanent actions, so nothing is deleted in place. After ruling 1 the piles
   cost nothing per call; §11 retires the chains that carry them.

**Owed and filed, not ruled here.** The conductor admin socket is unauthenticated, accepts any
origin and is exposed as a cluster service; it is the root authority over every chain on the
conductor, and all grant discipline sits under it. A hosted human's browser keeps both halves of an
all-function credential in `localStorage`; it should be a non-extractable key and a listed grant.

## 4. Provisioning — a saga nobody waits on

**Gradient reading.** `friction-mechanical`, `lane-borrowed`: genesis and init stand where a person
waits, timed as if they were a read. A longer budget would be the limit raise the method forbids.

**Rulings.**

1. **Registration is asynchronous.** The person's request records intent and returns; the doorway
   runs the saga (key, install, enable, first credential, `create_human`, account) and the client
   reads status. Init stays lazy inside the first grant, so no fork admin call is needed.
2. **Mint for the cell being called, not for every role.** Today five inits and five grants sit on
   a request that calls one cell.
3. **Faults are typed, and "sent, outcome unknown" is never re-offered.** Each attempt generates a
   fresh agent key, the app id contains the conductor id, and an unreachable conductor is skipped
   when looking for an existing app. A re-offer after an install that landed on a sick conductor
   therefore gives one person two agent keys. The present substring matcher misses
   `Websocket closed: ConnectionClosed`; making it match without this distinction would widen that
   fork. "Never sent" may be re-offered; "sent" resolves on the same conductor first.
4. **Compensation resumes forward.** Automatic uninstall is safe before `create_human` and
   destructive after it. A reaper may remove only an app with no account, older than a bound, whose
   chain holds no Human entry.
5. **The record is a saga log, not an authority.** Cells exist: the conductor says. A human exists:
   the DHT says. Account, password and session are legitimate doorway-local state. The log must be
   rebuildable from the first two.
6. **Capacity is the conductor's to report.** The doorway's count reads 94 on adam and matthew,
   which each hold five cells, because it counts persisted mappings; one doorway marks six of seven
   conductors full on that basis. The conductor exports its own headroom and hosted-cell count, the
   doorway reads it at placement and refuses honestly. This is `conductor-capacity-represented`'s
   own retire-when.

## 5. Observation plane — a record closes a period; liveness is not a notary fact

**Gradient reading.** `trustful-self` and `trustful-declared` paying the notary's price
(`fused-planes`, `unit-history`, `lane-borrowed`) for facts superseded minutes later. Nothing reads
the device-health history (code: `get_doorway_attestations` is a stub).

**Rulings.**

1. **No DHT write while a status is unchanged.** Doorway-only.
2. **One record closes each status period**, in the shape the kind already declares
   (`device_id`, `health_metric`, `period_start`, `period_end`, sample count, summary value), with
   the maximum period taken from the manifest's graduation window, not a constant. Silence after
   `period_end` then reads "unobserved", which is the honest-absence answer. Today's per-round write
   does not match its own kind's schema and passes only because the manifest-aware floors are not
   wired.
3. **Nothing ships without a reader, so today nothing is written.** The per-round device-health
   write is retired outright: the doorway keeps its readings in memory and on the observation
   substrate, and writes no chain action for them. Ruling 2's period record lands only together with
   its reader (`get_doorway_attestations` wired) and a stated dispute procedure. An unread record
   gives honest absence to nobody, and a timer that writes it breaks `idle-is-free`.
4. **A witness is diversity, not a signature.** A lone daily digest is a self-report. The target
   already exists: observation substrate, then threshold graduation to
   `attestation:doorway-health-summary`, which is already in the lamad integrity kind list and the
   infrastructure manifest. No DNA hash moves. Node-registry's `HealthAttestation` is not the home:
   it has no period field, no validation, and would reverse the consolidation that removed the
   infrastructure DNA's own health type.
5. **The storage heartbeat splits in two.** Posture — lifecycle, pool flags, archetype — is a
   standing declaration, written on change. Liveness is not: a killed peer never writes that it
   left. Liveness moves to the substrate presence that already exists (`peer_liveness`, 35 s).
   **Order is binding:** arm that presence on iroh and define the unarmed fallback; change shard
   placement, the resilience card and network posture to read posture and substrate liveness; only
   then change the writer. Changing the writer first drops every healthy peer out of placement
   after 900 s.
6. **The latest-status read is bounded** regardless of the above. Coordinator-only.

## 6. Head plane, machine side — nominate is an authority act; obey is not a chain act

**Rulings.**

1. **Attestations and governance actions leave head adoption and re-authoring.** Their content id
   is `attest-{kind}-{issuer}-{subject}`, so each issuance stacks a root under one id and enters
   head adoption on every peer (mesh: 724 of 730 trigger lines). An attestation is immutable and its
   author is the claim; it has no head to adopt and must never be re-authored. One function derives
   the lane from the declared kind; the trigger, the retained pass and re-anchor backfill all read
   it. This replaces the draft's special case in the adoption trigger.
2. **The trigger serves two bounded lanes** — a person's publication, and background — with a
   guaranteed minimum for background, so neither starves the other.
3. **The staging tier is closed at read time.** Today any DNA member can move the staging head of
   any id and every honest peer pays the adoption work: the symmetric cost the canon warns against.
   `gather_election_candidates` admits a staging candidate only from the id's root author, a
   verified delegate, the progenitor, or a household steward allowlist. It must be read-side because
   the write-side gate is bypassable. Coordinator-only, hot-swappable.
4. **The declare guard keys on the standing election.** Staging is newest-wins, so a re-declare
   refreshes the clock and a restarting node can pull peers back to a stale head. The declare skips
   when the visible winner already names the target at this tier or higher; a machine caller that
   would re-assert a superseded head is refused. The draft's same-author-same-target guard changed
   outcomes arbitrarily.
5. **Writers are marked in the link tag.** An in-process counter cannot answer what share of a
   stopped chain a machine wrote.
6. **No node agent now; the shape is decided now.** A second five-cell agent only moves the growth
   to another chain. When machine nomination needs its own identity it is a device key under the
   person's delegation, not a peer-level agent — decided today so delegations minted meanwhile do
   not have to migrate.

**Open before sizing.** Why adopting a head needs a DHT write on a peer that did not author it. If
most machine declares are by non-authors, the writer to remove is "obey", not "restart".

## 7. Owed whatever the relationship

- **One writer per chain per node.** The doorway's chain writes go through storage's write gate.
  Retry stays as a capped backstop that yields to a person's write. `ChainTopOrdering::Relaxed` is
  stock and unused here; it fits machine writes that never use their own action hash and cannot
  apply to the head declare.
- **Publish only new ops between sweeps** (option E). Pinned as fork `901b02607` by `95a0eb298`
  on 2026-10-03, ahead of the grant lookup. The eve reading is the shape it removes; read that pass
  again once the roll settles, and before the grant-lookup pin, so the two are measured apart.
- **The conductor's histograms use the SDK's default boundaries on second-valued data**, so every
  quantile is unusable until a second-scale view is added in the fork. Read means until then. Two
  series named in repo comments do not exist at this pin.
- **Storage `database is locked`.** The `BEGIN IMMEDIATE` design exists and is unrouted.

## 8. Measurements that could overturn a ruling

| Ruling | Reading that falsifies it |
|---|---|
| §3.1 | The grant statement still appears in the slow log after the roll, or no-op call latency still tracks grant count across conductors |
| §3 rejection of author-path | After §3.1, authorisation is still a measurable share of `record_peer_status` call time |
| §3.3–4 | More than one live grant per (grantee, cell) after a doorway restart and a forced mint timeout |
| §4 | Any enabled app with no account per day, or two apps for one identifier across conductors |
| §5.1–2 | Idle lamad actions per hour do not fall from 72 to 0; links on one subject anchor exceed 100 at 30 days |
| §5.5 | Kill one of three peers: it must leave placement within 35 s, and the survivors must still be selectable at 20 min with zero infrastructure-chain actions in between |
| §6.1–2 | `dropped_full` on the adoption trigger does not rise at a declaration with the backlog present — then the lane is not the cause of the missed 75 s deadline |
| §6.4 | A:H1, B:H2, A's sweep re-offers H1: a link is written, or the winner changes |
| Hosting memory | jemalloc `allocated`, `resident`, `retained` and a heap profile at each stage for 1, 2 and 4 hosted humans on a disposable mesh, taken after §3.1. If `allocated` returns to baseline and `resident` does not, it is allocator retention |

Still unread: the campaign household's real writer mix; the cause of the Lair spawn failure; whether
anything restricts the admin socket by network policy; why every `hc-metrics` scrape target reads
down while the exporter logs that it is listening.

## 9. What the draft got wrong

- It named the node as the bound party; the separation is of voice, cost and collision.
- It called a self-signed daily digest a witness and placed it on node-registry.
- It applied the transition-only rule to the heartbeat, which would have broken placement.
- Its design-gate record said the device-health observation has no head-plane cost; the content id
  stacks roots, so it has the largest one.
- It classed the summary as attested-private; an observation about another party gives the subject
  standing to contest it, and the draft gave them no way to.
- It proposed moving the author of machine nominations where it should have asked whether the write
  should exist.
- It chose cadences by fiat where the manifest already declares windows per kind.
- It treated retry as the collision answer and the persisted ledger as the restart answer.

## 10. Order

1. Read the publish pass again on the option E roll (§7), already pinned.
2. Fork: secret-keyed grant lookup (§3.1), written as `33722d5a1` and rebased onto the option E
   pin. It pins only after an independent review and a household mesh proof.
3. Doorway, stop the damage: single-cell mint, sent/unsent fault typing, no second key (§4.2–3).
4. Fork: idempotent grant and second-scale histogram boundaries; doorway: derived credential and
   listed functions (§3.2–4).
5. Coordinator, one hot-swap: bounded latest-status read, election-keyed declare guard, closed
   staging tier, writer tag (§5.6, §6.3–5).
6. Storage: attestations out of the head plane and the two-lane trigger (§6.1–2); substrate
   liveness on iroh, then the consumers, then the heartbeat writer (§5.5).
7. Doorway and storage: period-closing health records with their reader (§5.1–3); the registration
   saga, reaper and conductor-reported capacity (§4).

## 11. Retirement — the dataplane is in development and replaceable

Operator's word, 2026-10-03: anything that needs retiring may be retired. That removes the one thing
these rulings could not repair. A chain cannot be pruned, but in development it can be replaced.

**Retired outright, as each replacement lands:**

- the per-round device-health chain write (§5.3);
- the 60 s heartbeat timer as a liveness carrier (§5.5);
- the open staging tier (§6.3);
- all-function first-party grants and the doorway's process-memory credential cache and
  `grant_memory`, superseded by derived, listed, idempotent credentials (§3.2–4);
- eager five-cell minting on a registration request (§4.2);
- node-registry's `HealthAttestation` type with its uncalled writer and reader.

**One recast, once, after the guards in §12 exist.** The household's lamad chain is mostly machine
attestations already (the review cites 27,317 of 30,787 entries), and each fleet chain carries
9,000–20,000 never-expiring all-function grants whose secrets lived in browsers and process memory.
The fleet and the household are recast onto fresh identities at a single cutover. Changes that move
a DNA hash are batched into that cutover so the hash moves once: the manifest-aware attestation
floors, a first-class soak attestation kind, removal of the dead health type, and author pinning for
head links if §6 needs it. The recast comes after the guards so the piles cannot regrow, and after
§3.1 so the cutover is measured against a fixed lookup, not confounded with it.

**Not retired by a recast.** Hosted keys sit in the pool conductor's keystore, so graduation with the
same key leaves the custodian a working copy. That needs key lineage, in production as in
development, and is not solved here.

## 12. Guards — each trap is refused where it is written

A ruling that lives only in this document will be re-broken. Each trap gets the lightest guard that
catches it at the moment it is made, using the instruments the repo already has: a typed refusal at
a choke point, a fork or crate test bound to a habit, or a directory-local authoring signal. No new
register.

| Trap | Guard | Where |
|---|---|---|
| A machine writes on a person's chain on a timer | Every chain write declares its voice (person, delegate, machine) and its trigger (act, transition, period close, timer). The write gate refuses machine + timer with a named plane-violation error, and refuses an undeclared writer outside production builds | storage `chain_write_gate`; the doorway's chain writes pass through it (§7) |
| A lookup on the per-call path loads a collection to filter it | Cost tests at two sizes, including other agents' rows in the same database, with a query-plan assertion | fork tests; `zome-call-cost-bounded` checks |
| A coordinator read grows with the agent's history | The same two-size shape for `get_links`-then-sort and whole-chain `query`; authoring signal on those call shapes | DNA `.epr-meta`; sweettest |
| A mint that can be abandoned and repeated | The conductor's grant is idempotent by entry hash; a test forces a timeout mid-mint and asserts one live grant | fork; doorway test |
| A provisioning step timed as a steady-state call, or a limit raised to fit | Authoring signal on new deadline constants and on timeouts wrapped around admin calls | doorway `.epr-meta` |
| Fault classification by rendered error text | Typed transport faults with sent/unsent; authoring signal on substring matching of error strings | doorway `.epr-meta` |
| An all-function grant to a first-party process | Authoring signal on `functions: None`; the DNA's invocation gates already refuse it at run time | doorway and storage `.epr-meta` |
| Liveness read from a notarized timestamp | Authoring signal on staleness windows over DHT timestamps; the kill-one-of-three falsifier in §8 as the habit check | storage `.epr-meta`; `idle-is-free` |
| An immutable claim treated as contested content | One kind-to-lane function; a test that an attestation kind never enters head adoption or re-authoring | storage test |
| A capacity figure counted from the doorway's own records | The conductor reports headroom; `conductor-capacity-represented` reads it | fork exporter; habit check |
| A fleet figure quoted without its source | The 786 MB figure is withdrawn from sizing until §8's measurement; `CLAUDE.md` is corrected when that reading exists | measurement |

The authoring signals are advisory in the gate's present version: they inject the rule and its reason
at the moment a matching line is written, and do not block. The typed refusals and the tests are the
enforcing half.

## 13. Footprint — the notary must be small next to what it describes

The operator's test: the genesis source set is about 40 MB on disk, so a highly available copy at
5× replication is about 200 MB in total, and the DHT carries only heads — small manifests that
describe what the blob store holds. Any peer footprint in gigabytes means something is wrong.

A read-only byte census of the three-peer specimen (`genesis/local-dev/footprint-census/`,
2026-10-03; a probe mesh, so its growth rates are not the fleet's, but its per-record costs and its
fixed costs carry over) shows the heads are there and are minute, and names what surrounds them.
One peer's conductor databases, 905 MB:

| Share | Bytes | What |
|---|---|---|
| 54% | 490 MB | compiled wasm module cache: 10 modules, about 9.6× their 55 MB source |
| 18% | 159 MB | lamad attestation writes with their framing; 99% of Content entries are device-health attestations |
| 12% | 112 MB | write-ahead logs not checkpointed down; five near-empty peer-meta stores carry 4 MB each |
| 6% | 55 MB | source wasm |
| 3% | 30 MB | heartbeat records, 6.6 KB on disk per 207-byte entry |
| 0.2% | 2 MB | capability grants |
| 0.01% | 122 KB | human-authored content entries |

Content entries are manifest-shaped: 29 of 8,738 carry an inline body, 30.7 KB in total, and no
entry exceeds 12.6 KB. The cost is framing. An action fans out into three ops, each indexed, with
1.37 validation receipts per op; receipts and ops are 62% of the DHT bytes; a 1.5 KB attestation
costs about 18 KB on disk; entries over about 1,000 bytes spill to overflow pages (27 MB of slack).
Every peer holds every action (30,254 lamad actions on all three). A manifest-only notary for the
specimen's 7,094 content ids would be about 1.8 MB; the DHT databases are 115–130× that.

**Rulings.**

1. **Zome wasm is built for size.** Only the node-registry workspace sets a size profile; lamad,
   imagodei, infrastructure and mishpat build with Cargo's defaults, and nothing runs `wasm-opt`.
   Coordinator wasm is optimised and hot-swapped now. Integrity wasm changes the DNA hash, so it
   joins the recast batch in §11. Whether the compiled cache is also the bulk of the conductor's
   1.1–1.3 GiB resident memory is the first thing §8's heap measurement must answer.
2. **Logs are checkpointed down on small and idle stores.** Owed mechanically, fork.
3. **The non-head writers leave** (§5, §6): after them, what remains on a chain is what a person
   or a delegate stands behind, and the framing cost is paid only for that.
4. **Framing and custody are the open design question.** Receipts, op fan-out and every-peer
   custody are not reached by the recast. Smallest holder set that meets the resilience need, and
   what a receipt is for once trust is declared, need their own hypothesis and challenge before
   anything is built.

**The check.** After the recast, a peer's conductor footprint, with wasm excluded and reported
separately, is a small multiple of the manifest bytes it holds. The census is the baseline.

## 14. Fleet readings after option E (2026-10-04, measuring session)

Read from the conductor's own `hc_*` series on pin `901b02607`, three hours after the last restart.
Evidence: `genesis/a2o/reports/recovery/shem-perf-deep-dive-20261002/fleet-hc-settled-901b02607/`
and `fleet-cpu-split-reading/`. They sharpen three rulings and add one.

- **The heartbeat's cost is in everyone validating it and in reading it back, not in writing it.**
  The top zome function by time fleet-wide is `get_latest_peer_status_for_agent` (3.3 calls a minute
  at a mean of 2.6 s). The top wasm call is the infrastructure integrity zome's `validate` (822 calls
  a minute, 0.31 s per second), with its `entry_defs` third; the top host function is `zome_info`
  at 775 calls a minute, almost all inside that validate. `record_peer_status` itself is 25 ms. The
  validate figure is inflated by an integration burst on eve. This is §5.5 and §5.6 measured: a
  liveness signal on the notary is paid for by every peer, and the read grows with history. It adds
  a mechanical item: `validate` re-runs `entry_defs` through `zome_info` on every call, which the
  HDK skill already names as a landmine.
- **Publish time fell on five of six conductors with option E** (matthew 0.234 → 0.132 s per
  second, gertrude 0.702 → 0.466, susan 0.369 → 0.198); eve rose under the burst.
- **System validation is never idle.** Workflow time is 1.00–1.02 s per second on all seven
  conductors, with missing dependencies at the selection cap of 10,000 on gertrude and 8,734 on
  eve. Wall clock; not shown to be CPU. Tracked in
  `alpha-conductor-sys-validation-spin-unfetchable-deps.md`.
- **CPU is partitioned wrong, not short.** Each human's envelope is split half to the conductor and
  half to storage. The conductors of gertrude, eve and james run at 96–99% of their limit while
  every storage container is throttled in under 0.3% of periods. A budget-neutral re-split is
  prepared by the measuring session and held behind the campaign's timed run. It does not replace
  §3.1: the lookup's cost is per call and grows with history whatever the limit.

One instrument limit found in the household proof: `hc_ribosome_zome_call_duration` does not
include authorisation (it read a 5.7 ms mean for calls the client waited 1.5 s on), so it cannot
be the per-call cost that `zome-call-cost-bounded`'s retire-when names. The check needs the
authorising read counted, or a client-side measure.
