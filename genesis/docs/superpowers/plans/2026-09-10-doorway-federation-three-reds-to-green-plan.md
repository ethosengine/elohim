---
title: "Doorway federation — three reds to green with evidence (drain of failover · hosted-human · atom-home)"
id: doorway-federation-three-reds-to-green-plan
status: Draft
domain: D8
sprint: "unranked drain rung — not named in vision-readiness-sprint-roadmap 2026-09-10; drains three active plans; the epr-atom-home leg is verify-track"
cites:
  - "doorway-federation-failover-sprint-plan | Doorway Federation & Failover Sprint | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "hosted-human-lifecycle-e2e-plan | Hosted-human lifecycle E2E | sha256:17e945afeb8ea4ca | path: genesis/docs/superpowers/plans/2026-09-04-hosted-human-lifecycle-e2e-plan.md"
  - "epr-atom-home-frame-plan | 2026-09-02-epr-atom-home-frame-plan | sha256:939445d5c8474c3f | path: genesis/docs/superpowers/plans/2026-09-02-epr-atom-home-frame-plan.md"
  - "two-portals-sso-consolidation-design | Two portals, shared like SSO | sha256:38a88563ff60fb43 | path: genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md"
  - genesis/data/timeline/backlog/doorway-landing-humans-served-source-2026-06-23.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md
  - app/elohim-app/.epr-meta/epr-atom-home.habit.md
  - doorway/doorway-service/src/auth/http_permission.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/status.rs
  - elohim/elohim-storage/src/api/compute_grants.rs
  - genesis/a2o/features/auth/hosted-human/README.md
  - genesis/a2o/features/dataplane/doorway-apex-transition.feature
---

# Doorway federation — three reds to green with evidence

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** bring `doorway-failover`, `hosted-human-lifecycle` and `epr-atom-home` from red toward green with named evidence — a run id or a build number per flip — and nothing else.

**Architecture:** three disjoint write sets (a2o · doorway-service+doorway-app · elohim-storage) drained in four slices, then one measure slice. Hosted provisioning stops keying on `dev_mode` and keys on the declared network stage; a hosted cell becomes a notarized `delegates-compute` commitment with `scope: "hosted-cell"`; `humansServed` becomes the count of live such commitments this doorway's pool provides. No new HTTP route is added for the grant.

**Tech Stack:** Rust (doorway-service, elohim-storage; axum-style hyper dispatch, diesel projections, Holochain HDK 0.7 via `HcClient`), Angular 19 (doorway-app, elohim-app), Cucumber + Playwright (`genesis/a2o`), `just` verbs.

**Spec:** the three plans this drains, plus `genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md`. The decisions D1–D7 restated in **Global Constraints** are binding and are not re-litigated in-session.

## Position

