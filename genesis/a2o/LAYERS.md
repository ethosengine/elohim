# LAYERS — the a2o suite as three acts of one story

The suite is not a pile of tests with environment tags. It is **one story at three scales**, and every
scenario belongs to exactly one of them. A scenario **declares its act**; its `@requires:` caps and its
pipeline placement are *consequences* of that declaration, not independent facts to be maintained.

## The three acts

| | Act I — the household | Act II — the neighbourhood | Act III — the commons |
|---|---|---|---|
| **Cast** | matthew, jessica, james | + adam's household (and the deployed community peers) | community humans, affinity groups, local economy |
| **Stage** | peers :8090/:8091/:8092 · doorway A :8888 (matthew-primary) · doorway B :8889 (jessica-primary, pool matthew+james) · a mongod archive per doorway | the alpha fleet: real hosts, apex DNS, TLS, deploy churn, the observability stack, Jenkins | shem — the multi-tenant live P2P canvas |
| **Substrate** | local mesh / CI mesh — **strict, and it owns its substrate** (`processControl: true`) | `alpha-cluster-6peer` | `shem` |
| **What it proves** | device awakens → household forms → first in-family co-steward agreement. NetworkStage Simulacra → Bootstrap | adam's household federates; doorway B becomes adam's; cross-household agreement notarized → Coordinated | earned reach across communities → Enforced |
| **Lane contract** | `genesis/manifests/cluster-state.act1-household.yaml` | `cluster-state.act2-neighbourhood.yaml` | `cluster-state.yaml` (`shem: available: true`) |

Two lanes cut across the acts rather than sitting inside one:

- **browser** — Playwright is the *driving mechanism*, not a substrate. A browser scenario still has an
  act; 171 of the 181 browser scenarios are Act I underneath. Keep `@browser` / `@browser-only`; they say
  how the scenario drives, and the act tag says what it needs.
- **host** (`@act:host`) — 101 scenarios assert only on repo JSON (`devices.json`, `deployments.json`) or
  on the `epr` CLI against a scratch git root. No substrate at all. They are not "ci-only": they run in
  any lane, including a laptop.

## The rule: declare the act, and the caps follow

Each act's cluster-state file declares a **baseline** — the caps that act provides by definition. The act
tag carries the baseline; `@requires:<cap>` is emitted **only for a cap outside it**.

- Act I baseline: `household-nodes doorway doorway-pair multi-node seeded-content seeded-humans mongo-archive owned-substrate epr-cli ssr-bundle`
- Act II adds: `alpha-cluster-6peer dht-anchored-content per-human-conductor apex-dns tls deploy-churn observability harbor-registry jenkins` — and **drops** `owned-substrate`
- Act III adds: `shem`

`ssr-bundle` is Act I baseline because the **Prologue stages it** (`just mesh prologue` PATCHes the
landing browser+server and lamad-spa browser bundles through doorway A and DECLARE_ONLY-propagates
each head to B). It is a stage step, not a substrate acquisition — which is exactly why an Act I
scenario that renders the lamad SPA needs no `@requires:` tag at all.

Existing fixture-precondition tags (`@requires:doorway`, `@requires:seeded-content`, …) stay where they
are as documentation; the tag-plan removes only caps that actively **lie** — `@requires:shem` on a
scenario no longer cast in the commons, `@requires:alpha-cluster-6peer` on an Act I scenario.

### How the gate resolves it (`src/framework/fixtures/substrate-scope.ts`)

Two inputs, one rule. The **act** is a claim about what the scenario needs; the **lane** is a claim
about what this run has. A scenario runs when

> `actBaselineCaps(act) ⊆ lane-available` **and** every explicit `@requires:<cap>` is lane-available.

| | resolved from |
|---|---|
| act **baseline** | that act's own contract file: `i` → `cluster-state.act1-household.yaml`, `ii` → `cluster-state.act2-neighbourhood.yaml`, `iii` → the live `cluster-state.yaml` **plus `shem`**, `host` → **no caps at all** |
| the **lane** | `ELOHIM_CLUSTER_STATE_PATH_OVERRIDE`, defaulting to the live `cluster-state.yaml` |

