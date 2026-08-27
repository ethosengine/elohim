---
title: Doorway Auth Posture — Authority Derives From the Declared Stage
id: doorway-auth-posture-declared-stage
tier: architecture
status: accepted — Axis 1 (stage-derived seed authority + fleet seed authority) and Axis 2 (the permission ladder's fallthrough + the MongoDB-outage auth downgrade) landed in-tree and locally verified, NOT yet measured on the fleet; the conductor-admin passthrough remains open and is named in Known open items
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
- `extract_http_permission` still grants every anonymous caller `PermissionLevel::Authenticated` when the
  flag is set. **Open** — see "Known open items".

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
| Anonymous remote callers get an UNFILTERED conductor admin socket | `proxy/{admin,pool,nats}.rs`, `server/http.rs:5220`/`:5261`, `server/websocket.rs:426` | **OPEN — most severe.** `if dev_mode { passthrough }` skips `filter_message` entirely, so `permission_level` is never consulted; the WS ladder returns `Ok(Public)` rather than `Err` for an anonymous caller; and the ingress is a catch-all `path: /`. Net: an anonymous internet client can reach `install_app`/`uninstall_app`/`revoke_agent_key`. NOT closed here because the deployed app's ANONYMOUS visitors use this exact socket to self-provision (`connectViaAdminWs` runs whenever no `doorwayToken` exists), so closing it alone breaks anonymous onboarding — it is coupled to migrating anonymous visitors onto the chaperone. |
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
