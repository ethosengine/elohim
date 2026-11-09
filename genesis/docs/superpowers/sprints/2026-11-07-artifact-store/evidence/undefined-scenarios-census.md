---
id: artifact-store-undefined-scenarios-census
status: active
habits: [pain-is-answered, operator-dashboard-under-standing, custodial-authority-answerable]
---

# Census: the 37 undefined + 5 pending genesis scenarios (#1758), 2026-11-09

Read-only pass by Codex gpt-6-astra (high), commissioned and reviewed by ethosengine (Claude). Paths are
relative to dev c4c09bab4. It is a source reading, not a measured effort estimate.

**None of the 37 complete scenarios is established as glue-only.** All four features have existing building blocks, but each still requires product behavior beyond step definitions.

The CI issue is also more specific than missing tags: **35 Act I scenarios were held by the substrate hook and still became undefined.** The two Act II doorway scenarios were not held. [Build #1758](https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1758/consoleFull) shows:

- Lines 4992, 5033, 5160 and 5266: these features’ Act I scenarios explicitly logged as `HELD`.
- Line 5275: `443 scenario(s) held by @requires gate`.
- Line 8784: `498 scenarios (37 undefined, 5 pending, 429 skipped, 27 passed)`.

The reason is Cucumber’s evaluation order: [test_case_runner.js:252](node_modules/.pnpm/@cucumber+cucumber@11.3.0/node_modules/@cucumber/cucumber/lib/runtime/test_case_runner.js:252) returns `UNDEFINED` for a missing definition **before** checking whether steps are being skipped at line 265. The repository’s hold counter records the hook’s decision, not the final Cucumber classification. Another `@requires` tag would not solve this.

I inspected 374 TypeScript files under `genesis/a2o`, matched static definitions using Cucumber expressions/regular expressions, expanded the scenario outline, and checked the remaining dynamic registrations. Counts below are distinct expanded step texts within each file. No scenarios, product mutations, or repository writes were run.

The applicable convention is explicit:

> “Add `@wip` to scenarios with unimplemented step definitions.”

[genesis/a2o/CLAUDE.md:24](genesis/a2o/CLAUDE.md:24). Its `@requires` convention concerns unavailable substrate dependencies; fixture tags such as `@requires:doorway` are not substrate gates. [CLAUDE.md:26](genesis/a2o/CLAUDE.md:26). The local [.epr-meta:109](genesis/a2o/.epr-meta:109) delegates feature conventions to that document. Three of these features deliberately override the `@wip` convention, as detailed below.

**1. `operator-answers-pain.feature` — 8 scenarios**

Source: [operator-answers-pain.feature](genesis/a2o/features/dataplane/operator-answers-pain.feature:1).

**Verdict: UNBUILT as a complete feature; no complete scenario established as glue-only.** This does not mean custody, measurements and head elections are absent. Their required orchestration is absent, and the shared Background requires an enrolled runtime band and an operator-seat commitment that I could not locate as an implemented path.

Existing surfaces relevant to the table:

- **P1 — capacity measurement:** [`capacity_reporter::run_report_once`:302](elohim/elohim-storage/src/services/capacity_reporter.rs:302) measures the **blob path’s** directory size, filesystem capacity and available space. It does not implement enrolled runtime-band limits, Approach state links or the relief loop.
- **P2 — custody salvage:** [`reconcile::custody::salvage_pass`:485](elohim/elohim-storage/src/reconcile/custody.rs:485) authors custody commitments for under-replicated blobs; [`reconcile_pass`:183](elohim/elohim-storage/src/reconcile/custody.rs:183) fetches them. This is not the specified Approach → voted proposal → Breach activation sequence.
- **P3 — head answers:** [`view_federation.rs:582`](elohim/elohim-storage/src/p2p/view_federation.rs:582) explicitly answers with the head hash alone on conductor error, recording `ConductorError`; it does not implement the feature’s blanket `unverifiable` answer.
- **P4 — head election:** [`select_canonical_winner`:3190](elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:3190) gives earned declarations precedence. However, [`select_staging_candidate`:3251](elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:3251) exposes only staging declarations **newer than** the earned winner; it does not establish continued visibility beneath a later owner declaration.

| Scenario, feature line | Product surface and present state | Missing behavior |
|---|---|---|
| Approach raises care and moves nothing, 140 | P1 measurements; P2 custody primitives exist | Runtime-band threshold/state link, inactive neighbor intents, relief proposal and all three votes within one tick |
| Approach declines new carrying by name and keeps serving what it holds, 156 | P1/P2 and existing storage serving | Approach-conditioned custody admission and counted `capacity` refusal |
| Breach executes the plan agreed at Approach, 168 | P2 salvage/fetch exists | Previously consented proposal activation, seat-authored breach judgment, cited allow decision and steward notification |
| Half-alive peer says it cannot verify, 184 | P3 head-response path exists | Required response semantics differ from current code; blanket no-adoption behavior is not established |
| Peer without Approach gets a Refer, 196 | Existing storage/custody machinery | Seat’s evidence-bearing `intimate-quorum` referral and activation gate |
| Unanswered Refer becomes counted pending, 209 | No matching operator-loop implementation located | Referral windows, escalation, pending decision and counter |
| Carried work stays beneath owner’s earned head, 222 | P4 earned/staging election exists | Collective-grant carrying path not established; current candidate selection drops staging older than the new earned winner |
| Return releases relief safely, 238 | P2 custody reconciliation exists | Return-triggered release only after two verified other holders, coordinated revocation and holds-gauge update |

The owning habit also identifies the missing loop rather than claiming an implementation: [pain-is-answered.habit.md:40](elohim/elohim-storage/.epr-meta/pain-is-answered.habit.md:40).

Tags and lane:

- Feature: `@e2e @dataplane @concern:pain-is-answered @requires:multi-node @act:i`.
- Scenarios 3–8 additionally have `@requires:owned-substrate`.
- Intended lane: owned household mesh. Genesis CI nevertheless selects it through its broad API filter, [`e2e-verify-api.sh:108`](genesis/scripts/ci/e2e-verify-api.sh:108).
- Act I’s unavailable `owned-substrate` baseline held **all eight** in #1758. Undefined definitions still won under Cucumber’s ordering.
- **Tag disposition:** not an accidentally missing `@requires`. The [header:17](genesis/a2o/features/dataplane/operator-answers-pain.feature:17) deliberately omits `@wip`: “Wire the steps or argue with the assertion; do not silence it with a tag.” It allows a later `@wip` decision only with the habit returned to `unwired` in the same commit.

**Distinct undefined steps: 55. Existing matching definitions: none.** Numbers below are the first source occurrence; Given/When/Then/And keywords are omitted.

```text
131 the household mesh has peers "matthew", "jessica" and "james"
132 each peer has an enrolled runtime band with a declared pledge commitment
133 the seat holds a commitment naming which moves it may allow alone and which it must refer
134 jessica holds bytes that no other peer holds at all
141 jessica's runtime band reaches 85 percent of its limit
142 within one tick of that moment jessica writes an "approaching" state link on her own pledge commitment
143 that state link is visible on matthew
144 within one tick of that moment, each neighbour that has seen the "approaching" state link authors, on its own initiative, a custody intent for jessica's held bytes, and at least two do
145 every one of those custody intents is inactive
146 no custody intent was authored by jessica or on her behalf
147 zero bytes have moved off jessica
148 within one tick of that moment the seat mints a relief proposal naming those neighbours
149 jessica's own vote is recorded on the relief proposal
150 matthew's and james's own votes are recorded on the relief proposal
157 jessica's pledge commitment carries an "approaching" state link
158 jessica serves a page she already holds
159 matthew asks jessica to take on custody of new bytes
160 jessica declines the request with the reason "capacity"
161 the decline is counted under the reason "capacity"
162 jessica still serves the page she already held
170 a relief proposal naming matthew and james carries the votes of jessica, matthew and james
171 matthew and james each hold an inactive custody intent for jessica's held bytes
172 jessica's conductor is stopped
173 the seat records a "breached" state link on jessica's pledge commitment as its own judgment
174 after that record matthew and james each activate their custody intent
175 jessica's held bytes arrive on matthew and on james, fetched from the shared network and verified by those neighbours, not taken from jessica's own storage
176 the seat records a gate-decision of "allow" that cites the relief proposal minted at Approach
177 no new consent is requested from anyone
178 the seat informs jessica's device steward of what the neighbours did
185 jessica's storage still answers requests
186 jessica holds a page version that matthew does not hold
188 matthew runs a sweep of the page versions jessica holds
189 matthew receives "unverifiable" for the versions jessica holds
190 matthew adopts no page version from jessica
197 jessica's pledge commitment carries no "approaching" state link
200 the seat records a Refer naming the layer "intimate-quorum"
201 that Refer carries the evidence the seat holds: that jessica's conductor has stopped, that she declared no Approach, and which bytes only she holds
202 no custody activates on any neighbour until that layer answers
210 the seat has recorded a Refer about jessica at the widest layer
211 the Refer's window passes with no answer from that layer
212 the seat records a gate-decision of "pending"
213 the pending outcome is counted
214 the Refer is not turned into a gate-decision of "block"
223 jessica is the root author of a page
224 matthew is a steward member of the household collective under a grant for that page
226 matthew declares a successor head for that page
227 that head is visible as a staging candidate on every peer
228 the head every peer serves as earned is still jessica's
229 jessica's conductor is brought back and she declares a head for that page
230 the earned head on every peer is the one jessica declared
231 matthew's carried head remains visible beneath it on every peer
239 matthew and james hold jessica's bytes under activated custody commitments
240 jessica's conductor is brought back
241 jessica's own custody rows for those bytes are revoked only once at least two other peers hold them
242 jessica's holds gauge shows the released bytes let go
```

**2. `runtime-band-external-imposition.feature` — 6 scenarios**

Source: [runtime-band-external-imposition.feature](genesis/a2o/features/dataplane/runtime-band-external-imposition.feature:1).

**Verdict: UNBUILT as written; no complete scenario established as glue-only.** The regression anchor exercises real failure mechanisms, but its common Background also requires the missing runtime-band enrollment and measurement loop. Reproducing the exact SQLite error under the current fixture is **unknown**, not demonstrated by this census.

| Scenario, feature line | Product surface and present state | Missing behavior |
|---|---|---|
| Outside process exhausts runtime band, 122 | Conductor, doorway app-listing and adoption reader exist; [`HeadReader`:18](elohim/elohim-storage/src/services/release_adoption/watch/head_reader.rs:18) reads through the local conductor | Runtime-band limit/headroom contract and safe quota-exhaustion fixture absent; exact error and five-sweep observation unknown |
| Diagnose outside consumer by remainder, 139 | [`capacity_reporter`:302](elohim/elohim-storage/src/services/capacity_reporter.rs:302) measures blob-directory usage and filesystem free space | Conductor-volume accounting, declared limit, remainder history, three-tick threshold, Approach publication, diagnosis and hardware-steward Refer |
| Unaffected peers hold load before failure, 153 | [`salvage_pass`:485](elohim/elohim-storage/src/reconcile/custody.rs:485) and custody fetch exist | Proactive trigger from runtime-band state and required two-peer completion before zero free |
| Half-alive storage answers unverifiable, 161 | [`view_federation.rs:582`](elohim/elohim-storage/src/p2p/view_federation.rs:582) distinguishes conductor errors but still answers with a head hash | Blanket `unverifiable`, no adoption, and specified peer-state diagnosis |
| Status page shows diagnosis, 168 | Existing doorway dashboard is routed at [`app.routes.ts:40`](doorway/doorway-app/src/app/app.routes.ts:40) | No identified page/backend contract carrying runtime-band limit, named/free/remainder measurements and linked app-listing failure |
| Clearing imposition closes loop, 178 | Conductor restart and custody primitives exist | Recovered enrollment state, resolved gate decision, closed Refer notification and consent-controlled relief retention |

Tags and lane:

- Feature: `@e2e @dataplane @concern:pain-is-answered @requires:multi-node @act:i`.
- All scenarios except the status-page scenario additionally require `owned-substrate`; the regression also has `@regression`.
- Intended lane: owned household mesh. Genesis API CI selects it, and the Act I baseline held all six in #1758; missing definitions remained undefined.
- **Tag disposition:** [header:23](genesis/a2o/features/dataplane/runtime-band-external-imposition.feature:23) explicitly adopts the sibling feature’s deliberate no-`@wip` exception. Holding this work would be a changed measurement policy, not correction of an omitted dependency tag.

**Existing definition — not undefined:**

`the household mesh is running with peers "matthew", "jessica" and "james"` matches [conductor-spin.steps.ts:242](genesis/a2o/steps/conductor-spin.steps.ts:242). It checks process-control availability and each peer’s `/health`.

**Distinct undefined steps: 39.**

```text
117 each peer's runtime band has a declared limit its enrollment promised
118 each peer measures its named bytes, its available bytes and its unaccounted remainder every tick
119 "matthew" is the household's seat
127 "jessica" is the only peer holding some of the bytes the household has promised to keep
128 a process outside the protocol begins writing into the volume under "jessica"'s runtime band
129 that process has consumed everything but less than the headroom her conductor's database needs to open
130 "jessica"'s conductor fails to open its database with the error "unable to open database file"
131 the doorway reading "jessica"'s conductor reports that error on its app listing
132 "jessica"'s adoption channel answers "conductor unavailable" on five consecutive sweeps
134 "jessica" is half-alive: her storage process is still running and still answers "matthew"'s sweep
143 "jessica"'s named bytes have been flat for three ticks
144 a process outside the protocol is writing into the volume under "jessica"'s runtime band
145 "jessica"'s available bytes fall for three consecutive ticks while her named bytes stay flat
146 her unaccounted remainder exceeds 5 percent of her declared limit on the third tick
147 "jessica"'s unaccounted remainder is greater than zero and grew on each of those ticks
148 "jessica"'s enrollment promise carries an "approaching" state naming the runtime band, the disk, and the remainder as unaccounted
149 "matthew" reads that state within one tick and records a gate-decision whose reasoning names an outside consumer of "jessica"'s runtime band, not "jessica"'s own growth
150 "matthew" records a Refer to the hardware steward carrying the declared limit, the named bytes, the available bytes and the remainder
154 "jessica"'s enrollment promise carries an "approaching" state naming an unaccounted remainder
155 "matthew" and "james" read that state
156 each of "matthew" and "james" makes its own promise to hold a copy of the bytes "jessica" promised to hold
157 every byte "jessica" promised is held by at least two peers other than "jessica" before her available bytes reach zero
158 no byte was taken from "jessica"'s storage process for those copies; each was fetched from another peer on the shared network and verified by the fetching peer's own conductor
162 "jessica" is half-alive: her conductor cannot open its database and her storage process is still running
163 "matthew" sweeps "jessica" for the heads she holds
164 "jessica"'s storage process answers "unverifiable" for every head
165 "matthew" adopts nothing from "jessica" in that sweep
166 "matthew"'s record of "jessica" says her conductor is unreachable, not that her bytes are absent
171 "jessica"'s conductor has failed to open its database
172 a person opens the household's status page without reading any log
173 the page names "jessica" as the peer whose conductor cannot open its database
174 the page names the runtime band and shows its declared limit, named bytes, available bytes and unaccounted remainder
175 the doorway's app-listing failure for "jessica" links to that same reading rather than showing the database error alone
179 the outside process consuming "jessica"'s runtime band has been stopped and its bytes removed
180 "jessica"'s conductor opens its database again
181 "jessica"'s enrollment promise carries a "recovered" state within one tick
182 the gate-decision "matthew" recorded is marked resolved citing that state
183 the hardware steward is informed that the Refer is closed
184 the copies "matthew" and "james" hold remain until "jessica" herself releases her own promise
```

**3. `operator-standing.feature` — 15 expanded scenarios**

Source: [operator-standing.feature](genesis/a2o/features/doorway/operator-standing.feature:1).

**Verdict: UNBUILT for the asserted standing contract, with substantial existing endpoint infrastructure. No complete scenario is glue-only.**

The feature’s claim that some hosting mutations accept unauthenticated callers is now stale: [`http.rs:8695`](doorway/doorway-service/src/server/http.rs:8695) gates those mutations. But that gate requires an **Admin JWT or admin API key**, not a per-request notarized commitment.

Relevant existing surfaces:

- **D1 — dashboard:** [`auth.guard.ts:37`](doorway/doorway-app/src/app/core/guards/auth.guard.ts:37) admits any authenticated account.
- **D2 — actuator authorization:** [`http_permission.rs:122`](doorway/doorway-service/src/auth/http_permission.rs:122) enumerates hosting/graduation/federation mutations; [line 150](doorway/doorway-service/src/auth/http_permission.rs:150) checks Admin permission. Refusals contain generic authentication/Admin messages, not the required capability.
- **D3 — operator resolver:** [`resolve_operator_capability`:136](doorway/doorway-service/src/auth/operator.rs:136) exists but has no production caller found. It reads a JWT snapshot, not the current peer record; [line 68](doorway/doorway-service/src/auth/operator.rs:68) explicitly documents missing cross-doorway equality enforcement.
- **D4 — binding storage:** [`find_active_operator_binding`:1766](elohim/elohim-storage/src/db/rea_commitments.rs:1766) exists. The ordinary `operate-doorway` write falls through to Diesel in [`rea_commitment_service.rs:543`](elohim/elohim-storage/src/services/rea_commitment_service.rs:543); the soft conductor action list contains only `custody-blob` at [`rea_commitments.rs:1562`](elohim/elohim-storage/src/db/rea_commitments.rs:1562).

| Expanded scenario | Existing product surface | Why it is not glue-only |
|---|---|---|
| Holder sees standing, 131 | D1, D3, D4 | No network-read commitment/capability/time presentation |
| Signed-in nonholder refused, 138 | D1 | Authentication alone opens dashboard |
| Anonymous: begin hosting, example 156 | `POST /admin/hosted-users`, [`http.rs:9085`](doorway/doorway-service/src/server/http.rs:9085); D2 | Basic refusal exists; capability-named refusal and commitment-based setup do not |
| Anonymous: stop hosting, example 157 | `DELETE /admin/hosted-users/{key}`, [`http.rs:9110`](doorway/doorway-service/src/server/http.rs:9110); D2 | Same gap |
| Anonymous: record graduation, example 158 | `POST /admin/graduation/force/{key}`, [`http.rs:9135`](doorway/doorway-service/src/server/http.rs:9135); D2 | Same gap |
| Anonymous: add federation peer, example 159 | `POST /admin/federation/peers`, [`http.rs:9150`](doorway/doorway-service/src/server/http.rs:9150); D2 | Same gap |
| James: begin hosting, example 160 | Hosting POST above; D2 | Gate tests permission rank, not absence of doorway commitment; no capability-named refusal |
| James: stop hosting, example 161 | Hosting DELETE above; D2 | Same gap |
| James: record graduation, example 162 | Graduation POST above; D2 | Same gap |
| James: add federation peer, example 163 | Federation POST above; D2 | Same gap |
| Accepted action names commitment, 165 | [`handle_admin_refresh_federation_peers`:591](doorway/doorway-service/src/routes/federation.rs:591) | Handler lacks resolved commitment context and required receipt |
| Revocation closes next request, 175 | D1/D3/D4 | No per-request commitment re-read; JWT snapshot cannot prove this behavior |
| Stuck account shown with owner-only remedy, 183 | Hosted-user routes; provisioning diagnostics at [`admin_conductors.rs:882`](doorway/doorway-service/src/routes/admin_conductors.rs:882) | Diagnostics are not the specified account-key/app join and owner-addressed UI; full behavior not located |
| Holding B gives no standing on A, 201 | D3/D4 | Doorway-scoped live standing enforcement not wired |
| Deputy has only named capabilities, 208 | Permission mutation at [`admin_users.rs:1439`](doorway/doorway-service/src/routes/admin_users.rs:1439); D3 | Per-capability commitment gate and matching UI restrictions absent |

Tags and lane:

- Feature: `@e2e @doorway @concern:operator-dashboard-under-standing @requires:doorway`.
- First Rule: `@act:i`, yielding 13 household scenarios.
- Second Rule: `@act:ii`, yielding two alpha-fleet scenarios.
- The household command is explicitly named at [feature:119](genesis/a2o/features/doorway/operator-standing.feature:119). Genesis API CI selects both Rules.
- `doorway` is a fixture tag, not a declared substrate gate. Act I held the first 13 in #1758; the two Act II scenarios’ substrate hooks passed. All 15 subsequently had undefined steps.
- **Tag disposition:** unlike the other three files, this header contains no explicit no-`@wip` exception. `@wip` fits the general convention for this unimplemented contract. It would honestly classify deferred work, not make the product behavior green. The owning [habit](doorway/doorway-service/.epr-meta/operator-dashboard-under-standing.habit.md:1) must continue to represent that missing coverage.

**Existing definition — not undefined:**

`the request is refused` matches [operator-commitment-gated-verbs.steps.ts:418](genesis/a2o/steps/dataplane/operator-commitment-gated-verbs.steps.ts:418). **It is not directly reusable:** it asserts `state(this).lastReconcile`, populated by another feature’s reconcile steps. Adding another identical definition would introduce ambiguity.

**Distinct undefined steps: 54 after outline expansion**—45 undefined source templates, with the caller/action and capability substitutions expanded below.

```text
127 doorway "A" on the household mesh
128 "Matthew" holds an active commitment to operate doorway "A"
129 "James" has an account on doorway "A" and holds no commitment to operate it
132 "Matthew" signs in to doorway "A" and opens its operator surface
133 the surface opens
134 it names doorway "A" as the doorway the commitment covers
135 it names the commitment it opened under and the capabilities that commitment allows
136 it says the commitment was read from the network, with the time it was read
139 "James" signs in to doorway "A" and opens its operator surface
140 the surface is refused
141 the refusal says "James" holds no commitment to operate doorway "A"
142 no account, conductor or hosting state of doorway "A" is shown to "James"
149 a caller who isn't signed in asks doorway "A" to begin hosting a new person
149 a caller who isn't signed in asks doorway "A" to stop hosting a hosted person
149 a caller who isn't signed in asks doorway "A" to record a hosted person as graduated
149 a caller who isn't signed in asks doorway "A" to add a doorway to the doorways it federates with
149 "James" asks doorway "A" to begin hosting a new person
149 "James" asks doorway "A" to stop hosting a hosted person
149 "James" asks doorway "A" to record a hosted person as graduated
149 "James" asks doorway "A" to add a doorway to the doorways it federates with
151 the refusal names the capability "hosted_users.lifecycle" the action needs
151 the refusal names the capability "graduation.record" the action needs
151 the refusal names the capability "federation.peers.mutate" the action needs
152 doorway "A" hosts and federates exactly as it did before the request
170 "Matthew" refreshes the doorways that doorway "A" federates with, from its operator surface
171 the refresh is accepted
172 the response names the commitment "Matthew" acted under
173 the response names the capability "federation.peers.mutate" that commitment matched
176 "Matthew" is signed in to doorway "A" with its operator surface open
177 "Matthew"'s commitment to operate doorway "A" is revoked
178 the doorway's peer reports that commitment as revoked when asked for it
179 "Matthew" reloads the operator surface without signing in again
180 the surface is refused while that sign-in is still valid
181 the refusal says the commitment it last opened under has been revoked
184 an account on doorway "A" whose key has no app on any of the doorway's conductors
185 a hosted account on doorway "A" whose app is installed and answering
186 "Matthew" opens the hosting view of doorway "A"'s operator surface
187 the first account is listed as stuck, with the key it points at
188 the hosted account whose app is answering is not listed as stuck
189 the surface offers "Matthew" no control that changes which key the account points at
190 the stuck account carries a notice that only its owner can resolve it, with no action on it for the holder
196 the alpha fleet, whose steward of record is "Matthew"
197 "Matthew" holds the primary commitment to operate doorway "alpha-A"
198 "Adam" holds the primary commitment to operate doorway "alpha-B", issued by "Matthew"
199 "Matthew" holds a deputy commitment on doorway "alpha-B" without the capability "users.permissions.mutate"
202 "Adam" opens the operator surface of doorway "alpha-B"
203 the surface opens and names the commitment "Adam" holds
204 "Adam" opens the operator surface of doorway "alpha-A"
206 the refusal says "Adam" holds no commitment to operate doorway "alpha-A"
209 "Matthew" opens the operator surface of doorway "alpha-B"
210 the surface opens and names the deputy commitment "Matthew" holds
211 it offers "Matthew" no control that changes an account's permissions
212 "Matthew" asks doorway "alpha-B" to change an account's permissions
214 the refusal names the capability "users.permissions.mutate" the action needs
```

**4. `collective-steward-answerable.feature` — 8 scenarios**

Source: [collective-steward-answerable.feature](genesis/a2o/features/stewardship/collective-steward-answerable.feature:1).

**Verdict: UNBUILT for collective authority; no complete scenario established as glue-only.** Individual grant, membership, policy and appeal primitives exist. The grant → collective → currently authorized acting person join does not exist in the inspected stewardship coordinator.

Concrete distinctions:

- [`get_my_steward_tier`:219](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:219) looks up grants against the caller’s **human ID** and returns rank-like tiers.
- Collective membership and withdrawal exist separately: [`list_memberships_for_collective_cid`:371](elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs:371), [`withdraw_membership_clean`:716](elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs:716).
- The HTTP surface is not proof of implementation: [`handle_create_grant`:375](elohim/elohim-storage/src/api/stewardship.rs:375) and [`handle_file_appeal`:423](elohim/elohim-storage/src/api/stewardship.rs:423) merely acknowledge requests and tell callers to submit to Holochain.

| Scenario, feature line | Product surface and present state | Missing behavior |
|---|---|---|
| Who may act answers with people, 143 | Grant queries and membership queries above exist | Collective-held grant resolution to sponsored, unwithdrawn acting people |
| James sees who set limit, 153 | [`upsert_policy`:764](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:764) stores `author_id` | Collective authorization and grant reference on the act; subject-facing explanation |
| Live objection gets named answer before deadline, 170 | [`file_appeal`:1167](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:1167) exists | Deadline bounded by grant review; backlink update; decision-writing coordinator and named collective responder. Current expiry is fixed at 30 days |
| Overdue objection reported unanswered, 187 | [`get_appeal_by_id`:1278](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:1278) exists | Overdue/unanswered projection, grant backlink and subject-facing status |
| Authority covers time, not reading history, 197 | Grant booleans exist in [`StewardshipGrant`:133](elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/stewardship.rs:133) | Collective member authorization and dimension-specific reading-history enforcement |
| Co-op adds quiet hour; both limits apply, 208 | [`compute_policy_for_subject`:939](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:939) exists | Time windows currently overwrite at [line 1068](elohim/holochain/dna/imagodei/zomes/imagodei/src/stewardship.rs:1068); they do not implement this union |
| Co-op cannot lift household cutoff, 218 | Policy computation/upsert exist | Grant-scoped ownership refusal and order-independent composition; current policy sorting uses tiers |
| Withdrawn Matthew cannot act, 233 | Membership withdrawal exists | Stewardship policy authorization does not consult the collective membership path; required immediate explicit refusal is absent |

This matches the directory’s stated gap: “**The substrate already models most of this. Nothing above the DNA does.**” [stewardship/README.md:29](genesis/a2o/features/stewardship/README.md:29). The current code supports the narrower conclusion above; it should not be read as saying every primitive is absent.

Tags and lane:

- Feature: `@e2e @stewardship @concern:custodial-stewardship-06-collective-stewards @act:i`.
- No explicit `@requires` tags.
- Intended lane: household mesh, explicitly named at [feature:51](genesis/a2o/features/stewardship/collective-steward-answerable.feature:51).
- The missing explicit `@requires:owned-substrate` is **not** the explanation: the Act I baseline already held all eight in #1758. Undefined definitions still prevailed.
- **Tag disposition:** [header:27](genesis/a2o/features/stewardship/collective-steward-answerable.feature:27) explicitly says “The tag is deliberately omitted” and requires returning the habit to `unwired` if a later decision adds `@wip`.

**Existing definitions — not undefined:**

- `doorway "alpha" at "E2E_DOORWAY_ALPHA"` → [mode-aware.steps.ts:96](genesis/a2o/steps/mode-aware.steps.ts:96).
- `elohim-storage is reachable at "E2E_STORAGE_URL"` → [resilience.steps.ts:184](genesis/a2o/steps/resilience.steps.ts:184).

**Distinct undefined steps: 53.**

```text
135 the collective "family-dowell" has "human-matthew-manager" and "human-jessica-spouse" as steward members
136 "human-james-son" is a member of "family-dowell" holding no steward role
144 an active stewardship grant over "human-james-son" whose steward is the collective "family-dowell"
145 that grant rests on "minor_guardianship"
146 james asks who may act under that grant
147 the answer names "human-jessica-spouse" and "human-matthew-manager"
148 each name is backed by a steward membership in "family-dowell" that is sponsored and not withdrawn
149 the answer is never the collective on its own
154 an active grant over "human-james-son" held by the collective "family-dowell" covering time limits
155 "human-jessica-spouse" sets a nightly cut-off on james's device under that grant
156 the record of that change names "human-jessica-spouse" as the person who made it
157 the record names the grant the change was made under, so anyone can check which authority it claimed
158 when james opens his own account he is told which person changed it and when
171 the nightly cut-off on "human-james-son" is in force
172 the grant it was set under is still active and carries a review date
173 james objects that the cut-off is disproportionate and gives his reasons
174 the objection is recorded against that grant while the grant is still active
175 james was not blocked from filing the objection
176 the objection carries an answer deadline no later than that grant's review date
177 the grant carries a reference to the open objection for as long as it is open
178 "human-jessica-spouse" files a decision on that objection together with the grounds for it
179 the decision is recorded against the objection before its answer deadline
180 the objection is no longer open
181 james is shown the decision, the grounds for it, and "human-jessica-spouse" as the person who answered
182 james could read every restriction on him immediately before the decision was filed
188 an open objection by "human-james-son" against an active grant held by "family-dowell"
189 its answer deadline has passed with no decision recorded
190 james asks what became of his objection
191 he is told that it is unanswered and that its deadline has passed
192 it is not reported as decided, denied, closed, or withdrawn
193 the grant it names is reported as still carrying an unanswered objection
198 an active grant over "human-james-son" held by "family-dowell" that names time limits and content filtering
199 that grant does not name activity monitoring
200 "human-matthew-manager" asks for james's reading history under that grant
201 the request is refused with a reason naming the surface the grant does not cover
202 nothing about what james has been reading is returned or recorded
203 the grant is left intact, because matthew is still james's steward for the surfaces it does name
209 an active grant over "human-james-son" held by "family-dowell" whose policy sets a nightly cut-off
210 a second active grant over "human-james-son" held by the collective "homeschool-coop" resting on "organizational_role"
211 the co-op adds a quiet hour during lesson time under its own grant
212 both the nightly cut-off and the quiet hour are active on james
213 the household's grant and its cut-off are unchanged
221 the co-op attempts to lift the household's nightly cut-off
222 the attempt is refused with a reason naming the layer that set the cut-off
223 the nightly cut-off is still active on james
224 neither grant is recorded as the primary one
225 the restrictions james lives under are the same set whichever of the two grants is read first
234 an active grant over "human-james-son" held by the collective "family-dowell"
235 the steward membership of "human-matthew-manager" in "family-dowell" has been withdrawn
236 "human-matthew-manager" sets a restriction on james under the household's grant
237 the act is refused with a reason naming the withdrawn steward membership
238 nothing is recorded against james
239 the withdrawal counts from the moment it was made, not from some later sweep
```

**Ranked recommendation**

This is a source-based implementation priority, not a measured effort estimate; the actual cheapest full completion remains unknown.

1. **`operator-standing` — strongest first candidate.** The concrete mutations, dashboard, binding query and capability matcher exist. The work is identifiable: notarized bindings, a shared per-request scoped capability check, commitment-bearing responses, and the corresponding UI. Start with the eight actuator examples, but preserve their capability-name assertions.
2. **`collective-steward-answerable`.** Grant and membership primitives are colocated in imagodei. The missing join is clear, but honest completion also requires policy-composition corrections, appeal decisions and subject-visible attribution.
3. **`operator-answers-pain`.** Requires a new operational loop spanning capacity enrollment, sensing, proposals, consent, referrals, custody and recovery. Existing salvage and elections help, but do not supply that loop.
4. **`runtime-band-external-imposition`.** Depends on much of the same missing loop and adds conductor-volume accounting, trend diagnosis, status presentation and a controlled exhaustion fixture.

For CI classification, use the repository’s actual `@wip` exclusion if the decision is to defer these contracts: both the [genesis API filter](genesis/scripts/ci/e2e-verify-api.sh:108) and [mesh filter](genesis/a2o/mesh-profile.mjs:6) exclude it **before execution**. The runtime `Before` skip alone cannot reliably hide undefined steps. For the three deliberate-red features, that change must explicitly reconcile their documented exception and habit state. It would make the CI scope honest; it would not turn any of these product contracts green.