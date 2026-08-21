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

- Act I baseline: `household-nodes doorway doorway-pair multi-node seeded-content seeded-humans mongo-archive owned-substrate epr-cli`
- Act II adds: `alpha-cluster-6peer dht-anchored-content per-human-conductor apex-dns tls deploy-churn observability harbor-registry jenkins ssr-bundle` — and **drops** `owned-substrate`
- Act III adds: `shem`

So an Act I scenario that needs the lamad SPA carries `@act:i @requires:ssr-bundle`, and nothing else.
Existing fixture-precondition tags (`@requires:doorway`, `@requires:seeded-content`, …) stay where they
are as documentation; the tag-plan removes only caps that actively **lie** — `@requires:shem` on a
scenario no longer cast in the commons, `@requires:alpha-cluster-6peer` on an Act I scenario.

**A cap that is not declared in the cluster-state file the run is reading is a NO-OP.**
`substrate-scope.ts::unavailableRequiredCaps()` deliberately fails open on an unknown cap and treats it
as a fixture precondition. That is why both act files declare *every* cap the tag-plan uses, including
the ones that are `true`. Adding a new `@requires:` word without adding it to the act cluster-states
gates nothing and reads, falsely, like a gate. A lane opts in with
`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=<act file>`; runs that don't set it keep reading `cluster-state.yaml`,
so today's gating behaviour is unchanged until a lane asks for the act contract.

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
- `layering/profiles.md` — the `mesh` cucumber profile, the env block the mesh must emit (including the
  household fixture manifest with `processControl: true`), and the CI narrowing per act.
- `layering/code-reds.md` — the 25 real defects the mesh run produced for free, after subtracting the
  env-red classes.
- `genesis/manifests/cluster-state.act1-household.yaml`, `cluster-state.act2-neighbourhood.yaml` — the
  lane contracts.
