---
epr-habit-version: 1
id: reach-enforced-everywhere
invariant: >
  Receiver-side pre-authorization at EVERY egress plane (DHT, CRDT sync,
  shard, HTTP, doorway): a credential the substrate cannot verify grants
  nothing beyond no credential at all. Scoped tiers flow only to authorized
  receivers — enforcement, not exclusion. Authorization is unconditional at
  every posture; a declared dev stage may cheapen the DEPTH at which identity
  and relationships are verified, never WHETHER the decision is made, and
  never in the open direction.
status: red
active: false
checks:
  - "a2o @concern:reach-enforced-http (genesis/a2o/features/dataplane/reach-enforced-http.feature — runs in the edge Dataplane Validation stage; 3 scenarios asserting an unverifiable bearer token, a self-asserted X-Agent-Cid, and a self-asserted X-Agent-Id each return exactly the anonymous listing). Names NO reach tier: it asserts a RELATION, so it holds under any vocabulary and does not wait on the drift. A 4th scenario (2026-08-25) probes the BYTE route — a restricted row's bytes refused on GET /blob/{hash} to an anonymous caller — and is @wip (A2O_RUN_WIP=1 locally) until a fleet build carries the storage-side blob reach gate."
first_move: >
  HTTP egress is wired and cured (2026-08-20). Wire the remaining four planes
  the same way — one check per plane asserting a scoped-tier row does NOT
  reach an unauthorized peer/client. Next: the CRDT sync plane (today
  broadcast-only fail-closed = exclusion, not enforcement —
  reach_is_distribution_safe in sync/projector.rs), then shard/blob, where
  the responder does no reach check at all BY DESIGN (custody is deliberately
  reach-blind; the boundary is serving, so the check belongs on the serve
  path, not on custody).
notes:
  - "The invariant's original first clause ('One reach vocabulary') was moved out of the checkable statement 2026-08-20 and is tracked in refs below. It is a RECONCILIATION goal, not an enforcement property, and elohim-storage/CLAUDE.md forbids canonizing a single vocabulary while the drift is open — so a check honoring it could not be written without violating a standing rule. That clause is why this habit sat unwired. The wired check asserts a relation instead and names no tier."
refs:
  - "genesis/data/timeline/backlog/http-reach-enforcement-gap.md"
  - "memory: project_reach_enum_drift_reconciliation (3 vocabularies — unify, but not as a precondition for enforcement)"
evidence:
  - "2026-08-25b: the storage blob byte-route gate and the doorway loopback seed gate are DEPLOYED to alpha (edge #1380 SUCCESS, 286089679). First observable effect on the fleet: genesis #1503 Upload Blob-Backed Content 403 — CI had been uploading seed blobs unauthenticated through the dev_mode hole every build (a latent Jenkinsfile readFile-vs-container bug hid the missing bearer behind a 'credential not visible' echo); fixed 47fb60f58, genesis #1504 uploads HTTP 200 with the Admin bearer — the gate now refuses anonymous remote writes and admits the authenticated CI actor. reach-enforced-http.feature is @act:i (HELD on the fleet by LAYERS design); its authority is the household lane: 3/3 + byte-route 1/1 on the deployed commit's binary. Stays RED on the invariant, not the check: /head-record · /epr-head · /apps/{cid} · direct /shard·/ipfs still serve community anon (plane conflation) and the DEV_MODE JWT forgery still passes as Admin on every deployed doorway — both filed, operator-owned."
  - "2026-08-25: byte-route egress closed LOCALLY, stays RED (a flip needs a build number). GET /blob/{hash} now consults the referencing content rows (`blob_reach`: a blob serves to a caller iff some referencing row does; both address forms — `sha256-<hex>` and `bafkrei…` — resolve to one verdict after a red-team pass found the sha256 form walked straight through the first cut), the legacy x-agent-id fallback yields to the doorway-injected X-Agent-Cid, and the doorway pantry refuses to stock a credentialed answer under its bearer-blind hash key (+ nosniff on pantry hits). Measured on the household mesh from the built binary: reach-enforced-http 3/3 and the new byte-route scenario 1/1 (community bytes 403 anon, 200 with a resolved identity, public 200); the doorway seed/admin-cache authority now requires dev_mode AND a loopback socket peer (every deployed manifest sets DEV_MODE=true — an anonymous remote PUT /admin/seed/blob had reached the hash check). Residuals FILED, not closed: DEV_MODE JWT forgery passes as Admin and defeats every fleet reach gate (backlog security-doorway-devmode-auth-bypass, coupled to the operator posture decision); /head-record, /epr-head, /apps/{cid} and direct-to-storage /shard·/ipfs still serve community anon because reach_is_distribution_safe treats community as broadcast-safe — a reach-vs-replication plane conflation that is a vocabulary call, not canonized here."
  - "2026-08-20: flipped unwired -> RED on measured violation, which is stronger evidence than an absent check. Live on doorway-alpha, verified twice: anonymous GET /db/content?limit=1000&offset=1000 returned 90 rows (all commons); the identical request carrying the literal header 'Authorization: Bearer bogus' returned 1000 — familiar 906, commons 90, private 3, intimate 1 — every row with its contentBody. Cause: http.rs:5272 decided reach from header PRESENCE, never validation."
  - "2026-08-20: HTTP-plane cure committed (41f7780aa) — the listing now resolves the requester and authorizes per row via epr_service::authorize_reach_for_human_with_own_trust, deny-by-default when unresolvable; the same header-presence check on the single-item route's Layer 1 is cured too; and reach_level_index's `_ => 0` fail-open (an UNRECOGNIZED tier read as the most permissive, making the authorizer's own `_ => Err` unreachable) now fails closed to u8::MAX. Stays RED until the scenario runs green against a deploy carrying the fix — a flip needs a build number, not a commit."
  - "2026-08-20 scope, held honestly: the doorway strips client-supplied identity headers (probed: X-Agent-Cid/X-Agent-Id return the anonymous result), so the cure closes the bypass for doorway-fronted traffic. It does NOT make identity unforgeable — extract_agent_cid returns the header verbatim — so a caller reaching elohim-storage directly in-cluster can still assert an identity and receive that identity's authorized reach. That residual is identity-cross-signed's, not this habit's."
retire-when: >
  when every egress plane derives authorization from one shared verifier it cannot bypass
  by construction — so a NEW plane is enforced the moment it exists, rather than after
  someone remembers to gate it. The per-plane gate is what this habit is really watching.
---