- MAP: doorway federation + hosted identity = **D8 Web2 Projection & Doorway** ("Doorway is OPTIONAL, not architectural"; auth posture = write authority derived from declared network stage). The EPR atom home = **D1 EPR Envelope & Graph Substrate**. imagodei pillar concerns sit at D2/D4.
- Pillar walks: `doorway-service` → the doorway stanza (`public_observer/epic.md` → D8 seeds → `doorway/CLAUDE.md` → `doorway/doorway-service` + the app doorway pillar). App shell (`EprHomeComponent`, `app/elohim-app/src/app/elohim/components/epr-home/`) → the elohim protocol-core stanza.
- Gap Ledger: one STRADDLE only — `doorway-access-tier ↔ doorway-ssr-runtime` (low-risk). Nothing here crosses it.
- Roadmap: **none of these three plans or habits is named** in the 2026-09-10 vision-readiness sprint roadmap (top rungs: ⚑ formatted public-atom reading; #1 REA rails emit+graduation; #2 iroh delivery verification). This is an **unranked drain rung** — do not claim a Sprint-N.
- Composition rule, quoted from the brief: *"This sprint DRAINS three existing active plans; it does not fork them … The new plan cites all three, references their task ids where it drains them, and adds ONLY the hosted-compute-contract tasks (new) + the deploy/measure tasks that flip each habit."*
- Evidence ladder, quoted from the brief: *"Household mesh first; fleet confirms, never discovers. Every flip names a run id or build number … One push per batch; never during a build."*
- The local pair is `alpha-elohim-host` (:8888, doorway A) + `apex-elohim-host` (:8889, doorway B). `@concern:doorway-failover` runs there unchanged.

## What is already true

Three tables. **The ledger in each habit atom is the authority — a plan checkbox is not.** Where a drained plan's checkbox disagrees with the ledger or with the tree, the tree wins and the task below says so.

### `doorway-failover` (`doorway/doorway-service/.epr-meta/doorway-failover.habit.md`, status red)

| What the ledger proves landed | Evidence | What remains |
|---|---|---|
| Asset station GREEN | edge **#1450** SUCCESS, deployed **9b55e82f**; shell/SSR + unchanged-browser scenarios 3 scenarios / 24 steps | — |
| Stale slug-mapping asset bug fixed | edge **#1449**, deployed **f1191564** | — |
| Anonymous browser sibling station GREEN | run **20260909-codex-sibling-reader-delivery** (A SIGSTOP, B serves the manifesto with no credentials, A recovers) | explicitly *not* cold-boot, WAN, or apex proof |
| Runtime-endpoint station GREEN | production package **sha256-12013a92…** serves both doorways; runs 20260909-codex-runtime-endpoint-{delivery,fixture-delivery} | — |
| Native continuity chain | household recovery 10/10 (20260909-codex-failover-prepared); fixture 2/2 (20260909-codex-fixture-ready-strengthened); beacon 47/47 | — |
| Act I deliverability | run **20260909-codex-current-deliverability** — 5 scenarios / 102 steps, 0 findings | — |
| Apex transition | run **20260909-codex-apex-frontier NOT MEASURED** — 2 undefined scenarios, 14 undefined / 2 skipped steps, EXIT 1 | **Tasks 6, 17, 21** |
| Build stamp on both apex names | chief probe 2026-09-10: `/version.json` answers on `alpha.elohim.host` and `doorway-alpha.elohim.host` — commit **face02de**, buildTime 2026-09-09T13:40:18Z (the 2026-09-06 `404` is closed) | steady-state clauses need a build number of their own — **Task 19** |

**Flip requirement, verbatim from `checks:`** — *"Actual public-name transition authority: `A2O_RUN_WIP=1 just test mesh features/dataplane/doorway-apex-transition.feature`. This born-red feature must have every scenario and step passed before graduation; WIP filtering, undefined steps, pending, skips, absence, or steady-state doorway-failover.feature green cannot discharge it. It separately requires induced shed, owner-only membership withdrawal/rejoin, sibling delivery through the unchanged public name, browser bootstrap and recovery. WAN ingress continuity is a distinct prerequisite, not implied by multi-A membership."*

Consequence, stated plainly here so no task pretends otherwise: **Tasks 17 and 19 turn the steady-state clauses green; the HABIT stays red** until apex-transition passes every scenario and step *and* WAN ingress continuity is separately proven (Task 21, held).

### `hosted-human-lifecycle` (`doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md`, status red)

| Drained plan task | Ledger / tree verdict on 2026-09-10 | Where it goes |
|---|---|---|
| Task 1 — step definitions for 05-leaving | **LANDED, contrary to the plan's unticked box.** The glue lives in `genesis/a2o/steps/ui/hosted-human.steps.ts` (567 lines, close-account phrases at lines 133/264/458/472/504/564), not the `hosted-human-lifecycle.steps.ts` the plan named. Scoped dry-run: `6 scenarios (6 skipped) / 69 steps (69 skipped)` — **0 undefined**. `05-leaving.feature` carries **0 `@wip`**. | **Task 4** re-records the evidence in 5 minutes, then extends the same file |
| Task 2 — provisioning off `dev_mode` | **OPEN.** `auth_routes.rs:1081` (register/hosted) and `:1341` (login auto-provision) still read `if !state.args.dev_mode`; `:1040` / `:1303` are the singleton-recovery fallback arms | **Task 7** |
| Task 3 — `POST /auth/close-account` | **OPEN.** `grep -rn "close-account" doorway/doorway-service/src` → no match | **Task 9** |
| Task 4 — cell visibility route | **LANDED.** `GET /admin/agents/{agent_pub_key}/conductor` exists (`admin_conductors.rs:10, 66, 164`) | **Task 10** verifies only |
| Task 5 — close surface in doorway-app | **OPEN.** `grep -rn "account-close" doorway/doorway-app/src` → no match | **Task 11** |
| Task 6 — harness cleanup via the product path | **OPEN.** `auth-lifecycle.steps.ts:59` still does the admin soft-delete inside `this.onCleanup(...)` | **Task 12** |
| Task 7 — notarized closure marker | **OPEN.** `REVOCATION_REASONS` in `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2857` has no `account-closed` | **Task 13** |
| Task 8 — flip | pending everything above | **Task 20** |

**Flip requirement, verbatim from `checks:`** — *"a2o @concern:hosted-human-lifecycle (genesis/a2o/features/auth/hosted-human/05-leaving.feature — @browser-only @act:i; authority is the household lane: `just test mesh features/auth/hosted-human/05-leaving.feature`; on a deployed doorway run with ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml A2O_ALLOW_DESTRUCTIVE=0 — the story creates and removes its own human, so it is safe on a shared fleet)"*

The 2026-09-04 DELTA states the measured red: *"a fresh POST /auth/register answered with the operator's own Human profile … because every deployed doorway runs DEV_MODE=true and the hosted branch skips provisioning under it."* That is Task 7 of this plan.

### `epr-atom-home` (`app/elohim-app/.epr-meta/epr-atom-home.habit.md`, status red)

**The plan's 48 unticked boxes are stale.** The ledger's DELTA 2026-09-02b proves Slice 1 landed on dev — shas **fb0117114 … cc9cbe385**, a2o tail **b8b30686a** — covering **Tasks 2, 3, 4, 5, 6, 7 and the local half of Task 8**: `EprHomeComponent` owns `/epr/{id}`; `EprFocalComponent` extracted count-neutral (content-viewer import edge 2 → 1); four legs; designed gate; arrival from the nav stack; "Your mark"; "Open in Lamad"; `@concern:epr-atom-home` **7 passed / 0 failed twice** locally; `just gate elohim-app` green (AOT + 4656 tests).

| Plan task | Verdict | Action |
|---|---|---|
| Task 1 — brand foundation (Slice 0) | **not named in the ledger** — verify from the tree | **Task 18, Step 1** (5 minutes) |
| Tasks 2–7 | LANDED per DELTA 2026-09-02b (cite the shas) | tick with the sha line; **do not re-implement** |
| Task 8 — production build, render proof, habit delta | **local half landed; fleet half open** | **Task 18** |

**Flip requirement, verbatim from the ledger** — *"the flip to green needs a fleet render — a build number, not this note."* And from `checks:` — *"`pnpm look https://alpha.elohim.host/epr/evolution-of-trust` (genesis/a2o) — the shot carries `data-testid=epr-home` and no `viewer-back-home`"*.

Chief probe 2026-09-10: alpha is deployed at **face02de** (2026-09-09, after cc9cbe385); `pnpm look` already renders the atom-home frame (four legs, custody meter, address footer, no viewer chrome), 0 pageErrors, 0 httpErrors. The SSR HTML from `curl` carries no `data-testid` attributes, so **the flip measure is the a2o run in a browser against the deployed origin, not a curl** — a *measure* task, not a deploy task. `features/content/epr-atom-home.feature` dry-runs `10 scenarios (10 skipped) / 64 steps (64 skipped)` — 0 undefined; 3 scenarios stay `@wip` for the commons plan.

## Global Constraints

- **Decisions D1–D7 are binding.** Do not redesign them in-session; the p2p-design-gate walk that produced them is already run. They are restated where each task needs them.
- **D1 — key on declared NetworkStage, never `dev_mode`.** Precedent `doorway/doorway-service/src/auth/http_permission.rs:73` ("WHY NOT dev_mode: that flag is 'true' on every deployed manifest"), citing `2026-08-25-doorway-auth-posture-declared-stage.md`. The doorway already carries `state.network_stage` and `state.stage_provenance` (`server/http.rs:452-457`, initialised by `network_stage_at_boot()` at `:502`) and advertises them on `/status.json`.
- **D2 — a hosted cell is contracted compute.** Notarized (A), **existing** entry type, **DNA-hash-NEUTRAL**. `cid = entry hash` (`grantCid`); `action_hash` is only `dht_anchor_hash`. Provider = the steward's pool peer agent key; recipient = the hosted human's agent key. **No new HTTP route.**
- **D3 — `humansServed` is derived**, Ephemeral (C): live (unexpired, unrevoked) `hosted-cell` commitments this doorway's pool provides. Backlog **Option B**, not Option A (orchestrator heartbeat aggregate — k8s plane, rejected). A doorway hosting none shows `0`, not `—`, once the source exists. A federation-wide aggregate is a separate, separately-labelled number and is **out of scope**.
- **D4 — fixture hosted humans come from the Prologue, through the real portal** (`POST /auth/register`). They are real registrants with real cells and real commitments — **never** `humans` rows claiming presence. The `auth/hosted-human/01-06` series keeps its README discipline: its stories create and remove their **own** humans and never touch the Prologue's.
- **Sequential cargo.** The workspace RAM guard sheds concurrent doorway builds. Run one `just gate <project>` at a time. `elohim-storage` is pinned to `CARGO_BUILD_JOBS: "1"` — its gate takes a while (peak 5.4 GB, ~12% longer wall-clock than default parallelism); do not run anything else heavy beside it.
- **Never judge a cargo run from piped output.** Echo the status on its own line: `just gate doorway; echo "EXIT=$?"`. `cargo nextest` is NOT installed here — plain `cargo test`.
- **Commits are path-limited** (`git add <exact files>`; never `git add -A` — this is a shared worktree). **Never push.** Task 19 is the only push in this plan and it is the operator's act.
- **Commit trailer:** each executing session appends its own `Co-Authored-By:` and `Claude-Session:` lines exactly as its system prompt gives them.
- **Model tier (from `genesis/a2o/CLAUDE.md`):** *"Feature/scenario authoring is Opus work … Step definition wiring, fixture builders, and helper utilities are fine for Sonnet/Haiku."* Each task below names its tier.
- **Blind-reader loop is mandatory for every new `.feature`** (`genesis/a2o/.epr-meta`): dispatch the `blind-reader` agent with **the feature path only** and the `a2o-story` profile, revise, then **repeat with a fresh reader** until the reader reports READY.
- **This plan does NOT set `active:` on any habit.** The WIP fence (max 2 active) is full; promotion is the operator's call.
- **Scoping a2o runs.** `just test mesh <path-or-tag>` handles the profile-paths merge trap for you (it generates a paths-less config). A bare `npx cucumber-js -p mesh features/x.feature` runs the WHOLE tree plus that file — never do that. Ad-hoc scoping outside `just`: pass `--config` a JSON file with no `paths`, or scope by `--tags`.

---

# S1 — a2o: the stories, the glue, the cast

Six tasks. Tasks 1 and 2 are **Opus** (feature authoring). Tasks 3–6 are **Sonnet** (glue, wiring, README).

## Task 1 — `07-hosted-by-a-household.feature` (new, D5)

**Drains:** new — D5 (and it is the story that specifies D2).
**Tier:** Opus (feature authoring), then the blind-reader loop.

**Files:**
- Create: `genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature`

**Interfaces:**
- Produces the step phrases Task 4 defines in `genesis/a2o/steps/ui/hosted-human.steps.ts`.
- Produces `@concern:hosted-compute-contracted`, attached to `hosted-human-lifecycle.habit.md` `checks:` by Task 3.

- [x] **Step 1: Write the feature file**

Write exactly this to `genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature`. The file-level tags are the brief's, verbatim. The two portal-driven scenarios carry `@browser-only` at scenario level so the `mesh` profile (which excludes `@browser-only`) still runs the API-side scenarios and the `mesh-browser` profile runs the portal ones — the series is judged from two sides and the two sides run in two profiles.

```gherkin
@e2e @auth @requires:doorway @hosted-human @act:i @concern:hosted-compute-contracted
Feature: Hosted by a household — the compute a newcomer is lent is promised out loud
  As a newcomer with no computer of my own
  I want the household that runs my cell to have promised it where anyone can check
  So that being hosted is something someone undertook, not a favour that can quietly stop

  A hosted human's cell runs on somebody else's machine. Station 6 of this series says the
  doorway keeps that cell for them; this station says who is actually lending the hardware,
  and says it in the one place a promise survives the doorway that made it — the notary.

  Three words. The POOL is the set of conductors a doorway operates on other people's
  behalf; a pool conductor belongs to a household, and that household's peer holds a key.
  A COMMITMENT is a promise the network notarised: it names who promised, to whom, what,
  and until when, and anyone holding the network can read it back. DELEGATES-COMPUTE is the
  promise this story is about — "I will run your cell on my machine, within these bounds".
  Its SCOPE says what the lent compute is for; here it is "hosted-cell".

  What is being claimed, and what is not. The claim is that a newcomer who registers at a
  doorway ends up with a cell of their own AND a notarised promise naming the household that
  runs it, with an end date; and that closing the account withdraws that promise. The claim
  is NOT that the network stops holding what the human wrote — a notary keeps what it
  witnessed — nor that the household is obliged to renew. Withdrawal is honest, not amnesia.

  Judged from two sides, as this series requires. The portal side: the account page names the
  household hosting the person, in the words a person uses. The doorway side: asked directly,
  it returns the commitment, and the provider on it is the steward's key, not the doorway's
  own convenience. A portal can paint a household's name; only the notary can be asked.

  The person here is created by the story and removed by it, so this runs against the
  household mesh or a deployed doorway and leaves either as it was found. It never touches
  the Prologue's hosted humans, which belong to the humans-served story next door.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: A newcomer with no computer of their own is lent a cell, and the lending is promised
    Given a newcomer who has never registered at this doorway
    When they create an account at this doorway with the display name "Ada of no fixed machine"
    Then the doorway hosts a cell for them on one of its pool conductors
    And that cell's agent key belongs to no other account at this doorway
    And the doorway holds a live "hosted-cell" delegates-compute commitment naming that agent key as recipient
    And that commitment names a pool steward's agent key as provider
    And that commitment carries an end date in the future

  @browser-only
  Scenario: The account page says which household is hosting them
    Given a newcomer who has created an account at this doorway
    When they open their doorway account page
    Then the page names the household hosting them
    And the page names the date their hosting is promised until
    And the page never shows the raw commitment identifier as the household's name

  Scenario: Asked directly, the doorway returns the promise and not a claim about itself
    Given a newcomer who has created an account at this doorway
    When the doorway is asked for that human's hosting commitment
    Then the commitment's scope is "hosted-cell"
    And the commitment's provider is the agent key of the conductor's steward
    And the commitment's provider is not the doorway's own service identity
    And the commitment's recipient is that human's own agent key

  Scenario: Two newcomers are lent two cells under two promises
    Given a newcomer who has created an account at this doorway
    And a second newcomer who has created an account at this doorway
    Then the two humans hold different cells
    And each holds their own "hosted-cell" commitment
    And neither commitment names the other human as recipient

  @browser-only
  Scenario: Closing the account withdraws the promise
    Given a newcomer who has created an account at this doorway
    And the doorway holds a live "hosted-cell" delegates-compute commitment naming that agent key as recipient
    When they close their account through the portal
    Then no pool conductor holds a cell for that agent key
    And the doorway holds no live "hosted-cell" commitment for that agent key
    And the withdrawn commitment is still readable as a promise that was made and ended
```

- [x] **Step 2: Check it parses and count the undefined steps**

```bash
cd /projects/elohim/genesis/a2o
printf '{"default":{"requireModule":["tsx"],"require":["steps/**/*.ts"],"format":["summary"]}}' > ./.cuke-scope-tmp.json
npx cucumber-js --config ./.cuke-scope-tmp.json --dry-run features/auth/hosted-human/07-hosted-by-a-household.feature; echo "EXIT=$?"
rm -f ./.cuke-scope-tmp.json
```
Expected: `5 scenarios`, and a list of undefined steps (they are Task 4's work). A parse error is a failure of this step; undefined steps are not.

- [x] **Step 3: Blind-reader loop**

Dispatch the `blind-reader` agent with **only** this prompt content: the path `genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature` and the profile name `a2o-story`. Give it nothing else — no plan, no habit, no code. Revise the file against its report. **Then dispatch a FRESH `blind-reader`** on the revised file with the same prompt. Repeat until a fresh reader reports READY with no interpretability finding.

- [x] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature
git commit -m "story(a2o): hosted by a household — the lent compute is a notarized promise (D5)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "story 07 authored and blind-reader READY; @concern:hosted-compute-contracted attached; steps undefined until S1 Task 4."

---

## Task 2 — `doorway-humans-served.feature` (new, D5)

**Drains:** new — D3/D5. Closes the backlog atom's story half.
**Tier:** Opus (feature authoring), then the blind-reader loop.

**Files:**
- Create: `genesis/a2o/features/dataplane/doorway-humans-served.feature`

**Interfaces:**
- Consumes the Prologue cast from Task 5 (N=3 hosted registrants, one on doorway A's pool via adam, two via matthew).
- Produces `@concern:humans-served`, attached to `hosted-human-lifecycle.habit.md` `checks:` by Task 3.

- [x] **Step 1: Write the feature file**

Write exactly this to `genesis/a2o/features/dataplane/doorway-humans-served.feature`. File-level tags are the brief's, verbatim.

```gherkin
@e2e @requires:doorway @act:i @concern:humans-served
Feature: Humans served — a doorway counts the people it is actually hosting
  As someone deciding whether to trust a doorway with my identity
  I want its front page to tell me how many people it is hosting right now
  So that the number means something I could check, not a figure it inherited

  The threshold landing has always had a card called "Humans Served". Until now it showed a
  dash, because nobody had decided what it should count. This story decides it, in the only
  way this protocol can: the number is the count of live hosting promises this doorway's own
  pool has made — the "hosted-cell" commitments from the story next door. It is a doorway's
  statement about itself, derived from promises the network notarised, and it is checkable
  by anyone who can read those promises.

  Two boundaries, so the number cannot drift into a slogan. It is NOT federation-wide: a
  doorway counts its own hosting and nobody else's, and the sibling doorway's figure is its
  own. And it is NOT "people who visited": browsing anonymously is not being hosted, and a
  doorway that hosts nobody says nothing but zero.

  Zero and dash are different answers. A dash means "this doorway cannot tell you" — the
  honest answer before this story existed. Zero means "this doorway is hosting nobody", which
  is a fact. Once the source exists, a doorway that hosts nobody must say zero.

  The people this story counts come from the Prologue, and they got there the way a stranger
  would: through the doorway's own registration. This story never creates or removes them —
  except in its last scenario, which closes one through the product path and then hands the
  Prologue back its cast.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"

  Scenario: The hosting doorway's status names the number of humans it hosts
    Given the household mesh has cast its hosted humans through the doorway's own registration
    When the status of doorway "alpha" is read
    Then it names a humans-served count
    And that count equals the number of live hosted-cell commitments its pool provides

  Scenario: A doorway that hosts nobody says none, not nothing
    Given doorway "beta" hosts no humans of its own
    When the status of doorway "beta" is read
    Then it names a humans-served count of 0
    And it does not withhold the count

  Scenario: A registrant at one doorway is not counted at its sibling
    Given the household mesh has cast its hosted humans through the doorway's own registration
    When the status of doorway "alpha" is read
    And the status of doorway "beta" is read
    Then the two humans-served counts are not the same number
    And neither doorway's count includes a human hosted only by the other

  @browser-only
  Scenario: The threshold landing shows the doorway its own count
    Given the household mesh has cast its hosted humans through the doorway's own registration
    When a visitor opens the threshold landing of doorway "alpha"
    Then the humans-served card shows the same number the doorway's status names
    And the card does not show a dash

  Scenario: Closing an account through the product path drops the count by one
    Given the household mesh has cast its hosted humans through the doorway's own registration
    And the humans-served count of doorway "alpha" is recorded
    When one of those humans closes their account through the doorway's own close path
    Then the humans-served count of doorway "alpha" is one lower than it was
    And the household mesh casts that human again
    And the humans-served count of doorway "alpha" is what it was recorded as
```

- [x] **Step 2: Check it parses**

```bash
cd /projects/elohim/genesis/a2o
printf '{"default":{"requireModule":["tsx"],"require":["steps/**/*.ts"],"format":["summary"]}}' > ./.cuke-scope-tmp.json
npx cucumber-js --config ./.cuke-scope-tmp.json --dry-run features/dataplane/doorway-humans-served.feature; echo "EXIT=$?"
rm -f ./.cuke-scope-tmp.json
```
Expected: `5 scenarios`; `doorway "alpha" at "E2E_DOORWAY_ALPHA"` and `doorway "beta" at "E2E_DOORWAY_B"` resolve to the existing definition at `genesis/a2o/steps/mode-aware.steps.ts:95`; the rest are undefined (Task 4's work).

- [x] **Step 3: Blind-reader loop**

Same discipline as Task 1 Step 3: `blind-reader`, path only, `a2o-story` profile, revise, **fresh reader**, repeat until READY.

- [x] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/dataplane/doorway-humans-served.feature
git commit -m "story(a2o): humans served — a doorway counts the people it hosts (D3/D5)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "story doorway-humans-served authored and blind-reader READY; @concern:humans-served attached."

---

## Task 3 — Series README row + habit `checks:` rows

**Drains:** new — D5 bookkeeping.
**Tier:** Sonnet.

**Files:**
- Modify: `genesis/a2o/features/auth/hosted-human/README.md` (Stations table)
- Modify: `doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md` (`checks:` list only — not the body)

- [x] **Step 1: Add the station row**

In the `## Stations` table of `genesis/a2o/features/auth/hosted-human/README.md`, append one row after station 8. **Note the numbering:** the table's `#` column is a *station* number (0–8), not the file number — station 8 is `05-leaving.feature`. The new file is `07-hosted-by-a-household.feature` and it becomes station **9**:

```markdown
| 9 | Hosted by a household: whose machine runs my cell, and the promise that says so | `07-hosted-by-a-household.feature` | live (steps wired; red until the hosted-cell commitment lands) |
```

Set the State cell to `written, @wip` if Task 4 has not landed when you write this row, and to the text above once it has.

- [x] **Step 2: Attach both concerns to the habit's `checks:`**

Append two entries to the `checks:` list in `doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md` (frontmatter only; do not touch the DELTA body):

```yaml
  - "a2o @concern:hosted-compute-contracted (genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature — @act:i; household lane is the authority: `just test mesh features/auth/hosted-human/07-hosted-by-a-household.feature` for the API-side scenarios and `just test mesh-browser features/auth/hosted-human/07-hosted-by-a-household.feature` for the two @browser-only ones. The story creates and removes its own human.)"
  - "a2o @concern:humans-served (genesis/a2o/features/dataplane/doorway-humans-served.feature — @act:i; needs `just mesh prologue` to have cast the hosted humans: `just test mesh features/dataplane/doorway-humans-served.feature`. The count is doorway-local and substrate-derived; a federation-wide aggregate is a different number and is not this check.)"
```

- [x] **Step 3: Re-project and verify**

```bash
cd /projects/elohim
python3 .claude/scripts/habits-project.py; echo "EXIT=$?"
python3 .claude/scripts/habits-status.py --full | sed -n '1,40p'
```
Expected: `habits.yaml` regenerates with both new checks under `hosted-human-lifecycle`; the status render still shows it `red` and `active: false`.

- [x] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/auth/hosted-human/README.md \
        doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md \
        genesis/manifests/habits.yaml
git commit -m "habit(hosted-human-lifecycle): attach hosted-compute-contracted + humans-served checks; README station 9"
```

**Habit delta line this produces:** none yet — this is declaration, not evidence.

---

## Task 4 — Step glue: verify 05-leaving, then define the new phrases

> **Chief decisions on the S1 blind-reader findings (2026-09-11), binding for this task:**
> 1. *Notary-side read.* Story 07 scenario 3 now reads the commitment back **by cid from a household peer that is not the hosting doorway's pool** (on the mesh: jessica's storage, `:8091`, via its commitment read — the commitment is DHT-notarized, so any peer holding the mishpat cell answers). The step must NOT go through the doorway for the read-back; the doorway only supplies the cid.
> 2. *Steward-key resolution.* "the agent key that the pool conductor's own peer names as its steward" resolves from the pool peer's storage self-identity (the mesh roster / its `/p2p/status` `peerId`→agent binding), never from the doorway. Circular resolution = the assertion is void.
> 3. *Series order.* The story is station **7** (after "Being hosted", before "The operator and me"); Leaving is station 9. README renumbered; habit `checks:` cite file paths and are unaffected.


**Drains:** hosted-human plan **Task 1 verbatim** — but see the verdict below; plus new glue for Tasks 1 and 2 of this plan.
**Tier:** Sonnet (step definition wiring is explicitly Sonnet work).

**Files:**
- Verify (no edit): `genesis/a2o/steps/ui/hosted-human.steps.ts`
- Modify: `genesis/a2o/steps/ui/hosted-human.steps.ts` (phrases for `07-hosted-by-a-household.feature`)
- Create: `genesis/a2o/steps/dataplane/humans-served.steps.ts` (phrases for `doorway-humans-served.feature`)

**Interfaces:**
- Consumes: `doorway {string} at {string}` (`steps/mode-aware.steps.ts:95`); the portal primitives in `steps/ui/doorway-portal-login.steps.ts`; the close-account helper already in `steps/ui/hosted-human.steps.ts:133` (`POST {doorwayUrl}/auth/close-account`).
- Produces: nothing other tasks import — a2o step files are loaded by glob.

- [x] **Step 1: Verify hosted-human plan Task 1 (5 minutes) — do not re-implement it**

```bash
cd /projects/elohim/genesis/a2o
grep -c '@wip' features/auth/hosted-human/05-leaving.feature
printf '{"default":{"requireModule":["tsx"],"require":["steps/**/*.ts"],"format":["summary"]}}' > ./.cuke-scope-tmp.json
npx cucumber-js --config ./.cuke-scope-tmp.json --dry-run features/auth/hosted-human/05-leaving.feature
rm -f ./.cuke-scope-tmp.json
```
Expected (measured 2026-09-10): `0` `@wip`, and `6 scenarios (6 skipped) / 69 steps (69 skipped)` — **zero undefined**. That is hosted-human plan Task 1 landed, in `genesis/a2o/steps/ui/hosted-human.steps.ts` rather than the `hosted-human-lifecycle.steps.ts` the plan named. Tick that plan's Task 1 box with this evidence line and move on. **If the output shows undefined steps instead**, the glue regressed: write it into `steps/ui/hosted-human.steps.ts` per the drained plan's Task 1 text and only then continue.

Re-verified 2026-09-11 with `npx cucumber-js --dry-run --tags '@concern:hosted-human-lifecycle'` (dispatch used `--tags`, never a positional feature path, per the standing trap that a profile's `paths` merges with CLI positionals): `0` `@wip`; `6 scenarios (6 skipped) / 69 steps (69 skipped)` — zero undefined, unchanged by the new glue below.

- [x] **Step 2: Define the `07-hosted-by-a-household.feature` phrases**

Append to `genesis/a2o/steps/ui/hosted-human.steps.ts`. Reuse, do not re-mint: the registration helper the file already uses for `05-leaving`, and the close-account helper at line 133. New definitions needed, one per undefined phrase from Task 1 Step 2 — for the doorway-side assertions, read the commitment through the doorway's existing proxy of storage's commitment read (`GET /api/v1/commitments/{id}` by the `grantCid` the register response carries; see S2 Task 13, which puts `hostedCellGrantCid` on the register and account responses). For the pool-side assertion, read `GET /admin/agents/{agentPubKey}/conductor` (`admin_conductors.rs:164`), which is the same route `05-leaving` already uses for "no pool conductor holds a cell".

Every browser selector must be a real `data-testid` (page-model skill). The two `@browser-only` scenarios need `account-hosted-by-household` and `account-hosted-until` on the account page — S2 Task 11 adds them; agree the names here and use them verbatim on both sides.

Landed 2026-09-11, appended to `genesis/a2o/steps/ui/hosted-human.steps.ts` (no existing lines edited): all 30 undefined phrases from the dry-run against `07-hosted-by-a-household.feature` (the file had grown a blind-reader revision since Step 1 was authored — the feature text quotes `hosted-cell` scope and steward-key language verbatim, and the read-back-from-a-non-pool-peer phrasing already matches Decision 1). Two new local helpers carry the wiring the Chief decisions require, in a new file `genesis/a2o/src/framework/fixtures/hosted-cell.ts` (Decision 1: `readHostedCellCommitment(cid)` — `GET {peer}/api/v1/commitments/{cid}` on jessica's storage, never the doorway; Decision 2: `stewardAgentPubKeyForConductorOrigin` / `doorwayServiceIdentityAgentPubKey`, both riding a new `storagePeerForOrigin` export added to `household-mesh.ts`). `account-hosted-by-household` / `account-hosted-until` are declared in the file's existing local `TEST_ID` object (the established pattern here for UI still being built in parallel), matching the names above verbatim.

- [x] **Step 3: Define the `doorway-humans-served.feature` phrases**

Create `genesis/a2o/steps/dataplane/humans-served.steps.ts`. The status read is `GET {doorwayUrl}/status.json` → `humansServed`. "the number of live hosted-cell commitments its pool provides" is read the same way `07`'s doorway-side scenario reads a commitment, summed over the doorway's own hosted rows via `GET /admin/users` (admin-authorised; the household lane already holds the admin bearer — see `steps/ui/hosted-human.steps.ts` for how it obtains one). The threshold-landing scenario asserts on the landing's existing card; add a `data-testid` if the card has none, agreed with S2 Task 14.

Landed 2026-09-11 in a new file, `genesis/a2o/steps/dataplane/humans-served.steps.ts` (step files are loaded by glob and do not import each other, so its small amount of local state/helpers is its own — it reuses only the shared `hosted-cell.ts` fixture primitives, same as `hosted-human.steps.ts`). Independent verification of "that count equals the number of live hosted-cell commitments" walks the Prologue roster and reads each live entry's commitment back from jessica's storage — the same Decision-1 peer, not `GET /admin/users` as originally sketched here, since the roster already carries each entry's `hostedCellGrantCid` and a peer-side notary read is the more direct, doorway-independent check. The threshold-landing card's `data-testid` is `landing-humans-served` (declared locally in the new step file per the same "UI is being built in parallel" convention `hosted-human.steps.ts` already uses) — agreed here for S2 Task 14 to use verbatim. Task 5's `seed-hosted-humans.ts` landed in this same window (concurrently, on `dev`); its actual `{registrants:[...]}` roster shape — a fixed `DEFAULT_PASSWORD` for every registrant (`'Prologue2026!'`), a `'-'` sentinel for an unset `agentPubKey`/`conductorId`/`hostedCellGrantCid`, and a `result` outcome (`registered`/`exists`/`no-pool`/`unreachable`/`failed`) marking a soft-failed row — differed from this step's first-pass guess (a per-entry `password` field, no outcome filter); the glue was corrected to match it byte-for-byte (`fix(a2o): align humans-served roster reader with the landed seed-hosted-humans.ts shape`).

- [x] **Step 4: Dry-run both new features to zero undefined**

```bash
cd /projects/elohim/genesis/a2o
printf '{"default":{"requireModule":["tsx"],"require":["steps/**/*.ts"],"format":["summary"]}}' > ./.cuke-scope-tmp.json
npx cucumber-js --config ./.cuke-scope-tmp.json --dry-run \
  features/auth/hosted-human/07-hosted-by-a-household.feature \
  features/dataplane/doorway-humans-served.feature
rm -f ./.cuke-scope-tmp.json
```
Expected: `10 scenarios`, **0 undefined**.

Run 2026-09-11 with `--tags '@concern:hosted-compute-contracted or @concern:humans-served'` instead of the config-tmp/positional-path form above (dispatch instructions: a positional feature path merges with the default profile and runs the whole suite — use `--tags`): `10 scenarios (10 skipped)` / `77 steps (77 skipped)` — zero undefined. Re-ran `--tags '@concern:hosted-human-lifecycle'` alongside it: `6 scenarios (6 skipped)` / `69 steps (69 skipped)` — 05-leaving's glue is unchanged. `pnpm exec tsc --noEmit -p tsconfig.json` — clean, `EXIT=0`.

- [ ] **Step 5: Lint the Gherkin and run the a2o unit gate**

```bash
cd /projects/elohim
just gate genesis-a2o; echo "EXIT=$?"
```
Expected: `EXIT=0`.

Not run in this pass — the dispatching instructions for this slice named three verifications only (both dry-runs + `tsc --noEmit`, above) and explicitly withheld the live suite; `just gate genesis-a2o` is left for whoever next touches this tree to run before it lands on `dev`/`main`.

- [x] **Step 6: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/steps/ui/hosted-human.steps.ts genesis/a2o/steps/dataplane/humans-served.steps.ts \
        genesis/a2o/src/framework/fixtures/hosted-cell.ts genesis/a2o/src/framework/fixtures/household-mesh.ts
git commit -m "feat(a2o): step glue for hosted-compute-contracted and humans-served (runs red until S2/S3)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "hosted-human plan Task 1 verified LANDED (05-leaving: 0 @wip, 0 undefined, scoped dry-run 2026-09-10); glue for stories 07 and humans-served defined, both dry-run 0 undefined."

---

## Task 5 — Prologue casts N=3 hosted humans through the real portal (D4)

**Drains:** new — D4.
**Tier:** Sonnet.

**Files:**
- Create: `genesis/seeder/src/seed-hosted-humans.ts`
- Modify: `app/elohim-app/scripts/hc-mesh-prologue.sh` (add one `run_seed_leg` in section 5, after `seed-humans`)

**Interfaces:**
- Consumes: `DOORWAY_A_URL`, `DOORWAY_B_URL` (already exported by the Prologue); the doorway's `POST /auth/register` with `agencyPhase` hosted.
- Produces: three hosted registrants whose identifiers Task 4's steps read back; and, once S2 Task 13 lands, three live `hosted-cell` commitments.

- [x] **Step 1: Read how the Prologue already registers through the portal**

```bash
cd /projects/elohim
sed -n '300,345p' app/elohim-app/scripts/hc-mesh-prologue.sh
grep -n "auth/register" genesis/seeder/src/seed-humans.ts | head
```
Expected: the section-5 comment block explaining why `seed-conductor-identities` runs before `seed-humans`, and `seed-humans.ts`'s own `POST /auth/register` call. Your new leg must run **after** `seed-humans` for the same reason the comment gives: each household conductor must embody its own canonical `human-<name>` id before a hosted registration can mint a UUID onto it.

- [x] **Step 2: Write the caster**

`genesis/seeder/src/seed-hosted-humans.ts` — three registrants, idempotent (a re-run against an existing identifier reports `exists` and does not fail), through `POST {DOORWAY_URL}/auth/register` with the hosted agency phase, exactly the call a stranger's browser makes:

| identifier | doorway | intended pool |
|---|---|---|
| `prologue-hosted-1` | `DOORWAY_A_URL` | adam's pool conductor |
| `prologue-hosted-2` | `DOORWAY_A_URL` | matthew's pool conductor |
| `prologue-hosted-3` | `DOORWAY_A_URL` | matthew's pool conductor |

All three register at doorway **A** so the "sibling's count is its own" scenario has a doorway (B) hosting none. Print one line per registrant: `identifier`, `agentPubKey`, `conductorId`, and `hostedCellGrantCid` when the response carries one (it will once S2 Task 13 lands; before that, print `-`). Write the roster to `${MESH_DIR}/prologue-hosted-humans.json` so Task 4's steps read it rather than guessing names.

- [x] **Step 3: Wire the leg**

In `app/elohim-app/scripts/hc-mesh-prologue.sh`, immediately after the `seed-humans` leg:

```bash
run_seed_leg "seed-hosted-humans" soft \
  'DOORWAY_URL="$DOORWAY_A_URL" MESH_DIR="$MESH_DIR" npx tsx src/seed-hosted-humans.ts'
```
`soft`, not `hard`: a doorway with no pool must not fail the whole Prologue.

- [x] **Step 4: Prove it against a running mesh**

```bash
cd /projects/elohim
just mesh start && just mesh wait
just mesh prologue 2>&1 | tee /tmp/prologue-hosted.log | grep -A6 "seed-hosted-humans"
echo "EXIT=$?"
cat /tmp/elohim-local-mesh/prologue-hosted-humans.json
curl -s http://localhost:8888/status.json | python3 -m json.tool | grep -i humansServed
```
Expected: three registrant lines with three **distinct** `agentPubKey` values (this is also the first live proof of S2 Task 7 — before that task lands, expect three registrations sharing one key, which is the measured red). `humansServed` is `null` until S2 Task 14.

**Not run in this session** — the executing session's own instructions explicitly withheld a live-mesh proof ("Do NOT start a mesh and do NOT run against alpha"). Verified instead against a closed/unreachable doorway: `cd genesis/seeder && DOORWAY_URL="http://localhost:1" MESH_DIR=<scratch> npx tsx src/seed-hosted-humans.ts` prints one soft-failure line per registrant (`doorway http://localhost:1 unreachable (fetch failed) — soft failure, continuing.`), still writes the roster (3 entries, `result: "unreachable"`), and exits non-zero without a stack trace; `pnpm exec tsc --noEmit` is clean. Step 4's live-mesh proof (three distinct/shared `agentPubKey` values, `humansServed`) remains open for the next session that runs `just mesh start`.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/seeder/src/seed-hosted-humans.ts app/elohim-app/scripts/hc-mesh-prologue.sh
git commit -m "feat(prologue): cast three hosted humans through the doorway's own register path (D4)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "Prologue casts N=3 hosted registrants via POST /auth/register; roster at <mesh>/prologue-hosted-humans.json; distinct agent keys: <yes|no> on run <id>."

---

## Task 6 — Step definitions for the two apex-transition scenarios

**Drains:** doorway-failover plan **Task 1.2 (shed-vs-dead step glue)**, extended to the apex feature; and it is the first half of D7(iii).
**Tier:** Sonnet for the glue; the honest-failure shape is fixed here so no judgment call is left to the implementer.

**Files:**
- Create: `genesis/a2o/steps/dataplane/apex-transition.steps.ts`
- Do **not** modify: `genesis/a2o/features/dataplane/doorway-apex-transition.feature` (its `@wip @requires:owned-substrate` tags stay; `A2O_RUN_WIP=1` is how it runs)

**Interfaces:**
- Consumes: `signalOwnedDoorway(state, 'SIGSTOP'|'SIGCONT')` pattern from `genesis/a2o/steps/dataplane/doorway-sibling-reader.steps.ts:77`; the household fixture's `doorways.{alpha,beta,apex}` handles (written by `hc-mesh-prologue.sh` section 6); `Then('the raw response status is {int}')` and `Then('the raw response body contains {string}')`, already defined at `genesis/a2o/steps/dataplane/resiliency-saga.steps.ts:671` and `:681` — **reuse, do not redefine**.
- Produces: nothing other tasks import.

- [x] **Step 1: Confirm the current undefined count**

```bash
cd /projects/elohim/genesis/a2o
printf '{"default":{"requireModule":["tsx"],"require":["steps/**/*.ts"],"format":["summary"]}}' > ./.cuke-scope-tmp.json
npx cucumber-js --config ./.cuke-scope-tmp.json --dry-run features/dataplane/doorway-apex-transition.feature
rm -f ./.cuke-scope-tmp.json
```
Expected (measured 2026-09-10): `2 scenarios (2 undefined)` / `16 steps (14 undefined, 2 skipped)`. Those 14 are this task's work. Confirmed exactly as measured before writing the glue.

- [x] **Step 2: Define the 14 phrases — with an explicit missing-apparatus verdict**

Create `genesis/a2o/steps/dataplane/apex-transition.steps.ts` defining every phrase the dry-run listed.

The feature's own preamble is binding on how you write them: *"The household must own the public-name routing apparatus and its fault controls before exercising these scenarios … A test-only proxy that does not execute the actual routing configuration cannot certify the deployed path."* Today the household owns **no** membership authority: `relay-addr-beacon`'s `reconcile_membership` (`relay-addr-beacon/src/main.rs:315`) writes owner records through the Cloudflare sink only (`relay-addr-beacon/src/sinks/`: `cloudflare.rs`, `coturn.rs`, `pkarr.rs` — there is no household-ownable membership sink), and `app/elohim-app/scripts/hc-mesh.sh` stages no membership apparatus at all.

So write the membership steps to **fail with a named reason**, never to pass and never to return `'pending'`:

```ts
// steps that need a household-owned membership authority
function requireHouseholdMembershipAuthority(world: E2EWorld): MembershipAuthority {
  const authority = world.householdFixture?.membershipAuthority;
  if (!authority) {
    throw new Error(
      'no household-owned membership authority: relay-addr-beacon reconciles owner records ' +
      'through the Cloudflare sink only, and the household mesh stages none. This step is ' +
      'RED by missing apparatus, not by a defect in the doorways. See doorway-failover.habit.md ' +
      'checks: "WAN ingress continuity is a distinct prerequisite".'
    );
  }
  return authority;
}
```

The shed/serve steps ARE household-executable and must really run: induce the shed with the `SIGSTOP` primitive from `doorway-sibling-reader.steps.ts:77` against the fixture's `apex` doorway handle, restore with `SIGCONT`, and read the sibling through the fixture's other doorway handle. Assert the build stamp by comparing `/version.json` `commit` at both origins with the declared head, exactly as `served-shell-boots.feature` does.

**The deliverable of this task is that the feature stops being UNDEFINED and starts being MEASURED.** Turning "2 undefined scenarios, EXIT 1" into "2 scenarios failed at a named step" is the whole win here; a green is not available at home.

- [ ] **Step 3: Run it on the household pair**

```bash
cd /projects/elohim
just mesh start && just mesh wait && just mesh prologue
A2O_RUN_WIP=1 just test mesh features/dataplane/doorway-apex-transition.feature; echo "EXIT=$?"
```
Expected: `2 scenarios`, **0 undefined**, with failures naming either the missing membership authority or a real doorway defect. Record the run id printed by the runner and the report path under `genesis/a2o/reports/`.

Not run in this pass — glue-authoring lane was explicitly scoped to NOT start a mesh or run the live suite. Verified instead via `--dry-run --tags '@concern:doorway-failover'`: 0 undefined across the whole concern (25 scenarios, 230 steps), exit 0. This step remains open for the household-pair run.

- [x] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/steps/dataplane/apex-transition.steps.ts
git commit -m "test(a2o): apex-transition steps — measured red replaces undefined (D7 iii)"
```

**Habit delta line this produces:** `doorway-failover` — "apex-transition now MEASURED, not undefined: run `<id>`, 2 scenarios / 16 steps, 0 undefined, N failed at `<named step>`. RED preserved; the graduation rule is unchanged."

---

# S2 — doorway-service and doorway-app

Eight tasks. Tier: **Opus (rust-architect)** for Tasks 7, 8, 9, 13, 14; **Sonnet** for 10, 11, 12. **Sequential cargo**: `just gate doorway` after each commit-sized step, one at a time, `EXIT=$?` echoed on its own line.

## Task 7 — Provisioning stops keying on `dev_mode` (D1a)

**Drains:** hosted-human plan **Task 2 as written** — *"a pure predicate `should_provision(registry_configured, dev_mode) -> bool` pinned true for `(true, true)`. Keep the no-registry fallback (singleton `ZomeCaller`) exactly as it is for a doorway with no pool."*
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs:1081` (register/hosted), `:1341` (login auto-provision), `:1040` and `:1303` (the Simulacra fallback arms)
- Modify: `doorway/doorway-service/seam-registry.yaml` (one `decisionPoints` row)

**Interfaces:**
- Produces: `pub(crate) fn should_provision(registry_configured: bool, dev_mode: bool) -> bool` in `auth_routes.rs`. Task 13 calls the same register path immediately after provisioning succeeds.

- [x] **Step 1: Write the failing unit tests**

In `doorway/doorway-service/src/routes/auth_routes.rs`, in the module's `#[cfg(test)] mod tests`:

```rust
#[test]
fn should_provision_is_true_whenever_a_pool_is_configured() {
    assert!(super::should_provision(true, true), "dev_mode must not suppress provisioning");
    assert!(super::should_provision(true, false));
}

#[test]
fn should_provision_is_false_without_a_pool() {
    assert!(!super::should_provision(false, true));
    assert!(!super::should_provision(false, false));
}
```

- [ ] **Step 2: Run them and watch them fail** — NOT RUN. The host build guards (IO-pressure deny, then one-build-at-a-time behind another session's cargo) displaced the red run; the red is established by construction instead — the tests were written and left on disk while the predicate did not yet exist anywhere in the crate, so the compile could only have failed. The green run is the gate (`just gate doorway` EXIT=0, 2026-09-11).

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test should_provision 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL — `cannot find function 'should_provision'`.

- [x] **Step 3: Implement the predicate and use it**

```rust
/// Whether a hosted registrant gets their own provisioned cell.
///
/// WHY NOT `dev_mode`: that flag is `true` on every deployed manifest
/// (`auth/http_permission.rs:73`, citing 2026-08-25-doorway-auth-posture-declared-stage.md),
/// so keying provisioning on it made every deployed doorway hand every hosted
/// registrant the singleton conductor's existing Human — the 2026-09-04 measured red.
/// Provisioning depends on ONE fact: does this doorway operate a conductor pool?
pub(crate) fn should_provision(registry_configured: bool, dev_mode: bool) -> bool {
    let _ = dev_mode; // deliberately unread: see above
    registry_configured
}
```

Then at `:1081` and `:1341`, replace `if !state.args.dev_mode {` with `if should_provision(true, state.args.dev_mode) {` inside the `if let Some(registry) = &state.conductor_registry` arm (the arm already establishes `registry_configured == true`).

At `:1040` and `:1303`, replace `} else if state.args.dev_mode {` with a declared-stage guard so the cheap synthetic-identity fallback is reachable **only** under declared `Simulacra` **and** an unreachable imagodei zome:

```rust
} else if matches!(state.network_stage, seam_contracts::freshness::NetworkStage::Simulacra) {
```

- [x] **Step 4: Run the tests to green, then the whole gate**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test should_provision 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate doorway; echo "EXIT=$?"
```
Expected: both `EXIT=0`.

- [x] **Step 5: Add the seam-registry row**

Append to `decisionPoints:` in `doorway/doorway-service/seam-registry.yaml`, matching the existing row shape (`name`, `kind`, `sourceLocation{file,modulePath,line}`, `summary`, `concernIds`, `contractTests`):

```yaml
  - name: should_provision
    kind: pure-decision-predicate
    sourceLocation:
      file: src/routes/auth_routes.rs
      modulePath: routes::auth_routes::should_provision
      line: <line where you defined it>
    summary: >
      Whether a hosted registrant is provisioned their own cell. Depends only on whether a
      conductor pool is configured; dev_mode is explicitly not read, because it is true on
      every deployed manifest.
    concernIds:
      - id: C4
        status: answered
        justification: >
          should_provision_is_true_whenever_a_pool_is_configured pins (true,true) — the case
          that silently produced a shared singleton Human on every deployed doorway.
    contractTests:
      - path: doorway/doorway-service/src/routes/auth_routes.rs
        testName: should_provision_is_true_whenever_a_pool_is_configured
        kind: unit
      - path: doorway/doorway-service/src/routes/auth_routes.rs
        testName: should_provision_is_false_without_a_pool
        kind: unit
```

Re-run `just gate doorway; echo "EXIT=$?"` (the seam census runs inside it). Expected `EXIT=0`.

**Note on the brief's optional hardening:** a stage-gated *fallback ladder* (Simulacra → cheap synthetic identity; Bootstrap+ → refuse) beyond the single `matches!` guard above is **OPTIONAL hardening, not required by this plan**. Do not expand scope into it.

**Landed 2026-09-11 (commit `60fb28a39`):** the fallback guard landed as a named predicate
`synthetic_identity_fallback_allowed(stage)` rather than an inline `matches!`, so it is pinned by
`synthetic_identity_fallback_is_simulacra_only` and carries its own seam-registry row — the
"one-line addition WITH a test" bar. No fallback ladder was added. `auth_routes.rs` is now 5238
lines and tripped the `rs-loc-ceiling` soft nudge (3000; hard 7000) at commit time — recorded
here, not refactored mid-edit.

- [x] **Step 6: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/routes/auth_routes.rs doorway/doorway-service/seam-registry.yaml
git commit -m "fix(doorway): provision hosted cells on pool presence, not dev_mode (D1a; hosted-human Task 2)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "provisioning predicate moved off dev_mode (should_provision, seam row + 2 contract tests); `just gate doorway` EXIT=0. Unmeasured on mesh until S4."

---

## Task 8 — Signal subscriber keys on projection-writer alone (D1b)

**Drains:** new — D1b.
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `doorway/doorway-service/src/main.rs:1221`
- Modify: `doorway/doorway-service/templates/status.html:126` (the `Projection Subscribers` heading) and `:169` (the `Subscribers` stat label)
- Modify: `doorway/doorway-service/seam-registry.yaml`

**Interfaces:**
- Produces: `pub(crate) fn should_subscribe_to_signals(projection_writer: bool) -> bool` in `main.rs`.

- [x] **Step 1: Write the failing unit tests**

```rust
#[test]
fn a_projection_writer_subscribes_under_every_stage() {
    assert!(super::should_subscribe_to_signals(true));
}

#[test]
fn a_read_replica_never_subscribes() {
    assert!(!super::should_subscribe_to_signals(false));
}
```

- [ ] **Step 2: Run them and watch them fail** — NOT RUN. The host build guards (IO-pressure deny, then one-build-at-a-time behind another session's cargo) displaced the red run; the red is established by construction instead — the tests were written and left on disk while the predicate did not yet exist anywhere in the crate, so the compile could only have failed. The green run is the gate (`just gate doorway` EXIT=0, 2026-09-11).

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test should_subscribe_to_signals 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL — function not found.

- [x] **Step 3: Implement and rewire**

```rust
/// A projection WRITER subscribes to conductor signals. Stage-independent.
///
/// The old predicate was `!projection_writer || (dev_mode && !dev_signal_subscriber)`.
/// Because dev_mode is true on every deployed manifest, that dropped the signal sender at
/// boot on the fleet — the mechanism named in doorway-failover.habit.md DELTA 2026-09-04
/// ("DEV_MODE wiring drops the projection engine's signal sender at boot").
pub(crate) fn should_subscribe_to_signals(projection_writer: bool) -> bool {
    projection_writer
}
```

At `main.rs:1221`, replace the condition `if !args.projection_writer || (args.dev_mode && !args.dev_signal_subscriber)` with `if !should_subscribe_to_signals(args.projection_writer)`. Leave the inner `info!` branch that distinguishes reader mode; delete the now-unreachable dev-mode `else` message and update the surrounding comment block (`main.rs:1210-1219`) so it no longer describes dev-mode gating. Leave the `--dev-signal-subscriber` CLI flag in place as a no-op with a deprecation note in its doc comment; removing a flag is a separate change.

- [x] **Step 4: Relabel the tile**

`templates/status.html`: change the `<h2>Projection Subscribers</h2>` (line 126) to `<h2>Conductor signal subscriptions</h2>`, the HTML comment above it to match, and the stat-card label at line 169 from `Subscribers` to `Signal subscriptions`. Do not change `resource_usage.active_subscribers` — the field name stays.

- [x] **Step 5: Green the tests and gate**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test should_subscribe_to_signals 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate doorway; echo "EXIT=$?"
```
Expected: both `EXIT=0`.

- [x] **Step 6: Add the seam-registry row**

Same shape as Task 7 Step 5, `name: should_subscribe_to_signals`, `modulePath: main::should_subscribe_to_signals`, with the two tests as `contractTests`. Re-run `just gate doorway; echo "EXIT=$?"`.

- [x] **Step 7: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/main.rs doorway/doorway-service/templates/status.html \
        doorway/doorway-service/seam-registry.yaml
git commit -m "fix(doorway): a projection writer subscribes to signals under every stage (D1b)"
```

**Habit delta line this produces:** `doorway-failover` — "signal subscriber no longer suppressed by dev_mode (`should_subscribe_to_signals`, seam row + 2 contract tests); `just gate doorway` EXIT=0. The 2026-09-04 'signal sender dropped at boot' mechanism is closed in source; unmeasured on the fleet until S4."

---

## Task 9 — `POST /auth/close-account`

**Drains:** hosted-human plan **Task 3 verbatim**.
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs` (new handler)
- Modify: `doorway/doorway-service/src/routes/auth_discovery.rs` (add `closeAccount`)
- Modify: `doorway/doorway-service/src/server/http.rs` (`AUTH_OWNED_PATHS` symmetry guard + dispatch)

**Interfaces:**
- Consumes: `AgentProvisioner::deprovision_agent` (`conductor/provisioner.rs`).
- Produces: `POST /auth/close-account`, body `{ "confirmIdentifier": "<identifier>" }`, response `{ closed: bool, cellUninstalled: bool, alreadyClosed: bool }`. Task 12 and Task 13 both call it; Task 4's steps already expect it at `steps/ui/hosted-human.steps.ts:133`.

**Acceptance evidence (restated from the drained plan, not paraphrased away):** bearer-authorised; `confirmIdentifier` mismatch → `400 CONFIRMATION_MISMATCH` **and nothing changes**; on match, in this order — (1) revoke every session-transfer token and OAuth code for the human and drop the custodial key from the session cache; (2) if the row has a conductor assignment, `deprovision_agent` (uninstall + unregister), failure logged and reported in the response, **not fatal**; (3) set `is_active=false`, `metadata.is_deleted=true`, `closed_at=now`. A second call on a closed row answers `200` with `alreadyClosed: true`, **never 404**. `handle_login` already refuses inactive rows and `handle_me` already refuses suspended rows — assert both in a unit test. Declare the route in the auth discovery document and keep the `AUTH_OWNED_PATHS` symmetry guard passing.

- [x] **Step 1: Write the failing tests**

Four unit tests in `auth_routes.rs`'s test module:
`close_account_mismatched_confirmation_changes_nothing`, `close_account_is_idempotent_and_never_404s`, `closed_row_cannot_log_in`, `closed_row_is_refused_by_handle_me`. Plus extend the existing auth-discovery contract test to require the `closeAccount` key.

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test close_account 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL — handler not found.

- [x] **Step 3: Implement the handler, the discovery entry and the dispatch arm**

Follow the acceptance evidence above exactly, including the ordering and the non-fatal deprovision.

- [x] **Step 4: Green and gate**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test close_account 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate doorway; echo "EXIT=$?"
```
Expected: both `EXIT=0`, and the discovery-document contract test passes with `closeAccount` present.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/routes/auth_routes.rs \
        doorway/doorway-service/src/routes/auth_discovery.rs \
        doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): POST /auth/close-account — reclaim sessions, cell and row (hosted-human Task 3)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "close-account route landed with 4 unit tests + discovery contract; `just gate doorway` EXIT=0."

---

## Task 10 — Cell visibility: verify, then use

**Drains:** hosted-human plan **Task 4** — verified LANDED; this task confirms and wires nothing new unless the verification fails.
**Tier:** Sonnet.

**Files:**
- Verify (no edit expected): `doorway/doorway-service/src/routes/admin_conductors.rs:164`

- [x] **Step 1: Verify the route exists**

```bash
cd /projects/elohim
grep -n "admin/agents\|agent_pub_key.*conductor" doorway/doorway-service/src/routes/admin_conductors.rs | head
```
Expected (measured 2026-09-10): `GET /admin/agents/{agent_pub_key}/conductor` documented at line 10, response type at 66, handler at 164. Tick hosted-human plan Task 4 with this line.

- [x] **Step 2: If and only if it is absent, add it** — NOT NEEDED: the route is present at the measured lines (10 / 66 / 164), so this task produced no code and no commit of its own. Recorded in the Task 9 commit (`4a7145814`) instead.

Add `GET /admin/conductors/agents/{agentPubKey}` returning the registry entry or 404, with one unit test, per the drained plan's Task 4 text. Then `just gate doorway; echo "EXIT=$?"` → `EXIT=0` and commit. Otherwise this task produces no commit.

**Habit delta line this produces:** `hosted-human-lifecycle` — "hosted-human plan Task 4 verified LANDED: GET /admin/agents/{key}/conductor at admin_conductors.rs:164."

---

## Task 11 — Close-account surface in doorway-app (+ the hosted-by-a-household strip)

**Drains:** hosted-human plan **Task 5 verbatim**, extended with the two `data-testid`s story 07 needs.
**Tier:** Sonnet.

**Files:**
- Modify: `doorway/doorway-app/src/app/components/account/doorway-account.component.ts` and `.css`
- Modify: `doorway/doorway-app/src/app/components/account/doorway-account.component.spec.ts`

**Interfaces:**
- Consumes: `POST /auth/close-account` (Task 9); `GET /auth/account`.
- Produces the `data-testid`s Task 4's steps assert on: `account-close-begin`, `account-close-confirm-input`, `account-close-confirm`, `account-close-error`, `account-hosted-by-household`, `account-hosted-until`.

**Acceptance evidence (restated from the drained plan):** on `/threshold/account`, a "Close this account" section below the graduation CTA, shown for every signed-in human; it explains what is reclaimed and what the network keeps, asks the human to type their identifier, calls the route, then signs out locally and navigates to `/threshold/`. Also show the display name from `GET /auth/account`; if the wire does not carry it, add `displayName` to `AccountResponse` in doorway-service (the story asserts the name the human typed).

- [x] **Step 1: Write the failing component tests**

In the spec: the section renders for a signed-in human; a wrong identifier surfaces `account-close-error` and calls nothing; the right identifier calls the route once and navigates to `/threshold/`; `account-hosted-by-household` renders the household name and `account-hosted-until` the promised-until date when the account response carries them, and both are absent (not blank) when it does not.  
  > **Corrected 2026-09-11 (chief):** the surface always POSTs and lets the doorway refuse (400 `CONFIRMATION_MISMATCH`); it does not pre-judge the identifier in the browser. Account truth is the doorway's, and the committed glue (`submitClosure` awaits the response, `the account is not closed` asserts status 400) already encodes that. The prose above was wrong, the story was right.

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim
just gate doorway-app; echo "EXIT=$?"
```
Expected: FAIL naming the new spec cases.

- [x] **Step 3: Implement**

Angular 19 standalone, `ChangeDetectionStrategy.OnPush`, async state as signals — a plain field mutated from a callback will not re-render under implicit OnPush.

- [x] **Step 4: Green the gate and look at it**

```bash
cd /projects/elohim
just gate doorway-app; echo "EXIT=$?"
cd genesis/a2o && pnpm look http://localhost:8888/threshold/account
```
Expected: `EXIT=0`; the shot at `genesis/a2o/reports/look/<slug>/shot.png` shows the close section and, once Task 13 has run against this mesh, the hosting strip.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-app/src/app/components/account/
git commit -m "feat(doorway-app): close-account surface and hosted-by-a-household strip (hosted-human Task 5)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "account page carries the close surface and the hosting strip; `just gate doorway-app` EXIT=0; render at genesis/a2o/reports/look/<slug>."

---

## Task 12 — Harness cleanup goes through the product path

**Drains:** hosted-human plan **Task 6 verbatim**.
**Tier:** Sonnet.

**Files:**
- Modify: `genesis/a2o/steps/auth-lifecycle.steps.ts:59` (the `this.onCleanup(...)` body)
- Modify: `genesis/a2o/features/browser/doorway-portal-login-neighbourhood.feature` (preamble caveat)

**Acceptance evidence (restated):** the cleanup for API-registered ephemeral humans calls `POST /auth/close-account` with that human's bearer, falling back to the admin soft-delete **only** if the route is absent; the "not swept afterwards" caveat retires from the neighbourhood feature's preamble; a full auth-lane run on the household mesh leaves the doorway's user count unchanged before and after, asserted in the run log.

- [x] **Step 1: Rewrite the cleanup** — done in `genesis/a2o/steps/auth-lifecycle.steps.ts`: the registration `When`'s `this.onCleanup(...)` now calls `closeAccountCleanup()`, which POSTs `/auth/close-account` with the registering human's own bearer (`{ confirmIdentifier }`), logs which path fired, and falls back to the pre-existing admin soft-delete **only** on `404`/`405` (route absent) or a request-level throw — never on a genuine non-2xx from the route itself, and never rethrows. Covers the `features/auth/*` auth-lane (`auth-lifecycle.feature`, `operator-onboarding.feature`, `user-management.feature`) that `just test mesh features/auth` runs.

  **Found tree/plan mismatch — feature-preamble retirement withheld, not done.** The neighbourhood feature's Background step (`a hosted human is registered on doorway "alpha"`, `features/browser/doorway-portal-login-neighbourhood.feature:23`) does **not** route through `auth-lifecycle.steps.ts` at all — it matches a differently-worded step in `genesis/a2o/steps/ui/doorway-portal-login.steps.ts:123`, whose only `onCleanup` (line 186) closes the Playwright device, never the account. So the "not swept afterwards" caveat this task names is still literally true for that Background; retiring it as this task's acceptance evidence asks would put a false claim in the feature preamble. Missing node: `chain / auth-lane product-path cleanup (auth-lifecycle.steps.ts, Act I) → ??? → neighbourhood Background registration (doorway-portal-login.steps.ts:123, Act II) / missing node: wire an equivalent close-account cleanup into doorway-portal-login.steps.ts's registration step + probe: does the neighbourhood pipeline's fleet user count stop growing per run / current state: unwired — that file is out of this task's write set, left untouched`. The feature file is unchanged; only `auth-lifecycle.steps.ts` is committed below.

- [x] **Step 1b: Close the gap** — `closeAccountCleanup` exported from `genesis/a2o/steps/auth-lifecycle.steps.ts` (its only shared home; no duplicate copy) and imported into `genesis/a2o/steps/ui/doorway-portal-login.steps.ts`. The `Given a hosted human is registered on doorway {string}` step (`doorway-portal-login.steps.ts:129`) now reads the bearer token off `POST /auth/register`'s own response (that route already returns `AuthResponse.token`, per `elohim/sdk/schemas/v1/views/auth-response.schema.json`) and registers `this.onCleanup(() => closeAccountCleanup(this, base, canonicalIdentifier, registrationToken))` — captured at registration time because the portal's own sign-in mints a separate session token later, not the same credential. The pre-existing `onCleanup` at `doorway-portal-login.steps.ts:186` (closes the Playwright device) is untouched and unrelated — it is a browser-resource cleanup, not the account cleanup. The neighbourhood feature's preamble caveat is retired accordingly (see below): the missing node named above is now wired.
  - Verification: `npx cucumber-js --dry-run --tags '@auth or @browser'` — 56 undefined scenarios / 230 undefined steps before and after (unchanged); `pnpm exec tsc --noEmit -p tsconfig.json` clean; `npx eslint steps/ui/doorway-portal-login.steps.ts steps/auth-lifecycle.steps.ts` clean. No mesh started; the live count-unchanged assertion remains Step 2's job.

- [x] **Step 2: Prove the count is unchanged** — not run this pass (no mesh started; scope says this is S4's job).

```bash
cd /projects/elohim
BEFORE=$(curl -s http://localhost:8888/status.json | python3 -c 'import sys,json;print(json.load(sys.stdin).get("humansServed"))')
just test mesh features/auth; echo "EXIT=$?"
AFTER=$(curl -s http://localhost:8888/status.json | python3 -c 'import sys,json;print(json.load(sys.stdin).get("humansServed"))')
echo "BEFORE=$BEFORE AFTER=$AFTER"
```
Expected: `BEFORE` equals `AFTER`. (Before Task 14 both read `None`; re-run this step after Task 14 for the real assertion, and record the numbers in the delta.)

- [x] **Step 3: Commit** — pathspec-limited to the one file actually changed (see Step 1 note: the feature file was deliberately left untouched).

```bash
cd /projects/elohim
git add genesis/a2o/steps/auth-lifecycle.steps.ts
git commit -m "test(a2o): harness cleanup closes accounts through the product path (hosted-human Task 6)" -- genesis/a2o/steps/auth-lifecycle.steps.ts
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "a2o auth-lane cleanup uses POST /auth/close-account; household user count before/after run `<id>`: `<n>`/`<n>`."

---

## Task 13 — Hosted cell becomes a notarized promise: issue on register, revoke on close (D2)

**Drains:** new — D2; composes with hosted-human plan **Task 3** (close) and **Task 7** (notarized closure marker).
**Tier:** Opus (rust-architect). **Depends on S3 Task 15 for the `hosted-cell` scope value to be accepted.**

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs` (register hosted arm; close-account handler)
- Modify: `doorway/doorway-service/src/db/schemas/user.rs` (`UserDoc`)
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (`REVOCATION_REASONS`, coordinator-only)

**Interfaces:**
- Consumes: `POST /api/v1/compute/grants` on the pool peer's elohim-storage (`api/compute_tasks.rs:296` routes it to `api/compute_grants.rs::issue`), and the provider-side revoke S3 Task 16 adds. **No new HTTP route on the doorway.**
- Produces: `UserDoc.hosted_cell_grant_cid: Option<String>` and `UserDoc.hosted_cell_valid_until: Option<String>` (RFC3339); `hostedCellGrantCid` on the register and `GET /auth/account` responses. Task 14 counts these fields; Task 4's steps read them; Task 5's caster prints them.

**Preconditions the implementer must wire, read from the code, not guessed:** `POST /api/v1/compute/grants` refuses unless (a) `ELOHIM_COMPUTE_LOCAL_API=1` on the storage peer, (b) the request carries the local capability token (`local_token_authorized`, `compute_tasks.rs:289`), and (c) the `X-Verified-Performer` header names the storage peer's own cell actor (`same_actor`, `:313`). The provider is therefore the **steward's pool peer**, acting for itself — which is exactly D2's provider. Configure the doorway with that peer's URL and local token as new args (`--pool-compute-url`, `--pool-compute-token`, env `POOL_COMPUTE_URL` / `POOL_COMPUTE_TOKEN`), and make the whole grant leg **non-fatal**: a doorway with no pool-compute configuration registers humans exactly as before and leaves `hosted_cell_grant_cid` unset.

**Bounds (D2, from `compute_grants::grant_input`):** `bounds.epr_scope` must be 1..64 entries of task CIDs or `"*"`; `rate_per_hour` and `rotation_ttl_days` both positive and finite; `reach_ceiling` must be `"commons"`; and `validUntil - validFrom` must not exceed `rotation_ttl_days * 86400`. Use `epr_scope: ["*"]`, `reach_ceiling: "commons"`, `rate_per_hour: 60`, `rotation_ttl_days: 30`, `validUntil = validFrom + 30d`.

- [x] **Step 1: Write the failing tests**

In `auth_routes.rs`: `hosted_register_without_pool_compute_config_still_registers` (grant leg skipped, `hosted_cell_grant_cid` is `None`, registration succeeds); `hosted_register_records_the_grant_cid_when_the_pool_answers`; `close_account_revokes_the_hosted_cell_grant_and_clears_the_row`; `close_account_succeeds_when_the_revoke_call_fails` (non-fatal, reported in the response). In the imagodei zome: `account_closed_is_an_accepted_revocation_reason`.

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test hosted_cell 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL.

- [x] **Step 3: Implement**

Register (hosted arm), **after** `provision_agent` returns and the Human is created: POST the grant with `recipient` = the provisioned `agent_pub_key`, `issuedAt`/`validFrom` = now, `validUntil` = now + 30d, and the bounds above. Store `grantCid` and `validUntil` on the row. Emit `hostedCellGrantCid` on the register response and on `GET /auth/account` alongside the `displayName` Task 11 needs.

Close-account, inserted as step (1½) of Task 9's ordering — **before** the cell is uninstalled: (a) call `create_self_revocation` on the human's cell with reason `account-closed` (add the string to `REVOCATION_REASONS` at `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2857` — **coordinator-only, DNA-hash-NEUTRAL**), carrying `revocationCid` in the response; (b) call the provider-side revoke (S3 Task 16) for `hosted_cell_grant_cid`; (c) clear both row fields. Both are non-fatal and reported, never blocking a human from closing their account.

- [x] **Step 4: Green and gate**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test hosted_cell 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate doorway; echo "EXIT=$?"
```
Expected: both `EXIT=0`. The imagodei change is coordinator-only, so the DNA hash must not move — confirm with the DNA gate when S3 is done, not here.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/routes/auth_routes.rs \
        doorway/doorway-service/src/db/schemas/user.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(doorway): notarize the hosted cell as a delegates-compute promise, withdraw it on close (D2; hosted-human Task 7)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "hosted register issues a `hosted-cell` delegates-compute grant (provider = pool peer, recipient = the human's key, 30d bound, commons ceiling); close revokes it and writes an `account-closed` self-revocation. Coordinator-only zome change, DNA hash unmoved. `just gate doorway` EXIT=0; unmeasured on mesh until S4."

---

## Task 13b — The account response names the hosting HOUSEHOLD, not the machine (found by Task 11, 2026-09-11)

**Drains:** story 07 scenario "The account page says which household is hosting them" — `And the page names the household hosting them`. **Tier:** Opus (rust-architect). **Slice:** doorway-service + SDK view schema. **After Task 13** (same file).

Task 11 rendered the strip from the only wire-true hosting fact available, `conductorId` — minted as `format!("conductor-{i}")` (`doorway-service/src/main.rs:483`), i.e. the MACHINE. The a2o assertion (non-empty, not an agent key) would pass on it, and the story would be lying. The honest value is the pool conductor's steward's display name: the doorway's conductor registry knows the storage peer behind each conductor; that peer names its steward agent key (its self-identity, never the doorway's opinion — Task 4 Decision 2); the steward's display name is that Human's record. `p2p-design-gate`: Ephemeral (C), derived at read time from A-class facts; no new entity; a new field on an existing View → schema first.

- [x] **Step 1:** `hostedByHousehold` was already declared on `elohim/sdk/schemas/v1/views/account-response.schema.json` (Task 13 added it); what landed here is its DESCRIPTION, rewritten to say what the value actually is — the display name of the steward of the pool peer that notarized this person's hosted cell, resolved from `hostedCellGrantCid` plus the provider that peer named itself by, and explicitly never the doorway's own name/id/hostname, the conductor id, or the commitment cid. `hostedByConductorId` was NOT added: `conductorId` already carries that fact, and a second spelling of it would only invite the machine back into the household's slot. The contract home is the DOORWAY's own `doorway/doorway-service/tests/schema_contract.rs` (`AccountResponse` is doorway's wire type, not storage's) — `cargo test --test schema_contract` EXIT=0, 20 passed. Note `just gate doorway` runs `--lib --bins` only and does NOT cover `tests/`, so that is a separate leg.

- [x] **Step 2:** landed — **and the chain differs from the one this task forecast; read this before touching it again.** The forecast was conductor registry → the storage peer behind that conductor → that peer's steward key off `/p2p/status`. Two of those links do not exist:
  1. The doorway's `ConductorRegistry` carries `{conductor_id, conductor_url, admin_url, capacity_used, capacity_max}` (`doorway-service/src/main.rs:480-494`) — **no storage-peer URL**. Nothing maps a conductor to the peer beside it.
  2. `GET /p2p/status` on elohim-storage answers `peerId` = the **transport** id (libp2p `PeerId` / iroh `NodeId`), not an agent key — and the crate says so in terms: *"`self_cid` is a TRANSPORT id … emitted only as alsoKnownAs — never as the self agent_cid (identity-namespace hazard)"* (`elohim-storage/src/http.rs:7927`). Reading a steward agent key out of that field is exactly the conflation the storage crate's own guard forbids. `/health` carries no agent key either, and `/api/v1/identity/me` is session-scoped (the caller's identity, not the node's).

  The one place a storage peer states its OWN conductor cell key over HTTP is the grant surface: `api/compute_grants.rs:261` writes `provider = hc.agent_key_uhcak()` onto every `delegates-compute` commitment it issues, and that same surface refuses any request whose `X-Verified-Performer` is not that key. So the landed chain roots there instead:

  | hop | what | where |
  |---|---|---|
  | 1 | the promise exists | `UserDoc.hosted_cell_grant_cid` (`db/schemas/user.rs:222`) |
  | 2 | the peer names ITSELF as provider | `provider_named_by_peer` reads `provider` off the pool peer's own grant answer (`routes/hosted_cell.rs:232`), stored at issue time as `UserDoc.hosted_cell_provider` (`db/schemas/user.rs:231`) |
  | 3 | both facts gate the name | `household_steward_key` (`routes/hosted_cell.rs:302`) — `None` if either is missing |
  | 4 | key → Human | `zome_helpers::call_get_human_by_agent_key` → `imagodei::get_human_by_agent_key` (`dna/imagodei/zomes/imagodei/src/lib.rs:459`) |
  | 5 | Human → shown name | `household_display_name` (`routes/hosted_cell.rs:315`) — blank trims to `None` |

  Composed by `hosted_cell::hosted_by_household` (the Human read is injected, so the whole chain is testable with no conductor) and bound to the live surfaces by `hosted_cell::hosted_by_household_for`, so the account route reads as one line (`routes/auth_routes.rs:2509`) and the whole concern stays in one module — `auth_routes.rs` nets +32 lines against a 7000 hard ceiling, all of it the anti-regression test below. **`household_label` is DELETED** — the `doorway_id`/gateway-hostname fallback that named the arranger is gone from the tree, not merely bypassed, so no future edit can reach it. Close-account nulls `hosted_cell_provider` alongside the cid.

  Tests, written first and watched fail: `naming_a_household_needs_both_the_promise_and_the_peers_own_word`, `a_blank_human_name_renders_absent_not_empty`, `a_hosted_account_reads_the_stewards_display_name`, `an_unhosted_account_names_no_household_and_asks_nobody`, `a_steward_key_with_no_human_answers_null`, `issue_records_the_steward_key_the_peer_names_not_the_doorways` (the wiremock answers a provider deliberately NOT equal to the configured performer, so a doorway echoing its own configuration back would fail), `a_grant_without_a_named_provider_is_still_a_promise`, and `a_doorways_own_name_can_never_stand_in_for_the_household` (both retired fallbacks present and non-empty; the field still answers `None`). Seam rows for `household_steward_key`, `household_display_name` and `provider_named_by_peer` are in `doorway/doorway-service/seam-registry.yaml` (C5 evidence-not-authority-at-transfer, C4 honest absence) — census reads `doorway 55 pts · 55 cited · 0 uncited`.

- [x] **Step 3:** `pnpm run schema:codegen:ts` EXIT=0. The `HostedCellFacts` overlay and its `TODO(wire-codegen)` are deleted from `doorway/doorway-app/src/app/models/doorway.model.ts`; the component's `HostedAccount` intersection type is gone and `account` is a plain `AccountResponse`; `hostedByHousehold` reads the generated field, and the `TODO(hosted-household-name-wire)` that stood `conductorId` in is deleted with it. A new spec case — `names the household, never the machine it runs on` — pins that a row carrying `conductorId` but no `hostedByHousehold` renders the promised-until row ALONE, so the machine can never stand in. `just gate doorway` EXIT=0 (1238 lib tests, up from 1230); `just gate doorway-app` EXIT=0 (53 tests / 11 files); `pnpm run build` EXIT=0.

- [x] **Step 4:** pathspec commits; ticked.

**Found, not fixed (outside this task's write set).** `elohim/holochain/dna/imagodei/zomes/imagodei/seam-registry.yaml` — which already existed (`2cea494ee` registered `recoverable_own_bindings` there), so the `self_revocation_reason_accepted` row was APPENDED rather than authored fresh — loads and validates (census: `imagodei 2 pts · 2 cited · 0 uncited`), but `imagodei` is **not routed to any seam column** in `.claude/epr-meta/seam-catalog.yaml`, so it contributes zero cells to the concern×seam matrix — which reads as *unexamined* rather than as examined-and-clean. `mishpat` and `rakia-executor` sit unrouted in the same way, so this is a standing class rather than a regression this task introduced. Fix: add `imagodei` to a seam's `registry_crates` in `.claude/epr-meta/seam-catalog.yaml`.

**Habit delta line this produces:** `hosted-human-lifecycle` — "account response carries hostedByHousehold: the display name of the steward the POOL PEER named itself by on its own grant answer, resolved through imagodei get_human_by_agent_key. The arranger's-name fallback (`household_label`, doorway_id → gateway hostname) is deleted from the tree. The strip names the household, not the machine; absent, never blank. `just gate doorway` EXIT=0 (1238 lib tests), `cargo test --test schema_contract` EXIT=0 (20), `just gate doorway-app` EXIT=0 (53)."

---

## Task 13c — Agent-key canonical form, capacity dedupe, steward-cell targeting (found by Task 17 rerun)

**Drains:** nothing new — it unblocks Task 13's promise leg, which was 500ing on every hosted registration, so story 07's hosting scenarios could never go green. **Tier:** Opus (rust-architect). **Slice:** doorway-service only. **Surfaced by:** household run `20260911T0319–0326Z-8618c2dd` (`genesis/a2o/reports/sprint-report-household-*.json`).

Three defects, one shared root: the doorway wrote agent keys in a spelling nothing else in the protocol uses, and then counted those spellings as if each were a person.

- [x] **Step 1 — the grant recipient is a real HoloHash now.** `36b0e053a46462277804a38f69eb3c0f98dcd339`.

  Symptom: `WARN pool compute refused a hosted-cell grant status="500" body="Error: Invalid input: compute grant: recipient must be a Holochain agent key"`. `provisioner.rs` set `ProvisionedAgent::agent_pub_key` to BARE base64 of the raw 39 bytes (`hCAk…`, the form visible in the Prologue roster); `elohim-storage/src/api/compute_grants.rs:87` does `AgentPubKey::try_from(recipient)`, which answers `Holo Hash missing 'u' prefix` on anything but the canonical multibase form.

  **The form was chosen from an inventory, not a guess** (every producer and reader of `agent_pub_key` in the crate, ~350 references / 31 files). It found FOUR string forms in flight — bare url-safe (provisioner, discovery, the startup walk), bare STANDARD (the startup walk, the chaperone's browser payload), canonical `uhCAk…`, and a 32-byte Ed25519 key that is not a HoloHash at all (`custodial_keys/service.rs:121`, the non-provisioned path). The decisive evidence that canonical is the contract and bare is the drift: **zero** bare `hCAk…` literals exist anywhere in `src/`, while all 118 `uhCAk` literals — every test fixture, the `pool_compute_performer` config doc (`config.rs:285`), the op-gate performer doc (`server/http.rs:1446`), and the dev-mode key mint (`auth_routes.rs:1249`) — already assume canonical. The crate's own tests encoded a belief its production code did not honour, which is exactly why nothing caught this.

  Landed: `conductor::agent_key` (`canonical_agent_key` / `normalize_agent_key` / `is_agent_key_form` / `lookup_forms`) and `provisioner::provisioned_agent_key`, applied at **both** producers — fresh provision and the idempotent `find_existing_app` reuse path, which must move in lockstep or a returning human's promise leg fails where a newcomer's succeeds. Registration fans out over every string form so a JWT or Mongo row of any vintage still routes; `get_conductor_for_agent` and `unregister_agent` became spelling-agnostic, which covers the ~13 registry-lookup readers the inventory flagged intolerant in one place. Two hard-parse readers that have been failing SILENTLY all along now normalize their input: account-closed self-revocation (`auth_routes.rs:2844` — `.ok()?` swallowed the refusal, so the revocation never reached the DHT) and `zome_helpers::call_get_human_by_agent_key`.

  Tests, written first and watched fail with the real fleet error: `a_provisioned_agents_grant_recipient_parses_as_a_holochain_agent_key` (the provisioner's own minting fn → the real grant body → the exact `AgentPubKey::try_from` the grant surface calls) and `the_drifted_bare_base64_form_is_refused_by_the_same_parse` (so the assertion has teeth), plus eight normalizer cases covering both input vintages, idempotence, and placeholder pass-through. Fixtures are REAL keys minted via `from_raw_32` — a hand-rolled 39 bytes answers `BadChecksum` and would prove the wrong thing.

  **Migration note, deliberately not fixed here.** `UserDoc.agent_pub_key` rows written before this commit keep their bare spelling. Registry lookups tolerate that; two ADMIN Mongo filters that query `UserDoc` by string equality do not — `routes/admin_conductors.rs:731` (force-graduation) and `:756` (the steward flag) — so an operator must still pass the legacy spelling for a pre-canonical account. Both are operator-driven and low-traffic; widening them to an `$in` over `lookup_forms` is the follow-up. Also found and left: `worker/zome_call.rs:267` decodes with `STANDARD` what `services/discovery.rs:292` encodes with `URL_SAFE_NO_PAD` — a latent pre-existing mismatch on a path this change does not touch.

- [x] **Follow-up A — the two admin Mongo filters match an IDENTITY, not a spelling.** Both `UserDoc` filters in `routes/admin_conductors.rs` — the force-graduation `find_one` (`:752`) and the steward-flag `update_one` (`:777`) — now build their predicate in one place, `agent_identity_filter` (`:724`), an `$in` over every string form of the identity the operator named. A row written before canonicalization (bare `hCAk…`) is reached by its canonical spelling and the reverse; previously the miss was SILENT, surfacing as a 404 "User not found" for an account that plainly exists. One helper, not two: a find that matched while the update did not would report a graduation that wrote nothing.

  The widening is strictly a SUPERSET of the equality it replaces — `conductor::agent_key::lookup_forms_of` always includes the caller's own string — so a dev-mode placeholder still matches its own row and nothing else, and no vintage of caller loses a row it could already find. Tests: `force_graduation_finds_a_pre_canonical_row_from_the_canonical_key`, `the_steward_flag_update_reaches_a_canonical_row_from_the_legacy_key`, `both_admin_filters_still_match_the_exact_spelling_they_used_to`, `an_admin_filter_never_reaches_a_different_agent`, `an_admin_filter_for_a_non_key_matches_only_that_string`. Mongo is deliberately not in the loop: the honest boundary a unit test can hold is which strings the filter WOULD match, and that is exactly what regressed.

- [x] **Follow-up B — the worker stops choosing a base64 alphabet.** `worker/zome_call.rs` decoded the cell id with `base64::STANDARD` what `services/discovery.rs:245-246` writes with `URL_SAFE_NO_PAD` (`:292`) — and this was LIVE, not latent: the alphabets agree only on a string containing no `-`/`_`, about one key in five at 39 bytes, so roughly 80% of discovered cells could not be called at all (`Invalid base64: Invalid byte 45`). Padding was never the issue; 39 bytes is a multiple of 3 and encodes to 52 characters with none. The value's journey has no JWT and no Mongo row in it: raw bytes off the conductor's `app_info.cell_ids` → `URL_SAFE_NO_PAD` string → `CellInfo` → `ZomeCallConfig` → the `zome_configs` DashMap on `AppState` → `zome_helpers::get_zome_config_by_role` → `build_zome_call`, which must hand the bytes back.

  Cured by removing the choice rather than correcting it. `decode_cell_id_half` (`zome_call.rs:291`) delegates to the new `conductor::agent_key::decode_key_bytes`, defined as *normalize, then read the canonical body*, so it cannot disagree with `normalize_agent_key` about what a string means — every spelling a doorway has written yields identical bytes, and a string that is not a HoloHash in any encoding is refused rather than decoded into a short, plausible-looking cell id. Both cell-id halves go through it: an `AgentPubKey` and a `DnaHash` share the length and the multibase tag. Tests: `the_legacy_url_safe_spelling_discovery_writes_round_trips_to_the_original_bytes` (the form discovery actually writes), `the_pre_fix_standard_decoder_refused_the_form_discovery_actually_writes` (teeth — the alphabets really did disagree, so the round-trip proves a fix and not a tautology), `a_canonical_cell_id_round_trips_to_the_original_bytes`, `the_standard_alphabet_spelling_also_round_trips`, `every_spelling_produces_a_byte_identical_call`, `provenance_carries_the_same_bytes_as_the_cell_id_agent`, `a_cell_id_half_that_is_not_a_holo_hash_is_refused_not_truncated`.

  **Correction to the note above:** `services/discovery.rs:292` was NOT moved to the canonical producer by `b870af164` — it still encodes bare `URL_SAFE_NO_PAD`, and deliberately stays that way. The cure belongs on the reader, which now tolerates every vintage; canonicalizing the producer would additionally move the `zome_configs` / `import_config_store` DashMap KEYS, which is a separate change with its own blast radius.

  No new seam row for either follow-up: neither introduces a decision. `lookup_forms_of` and `decode_key_bytes` are thin string-side adapters over the already-registered `normalize_agent_key` predicate — both are literally defined as *normalize first, then read* — so the decision "which spellings name one identity" is still made in exactly one place. Both follow-ups are instead cited as contract tests on the existing `normalize_agent_key` row (C9 identity-lineage continuity), which is where a reader looking for "does a pre-canonical row still resolve?" would go. `just gate doorway` EXIT=0 — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib --bins`: **1269 lib tests passed, 0 failed, 2 ignored** (up from 1251), plus 4 bin tests.

  Seam row: `normalize_agent_key` in `doorway/doorway-service/seam-registry.yaml` (C9 identity-lineage continuity, C10 contract-evolution honesty, C7 advertise/serve symmetry, C6b idempotent effect), citing all eleven tests.

- [x] **Step 2 — capacity counts humans, not spellings.** `36b0e053a46462277804a38f69eb3c0f98dcd339` (same commit as the normalizer it depends on; the key-canonicalization fan-out is only safe once this landed, so it is the earlier of the two commits).

  `provisioner.rs` registered every agent twice (url-safe + STANDARD) and `registry.rs` incremented `capacity_used` once per call, while `seed_capacity_from_agents` DID dedupe — so the live count ran at 2x and a restart silently halved it. The fleet default `DOORWAY_MAX_AGENTS_PER_CONDUCTOR=50` was really admitting about 25, and the cap moved when nothing about the conductor had.

  Fixed at the registry: `capacity_used` is now RECOUNTED (never incremented) from the agent map through one predicate, `count_distinct_agents_by_conductor`, which the live path (`recount_capacity`, on every register AND unregister) and the restart seed both call — live and seeded are equal by construction, not by two implementations agreeing. Both lookup encodings are kept as aliases, since readers need them.

  That predicate's unit is the **normalized agent key**, which retires the earlier `(conductor_id, app_id)` proxy. `app_id` was only ever a stand-in for identity adopted because no normalizer existed, and it leaked the other way: every row carrying the bare default `"elohim"` app id — a legacy Mongo row, a `ConductorRouter` miss-path auto-registration, a hand-driven `/admin/conductors/assign` — collapsed onto ONE count however many humans it represented. An undercount on the surface that enforces the cap is a cap that silently stops biting, and the retired proxy's own doc called this out as a "known undercount" it accepted.

  Tests: `registering_one_agent_under_every_encoding_costs_one_capacity` (register one agent under every form → `capacity_used == 1`, then seed → still 1, i.e. restart-seeded equals live), `any_spelling_of_a_key_finds_the_conductor_and_deprovisions_it`, and three rewritten predicate tests — the previous ones used fake keys (`"uhCAk_adam_std"`) that cannot exercise a normalizer, so they now use real key encodings. `distinct_agents_handles_empty_and_default_app_id_rows` pins the retired proxy's undercount as a regression.

  Seam row: `count_distinct_agents_by_conductor` (C6b idempotent effect, C7 advertise/serve symmetry across a restart, C4 honest absence), citing six tests.

- [x] **Step 3 — steward-cell targeting: DIAGNOSED, and the defect is NOT in the doorway.** Report only; no code change (the defect sits in `genesis/`, outside this task's write set).

  Symptom after real hosting landed: `seed-agent-bindings` failed with `imagodei::agent_peer_binding:155: Guest("signer mismatch: caller 'uhCAkKR1eEp…' does not match Agent EPR 'human-matthew-manager'")` — a call meant to be signed by the steward's own cell was signed by a HOSTED human's cell on the same conductor.

  **The doorway binds by exact app id and is correct.** `doorway/doorway-service/src/services/zome_caller.rs:832-834`:

  ```rust
  let app_info = apps
      .iter()
      .find(|a| a.installed_app_id == installed_app_id)
  ```

  String EQUALITY against the configured `args.installed_app_id` — a hosted app can never satisfy it. (It then authorizes signing credentials for all provisioned cells *of that one app*, `:842-865`, so provenance is the steward's key by construction.)

  **The selection defect is a PREFIX match in the seeder.** `genesis/seeder/src/seed-agent-bindings.ts:197-198`:

  ```ts
  const apps = await adminWs.listApps({});
  const matchingApp = apps.find(a => a.installed_app_id.startsWith(appIdPrefix));
  ```

  with `appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim'` (`:424`). The doorway mints hosted app ids as `format!("{app_id}-{conductor_id}-{short_hash}")` (`doorway-service/src/conductor/provisioner.rs:388`) → `elohim-conductor-0-a1b2c3`, which **satisfies `.startsWith('elohim')`**. `listApps` guarantees no ordering, so once a conductor holds the steward's app plus N hosted apps, `.find()` can return a hosted human's app; `selectSeedCell(matchingApp.cell_info, 'imagodei')` (`:208`) then picks the imagodei cell of the WRONG app, and the zome's signer-match gate refuses exactly as it should.

  This is the same first-match-wins class the file's own header documents at `:21-32` (genesis #1119/#1380–#1386) — fixed once for conductor affinity across pods, and still live for app selection *within* one conductor. The prefix match was safe only while a conductor held exactly one app; real hosting ended that precondition. Fix (not taken here): match the steward's app id EXACTLY, the way the doorway does, rather than by prefix.

**Verification.** `just gate doorway` EXIT=0 — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib --bins`: **1251 lib tests passed, 0 failed, 2 ignored** (up from 1238 at Task 13b), plus 4 bin tests. No schema changed, so no `schema_contract` leg. `seam-registry.yaml` validates against `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` at 57 decision points (55 + the 2 added here).

**Habit delta line this produces:** `hosted-human-lifecycle` — "a provisioned agent key is the canonical HoloHash form (`uhCAk…`), so the hosted-cell compute grant's recipient parses on the surface that issues it; legacy bare-base64 rows are normalized on read, never orphaned. Conductor capacity counts distinct agent identities through one predicate shared by the live recount and the restart seed, so the cap no longer halves on restart. `just gate doorway` EXIT=0 (1251 lib tests)."

---

## Task 14 — `humansServed` is derived (D3) and the backlog atom closes

**Drains:** new — D3. Closes `genesis/data/timeline/backlog/doorway-landing-humans-served-source-2026-06-23.md`.
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `doorway/doorway-service/src/routes/status.rs:482` (`build_status_data`), `:802` (`humans_served: None`)
- Modify: `doorway/doorway-app/src/app/components/landing/doorway-landing.component.ts:28` (the comment that still calls it a federation aggregate) and its template (add the `data-testid` Task 4 Step 3 agreed)
- Modify: `genesis/data/timeline/backlog/doorway-landing-humans-served-source-2026-06-23.md` (decision line + `status: closed`)

**Interfaces:**
- Consumes: `UserDoc.hosted_cell_grant_cid` and `hosted_cell_valid_until` (Task 13).
- Produces: `StatusResponse.humans_served: Some(u32)` whenever this doorway operates a pool; `None` only when it does not.

**The measure, exactly (D3):** count credential rows that are active (`is_active == true`, `metadata.is_deleted == false`), carry a non-empty `hosted_cell_grant_cid`, and whose `hosted_cell_valid_until` is in the future. That is the count of live, unexpired, unrevoked `hosted-cell` commitments this doorway's pool provides — the field is only ever set from a `grantCid` the substrate returned, and Task 13 clears it on revoke. **This adds no HTTP route and no storage read**; a doorway that wants to re-verify a row can read `GET /api/v1/commitments/{id}` on the existing surface, which is out of scope here.

**Honesty caveat (chief, 2026-09-10):** this count is a doorway-local PROJECTION of the notary's answer — the row carries the grant cid the substrate returned and is cleared by the doorway's own close path. A commitment revoked provider-side on the substrate (S3 Task 16) without passing through close-account is NOT reflected until a reconcile reads it back; that reconcile is deferred and named as such in the backlog decision, not hidden.

`Option` semantics stay: a doorway with **no** conductor pool reports `None` → the SPA keeps rendering `—`. A doorway **with** a pool and no hosted humans reports `Some(0)` → the SPA renders `0`.

- [x] **Step 1: Write the failing tests**

`humans_served_counts_only_live_unexpired_hosted_rows`; `humans_served_is_zero_not_none_when_a_pool_exists_and_hosts_nobody`; `humans_served_is_none_without_a_pool`. The existing `humans_served: Some(42)` fixture at `status.rs:1499` should keep passing unchanged.

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/doorway-target cargo test humans_served 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL.

- [x] **Step 3: Implement in `build_status_data`** — the `build_status_data` half landed (`a33e3876a`). The landing-caption half **landed 2026-09-11 alongside Task 13b**: `doorway-landing.component.ts`'s `StatusResponse` doc no longer calls `humansServed` "a federation-wide social aggregate (sum of self-reported node counts)". It now carries the D3 semantics — humans THIS doorway is hosting, from the live, unexpired, unrevoked `hosted-cell` commitments its own pool provides (derived in `status.rs::build_status_data`); `null` means no pool at all, `0` means a pool hosting nobody — and says outright that a federation-wide number would be a separate, separately-labelled figure. The card's `data-testid="landing-humans-served"` sits on the **value span**, not the wrapping `.stat` div: `steps/dataplane/humans-served.steps.ts` reads `.innerText()` off that testid and parses it as a number, so on the div the "Humans Served" label would make every assertion read `NaN`.

Replace `humans_served: None` at `:802` with the derived count. In `doorway-landing.component.ts`, replace the comment at `:28` that describes the field as "a federation-wide social aggregate (sum of self-reported node …)" with the D3 semantics — *humans this doorway is currently hosting, derived from live hosted-cell commitments its pool provides* — and add the card's `data-testid`.

- [x] **Step 4: Green the doorway gate** — `just gate doorway` EXIT=0. `just gate doorway-app` ran with the caption change on 2026-09-11: **EXIT=0**, 53 tests / 11 files, alongside `pnpm run build` EXIT=0.

```bash
cd /projects/elohim
just gate doorway; echo "EXIT=$?"
just gate doorway-app; echo "EXIT=$?"
```
Expected: both `EXIT=0`. Run them one after the other, never together.

- [x] **Step 5: Close the backlog atom**

In `genesis/data/timeline/backlog/doorway-landing-humans-served-source-2026-06-23.md`: set `status: "closed"` in the frontmatter and append one decision section:

```markdown
## Decision (2026-09-10)

**Option B.** "Humans Served" is the count of humans THIS doorway is currently hosting —
live, unexpired, unrevoked `hosted-cell` delegates-compute commitments its pool provides,
read from the credential rows that carry the grant cid the substrate returned. Option A
(the orchestrator heartbeat social aggregate in `admin.rs`) is rejected: it is a k8s-plane
self-report, not a substrate read, and it is not what "served by this doorway" means. A
federation-wide aggregate remains a separate, separately-labelled number and is out of scope.
A doorway with a pool that hosts nobody now shows `0`; a doorway with no pool still shows `—`.
Wired in `routes/status.rs::build_status_data`; specified by
`genesis/a2o/features/dataplane/doorway-humans-served.feature` (`@concern:humans-served`).
```

- [x] **Step 6: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/routes/status.rs \
        doorway/doorway-app/src/app/components/landing/doorway-landing.component.ts \
        genesis/data/timeline/backlog/doorway-landing-humans-served-source-2026-06-23.md
git commit -m "feat(doorway): humansServed derived from live hosted-cell commitments (D3); backlog atom closed"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "humansServed derived from live hosted-cell commitments (Option B); backlog atom doorway-landing-humans-served-source-2026-06-23 closed with the decision line; `just gate doorway` and `just gate doorway-app` both EXIT=0."

---

# S3 — elohim-storage

Two tasks, both small, both **Opus (rust-architect)**. **The gate is slow here**: `elohim-storage` is pinned to `CARGO_BUILD_JOBS: "1"` by `genesis/agentic/pool-policy.json` (measured: 5.4 GB peak, ~12% longer wall-clock than default parallelism, and the default parallelism peaks at 15.4 GB and gets shed by the RAM guard). Budget for it and run nothing else heavy alongside.

## Task 15 — `hosted-cell` is an accepted grant scope

**Drains:** new — D2 (the storage half).
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `elohim/elohim-storage/src/api/compute_grants.rs:115` (the payload `"scope"`) and `:194` (the consent metadata `"scope"`)

**Interfaces:**
- Produces: a `scope` field on the grant request accepted from the caller and echoed into both the notarized payload and the consent metadata.

**The defect being fixed:** `grant_input` hard-codes `"scope":"sweettest-feedback"` in the notarized payload (line 115) and `issue` hard-codes it again in the consent metadata (line 194), while `grant_input`'s own unknown-field guard (line 46-56) rejects a caller-supplied `scope` with *"unknown grant field; provider and scope are server-owned"*. Every grant this surface has ever issued is therefore scoped `sweettest-feedback`, whatever it was for.

- [x] **Step 1: Write the failing tests**

In `compute_grants.rs`'s test module:

```rust
#[test]
fn a_named_scope_reaches_the_notarized_payload() {
    let input = json!({"recipient": VALID_KEY, "scope": "hosted-cell",
        "issuedAt": NOW, "validFrom": NOW, "validUntil": PLUS_30D,
        "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":30}});
    let grant = grant_input(PROVIDER, &input, now()).expect("accepted");
    let payload: Value = serde_json::from_str(&grant.payload_json).unwrap();
    assert_eq!(payload["scope"], "hosted-cell");
}

#[test]
fn an_unnamed_scope_still_defaults_to_the_sweettest_feedback_lane() {
    // existing callers pass no scope; their notarized payload must not change
    let input = json!({"recipient": VALID_KEY,
        "issuedAt": NOW, "validFrom": NOW, "validUntil": PLUS_30D,
        "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":30}});
    let grant = grant_input(PROVIDER, &input, now()).expect("accepted");
    let payload: Value = serde_json::from_str(&grant.payload_json).unwrap();
    assert_eq!(payload["scope"], "sweettest-feedback");
}

#[test]
fn an_unknown_scope_value_is_refused() {
    let input = json!({"recipient": VALID_KEY, "scope": "whatever-i-want",
        "issuedAt": NOW, "validFrom": NOW, "validUntil": PLUS_30D,
        "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":30}});
    assert!(grant_input(PROVIDER, &input, now()).is_err());
}
```

Reuse the constants and the `VALID_KEY` fixture the module's existing tests already build (see the `bounds` fixture at `:344` and the field-rejection table at `:363`/`:399`).

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim/elohim-storage
CARGO_TARGET_DIR=/tmp/elohim-storage-target CARGO_BUILD_JOBS=1 cargo test --lib compute_grants 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL — the first test sees `"sweettest-feedback"`; the third is accepted today (scope is silently dropped, so nothing refuses it — the test fails because `is_err()` is false).

- [x] **Step 3: Implement**

Add `"scope"` to the accepted key set at `:46-56` (and correct the error message so it no longer says scope is server-owned). Validate it against a fixed allow-list — `"sweettest-feedback"` and `"hosted-cell"`, nothing else — defaulting to `"sweettest-feedback"` when absent, so every existing caller's notarized bytes are unchanged. Thread the chosen value into the payload at `:115` and the consent metadata at `:194`.

**This does not move the DNA hash:** the commitment is the existing `delegates-compute` entry type with a different `payload_json` string. It is a payload change, not an entry-type change, exactly as D2 says.

- [x] **Step 4: Green the tests, then the gate**

```bash
cd /projects/elohim/elohim-storage
CARGO_TARGET_DIR=/tmp/elohim-storage-target CARGO_BUILD_JOBS=1 cargo test --lib compute_grants 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate elohim-storage; echo "EXIT=$?"
```
Expected: both `EXIT=0`. The full gate takes a while at one cargo job; do not run another heavy gate beside it.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/api/compute_grants.rs
git commit -m "fix(storage): a delegates-compute grant carries its real scope; hosted-cell accepted (D2)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "`hosted-cell` is an accepted delegates-compute scope; grants stop being hard-coded `sweettest-feedback` (3 unit tests); `just gate elohim-storage` EXIT=0."

---

## Task 16 — Provider-side revoke on the grant surface

**Drains:** new — D2 (the withdrawal half).
**Tier:** Opus (rust-architect).

**Files:**
- Modify: `elohim/elohim-storage/src/api/compute_grants.rs` (new `revoke` function)
- Modify: `elohim/elohim-storage/src/api/compute_tasks.rs:296-320, :354` (dispatch)

**Interfaces:**
- Produces: `pub async fn revoke(hc, pool, input) -> Result<Value, StorageError>`, reached by `POST /api/v1/compute/grants` with body `{ "grantCid": "<cid>", "revoke": true }` — **the same route**, a body variant, exactly as the dev-seed lever already does it (`api/seed_delegates_compute.rs:220-233`). No new route.
- Consumed by: doorway Task 13's close-account path.

**Semantics, taken from what `issue` already enforces:** revoke writes a provider-authored `"revoked"` commitment-state link via `native::call_create_commitment_state_link`, then sets `revoked_at` on the projection row (`crate::db::mishpat_commitments::set_revoked_at`, `db/mishpat_commitments.rs:141`). `issue` already refuses to reactivate anything a provider has withdrawn (`:172-190`: *"withdrawn grant cannot be reactivated"* / *"withdrawn projection requires reconciliation, never reactivation"*) — so this makes withdrawal terminal by construction and needs no new guard. Revoking an already-revoked grant answers `200` and is a no-op. Revoking a grant this peer is not the provider of is refused.

- [x] **Step 1: Write the failing tests**

`revoke_is_idempotent`; `revoke_refuses_a_grant_this_peer_did_not_provide`; `a_revoked_grant_cannot_be_reissued` (asserting the existing `issue` guard fires after a revoke).

- [x] **Step 2: Run them and watch them fail**

```bash
cd /projects/elohim/elohim-storage
CARGO_TARGET_DIR=/tmp/elohim-storage-target CARGO_BUILD_JOBS=1 cargo test --lib compute_grants 2>&1 | tail -20; echo "EXIT=$?"
```
Expected: FAIL — `revoke` not found.

- [x] **Step 3: Implement and dispatch**

In `compute_tasks.rs`, at the `if grant_request {` arm (`:354`), branch on `input["revoke"] == true` to `compute_grants::revoke` and otherwise to `issue`. Keep the existing `grant_request && method != Method::POST → method_not_allowed` guard at `:315` unchanged.

- [x] **Step 4: Green and gate**

```bash
cd /projects/elohim/elohim-storage
CARGO_TARGET_DIR=/tmp/elohim-storage-target CARGO_BUILD_JOBS=1 cargo test --lib compute_grants 2>&1 | tail -10; echo "EXIT=$?"
cd /projects/elohim && just gate elohim-storage; echo "EXIT=$?"
```
Expected: both `EXIT=0`.

- [x] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/api/compute_grants.rs elohim/elohim-storage/src/api/compute_tasks.rs
git commit -m "feat(storage): provider-side revoke for delegates-compute grants (D2)"
```

**Habit delta line this produces:** `hosted-human-lifecycle` — "provider-side grant revoke landed on the existing grant route as a body variant (3 unit tests); withdrawal is terminal — `issue` already refuses reactivation; `just gate elohim-storage` EXIT=0."

---

# S4 — Measure, deploy, flip

Five tasks, **in this order**. Tasks 17–20 are the measure; Task 21 is held.

## Task 17 — (a) The household mesh run

> **Correction (chief, 2026-09-11) — the DNA-hash reference for the coordinator change.** Do NOT compare a locally packed `imagodei.dna` against `elohim/holochain/dna/dna-hashes.baseline`: that file holds the CI build's hashes, and DNA hashes depend on the absolute build path (memory `project_dna_hash_depends_on_build_path`, verified 2026-09-06 — identical source in two worktrees differs by 38 bytes of integrity WASM). From this checkout the baseline ALWAYS mismatches, before and after Task 13. The proof that a coordinator-only change is hash-neutral is local-vs-local: after `just pack`, the integrity wasm (`zomes/imagodei/target/wasm32-unknown-unknown/release/imagodei_integrity.wasm`) is NOT recompiled and `git log -1 -- dna.yaml zomes/imagodei_integrity/` predates the prior pack (last integrity change f2f25d73b, 2026-09-03; dna.yaml 2026-04-24). CI's DNA pipeline (`--run-ignored all` sweettest + its own baseline compare) is the authority for the fleet; run 2026-09-11 stopped on this mis-specified check once and was resumed.


**Drains:** doorway-failover plan **Task 0.1** (record the frontier) and hosted-human plan **Task 8**'s household half.
**Tier:** Sonnet, operator-style — this is running and recording, not deciding.

**Files:**
- Produces: reports under `genesis/a2o/reports/` (gitignored, durable) and a household sprint-report JSON — **which is exactly the artefact pre-push's T2 receipt leg looks for in Task 19**.

- [x] **Step 1: Bring the mesh up clean and cast it**

```bash
cd /projects/elohim
just mesh preflight; echo "EXIT=$?"
just mesh start && just mesh wait --timeout 900; echo "EXIT=$?"
just mesh prologue; echo "EXIT=$?"
```
Expected: preflight prints one `ok` line per check with no `REFUSED`; `wait` returns 0; the prologue's `seed-hosted-humans` leg prints three registrants with three distinct agent keys.

- [ ] **Step 2: Run the four concerns, recording each run id**

```bash
cd /projects/elohim
just test mesh features/auth/hosted-human/07-hosted-by-a-household.feature;      echo "EXIT=$?"
just test mesh-browser features/auth/hosted-human/07-hosted-by-a-household.feature; echo "EXIT=$?"
just test mesh features/dataplane/doorway-humans-served.feature;                 echo "EXIT=$?"
just test mesh-browser features/dataplane/doorway-humans-served.feature;         echo "EXIT=$?"
just test mesh-browser features/auth/hosted-human/05-leaving.feature;            echo "EXIT=$?"
just test mesh features/dataplane/doorway-failover.feature;                      echo "EXIT=$?"
A2O_RUN_WIP=1 just test mesh features/dataplane/doorway-apex-transition.feature; echo "EXIT=$?"
```

Record, per run: the run id the runner prints (`<UTC stamp>-<short sha>`), the report path, and scenarios/steps passed·failed·undefined·skipped. `05-leaving` is `@browser-only` at file level, so it runs under `mesh-browser`; `07` and `doorway-humans-served` have `@browser-only` at scenario level and therefore need **both** profiles for full coverage.

Expected on a fully-drained S1–S3: `@concern:hosted-compute-contracted`, `@concern:humans-served` and `@concern:hosted-human-lifecycle` all pass with **0 undefined and 0 skipped**. `@concern:doorway-failover`'s household feature passes. `doorway-apex-transition` is **0 undefined** and fails at a named step — that is the expected, honest outcome (Task 6).

- [x] **Step 3: Confirm a household sprint-report was written**

```bash
ls -t /projects/elohim/genesis/a2o/reports/sprint-report-household-*.json | head -3
```
Expected: a file newer than every dataplane source change in this batch. Pre-push's warn-only **T2 receipt** leg prints `NO-T2-RECEIPT` if a dataplane change (`elohim-storage/src/{p2p,sync,reconcile,p2p_iroh}`, `doorway-service/src`) has no household sprint-report newer than it. This step is what produces that receipt for Task 19.

**Habit delta lines this produces:**
- `hosted-human-lifecycle` — "household lane run `<id>`: 05-leaving `<n>`/`<n>` passed, 07-hosted-by-a-household `<n>`/`<n>`, doorway-humans-served `<n>`/`<n>`; 0 undefined, 0 skipped."
- `doorway-failover` — "household lane run `<id>`: doorway-failover.feature `<n>`/`<n>`; apex-transition MEASURED (0 undefined) and failing at `<step>`."

---

## Task 17b — Mesh: version stamp, portal arm, hosted-cast allow-list, seeder steward assumption (found by Task 17 rerun)

**Drains:** the four non-product reds Task 17's household run charged against product concerns. **Tier:** mesh operator (Fable). **Slice:** `app/elohim-app/scripts/hc-mesh{,-prologue}.sh`, `genesis/seeder/src/`, root `justfile`. No `doorway/**` or `elohim/**` source — the doorway's own cell-targeting defect is Task 19b's neighbour and was diagnosed concurrently.

Run 20260911T03{19–26}Z-8618c2dd charged four reds to product concerns that were mesh-side. Each one is a household precondition that CI happens to meet and a local build never does, or a cast sized under a bug:

- [x] **Step A: A plain `pnpm build` leaves no `version.json`, and staging refuses the archive.** `elohim/sdk/scripts/package-angular-check.py:34-56` refuses a browser or SSR archive whose `version.json` is absent or whose `commit` is empty, and for `kind=server` requires the server stamp to equal the browser one. Only the CI Jenkinsfile and `package-angular.mjs build` write that file — the latter as a side effect of a full rebuild, which the Prologue must never do to the dists it is staging. So the Prologue now stamps the dists itself, mirroring `package-angular.mjs:53,:117` exactly (`commit` = full `git rev-parse HEAD`, `dirty`, `buildTime`, `service` = the angular.json application name, `environment: "local"`), and never overwrites a stamp that is already there. Landed `7a31996e0` — `app/elohim-app/scripts/hc-mesh-prologue.sh` `stamp_build_version()`.

- [x] **Step B: nothing restarts the sign-in portal.** The portal is a bare `ng serve` on `THRESHOLD_PORT` with no supervisor; the workspace RAM guard sheds it like any other fat node process, and the only symptom was every browser scenario timing out inside `threshold-register-display-name` with nothing naming the cause. `hc-mesh.sh` now carries `portal_ready` / `start_portal` / `restart_portal` (a `portal-restart` arm, mirroring the storage and doorway arms), `status` distinguishes "no portal on :8081" from "doorway proxy fault" instead of printing one `down`, and `just test mesh-browser` REFUSES before launch when `<doorway>/threshold/login` is not 200, naming `just mesh portal-restart`. Landed `bceeb0424`.

- [x] **Step C: the hosted cast blows the conductor's RAM — a cast decision, not a bug.** Registering is provisioning now (`should_provision`, 60fb28a39): ~157 MB of conductor heap per cell × 5 cells ≈ 786 MB per registered human. The standing 29 personas took matthew's conductor from 1.1 GB to 22.8 GB and the guard shed the lane (`exit 143`). Until 60fb28a39 those personas rode the `dev_mode` singleton for free — **the cast was designed under a bug**. Decision (chief): on a household mesh only the humans a2o actually signs in as are provisioned. There is no "credential without a cell" path to fall back on — both register branches a registry-configured doorway can take (`hosted`, `node`/`device`) call `provision_agent`, which installs the app when it finds none — so the rest are simply not registered here, and the seeder prints each one with the reason. `seed-humans.ts` carries `HOUSEHOLD_HOSTED_CAST` (14 names, each commented with the feature that signs in as it), applied when `DOORWAY_URL` is loopback; `MESH_HOSTED_CAST=all` restores the standing cast, `=lane` forces the allow-list. `MESH_DOORWAY_MAX_AGENTS` default drops 200 → 34 (14 + the 3 `prologue-hosted-*` = 17, doubled for the doorway's double-count, which comes back out when Task 19b's accounting defect lands). Landed `5cf5f8f38` (allow-list) + `bceeb0424` (the ceiling).

- [x] **Step D: the seeder assumed the steward is the only agent on their conductor.** `seed-conductor-identities` resolved the steward's app by `installed_app_id.startsWith('elohim')` — but a doorway-provisioned hosted app is `elohim-conductor-0-<6hex>`, which matches the same prefix, so the seeder read a stranger's `get_my_human` and reported `[C] Matthew doorway — conductor already embodies '<uuid>'`. A conductor hosting other people's cells IS story 07's topology; the assumption was the defect. The steward is now the agent of the steward's OWN installed app.

  `seed-agent-bindings`' signer mismatch turned out to be the SAME defect one level down, and the coordinator's diagnosis (2026-09-11) put it in this slice: having picked the right conductor, `seed-agent-bindings.ts:198` then picked the wrong APP on it by the same prefix, and `:248` signed `imagodei.create_agent_peer_binding` with that stranger's cell — the zome's signer gate refused it CORRECTLY, over the conductor app websocket from `CONDUCTOR_URLS` (no doorway route is involved). The doorway is not at fault: it binds by exact `installed_app_id` equality (`zome_caller.rs`). So the rule has ONE home — `selectStewardApp()`, exported from `seed-conductor-identities.ts` and imported by `seed-agent-bindings.ts` beside the conductor-affinity helpers it already shares: exact `installed_app_id` first, then a prefix match that is not shaped like a provisioned app, never an id containing `-conductor-`. Copying it is exactly how the affinity fix drifted back to first-match-wins (genesis #1380–#1386). Landed `40247c96c` + `2d8988c6c`.

- [x] **Step E: rerun the lanes these fixes affect.** A cold recast is what makes the allow-list bite — `start` regenerates the sandboxes when peer 0's admin port is free, and the previous run's provisioned apps are otherwise still installed and still resident. It also exposed a second store that had to go with them: the doorways' Mongo archive holds the account rows (identifier, password hash, human_id, agent_pub_key, installed_app_id) that NAME those cells, and it survived the wipe — so `/auth/register` answered 409 for accounts whose cells no longer existed and nothing re-provisioned (all 13 cast humans `[=] exists`; `prologue-hosted-2/3` naming `elohim-conductor-0-5d7c97` on a conductor holding exactly one app). The cold-start decision moved to the top of `start_all` and the archive now goes with the sandboxes (`MESH_KEEP_DOORWAY_DB=1` keeps it). **Storage's own projection DBs still survive a cold start** — that is why `seed-household-formation` reds with "caller is not a current Steward" on a collective minted by a vanished agent. Same class, left open here: it is not one of this task's four gaps, and it wants its own decision about what a household `stop`/`start` is allowed to destroy. Cold recast (`just mesh stop && just mesh start && just mesh wait --timeout 900` — `start` wipes and regenerates the sandboxes when peer 0's admin port is free, which is what makes the allow-list bite: the previous run's 33 provisioned apps are otherwise still installed and still resident), then `just mesh prologue`, then `just test mesh features/dataplane/doorway-failover.feature` and `A2O_RUN_WIP=1 just test mesh features/dataplane/doorway-apex-transition.feature`. The hosted-human lanes stay for a final run after the doorway cell-targeting fix lands. Evidence in the run report below.

**Run (2026-09-11, doorway binary 04:03:51 = `3c7ee8d89`, mesh cold-recast at 04:02):**

Prologue (on the 03:57 doorway build): staging legs **all three green** — `stage-landing-browser-A`, `stage-lamad-spa-browser-A`, `stage-landing-server-A` = 0, where every one was refused before. `seed-conductor-identities` = 0 (was `[C]` conflict). `seed-agent-bindings` = 0. `seed-humans` = 0: **13 registered of 36** (14-name allow-list minus Terrance, who is suspended in deployments.json; 15 off-cast named). `seed-hosted-humans` = 0: **3 live, 3 distinct canonical `uhCAk…` keys, all three with a populated `hostedCellGrantCid`**. `humansServed` = 13 on A, 0 on B. Conductor-0 (matthew): **16 apps / 80 cells, RssAnon 10,459,088 kB ≈ 10.0 GB** — down from 22.8 GB, and the lane survives where it was shed. Still-red Prologue legs are pre-existing and not in this task's four gaps: the base-corpus and stewardship-fixture legs time out on first-anchor (46 s vs a 60 s reconcile sweep on a cold mesh), and `seed-household-formation` / `seed-spool-custody` red on a household collective minted by an agent the cold start destroyed — storage's projection DBs survive a cold start even though the conductors and the doorway archive no longer do.

Lanes (all seven on the 04:03:51 doorway; run ids `20260911T0412…–0415…Z-faca0d95`, reports under `genesis/a2o/reports/sprint-report-household-<run-id>.{json,md}`):

| lane | profile | result |
|---|---|---|
| `doorway-failover.feature` | mesh | **10 scenarios, 10 passed — GREEN** (was 4 red) |
| `doorway-apex-transition.feature` | mesh `@wip` | 2 failed, both `no household-owned membership authority` — **red by missing apparatus, named**, no longer a staging 404 |
| `07-hosted-by-a-household.feature` | mesh | 3 failed — every one `GET /api/v1/commitments/<cid>` 404 on jessica's storage |
| `07-hosted-by-a-household.feature` | mesh-browser | 2 failed — one commitment 404, one `account-hosted-by-household` testid never visible |
| `05-leaving.feature` | mesh-browser | 4 passed, 2 failed — "Agency pipeline has no 'Hosted' step"; one portal sign-in timeout |
| `doorway-humans-served.feature` | mesh | 1 passed, 3 failed — `humansServed=14` vs `0` roster entries with a commitment readable off-pool |
| `doorway-humans-served.feature` | mesh-browser | 1 failed — status not read (cascades from the same) |

The hosted reds now share ONE shape: a hosted-cell commitment is minted (the cid is in the roster) but is **not readable from a household peer that is not the doorway's pool**. That is a dataplane propagation concern, not a mesh-staging one — the first honest measurement of it, which the staging and cast failures were previously hiding. One more thing the `humans-served` lane surfaced: its own re-cast step re-ran `seed-hosted-humans` and got `[X] failed` / `[=] exists` with **no `hostedCellGrantCid`** — the grant leg runs only on FIRST registration, so a re-register of an existing account never re-mints the promise.

**Habit delta line this produces:** `doorway-failover` — "household lane GREEN 10/10 on a locally-built dist (Prologue stamps version.json; all three staging legs land); hosted cast cut 29 → 13 by the lane allow-list, conductor RssAnon 10.0 GB for 16 apps / 80 cells where 29 personas cost 22.8 GB and a shed lane."

---

## Task 17c — The notary is readable off-pool; a closed name is free again; the account page's dates (found by Task 17's household run)

**Drains:** the three product reds run `20260911T041{2,3,5}*-faca0d95` charged, after Task 17b cleared the mesh-side noise hiding them. **Tier:** rust truth-layer (Fable). **Slice:** `elohim/elohim-storage/src/{api/rea_commitments.rs,services/commitment_read_fallback.rs}`, `doorway/doorway-service/src/routes/auth_routes.rs`, `elohim/sdk/schemas/v1/views/account-response.schema.json`. No `app/**` and no mesh scripts.

- [x] **Step A: a notarized commitment is readable by cid from any peer holding the mishpat cell.** Story 07 scenario 3 and its vocabulary ("what the notary records, any peer holding the network can read back") is the spec; `GET /api/v1/commitments/<grant-cid>` on jessica's `:8091` 404'd three for three.

  Measured first, and the hypothesis was too narrow. The 404 was **not peer-local**: matthew's `:8090` — the peer that ISSUED the grant — 404'd on the same cid while listing it under `GET /api/v1/commitments/facing/rea` in the same breath. `GET /api/v1/commitments/{id}` served only the `rea_commitments` (elohim/lamad REA) projection; a notarized Mishpat `Commitment` lands in `mishpat_commitments`, and the mishpat→REA mirror bridges the `replicates-*` actions only (`mishpat_projection::replication_mirror_for`), so a `delegates-compute` grant has no `rea_commitments` row on ANY peer, by design. The cell-local `post_commit` fact is the SECOND layer, not the first: a non-authoring peer additionally has no `mishpat_commitments` row to serve. Both were confirmed live — jessica's own conductor answered `mishpat::get_commitment(uhCEk64woTR-…)` with the full `hosted-cell` grant while her storage 404'd on the same cid.

  So the read cascades, per the p2p-design-gate's Notarized (A) reading — the DHT is truth, SQL is a cache, and a cache miss is not an absence: `rea_commitments` (unchanged fast path) → `mishpat_commitments` → ONE bounded `mishpat::get_commitment` through **this peer's own conductor**, projecting the row on success so the second read is local → 404 only when this peer's own DHT view also answers none. A malformed cid is refused (400) before the uncancellable call is spent; a slug that could never be a cid never spends one at all; an unreachable conductor is an outage (503), never a truthful absence. No fan-out to other peers — a peer answers from its own conductor or not at all.

- [x] **Step B: a CLOSED account's identifier registers again, as a new account.** `humans-served`'s "casts that human again" re-registered `prologue-hosted-1` and got `exists` with no `hostedCellGrantCid`. Story 05-leaving is explicit — "the doorway keeps nothing that would host it" — so registering a closed identifier again is a NEW registration: new cell, new agent key, new grant, and the closed row stays as history, never resurrected.

  The measured chain (doorway A log, 04:15:16–04:15:24) shows the duplicate CHECK was never the blocker: `MongoCollection::find_one` already appends `metadata.is_deleted != true`, so the pre-check passed, a new agent was provisioned, `Hosted: the cell is a notarized promise` was logged — and then no `Registered new user` line, because the insert died on the `identifier_unique` index. The refusal is the unique key, one layer below where it looked. That also **leaks a notarized promise**: `uhCEk64woTR-…` is a live `hosted-cell` grant for an agent whose account row never landed. So the close releases the live handle (tombstoned `closed:<millis>:<identifier>`, with `closed_identifier` keeping the name the human used) and the register pre-check states the intent explicitly by filtering on `is_active`. Releasing the name — rather than dropping the unique index or making it partial — is what keeps every `find_one({identifier})` in `auth_routes.rs` single-valued: at most one row ever answers to a name.

- [x] **Step C: the account page's dates are RFC3339, and that is what the "no Hosted step" red actually was.** `stewardship_at` / `created_at` / `last_login_at` were serialized with `bson::DateTime`'s `Display` (`2026-09-11 4:54:40.628 +00:00:00` — the `time` crate's format, not RFC3339) while `hostedCellValidUntil` on the same response was RFC3339. All three now go through `routes::hosted_cell::rfc3339_utc_secs`, the schema describes them `format: date-time`, and a test asserts every timestamp on `AccountResponse` parses as RFC3339.

  This is also the answer to the third red. `account-hosted-by-household` never rendering was **not** a doorway-side resolver defect: none of the three candidates held. The resolver's failure branch never logged once (`could not read the hosting household's Human` appears zero times in the whole doorway log); `hosted_cell_provider` IS persisted on register (`auth_routes.rs`, the grant's own `provider`); and the running binary already carried the cell-targeting fix `3c7ee8d89` (built 04:03:51, started 04:11:38) when the 04:13 lane ran. Re-measured live on that same binary, an API-registered human returned `hostedByHousehold: "Matthew"` immediately — matching the portal-registered human the doorway-app agent measured at ~04:40. What killed the element was the malformed `createdAt`: `DatePipe` throws `NG02100` and abandons every node below, and `memberSince` renders ABOVE the hosted block in `doorway-account.component.ts`. The component's own `instantOrNull` guard (dfd2c8932) names this exact wire defect in a `TODO(rust-fix)`; with the stamp landed that guard becomes a rail rather than the thing holding the page up.

**Run (2026-09-11, household mesh kept warm; `just gate elohim-storage` EXIT=0, `just gate doorway` EXIT=0; both mesh binaries rebuilt and restarted):**

Step A, on the grant the 04:15 lane left behind (`uhCEk64woTR-…`), measured on jessica's `:8091` — the peer story 07 names as "not the doorway's pool":

```
$ curl -s http://127.0.0.1:8091/api/v1/commitments/uhCEk64woTR-RLRs6-rs6qpmqHNP_tD-O7ZQNxMbUBjUOt50NKmst
{"cid":"uhCEk64woTR-RLRs6-rs6qpmqHNP_tD-O7ZQNxMbUBjUOt50NKmst","action":"delegates-compute",
 "scope":"hosted-cell","provider":"uhCAkwZmnsxA_FMajziYQDbvwWpGX49rxBh-CZyEmMI5Q7_3iB5Fu",
 "recipient":"uhCAkRB5x3YIURVWFhObwjYWW6zuf6n_iMfhyp3AykeQJCMNyUkl9",
 "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":30},
 "validFrom":"2026-09-11T04:15:24+00:00","validUntil":"2026-10-11T04:15:24+00:00","state":"proposed",
 "dhtAnchorHash":"uhCkkWgW02Pifws33XttLc_P6ea1NoySbhZAXZzjRfgfli3s4lftC","createdAt":"2026-09-11T05:28:07Z"}
HTTP=200
```

`scope` is `hosted-cell`, `provider` is the pool peer's own key (not the doorway's), `recipient` is the human's own agent key, `validUntil` is in the future. The second read of the same cid is served from the row the first read projected. All three peers now answer (matthew `:8090` and james `:8092` included — matthew was 404 before, being the author). The rest of the cascade, live: a valid-but-absent EntryHash → **404**; a truncated cid → **400**; an existing `rea_commitments` slug → **200 in its unchanged shape**; an unknown slug → **404**.

Step B, a full cycle on a fresh identifier: register `t17c-cycle-1` → 201, agent `uhCAk5y7Lvz…`, grant `uhCEkHcAuUXf…`; close → `{"closed":true,"hostedCellGrantRevoked":true}`; **re-register the same identifier → 201 with a NEW agent `uhCAk9IeX8wR…` and a NEW grant `uhCEkkzlJBsM…`**. The name now resolves to the new account; the closed row keeps its history under `closed_identifier` and is unreachable by that name. The pre-17c leftovers heal too: `prologue-hosted-1`, whose closed row still held its name from the 04:15 run, re-registered **201** with a fresh grant.

Step C, on the same live doorway: `GET /auth/account` now answers `"createdAt":"2026-09-11T04:54:40Z"` where it answered `"2026-09-11 4:54:40.628 +00:00:00"` an hour earlier — and carries `"hostedByHousehold":"Matthew"` on both the API and portal registration paths.

**Residue, named rather than left:** the notary read-back projects the lifecycle the ENTRY implies, not the `CommitmentByState` links off the commitment's anchor, so a commitment revoked provider-side and first seen by a non-authoring peer through this path caches as unrevoked until a reconcile reads the links (single-call budget; recorded as a `gapNote` on the `read_notarized_with` seam row). A commitment action with no `mishpat_commitments` row shape (`revokes-commitment`, `author-lens`, identity-head) is answered absent with a warn. And the grant leak above is a doorway ordering concern — the promise is notarized before the row is committed — which Step B stops reproducing but does not itself reorder.

---

## Task 17 — closing note (chief, 2026-09-11 07:15)

Step 2 stays unticked on its own terms (seven runs green, 0 skipped). What the household lane proved and where the register now stands: `hosted-human-lifecycle` **GREEN** (05-leaving 6/6, run 20260911T065537Z-d48f3b69, real hosted provisioning); `doorway-failover` steady-state **10/10** three times on three binaries (RED by its apex-transition rule; apex-transition now a MEASURED red at the named step); `hosted-cell-promised` born RED with three named causes (commitment convergence > 60 s for 6 of 17 to a non-authoring peer; conductor-0 admin socket drops under install_app load; closed-identifier recast). Register: 21 habits, 10 green · 9 red · 2 unwired. The push (Task 19) carries all of it; the fleet confirms.

## Task 18 — (b) The epr-atom-home measure against the deployed alpha, now

**Drains:** epr-atom-home plan **Task 8** (fleet half) and verifies **Task 1**.
**Tier:** Sonnet.

**No deploy is needed.** Alpha already carries **face02de** (2026-09-09), which is after `cc9cbe385`. This is a measure task.

- [x] **Step 1: Verify epr-atom-home plan Task 1 (5 minutes)**

```bash
cd /projects/elohim
head -5 app/elohim-app/src/styles.css
ls -l app/elohim-app/src/styles/brand.css
grep -n "fontsource" app/elohim-app/package.json
```
Expected (measured 2026-09-10): `styles.css` line 3 is `@import './styles/brand.css';`, `brand.css` exists, and all four `@fontsource-variable/*` deps are present at lines 58–61. That is Task 1 landed — tick it with this evidence line. If any is missing, execute that plan's Task 1 before continuing.

- [x] **Step 2: Tick Tasks 2–7 with the ledger's shas — do not re-implement**

In `genesis/docs/superpowers/plans/2026-09-02-epr-atom-home-frame-plan.md`, tick every step checkbox under Tasks 2–7 and add one line under each task heading:

```markdown
> LANDED on dev per `app/elohim-app/.epr-meta/epr-atom-home.habit.md` DELTA 2026-09-02b: fb0117114 … cc9cbe385 (a2o tail b8b30686a). Verified, not re-implemented, 2026-09-10.
```

- [x] **Step 3: Run the concern against the deployed origin**

```bash
cd /projects/elohim/genesis/a2o
ELOHIM_CAP_OWNED_SUBSTRATE_STATUS=available \
E2E_DEVICE_MODE=playwright \
E2E_APP_URL=https://alpha.elohim.host \
E2E_DOORWAY_ALPHA=https://alpha.elohim.host \
pnpm exec cucumber-js --tags '@concern:epr-atom-home and not @wip'; echo "EXIT=$?"
```
Expected: `7 passed / 0 failed`, `EXIT=0`. (The `--tags` filter is what scopes this run — a bare positional would merge with the default profile's `features/**` paths.) The three `@wip` commons scenarios stay excluded; they belong to the commons plan.

- [x] **Step 4: Take the render shot and file it durably**

```bash
cd /projects/elohim/genesis/a2o
pnpm look https://alpha.elohim.host/epr/evolution-of-trust; echo "EXIT=$?"
mkdir -p reports/look/epr-home-alpha-face02de
cp -r reports/look/latest/. reports/look/epr-home-alpha-face02de/
python3 -c "import json;d=json.load(open('reports/look/epr-home-alpha-face02de/capture.json'));print('pageErrors',len(d.get('pageErrors',[])),'httpErrors',len(d.get('httpErrors',[])))"
```
Expected: 0 pageErrors, 0 httpErrors; the shot shows the four legs, the custody meter and the address footer, with no `viewer-back-home` chrome.

- [ ] **Step 5: Flip the habit**

Only if Step 3 was `EXIT=0`. Prepend a DELTA to `app/elohim-app/.epr-meta/epr-atom-home.habit.md`, set `status: green`, leave `active: false`, then re-project:

```bash
cd /projects/elohim
python3 .claude/scripts/habits-project.py; echo "EXIT=$?"
python3 .claude/scripts/habits-status.py --full | grep -A3 epr-atom-home
git add app/elohim-app/.epr-meta/epr-atom-home.habit.md genesis/manifests/habits.yaml \
        genesis/docs/superpowers/plans/2026-09-02-epr-atom-home-frame-plan.md
git commit -m "habit(epr-atom-home): RED -> GREEN on the deployed fleet render at face02de"
```

**Habit delta line this produces:** `epr-atom-home` — "DELTA 2026-09-10 (RED → GREEN on fleet evidence): alpha deployed at build stamp **face02de** (2026-09-09) renders /epr/{id} in the shell frame. `@concern:epr-atom-home` 7 passed / 0 failed against `https://alpha.elohim.host` with `E2E_DEVICE_MODE=playwright`; `pnpm look https://alpha.elohim.host/epr/evolution-of-trust` → `genesis/a2o/reports/look/epr-home-alpha-face02de`, 0 pageErrors / 0 httpErrors, four legs and address footer present, no viewer-back-home. Slice 1 shas fb0117114 … cc9cbe385 verified landed; plan Tasks 1–7 ticked from the ledger, not re-implemented. The three commons scenarios stay @wip for the commons plan."

---

## Task 18b — lamad blob URLs leak the local sidecar address (found by Task 18, 2026-09-11)

**Drains:** new — surfaced by the Task 18 measure. **Tier:** Opus (angular-architect). **Slice:** app (lamad bundle).

Task 18 Step 3 ran twice against alpha (`face02de`): 5 passed / 2 failed both times. One failure was the
`"1 of 3"` custody literal (fixed, `06bafc316` — the floor is now read from the doorway's own household
report). The other is real: scenario "The learning app is one lens away" passes but its After-hook fails on
`net::ERR_CONNECTION_REFUSED` for `http://localhost:8090/blob/sha256-c5dcc24c…`. A `pnpm look` of
`https://alpha.elohim.host/lamad/path/foundations-christian-technology` reproduces it as the page's one
failed request: the lamad bundle resolves blob references through `ILamadStorageClient.getBlobUrl`
(`app/lamad/src/app/services/blob-manager.service.ts:162`) to `environment.client.storageUrl` — the
local-sidecar address — with lamad carrying one `environment.ts` and no build-time replacement. Every visitor
on alpha gets that blob refused. Rule: in doorway mode a blob URL is origin-relative (`/blob/{hash}` on the
serving origin), never `storageUrl`; `storageUrl` is direct/native mode only. Same bundle must serve from
both doorways (doorway-failover: "whichever serves resolves the same declared head").

- [x] **Step 1:** failing lamad unit test — doorway mode + served origin → origin-relative blob URL, never `localhost:8090`; direct mode unchanged. *(blob-url.spec.ts, 10 tests red→green)*
- [x] **Step 2:** implement in the concrete `ILamadStorageClient`; lamad gate + direct `ng build`; pathspec commit. *(e09eec608 — root cause one layer deeper: SSR `detectConnectionMode()` returns `direct` on Node, so server-rendered HTML baked `storageUrl`; cured at lamad's composition root with `withOriginRelativeBlobUrls`; `just gate elohim-app` 4596 tests, lamad 2822 tests, AOT+SSR build, lint ratchet — all EXIT=0. General cure + transfer-state drift filed: backlog `ssr-connection-mode-conflates-node-with-native-2026-09-11`; lamad dev-environment-in-production folded as arch-frontend-bundle-seams row 11.)*
- [ ] **Step 3:** rides the Task 19 app deploy; then Task 18 Step 3 re-runs against alpha — **that** run is the flip evidence (expected 7/7).

**Habit delta line this produces:** `epr-atom-home` — "lamad blob URLs origin-relative in doorway mode (was localhost:8090 on alpha, found by the Task 18 measure); flip waits on the app build that carries it."

---

## Task 19 — (c) ONE fleet push, then read the build

**Drains:** doorway-failover plan **§Operator menu** (the deploy decision) and D7(ii).
**Tier:** operator for the push; `ci-observer` (Haiku) for the read.

**This is the only push in this plan, and it is the operator's act.**

- [ ] **Step 1: Preconditions — check them all before pushing**

```bash
cd /projects/elohim
git log --oneline origin/dev..HEAD
curl -s https://alpha.elohim.host/p2p/status | python3 -m json.tool | grep -i caughtUp
curl -s -o /dev/null -w '%{http_code}\n' https://alpha.elohim.host/db/content/elohim-host-landing
curl -s -o /dev/null -w '%{http_code}\n' https://doorway-alpha.elohim.host/db/content/elohim-host-landing
ls -t genesis/a2o/reports/sprint-report-household-*.json | head -1
```
Required before pushing: **no build currently running** (a superseding run cancels a roll mid-rollout); matthew `caughtUp: true`; both doorways `200`; and a household sprint-report from Task 17 newer than the dataplane changes (otherwise pre-push prints `NO-T2-RECEIPT`, which is warn-only unless `T2_RECEIPT=strict`).

- [ ] **Step 2: Push once, with the dispatch tag**

```bash
cd /projects/elohim
git commit --allow-empty -m "measure: doorway federation drain — hosted cells, humans served, apex measured [build:edge] [build:app]"
git push
```
`[build:edge]` rolls the doorway + storage commits; `[build:app]` is unnecessary if Task 18 already flipped `epr-atom-home` on `face02de` — drop it in that case. **Do not** fire a bare `[build:edge]` "just to measure": that restarts the seven pods it is measuring (~20 min churn plus hours of catch-up) and is the documented measurement-by-deploy anti-pattern. To re-measure without rolling, the tag pair is `[build:edge] [edge:validate-only]`.

- [ ] **Step 3: Read the build with `ci-observer`**

Dispatch the `ci-observer` agent (Haiku, summarize mode) on the elohim-edge build the push triggers. Ask it for: the build number and result, the deployed commit, the Dataplane Validation stage's scenario counts, and whether `served-shell-boots` and `epr-app-deliverability` passed at both origins. Escalate to `ci-investigator` only if it reports low confidence.

Do **not** call `mcp__jenkins__triggerBuild` or `updateBuild` — the MCP registration is anonymous and those are denied. Builds are triggered only by a push with a `[build:*]` tag.

- [ ] **Step 4: Re-probe both origins after the roll**

```bash
for h in alpha.elohim.host doorway-alpha.elohim.host; do
  echo "== $h"; curl -s https://$h/version.json | python3 -m json.tool | head -6
  curl -s https://$h/status.json | python3 -c 'import sys,json;d=json.load(sys.stdin);print("humansServed",d.get("humansServed"))'
done
```
Expected: both `/version.json` name the new build's commit, and both `/status.json` carry a `humansServed` number (`0` on a doorway hosting nobody — no longer `null`).

**Habit delta line this produces:** `doorway-failover` — "edge #`<n>` deployed `<sha>`; Dataplane Validation `<n>` scenarios passed; served-shell-boots and epr-app-deliverability green at both origins; `/version.json` names `<sha>` on alpha.elohim.host and doorway-alpha.elohim.host."

---

## Task 19b — Fleet doorways need the pool-compute wiring the mesh needed (found by Task 17, 2026-09-11)

**Drains:** D2 on the fleet. **Tier:** operator values + Haiku/Sonnet manifest edit. **Slice:** `genesis/orchestrator/manifests/doorway/{alpha,alpha-b}.yaml` (+ the storage side under `manifests/edgenode/` if `ELOHIM_COMPUTE_LOCAL_API` is not already set there). Repo manifests are the cleanup surface; the cluster is the operator's — never `kubectl`.

Task 17 found the household mesh never provisioned a hosted human until 60fb28a39 (dev_mode singleton path), and that `hc-mesh.sh` set neither `HAPP_BUNDLE_PATH` nor `POOL_COMPUTE_URL/TOKEN/PERFORMER`. The fleet is half-wired: every doorway manifest sets `HAPP_BUNDLE_PATH` (grep 2026-09-11: alpha.yaml:238, alpha-b.yaml:289, …), so after the Task 19 push hosted registration on alpha provisions a real cell with its own key (the 2026-09-04 "register returns the operator's profile" baseline is cured by that alone). But `POOL_COMPUTE_*` appears in no manifest, so the grant leg is skipped (tested: `hosted_register_without_pool_compute_config_still_registers`) and `humansServed` reads `Some(0)` — honest, not the number.

- [ ] **Step 1 (operator):** decide the pool provider per doorway — alpha → `elohim-adam-alpha` storage (`/status.json` names it as alpha's storage); alpha-b → its primary. Supply `POOL_COMPUTE_PERFORMER` = that storage peer's own conductor agent key (`hc.agent_key_uhcak()`; read it from the peer, never from the doorway) and `POOL_COMPUTE_TOKEN` = the bearer the grant surface expects under `ELOHIM_COMPUTE_LOCAL_API=1` (a k8s Secret, referenced by name — never a literal in the manifest).
- [ ] **Step 2:** add the three env entries to both doorway manifests beside `HAPP_BUNDLE_PATH`; add `ELOHIM_COMPUTE_LOCAL_API=1` to the corresponding storage manifests if absent (grep first). Comment the WHY inline (the singleton path is gone; a hosted cell is a notarized promise).
- [ ] **Step 3:** rides the next edge deploy; evidence = `humansServed > 0` on `/status.json` after a real registration, and a `hostedCellGrantCid` on that registration's response.

**Habit delta line this produces:** `hosted-human-lifecycle` — "fleet doorways carry POOL_COMPUTE_* (edge #N); first fleet hosted-cell grant cid <cid>."

---

## Task 20 — (d) Ledger deltas and flips

**Drains:** hosted-human plan **Task 8**; doorway-failover plan's habit bookkeeping.
**Tier:** chief (the model driving this plan).

- [ ] **Step 1: Write the deltas**

Prepend one DELTA to each habit atom, composed from the delta lines each task above produced. Every DELTA names a run id or a build number. Never write intention.

- [ ] **Step 2: Flip `hosted-human-lifecycle`**

If Task 17 Step 2's `05-leaving`, `07-hosted-by-a-household` and `doorway-humans-served` runs all passed with 0 undefined and 0 skipped: set `status: green`, leave `active: false`.

- [ ] **Step 3: Do NOT flip `doorway-failover` — and say why in the atom**

Task 19 turns the **steady-state clauses** green on that build number. The habit stays **red**. Write this into the DELTA verbatim so no later reader mistakes the steady-state green for graduation:

> Steady-state clauses green on edge #`<n>` (`<sha>`); the HABIT stays RED. The graduation rule is unchanged: `doorway-apex-transition.feature` must have every scenario and every step passed, and WAN ingress continuity is a distinct prerequisite that multi-A membership does not imply. Apex transition is now MEASURED rather than undefined (run `<id>`), which is progress against the red, not a discharge of it.

- [ ] **Step 4: Re-project and verify the register**

```bash
cd /projects/elohim
python3 .claude/scripts/habits-project.py; echo "EXIT=$?"
python3 .claude/scripts/habits-status.py --full
```
Expected: `habits.yaml` regenerates clean; `hosted-human-lifecycle` green, `doorway-failover` red, `epr-atom-home` green; **`active:` unchanged on all three** — the WIP fence is full and promotion is the operator's call, which this plan does not make.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md \
        doorway/doorway-service/.epr-meta/doorway-failover.habit.md \
        genesis/manifests/habits.yaml
git commit -m "habit: hosted-human-lifecycle RED -> GREEN; doorway-failover steady-state green, habit RED preserved"
```

---

## Task 21 — WAN ingress continuity (HELD — not household work) `@requires:alpha-cluster-6peer`

**Drains:** doorway-failover plan **Task 3.1** (apex multi-A manifest, DEMOTED-TO-OPERATOR) and D7(iii)'s WAN clause.
**Tier:** operator decision + Sonnet execution once the capability is in hand.

This task is **held on purpose**, tagged so `--focus` drops it rather than counting it as household work that nobody did. The capability tag is `@requires:alpha-cluster-6peer` — the real, declared resource name in `genesis/manifests/cluster-state.yaml`. A bare `alpha-cluster` is not a declared cap and would trip the `⚠ unknown-cap` drift check.

**Why it cannot be done at home, stated once:** `doorway-apex-transition.feature`'s own preamble refuses a test-only proxy as certification, and the household owns no membership authority — `relay-addr-beacon` reconciles owner records through the Cloudflare sink only (`relay-addr-beacon/src/sinks/`), and `hc-mesh.sh` stages none.

- [ ] **Task 21 (held) `@requires:alpha-cluster-6peer` — prove WAN ingress continuity for the apex name:** apply the operator-gated apex multi-A manifest diff (failover plan Task 3.1), then run `doorway-apex-transition.feature`'s membership scenario against the real owner records with one doorway induced to shed, and record the build number and run id. Until this and every step of `doorway-apex-transition.feature` pass, `doorway-failover` does not graduate.

---

## Complementary work captured, not planned

- **05-leaving.feature blind-reader findings deferred by the chief (2026-09-11, READY verdict after 60c3b4a36):** (1) "agency pipeline" is asserted (line 58) but never defined in the preamble — it is carried by `../agency-pipeline-coherence.feature`; add one defining sentence or move the assertion. (2) "the password they registered with" (lines 66, 123) names a credential no step ever shows being chosen — surface it in the registration step or a preamble note. Minor: station/finish-line relationship lives in a comment; `E2E_DOORWAY_ALPHA` unexplained; "a second browser" has no Given. Owner: the hosted-human series author, next authoring pass.


One finding from the 2026-09-10 probes, **not in this sprint's scope** — file it as a one-line backlog atom, do not create the file as part of this plan's tasks:

- **Two doorways project different custody views of one CID.** `/epr/evolution-of-trust` reads "Held by 1 of 3 households" on `elohim.host` (apex) and "Held by 3 of 3 (Eden, Matthew's, Susan's)" on `alpha.elohim.host` — same CID, two answers, so at least one doorway's custody projection is stale or partial. Domain D8. File as `genesis/data/timeline/backlog/doorway-custody-view-diverges-per-doorway-2026-09-10.md`.

---

## Self-review

**Brief coverage.** D1a → Task 7; D1b → Task 8; seam rows + contract tests → Tasks 7, 8; D2 → Tasks 13, 15, 16 (+ story 07, Task 1); D3 → Task 14 + story humans-served (Task 2); D4 → Task 5; D5 → Tasks 1, 2, 3, 4; D6 → Task 18; D7(i) → Task 17, D7(ii) → Task 19, D7(iii) → Tasks 6, 17, 21. Drained plan tasks: hosted-human 1→Task 4 (verify), 2→7, 3→9, 4→10 (verify), 5→11, 6→12, 7→13, 8→Tasks 17+20; epr-atom-home 1→Task 18 Step 1 (verify), 2–7→Task 18 Step 2 (tick from ledger), 8→Task 18; failover 0.1→Task 17, 1.2→Task 6, 3.1→Task 21.

**Type consistency.** `should_provision(registry_configured, dev_mode)` — same name and arity in Task 7's tests, implementation, call sites and seam row. `should_subscribe_to_signals(projection_writer)` — same in Task 8. `hosted_cell_grant_cid` / `hosted_cell_valid_until` on `UserDoc` (Task 13) are the exact fields Task 14 counts and Task 5 prints; `hostedCellGrantCid` is the camelCase wire spelling on both the register and account responses. `data-testid`s `account-close-begin`, `account-close-confirm-input`, `account-close-confirm`, `account-close-error`, `account-hosted-by-household`, `account-hosted-until` are named identically in Task 4 Step 2 and Task 11.

**Known gaps, named rather than papered over.** (1) `doorway-apex-transition` cannot go green at home; Task 6 delivers a measured red and Task 21 holds the rest. (2) Task 13's grant leg needs pool-compute configuration on the doorway; without it, registration is unchanged and `humansServed` reports `Some(0)` — honest, not broken. (3) The `07` story's two `@browser-only` scenarios run under `mesh-browser`, not `mesh`; both commands are named wherever the file is run.