An act's baseline is literally "every resource that act's file declares `available: true`" — so
`available: false` in an act file is what *drops* a cap from the baseline (Act II dropping
`owned-substrate` is exactly this, not a separate mechanism). Per-cap env overrides
(`ELOHIM_CAP_<UPPER_SNAKE>_STATUS`, and `ELOHIM_REMOTE_COMPUTE_STATUS` for `shem`) still win over
the lane file, for baseline caps as well as explicit ones.

A held scenario names the act in its skip line, so the reason is legible without cross-referencing:

```
⏭️  HELD (substrate): "Doorway B serves adam's household" requires unavailable
    owned-substrate (act i baseline) — skipped, not failed.
```

`@act:host` is never held by an act baseline — it has none. 101 scenarios assert only on repo JSON or
the `epr` CLI against a scratch git root; they run in any lane, laptop included.

**A cap that is not declared in the cluster-state file the run is reading is still a NO-OP — but it is
no longer a SILENT one.** `unavailableRequiredCaps()` fails open on an unknown cap (an undeclared cap
must never invent a gate) and now **warns loudly, once per run per cap**:

```
⚠️  UNDECLARED CAP: @requires:iroh GATES NOTHING — "iroh" is not a resource in
    genesis/manifests/cluster-state.act1-household.yaml. Declare it there with an
    `available:` line, or drop the tag …
```

The same fail-open-but-declared rule applies to act baselines: a baseline cap the *lane* file does not
declare is not a gate either. That is what keeps a lane which has not opted into an act contract
behaving exactly as it did before the layering — runs that don't set
`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE` keep reading `cluster-state.yaml`, where most of this vocabulary
is undeclared. It is also why both act files declare *every* cap the tag-plan uses, including the ones
that are `true`.

Two `@act:` tags on one scenario is an authoring error: the gate warns (`MULTIPLE ACT TAGS`) and gates
on the first.

### Running the mesh lane

```bash
just mesh start && just mesh prologue     # bring up the household and cast it
just test mesh                            # the whole Act I lane
just test mesh features/dataplane/doorway-failover.feature   # scoped to one feature
just test mesh '@act:i and @dataplane'                       # scoped by tag expression
```

`just test mesh` sources `hc-mesh.sh`'s `mesh_seed_env` and re-derives the Prologue's a2o env block —
`E2E_DOORWAY_ALPHA/_B/_BETA`, `E2E_STORAGE_URL`, `E2E_STORAGE_B`, `E2E_STORAGE_<PEER>`,
`E2E_DOORWAY_POOL_STORAGE_URLS`, `E2E_HOUSEHOLD_FIXTURE_PATH=/tmp/elohim-local-mesh/household-fixture.json`,
`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml`,
`ELOHIM_REMOTE_COMPUTE_STATUS=unavailable`, `CUCUMBER_JSON_REPORT=/tmp/elohim-local-mesh/reports/mesh.json`
— then runs `cucumber-js --profile mesh`. The profile carries its own tag filter
(`@e2e and not @wip and not @browser and not @browser-only`); a tag argument is **ANDed** onto it by
cucumber, never substituted for it.

**The scoping rule, because it has bitten twice.** cucumber-js *merges* a profile's `paths` with CLI
positionals instead of replacing them, so `cucumber-js --profile mesh features/x.feature` runs the
whole tree **plus** that file. When a scope argument is given, `just test mesh` therefore generates a
config at `genesis/a2o/reports/cucumber-mesh-scoped.mjs` that re-exports the `mesh` profile **minus
`paths`**, and passes it as `--config`. Two consequences worth knowing: cucumber resolves `--config`
with `path.join(cwd, …)`, so the path must be **relative to `genesis/a2o`** (an absolute one is
silently mangled); and with no `paths` at all cucumber's own default is `features/**/*.feature`, so an
unscoped fallback is identical to the profile. The same merge trap is why `saga` stays directory-scoped
and is measured with `--profile saga`, never with bare CLI paths.

`available: false` in an act file is a **lane statement, never a health report**. `shem: false` in Act II
does not contradict `shem: available: true` in `cluster-state.yaml` — it says the neighbourhood lane does
not run the commons.

## Two discriminators that are usually mistaken for peer count

- **`owned-substrate`** is the real Act I/Act II boundary. Not "how many peers" — *may this run write,
  delete, re-seed, restart, kill and tail?* On shared alpha the answer is no by policy (see
  `CLAUDE.md` § Authorized writes on shared alpha) and by physics (a deployed doorway logs to a pod's
  stdout; there is no file to tail, no PID to signal). 35 scenarios are `@wip` for exactly this reason
  and for no other. On a mesh the suite owns, they are Act I.
