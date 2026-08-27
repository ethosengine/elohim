---
title: Doorway Auth Posture — Authority Derives From the Declared Stage
id: doorway-auth-posture-declared-stage
tier: architecture
status: accepted — Axis 1 (stage-derived seed authority + fleet seed authority), Axis 2 (the permission ladder's fallthrough + the MongoDB-outage auth downgrade) and the conductor-admin passthrough have all landed in-tree and been verified live in both postures; NOT yet measured on the fleet. The remaining named gap on this surface is handle_app_upgrade, which has no doorway-side permission check at all
created: 2026-08-25
pillar coupling: doorway (web2 projection + chaperone), elohim (peer-native substrate boundary), imagodei (identity)
informed-by:
  - The apex seed 403 (app #1672/#1673) — one deploy pipeline, several doorways, each checking its own admin identity
  - elohim/elohim-storage/src/trust/stage.rs — the NetworkStage discipline this doc brings the doorway into line with
  - genesis/a2o/LAYERS.md — the acts layering that gives each context its stage
informs:
  - Every future doorway gate — the questions in "Adding a gate" are the checklist
  - The migration from standing keys to bounded, revocable authority (delegates-compute)
  - Red-team work on the doorway's auth surface
cites:
  - "doorway-auth-refusal-runbook | the operational companion to this canon — the probes, the per-refusal decision trees, and the test that separates an authorization refusal from a dataplane divergence; this doc is the rule, that one is what to do at 2am | sha256:0929079216f0c37d | path: genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-refusal-runbook.md"
  - "trust-as-efficiency-signal | the canonical principle the NetworkStage ladder prices against — why declared stakes, not a mode flag, are what a verification-cost decision may key on; this doc applies that axis to the doorway auth surface for the first time | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "doorway-access-tier-patterns | the sibling doorway pattern catalog (tiers A / B / Recovery) that names WHO may read through the projection; this doc answers the orthogonal question of WHAT authority a caller must carry to write, and shares its chaperone framing for hosted humans | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md"
  - "substrate-trust-contract-runbook | the operating runbook whose invariants a doorway refusal must not contradict — read it when a seed or cache refusal shows up as a dataplane red, since a 403 here and a divergence there have been mistaken for each other | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
---

# Doorway Auth Posture — Authority Derives From the Declared Stage

**Audience:** anyone adding or changing a gate in `doorway-service`; red-team; operators reading a refusal.

This is the entrypoint. If you are about to write `if state.args.dev_mode` in an auth path, stop and read
"The rule" — that expression is the defect this document exists to prevent recurring.

**If you arrived here from a symptom** — a red deploy seed leg, a host serving an old bundle, a local
seed suddenly demanding a credential — you want the companion instead:
`doorway-auth-refusal-runbook` carries the probes and the per-refusal decision trees, including the
one that separates an authorization refusal from a dataplane divergence. This doc is the rule; that
one is what to do at 2am.

---

## The rule

> **A doorway's auth posture derives from the network's DECLARED operating stage, never from a mode flag.**

The declared stage is `ELOHIM_NETWORK_STAKES`, resolved **once at boot** into `AppState::network_stage`
(`doorway-service/src/routes/freshness.rs`), parsed from the exact lowercase vocabulary
`simulacra | bootstrap | coordinated | enforced`. It is the *same* `NetworkStage` type elohim-storage
declares — `seam_contracts::freshness::NetworkStage` — so the doorway and the authoritative layer cannot
drift apart on what a stage means.

Three properties come with it, and they are the reason to reuse it rather than invent a posture flag:

1. **Fail-closed.** Absent or unparseable resolves to `Bootstrap`, never to the cheapest stage.
2. **`Simulacra` is never inferred.** It is reachable *only* by an exact, positive declaration.
3. **Provenance travels.** `StageProvenance` distinguishes `OperatorConfig` from `BootstrapDefault`, so a
   reader can tell a declared stage from an applied default. It is advertised on `/status.json` and
   `/health/serving`.

`trust/stage.rs` names the doorway explicitly:

> *"`NetworkStage` must never derive from any `DEV_MODE` flag — neither elohim-storage's inert one
> (`p2p/mod.rs`) nor doorway's live, auth-permissive one (`config.rs`). Both are orthogonal runtime
> toggles; neither is the network's declared trust stage."*

That instruction predates this document. Until 2026-08-25 the vocabulary was wired into the doorway for
**freshness pricing only** and had **zero auth consumers**. The seed gate is the first.

### Why `DEV_MODE` is not a posture

`DEV_MODE: "true"` is set on **every** deployed doorway manifest — alpha, alpha-b, prod, staging,
staging-read. A flag that is true everywhere carries no information, so any gate keyed on it is, in
practice, ungated. Two live consequences have been observed:

- The seed gate and four `/admin/cache/*` mutation routes were reachable with **no credential from the
  open web** (proven 2026-08-24 on the local mesh: an anonymous `PUT /admin/seed/blob` with a deliberately
  wrong `X-Blob-Hash` answered `409 Hash mismatch` — it passed the gate and was stopped only by content
  addressing). Closed by `62b658784`.
- `extract_http_permission` granted every anonymous caller `PermissionLevel::Authenticated` when the flag
  was set. Closed 2026-08-27.
- The conductor admin socket was reachable **unauthenticated AND unfiltered** from the open internet on
  the whole fleet: three independent `dev_mode` gates (the `/hc/admin` route, the WebSocket permission
  ladder, and the proxy's message filter) were each ungated by the same always-true flag, so an anonymous
  caller could drive `install_app` / `uninstall_app` / `revoke_agent_key`. Closed 2026-08-27 — see "What
  landed".

---

## The stage ladder, mapped to our contexts

| Context | Declares | Resolves to | What that grants at the seed gate |
|---|---|---|---|
| Local mesh (`hc-mesh.sh`) | `ELOHIM_NETWORK_STAKES=simulacra` | `Simulacra` | loopback + fleet seed key |
| `just dev start` | nothing | `Bootstrap` (fail-closed) | loopback (caller is on the box) |
| alpha fleet (doorway-A + doorway-B) | nothing | `Bootstrap` | fleet seed authority (`API_KEY_SEED`) |
| staging / staging-read | nothing | `Bootstrap` | admin identity only (no seed key declared) |
| prod | nothing | `Bootstrap` | admin identity only (no seed key declared) |

The alpha fleet is deliberately **not** declared `Simulacra`. `genesis/a2o/LAYERS.md` aims Act II
(`alpha-cluster-6peer`) at `Coordinated`; declaring the fleet cheapest to make a deploy work would be
posture-washing — precisely the move `DEV_MODE: "true"` made on doorway-B. Leaving it at the fail-closed
default is the honest reading, and it costs nothing on the read path: `Simulacra` and `Bootstrap` share the
same `AmberOk` arm in `seam-contracts/src/freshness.rs`.

---

## The three developer modes

The ladder above maps DEPLOY targets. The orthogonal question — *how does a **developer** prove
authority to drive a conductor?* — has three answers, and conflating them is what left a hole open.

| Mode | Where the developer's key lives | Admission authority | Conductor path |
|---|---|---|---|
| **Native local-first** (own box, own conductor; Tauri, CLI, bare `hc-start.sh`) | on the box — and often nowhere at all, because no identity system is configured | `native_local_first_operator`: loopback peer **and** pre-coordination stage **and** no declared `JWT_SECRET` | admin WebSocket, granted `Admin` |
| **Web workspace** (Eclipse Che today, `elohim/lvi` intended) | the browser, as a session JWT minted by *this workspace's own doorway* | a valid JWT — no permission level required | `POST /hc/connect` (chaperone) |
| **e2e / CI** (a2o against a deployed doorway) | nowhere: remote by construction | `jenkins-ci@…` self-provisions an Admin account from `API_KEY_ADMIN` (`doorway-seed-ensure.sh`) | `POST /hc/connect`, or an explicit admin credential |

The three are one predicate with three carriers, not three postures. `DEV_MODE` is a carrier of none of
them.

### The web-workspace mode had no cheap arm — and that was the whole story

The conductor admin passthrough was **not an oversight**. It was a scaffold standing in for a capability
the protocol had not grown: there was no way to drive Holochain dev over the web, because a workspace
browser could present no credential the doorway would accept and — the operator's constraint — no
reproducible OAuth redirect URL is possible against `127.0.0.1`.

**The redirect constraint never bound.** It is the problem of being an OAuth *client*. This doorway is
itself an authorization server (`/auth/register`, `/auth/login`, `/auth/refresh`, `/auth/authorize` with
its own registered-client table). A workspace developer logs into *their own workspace doorway*. There is
no third party in the loop, so there is no redirect URI to reproduce.

**And the developer never needed `Admin`.** What made the admin socket look necessary is the 11-step
`connectViaAdminWs` flow, which calls `generate_agent_pub_key` (Authenticated) and
`install_app`/`enable_app` (Admin). The chaperone removes that requirement: `handle_hc_connect` gates on a
VALID JWT with **no permission-level check**, then performs cap grants, app-token issuance, and
`auto_provision` — which installs and enables the happ **server-side, under the doorway's own admin
connection**. An `Authenticated` developer gets a fully provisioned app and never opens an admin socket.

What held the workspace on the old path was one clause — `useChaperone = !isCheEnvironment() &&
!!doorwayToken` — which EXCLUDED the workspace from the chaperone, while the workspace branch of
`resolveAdminUrl` sent no query string at all. The developer's JWT already existed in the browser
(`elohim-auth-token`, minted by the workspace doorway); it simply was never attached.

### What landed (2026-08-27)

**Doorway.** Three `dev_mode` decision points collapsed to one predicate:

- `server/websocket.rs::extract_permission` — the two credential-free arms closed **together**: the
  `dev_mode` fallthrough, and the `|| !api_validator.is_configured()` disjunct that returned `Ok(Public)`
  to a caller presenting nothing whenever no API keys were configured (live even with `dev_mode` off, and
  precisely the workspace/mesh shape). Absence is now refusal, naming the chaperone in the 401.
- `server/http.rs` `/hc/admin` and the legacy `/` upgrade — route-level posture gate **deleted**. It read
  `if !dev_mode { 403 }`, so on all five deployed manifests the intended production 403 never fired and
  the message "disabled in production" described a state the fleet was never in.
- `proxy/{admin,pool,nats}.rs` — the `dev_mode` passthrough **deleted**; `filter_message` is now
  unconditional, defence in depth even for a caller who reaches the proxy with a level they should not
  have. An unparseable frame is never forwarded: an operation we cannot name is one we cannot authorize.

The replacement grant, `native_local_first_operator`, needs three conjuncts: loopback peer (kernel-observed,
never `X-Forwarded-For`) **and** pre-coordination stage **and** **no declared `JWT_SECRET`**. That last one
is what a deployment cannot fake — all five deployed manifests populate `JWT_SECRET` from a `secretKeyRef`,
and all five sit at the fail-closed `Bootstrap` stage, so **stage alone would not have saved them**.

Two adjacent fail-fasts stopped keying on `dev_mode` in the same pass, because the secure workspace posture
is the first non-`dev_mode` doorway that legitimately declares neither: MongoDB and NATS are now fatal iff
**declared** (`mongodb_is_declared` / `nats_is_declared`). The old MongoDB arm additionally claimed "a
MongoDB was declared" in an error raised when none was.

**Client.** `useChaperone` is now just `!!config.doorwayToken` — every environment with a session token
takes the chaperone. The admin WebSocket remains only for native local-first. Both workspace URL branches
now carry credentials.

**Workspace.** `hc-start.sh` resolves an auth posture from what the box actually has (`DOORWAY_AUTH`,
default `auto`):

- **secure** — mongod present: starts it, generates and persists a per-workspace `JWT_SECRET` and
  `API_KEY_ADMIN` under `elohim/holochain/local-dev/doorway/`, passes `HAPP_BUNDLE_PATH`, and does **not**
  pass `--dev-mode`. The developer registers once in the app and reaches the conductor through the
  chaperone, exactly as a hosted human does on the fleet.
- **keyless** — no mongod: native local-first, `--dev-mode` retained.

`--dev-mode` survives in exactly one honest role: a startup-time *declaration* that this is a developer's
box, which the config validator requires before letting a doorway run with **no signing secret at all**
(`!dev_mode && jwt_secret.is_none()` ⇒ refuse to start). That is fail-closed and stays. What it must never
again do is decide a per-request grant.

A per-workspace secret is not cosmetic: a doorway with no `JWT_SECRET` signs with the publicly-known dev
placeholder (`JwtValidator::new_dev`), so **turning on the chaperone without one would have been security
theatre** — anyone could forge a token and be provisioned. Giving the workspace its own identity and
retiring its local-first grant are therefore the *same act*, which is why the grant needs no flag to unset.

**Measured live, both postures** (loopback, anonymous unless noted):

| Posture | `/hc/admin` | `/health` | `POST /hc/connect` |
|---|---|---|---|
| keyless | `101` (conductor operator) | `200` | `401` |
| secure | `401` "Use POST /hc/connect." | `200` | `401` |
| secure + valid `X-API-Key` | `101` | — | — |
| secure + wrong key | `401` | — | — |

### Still open on this surface

`handle_app_upgrade` (`/hc/app/{port}` and legacy `/app/{port}`) performs **no doorway-side permission
check at all** — the only gate is the numeric port range; it relies entirely on the conductor's own
app-interface authentication. That is a different surface from the admin socket and is **not** closed by
the above. Declared here rather than left to be rediscovered.

Past that, the standing credential is itself the scaffold: the successor for all three modes is one
`delegates-compute` commitment — bounded, revocable, auditable, identity-bound, and verified by a
local indexed read over a projection the DHT signal already filled (no per-request DHT round-trip).
Tauri carries it as node-local self-operator, the devspace as its session JWT, CI as its actor bearer:
one predicate, three carriers.

---

## The worked example: the seed gate

`require_seed_authority` (`doorway-service/src/routes/seed.rs`) guards `PUT /admin/seed/blob` and the four
`/admin/cache/*` mutation routes. It admits a caller three ways:

| # | Authority | Valid when | Why it is safe |
|---|---|---|---|
| 1 | **On the box** — loopback peer | stage `< Coordinated` | Derived from the ACCEPTED SOCKET's peer address, never `X-Forwarded-For`. An attacker sets headers; they do not set the kernel's notion of who connected. Behind an ingress the peer is the ingress pod, so the fleet authenticates. |
| 2 | **The fleet's seed authority** — `API_KEY_SEED` | stage `< Coordinated` **and** a key is declared | Scoped to these routes only; never enters the permission ladder. Presence-keyed, so a doorway declaring none has no such authority at all. |
| 3 | **This doorway's operator identity** — Admin JWT or `API_KEY_ADMIN` | **every** stage | Unchanged. Identity is stage-independent. |

### Two credentials, two different questions

This distinction is the load-bearing one, and collapsing it is what stranded the apex:

- **`API_KEY_ADMIN`** answers *"is this caller **my** admin?"* — it is **this doorway's** operator identity.
  A federation fleet deliberately runs doorways with **distinct** identities (`alpha-b.yaml`: *"distinct keys
  so cross-doorway auth exercises real JWT-validation rather than shared-secret fallthroughs"*).
- **`API_KEY_SEED`** answers *"may this caller seed?"* — it is **the fleet's** deploy authority. One pipeline
  drives many doorways, so it is uniform across the fleet by design.

When one key had to answer both questions, the fleet became undeployable the moment the gate genuinely
authenticated. `API_KEY_SEED` is also **strictly narrower** than the admin key it replaces on this path: it
cannot read a user, mint a token, promote an account, or reach the conductor.

### The designed expiry

Authorities (1) and (2) switch off **by themselves** when a doorway declares `coordinated` — no flag to
remember to unset. That is not a landmine; it is the migration trigger. Past coordination, a deploy pipeline
is expected to carry bounded, revocable authority rather than a standing key.

---

## The chaperone exception

The doorway should otherwise be as narrowly concerned as possible with the web2 DNS ceremony, deriving
authority from the p2p plane and reusing the authoritative layers rather than growing parallel machinery.

**The exception is the chaperone pattern for hosted humans.** A person arriving through a browser has no
conductor of their own yet; the doorway holds their session and acts on their behalf. That is a
transitional flywheel stage — people graduating from users back to stewards — and it legitimately lives
in the doorway.

**A deploy pipeline is not a hosted human.** Infrastructure authority is not covered by the exception, which
is why `API_KEY_SEED` is documented here as transitional plumbing for a web2 ceremony (pushing SPA bytes to
a DNS-fronted host) with a declared expiry, and not as a pattern to extend.

---

## Where authority is going

In reach order — each step composes with the gate as written, so none is a rewrite:

1. **Chaperoned seed actor.** `genesis/scripts/ci/doorway-seed-ensure.sh` already registers `jenkins-ci@…`
   as a doorway-hosted account and seeds under its **bearer** — identity and audit, not a shared secret.
   `API_KEY_SEED` is exactly the fleet-uniform bootstrap that lets a pipeline do this on *any* doorway,
   after which seeding rides the actor's JWT.
2. **Bounded, revocable authority from the authoritative layer.** The REA compute-commitment path
   (`Mishpat::Commitment` + delegates-compute) already shadow-runs behind `DELEGATES_COMPUTE_OP_GATE` on
   `POST /db/content(/bulk)`, and is the canonical displacement for `X-API-Key` grants. Routing the seed
   gate through the same `authorize-operation` call makes the migration a config flip.
3. **Self-authorizing bytes.** The strongest form needs no credential at all: a `PUT` whose `X-Blob-Hash` is
   already referenced by a notarized head is materializing bytes the network has blessed — content
   addressing plus the notarized head *is* the authority (verify-locally-then-serve). It requires the
   pipeline to author and declare **before** it seeds; today it seeds first.

---

## Known open items

Named here so they are not rediscovered as surprises. None is fixed by the stage derivation above.

| Item | Where | Status |
|---|---|---|
| Anonymous callers resolve to `Authenticated` under `DEV_MODE` | `auth/http_permission.rs` | **CLOSED 2026-08-27.** Now derives from `network_stage < Coordinated && peer_is_loopback`, mirroring seed authority (1). The feared blast radius was measured and is one route: the crate has exactly ONE `Authenticated` gate (the elohim-agent invocation proxy), whose own contract already says it should refuse anonymous traffic. Content/blob/apps/cache routes carry no permission gate at all, so browsing is untouched. Proven live: remote anon → 401, loopback anon → passes, `/health` public on both. |
| Anonymous remote callers get an UNFILTERED conductor admin socket | `proxy/{admin,pool,nats}.rs`, `server/http.rs` `/hc/admin` + legacy `/`, `server/websocket.rs::extract_permission` | **CLOSED 2026-08-27.** Three independent `dev_mode` gates collapsed to one predicate (`native_local_first_operator`): the route gate deleted, the ladder's two credential-free arms closed together, proxy filtering made unconditional. The coupling that blocked this — anonymous visitors self-provisioning via `connectViaAdminWs` — was resolved on the client side instead: `useChaperone` no longer excludes the workspace, so every environment holding a session token takes `POST /hc/connect`. A caller with NO token still falls to the admin socket and is now refused unless it is a native local-first box. Proven live in both postures. |
| `handle_app_upgrade` has no doorway-side permission check | `server/websocket.rs`, `/hc/app/{port}` + legacy `/app/{port}` | **Open.** The only gate is the numeric port range; there is no `extract_permission` call on this path at all. It relies entirely on the conductor's own app-interface authentication, which is a real control but not this doorway's. Found while closing the admin socket; a different surface, deliberately not closed blind. **Sharpens under question 8:** a ward's app socket is exactly the surface that needs RELATIONAL gating, so whatever closes this must take `StewardshipGrant` into account rather than assume the caller acts on their own account. |
| A MongoDB outage was an authentication downgrade | `main.rs`, `routes/auth_routes.rs:1626` | **CLOSED 2026-08-27.** Four auth paths branch on `dev_mode && mongo.is_none()`, and the login one accepted ANY credentials and minted **Admin**. A configured-but-unreachable `MONGODB_URI` is now fatal at startup (mirroring the bootstrap-store fail-loud precedent directly above it), so `mongo.is_none()` can only mean "none configured"; and that branch's ceiling dropped to `Authenticated`. Proven live: `EXIT_CODE=1`. |
| The canonical-head declare is not seed-gated | `POST /db/content/{slug}/canonical-head` | **Open.** Through #1672–#1673 doorway-B accepted a canonical head for bytes whose `PUT` it had just refused — a declare outrunning its bytes. |
| Fleet credentials are committed in plaintext | `genesis/orchestrator/manifests/doorway/*.yaml` | **Open.** `stringData` is applied verbatim; no sealed-secret controller or injection machinery exists in the repo. |
| `/apps/{id}` bypasses the `/blob` reach gate | `routes/` | **Open.** Correct gating needs slug-vs-CID reach resolution. Tracked in `security-doorway-blob-pantry-ungated.md`. |

---

## Adding a gate

Answer these before writing the predicate. They are the questions that would have prevented the apex
outage:

1. **Whose question is it?** *"Is this caller my admin?"* and *"may this caller do X?"* are different
   questions. If one credential answers both, a fleet of more than one doorway will eventually break.
2. **What stage does this affordance belong to?** If the answer is "only when stakes are low", derive it
   from `state.network_stage` — never from `dev_mode`, and never from a new flag.
3. **Does it fail closed?** An undeclared stage must yield *more* scrutiny, not less. An undeclared
   credential must yield *no* authority, not an empty one that an empty header matches.
4. **Is it narrower than Admin?** If the caller does not need to read users or mint tokens, do not hand
   them an identity that can.
5. **Does it expire?** Say in the predicate what makes the affordance stop applying, so the scaffold
   removes itself instead of waiting to be remembered.
6. **Could the p2p plane answer this instead?** Prefer reusing the authoritative layer over growing the
   doorway. The chaperone pattern for hosted humans is the exception, not the precedent.
7. **Which developer mode is this affordance for?** Native local-first, web devspace, and e2e/CI have
   different carriers for the same authority. An affordance that only the web devspace needs must not
   be granted to the open web to reach it — that is the shape of every hole in "Known open items".
8. **Whose account is this caller acting on — their own, or someone else's?** Every gate in this crate
   currently answers *"their own"*, silently and by omission. That is an assumption, not a fact, and the
   substrate already disagrees with it (see below). Write the answer down even when it is "their own".

### Question 8 is not hypothetical — the substrate already models the other answer

The imagodei integrity zome carries a **`StewardshipGrant`** entry type (with `StewardshipAppeal`,
`DevicePolicy`, `ActivityLog`, `RelationshipRenewal`, `HumanRelationship`), and it is a far more careful
model than the "admin console over a managed account" framing suggests:

- `steward_id` + `subject_id` — **two subjects**, explicitly separated.
- `authority_basis` drawn from a closed set: `minor_guardianship`, `court_order`, `medical_necessity`,
  `community_consensus`, `organizational_role`, `mutual_consent` — with `evidence_hash` and `verified_by`.
- Capability scope as separate booleans (`content_filtering`, `time_limits`, `feature_restrictions`,
  `activity_monitoring`, `policy_delegation`) — relational authority over named surfaces, never a rank.
- **Mandatory `expires_at` AND `review_at`**, a `status` lifecycle including `revoked`, bounded
  `delegation_depth`, and an `appeal_id`: the subject has standing to contest the grant
  (`StewardshipAppeal` types: `scope`, `excessive`, `invalid_evidence`, `capability_request`).
- `DevicePolicy` composes one-way — each layer may **only ADD** restrictions, never remove a parent's.

Its own module header states the framing that must survive contact with this crate: *"This is NOT
external control — it's about identity and self-knowledge… Power scales with responsibility, not role
assignment."* A steward is accountable to the subject, not merely over them.

**The doorway knows none of this.** There is no reference to `StewardshipGrant` anywhere in
`doorway-service`. So three foreclosures are worth naming before they harden:

- **`Claims` is single-subject.** `human_id` / `agent_pub_key` / `identifier` describe one person, and
  `auto_provision` keys on `claims.identifier`. A custodial session has two subjects. Every consumer
  downstream is baking in the one-subject assumption invisibly; that is the cheapest thing to record
  now and the most expensive to retrofit later.
- **`PermissionLevel` is a total order** (`Public < Authenticated < Admin`, derived `Ord`). Steward
  authority is *relational* — authority over **this** subject's named surfaces. Adding `Steward = 3`
  would make a steward globally more powerful rather than powerful over one relationship, which is the
  same category error this document exists to unwind. **Never put custodial authority on this axis.**
- **`is_steward` is already a homonym, and the two meanings are opposites.** In `Claims` and
  `admin_users.rs` it means *self-custodial* — the human proved they hold their own key, so the doorway
  stops holding it and the pool may deprovision their hosted cell. In the imagodei zome, a steward is
  someone holding authority over **another** person. One word, two meanings, one of which is
  "maximally independent" and the other "responsible for a dependent". Do not overload it further; a
  custodial session needs its own field, not this flag.

Beware a **third** homonym pulling the same way: `Claims.session_id` is documented *"(custodial mode)"*
and `src/custodial_keys/` is "Custodial Key Management for Hosted Humans" — but both mean the doorway
holds **your own** key on your behalf until you graduate to self-custody. That is not custody *of a
person*; it is the same becoming-a-self-steward axis as `is_steward`, and it is **not** a foothold
for guardianship. Two of this crate's most natural words for the custodial-account concept are already
taken by its opposite, so a custodial-session field must be named for the relationship
(`acting_on_behalf_of` / `subject_id`), not for custody.

Ward → **self-steward** is structurally a **graduation event**, the same unbuilt
source-chain migration `admin_conductors.rs` already tracks as MongoDB flag-state for hosted users; it
is not a second system. The bounded-authority primitive is `Mishpat::Commitment` / delegates-compute,
and the constraint that has to give for custodial delegation is `performer == recipient`.

**No stub belongs in the doorway.** A steward relationship is a witnessed, revocable, DHT-notarized
fact; a placeholder here would land it in the wrong layer and invite exactly the rank-based shortcut
above. What belongs here is question 8, answered out loud in every new gate.

---

## Verification status

**Locally verified (2026-08-25):** `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check`
clean; 1116/1116 doorway lib tests; 7 new gate tests pinning the fleet-key admission, its refusal when
undeclared / blank / wrong, that it never grants general Admin, and that `Coordinated` retires both
pre-coordination affordances while an Admin identity still seeds.

**NOT measured on the fleet.** It needs an edge deploy (image and manifests land together; both half-states
are safe — an old binary ignores the new env, a new binary with no env falls back to admin identity) and
then an App run whose four `seed elohim.host` legs go green. The a2o scenario that pins it
(`@concern:federation-deploy`) is `@wip` pending its step definition.
