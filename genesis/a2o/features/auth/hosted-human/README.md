# The hosted human — a series

Three words, before anything else. A **doorway** is the gateway service that makes the
peer-to-peer network reachable from an ordinary browser; it serves web pages, answers an HTTP
API, and can run small pieces of the network on people's behalf. The **portal** is the sign-in
and account web UI the doorway serves (the pages a person actually sees); "the doorway" in a
scenario means the service answering the API directly, behind that UI. A **conductor** is the
network runtime, and a **cell** is one person's own instance inside it — the home of their
record chain, the thing that makes their writes theirs.

A **hosted human** is a person whose doorway keeps their identity for them. The doorway holds
their credential, mints their session, and runs a cell on their behalf on one of the
conductors it operates. It is the first stage of agency — the progression from being hosted
toward running your own node, which the account page draws as an "agency pipeline" — and the
one most people will live in for a long time, so its stories should be the most ordinary ones
in the suite: the things anyone expects of an account, told from the person's side, on the
pages the doorway actually shows them.

This directory is the series index. A station's story lives here (numbered) when it was born
for the series; it lives elsewhere (listed by path) when it was born earlier and is cited by a
habit or pipeline where it is. Do not move the latter.

Series discipline, in four lines:

- One life, in order. A station may assume every earlier station holds; never a later one.
- The person is created by the story and removed by it. No fixture human, no operator hand,
  so the same story runs on any doorway — the local multi-peer mesh a developer starts on
  their own machine (the "household"), a mesh with one deployed peer mixed in, or a doorway
  on the shared deployed cluster — and leaves it as it was found.
- Judged from two sides: what the portal shows, and what the doorway answers when asked
  directly. A portal can paint a form and mint nothing.
- The stage ends at the door. Nothing here exports a key, installs an app, or confirms
  stewardship — graduation is its own series.

## Stations

Paths are relative to this directory; `..` is `features/auth/`, `../..` is `features/`.
State means: **live** — the file exists and its scenarios run in the suite (some may still be
tagged `@wip`, meaning their step definitions are not yet written, so they are skipped rather
than failed); **written, @wip** — the file exists, every scenario is `@wip`; **to author** —
no file yet.