- **`observability` is not "a metric assertion".** Every metric step in this suite reads the *service's
  own* `/metrics` text endpoint on elohim-storage or the doorway. No step queries Grafana, Loki or
  Prometheus. Metric-bearing saga chapters are therefore Act I; the `observability` cap is reserved for
  scenarios that query those servers.

And the perennial one, already in `CLAUDE.md`: **`shem` ≠ multi-node.** The household triad is itself a
three-node cluster.

## The census, honestly

949 scenarios in `features/**`. 516 carry `@wip`; 174 have no `@e2e`; 65 are `@browser`. **194 — 20% —
were eligible to run anywhere**, and that run (against the mesh, 2026-08-21) returned 101 passed, 39
failed, 27 held, 26 pending, 1 undefined. Joining that run against a full four-reader classification of
all 949 scenarios gives: 90 micro-green, 54 micro-red, 20 mismatch, 16 held-ok, 10 env-red, 759 never
executed by any lane.

After the layering (`layering/tag-plan.json`):

| | Act I | Act II | Act III | host | browser lane (cross-cutting) |
|---|---|---|---|---|---|
| scenarios | **773** | 20 | 55 | 101 | 181 |
| less hold candidates | 728 | 15 | 52 | 92 | — |

62 scenarios are **hold** candidates — unfalsifiable, superseded, duplicative of a unit/pre-push gate, or
asserting a surface that was deliberately deleted. 35 more are **held-by-form**: `@wip` only because a
shared substrate forbade the write, delete or kill they need. Those are Act I work items, not fleet ones.

The headline is not "773 scenarios pass on a laptop." It is that **773 of 949 need nothing a household
cannot provide**, and the suite has been pricing most of them as fleet work. The gap between 773 and
today's 194-eligible is step debt, `@wip` debt, and one unstaged SPA bundle — not substrate.

## Where the rest of this lives

- `layering/tag-plan.json` — machine-applicable, per feature: act, tags to add/remove, persona swaps,
  holds, held-by-form.
- `layering/profiles.md` — the `mesh` cucumber profile (now LIVE in `cucumber.mjs`), the env block the
  mesh must emit (including the household fixture manifest with `processControl: true`), and the CI
  narrowing per act.
- `src/framework/fixtures/substrate-scope.ts` — the runtime gate: `actFromTags`, `actBaselineCaps`,
  `unavailableRequiredCaps`. Tests: `src/framework/fixtures/__tests__/substrate-scope.test.ts`.
- `layering/code-reds.md` — the 25 real defects the mesh run produced for free, after subtracting the
  env-red classes.
- `genesis/manifests/cluster-state.act1-household.yaml`, `cluster-state.act2-neighbourhood.yaml` — the
  lane contracts.

## Wire vs implement vs design — how a red or a `@wip` is classified

Three mechanical facts, read in order (made runnable by `pnpm census` → `layering/surface-census.md`):

1. **Does the surface the scenario asserts on exist?** Route answers non-404, metric appears in `/metrics`,
   field appears in the response, CLI verb exists. *Exists* → the gap is wiring or a defect, never design.
   *Absent* → **IMPLEMENT** — *bounded* when it is a missing metric/field/registration, *design* when it is
   a missing capability (`POST /p2p/sync-mode` → 404 with no syncMode concept anywhere is a feature, not a wire).
2. **Is the precondition constructible on the substrate we own?** Unseeded/unset → **FIXTURE** (Prologue,
   seed, cast — still wiring, different file). Impossible in the topology (three peers cannot discriminate
   two losses; a path that only runs with an embedded conductor) → **STRUCTURAL**: re-aim the story or
   change the topology — a design decision, never a step.
3. **Do the steps bind?** (scoped dry-run: bound / partial / none). Unbound + surface exists + constructible
   → **WIRE**, the cheapest class. Unbound + surface absent → a story ahead of its code — design.

The residual — bound, constructible, surface exists, still red — is either a **DEFECT** (reach PATCH that
silently applied nothing) or a **STALE** assertion (sha256 where the wire now carries a CID). Only this
cell needs a human read; the blind-reader verdict and "does a spec doc cite the behaviour" decide which.
Never weaken an assertion to move a scenario out of this cell.