| # | Station | Story | State |
|---|---|---|---|
| 0 | Arriving as a visitor: browse the commons, be invited in | `../visitor-boundaries.feature`, `../reach-commons.feature` | live (some @wip) |
| 1 | Creating an account at the portal: my name, my identifier, refusals that make sense | `01-creating-an-account.feature` | written, @wip |
| 2 | Signing in: the portal signs me in, refuses a wrong password, remembers me | `../../browser/doorway-portal-login.feature`, `../threshold-login-domain-scoping.feature`, `../../browser/auth-browser.feature` | live |
| 3 | Staying signed in: my session survives a reload, expires honestly, ends when I sign out | `02-staying-signed-in.feature` | written, @wip |
| 4 | Reaching the app: the doorway tells the app where I sign in, and hands me over | `06-reaching-the-app.feature` (the crossing, from the person's side), `../auth-discovery.feature`, `../oauth-authorization-code.feature`, `../session-handoff.feature`, `../agency-context-labels.feature` | partly live, `06` written, @wip |
| 5 | My account: who I am here, what I use, and where I stand on the agency pipeline | `../agency-pipeline-coherence.feature` (Hosted is current), `03-my-account.feature` | partly live, rest written, @wip |
| 6 | Being hosted: the doorway keeps my cell for me, and gets it back to me when its pool changes | `../conductor-pool-recovery.feature` | live (@wip) |
| 7 | The operator and me: suspension, quota, and what I am told when they act | `../user-management.feature` (operator side), `04-the-operator-and-me.feature` (my side) | partly live, rest written, @wip |
| 8 | Leaving: I close my account from the portal and the doorway stops hosting me | `05-leaving.feature` | live (steps wired; red until the close-account route and UI land) |

## What is deliberately not here

- Anything past the hosted stage: key export, installing the app, confirming stewardship, the
  steward portal hand-off. Those are the graduation series (`../steward-login-portal-handoff.feature`,
  `../stewarded-device-sync.feature`, `../recovery/`).
- Recovery of a lost key. A hosted human's "I forgot my password" is a doorway credential reset
  and belongs at station 3; key recovery is a steward's concern.
- The other sign-in portal. There are exactly two in the protocol, and only one of them is a
  hosted human's: the doorway's own portal, which every station here is told against. The
  portal for a person whose own runtime holds their key has its own story at
  `../../peer-oauth-portal/peer-conductor-login.feature`, and it belongs to the graduation
  series — station 4 hands a hosted human to their doorway and stops there.

## Running

The suite runs in acts, each against a different kind of environment. Act I is the household:
the run owns its mesh outright, so its stories are allowed to restart peers and delete data.
Act II is the neighbourhood: the shared deployed cluster, which no story may rewrite. These
stories are tagged `@act:i` (full detail in `genesis/a2o/LAYERS.md`).

- **Household mesh (the authority).** Start the local mesh (`just mesh start`, then `just mesh
  prologue`) and run `just test mesh features/auth/hosted-human`.
- **A deployed doorway.** Act-I stories are skipped by default on shared infrastructure, because
  that act is allowed to restart peers and delete data. These stories never do that: they only
  create and remove their own human. So they are safe to force, with the destructive stories
  explicitly kept off:

  ```
  E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml \
  A2O_ALLOW_DESTRUCTIVE=0 \
  E2E_DEVICE_MODE=playwright npx cucumber-js --tags '@hosted-human and not @wip'
  ```

  The override tells the scope gate to treat this run as owning its environment (so act-I
  stories execute); `A2O_ALLOW_DESTRUCTIVE=0` keeps the peer-killing scenarios off regardless;
  `E2E_DEVICE_MODE=playwright` drives a real browser (these stories are `@browser-only`, so
  without it every browser step reports pending rather than running).

## Step phrases this series still needs

Stations 1, 3, 4, 5 and 7 are written and every scenario in them is `@wip`, because the phrases
below have no step definition yet. They are listed so the glue can be written in one pass
rather than discovered one red at a time. New glue belongs in `steps/ui/hosted-human.steps.ts`;
reuse the portal primitives in `steps/ui/doorway-portal-login.steps.ts` (the browser, the
sign-in form, the two `the doorway confirms …` session checks) before defining anything new,
and reuse the phrases `05-leaving.feature` already introduced (registering through the portal,
`the human opens their doorway account page`, the two agency-pipeline marks) rather than
minting a synonym.

`"X"` marks a parameter. A phrase repeated under two stations is one definition, listed twice
because both stories depend on it.

**Station 1 — `01-creating-an-account.feature`**

- the newcomer types a username of their own into the registration form
- the newcomer types their whole email address into the username field
- the newcomer tries to register with the username that human already holds
- the newcomer tries to register with a password shorter than the doorway allows
- the newcomer tries to register with a confirmation that does not match the password
- the newcomer finishes creating the account with a password the doorway accepts
- the newcomer follows the link for people who already have an account
- an application sent the newcomer to this doorway to sign in
- the registration form shows the doorway's domain beside the username field
- the registration form keeps only the part before the "X" as the username
- the registration form shows the account name it will create at this doorway
- the registration form does not offer to create an account at the domain they typed
- the portal shows a registration error naming the username as already taken
- the portal shows a registration error naming the password as too short
- the portal refuses to submit the registration
- the portal names the application that asked for the account
- the sign-in form already holds the username they had typed
- the identifier the doorway issued joins that username to the doorway's own domain
- the identifier the doorway issued does not carry the domain they typed
- the doorway holds exactly one account for that identifier
- the doorway holds no account for the username they typed
- that account still belongs to the human who registered first
- the browser is handed back to the application that asked
- the application is handed back the same request it sent

**Station 3 — `02-staying-signed-in.feature`**

- the browser reloads the page
- a second browser opens the doorway sign-in portal
- the human signs in through the portal on the second browser
- the human opens their doorway account page on the first browser
- the human signs out from the doorway toolbar
- the session the doorway minted for the human reaches the end of its life
- the doorway confirms a session for that human on the first browser
- the doorway confirms a session for that human on the second browser
- the doorway refuses the session it minted before the human signed out
- the doorway refuses the session with the reason "X"
- the doorway's operator suspends that human
- the portal tells the human their session ended rather than that something went wrong
- the account page is not shown

**Station 4 — `06-reaching-the-app.feature`**

The application's side of the crossing. Nine of this story's phrases already have definitions
and are deliberately reused rather than re-minted: `a hosted human "X" is registered on doorway
"alpha"`, `the portal renders its sign-in form`, `the portal renders its registration form`,
`the human signs in through the portal`, `the human submits a wrong password through the
portal`, `the portal shows a sign-in error`, and the two `the doorway confirms …` session
checks. `the browser is handed back to the application that asked` is shared with station 1 and
is listed under both. New glue for the rest belongs in `steps/ui/hosted-human.steps.ts`.

- a browser is open on the application served by doorway "X"
- the browser opens the application's sign-in
- the browser opens the application's sign-in on the way to "X"
- the human resolves their identifier at the application
- the human resolves an identifier at a host no doorway has claimed
- the human asks the application for an account instead
- the application asks which doorway signs this human in
- the application never asks for a password
- the application refuses to resolve that identifier
- the application opens "X"
- the trust chrome reads "Hosted via" and names doorway "X"
- the trust chrome shows the flywheel hint
- the trust chrome still names doorway "X"
- the trust chrome names the human rather than only the doorway
- the browser is at the sign-in portal of doorway "X"
- the browser is at the registration portal of doorway "X"
- the browser is still at the application's sign-in
- the browser is handed back to the application that asked
- the browser was not handed back to the application
- the browser was never sent to the host named in that identifier
- the portal carries the request the application sent
- the portal offers the human a way to recover their account
- the human's password was typed only on the doorway's own origin

**Station 5 — `03-my-account.feature`**

- the account page shows the display name the human registered with
- the account page shows storage used and the storage the human is allowed
- the account page shows daily queries used and the daily queries the human is allowed
- the account page shows daily bandwidth used and the daily bandwidth the human is allowed
- the account page reads as nothing used for each thing the doorway spends
- each allowance is still shown
- every allowance the account page shows is the one the doorway answers with
- the identifier the account page shows is the one the doorway answers with
- the doorway answers that nothing has been used on that account
- the agency pipeline draws every rung from "X" to standing as a steward
- the human follows the link to their full profile in the application
- the application opens on that human's own profile
- the application does not ask the human to sign in again

**Station 7 — `04-the-operator-and-me.feature`**

- the doorway's operator suspends that human
- the doorway's operator has suspended that human
- the doorway's operator lifts the suspension on that human
- the doorway's operator lowers that human's storage ceiling
- the doorway's operator lowers that human's storage ceiling below what the human uses
- the human has stored content on the doorway
- the browser reloads the page
- the doorway refuses the session with the reason "X"
- the portal tells the human their account is suspended
- the portal does not tell the human their password was wrong
- the account page shows the lowered storage ceiling
- the account page shows the human as over their storage ceiling
- the storage the account page shows as used is unchanged
- every allowance the account page shows is the one the doorway answers with

The four operator phrases are the only place in this series where a hand other than the
person's own touches the story. They act through the doorway's administrative surface with
whatever operator credential the deployment under test provides; the human is still created
and removed by the story itself, and the operator's own view of that act belongs to
`../user-management.feature`.
