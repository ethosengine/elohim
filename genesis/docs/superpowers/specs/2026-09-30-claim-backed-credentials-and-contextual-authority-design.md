---
id: claim-backed-credentials-and-contextual-authority-design
title: "Claim-backed credentials and contextual authority — the Che publishing contract"
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: Matthew Dowell
stewardship-frame: bounded
created: 2026-09-30
habits: [dataplane-convergence]
graduation-trigger: >
  The existing commons-path-steward-publish story proves content scope at the native
  authority boundary, independent receiver verification, historical acceptance after
  expiry, and device-specific revocation without replacing the human; then graduate
  the implemented contract and retain unbuilt social-recovery work as explicit gaps.
informed-by:
  - genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/docs/architecture/stewardship-over-sovereignty.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
cites:
  - "trust-compute-gradient-brainstorm | The substrate-thin, manifest-medium, agent-thick model this credential contract applies to publishing authority | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md"
  - "wisdom-layer-floor-ceiling-judgment-culminating-design | The deterministic floor, contextual judgment, and accountable human recognition this contract preserves | sha256:f5d694c382a76c1f | path: genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md"
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - "stewardship-over-sovereignty | Community-grounded identity and recovery prevent credential withdrawal from becoming loss of personhood | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "substrate-trust-contract-runbook | Receiver-local validation and canonical head invariants remain mandatory beneath contextual trust | sha256:eb5f8342e17c361f | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
---

# Claim-backed credentials and contextual authority

## 1. Purpose and standing

Trust follows the compute curve. Participation starts with a small deterministic
floor; collective policies and increasingly capable elohim add contextual judgment.
Ordinary cooperation should be easy. When something goes wrong, peers can inspect
claims, raise concerns, correct harm, take responsibility, and learn.

A credential expresses a recognized relationship and a bounded mandate. Possession
of a secret enables an action; it does not, by itself, establish the legitimacy of
that action. Human dignity and continuing identity do not expire with a credential.
Authority is negotiated through relationships, not vested absolutely in a key.

This draft records Matthew's September 30 design conversation and connects existing
specs to the Che/FCT demonstration. It is an implementation contract under review,
not evidence of community ratification or a claim that the complete model is built.
Its owning habit is `dataplane-convergence`; its existing acceptance story is
`genesis/a2o/features/lms/commons-path-steward-publish.feature`. No new register is needed.

## 2. P2P design gate — existing primitives first

Source audit baseline: published superproject `e763b68ffec5f77192d8fd1d681bf018b522c7da`.
The authoritative entry declarations were inspected in the imagodei, mishpat, and
content-store integrity zomes; Authority and DeviceBinding are payloads of the
existing Mishpat Commitment, not separate entry types.

| Concern | Classification and address | Existing authority and creation seam | Projection and cost |
|---|---|---|---|
| Human identity | A; original notarized Human reference, independent of device keys | `imagodei_integrity::Human`; `imagodei::create_human` | Existing identity/DID projection; zero new humans for device enrollment |
| Controller policy and device association | A; immutable Commitment hashes, linked to the original Human | `mishpat_integrity::Commitment`; `mishpat::{bootstrap_device_identity,enroll_identity_device,revoke_identity_device}` | Existing identity projection; explicit policy/binding/revocation events only |
| Publishing mandate and acceptance | A2; signed proof on an existing immutable content root and exact version | `content_store_integrity` canonical links; `content_store::{grant_head_delegation,accept_delegated_head,revoke_head_delegation}` | Existing head projection; no standalone content head per credential |
| Public policy and supporting outcome claims | A where independently consequential; A2 for attributes; content-derived CIDs | Existing Manifest/Claim/content-typed attestation vocabulary; exact adapter reuse must be verified before implementation | Scope-visible proofs; no credential-per-read or judgment-per-packet heads |
| Wallet secrets and private supporting evidence | B for secrets; B2 when private evidence produces a verifiable attestation | Existing keystore/private-chain custody; outcome attestation alone crosses its authorized reach | No secret-bearing public EPR; no head for private material |
| Local evaluation cache | C; keyed by proof, policy version, subject, action, and verified state | Reconstruct from signed proofs and locally integrated records | Operational projection only; invalidated by relevant policy/revocation changes |

**Network stakes:** the deterministic floor holds without AI. Trust can price proof
discovery, scrutiny, and cached derivation; it cannot fabricate missing evidence or
remove protected dignity, appeal, or accountability. All participation stages retain
these floors. Byte-bearing descriptors use existing EPR transport selection; secrets
never enter public propagation. No new HTTP route or sync protocol is proposed here.

**Head-plane budget:** the demo reuses one real Human, existing identity Commitments,
and 59 foreign-root grant proofs for a 94-item closure. Acceptance is attached to
authored versions. Additional proof heads per read are zero. An illustrative year
of monthly renewals would produce 708 individual root-grant proofs; do not promote
those to 708 independently swept content heads. A future author-scoped manifest
could bound those renewals to 60 manifests/year for five authors, with exact root
membership proofs. That bundling is a design option, not an implemented wildcard
grant; implementation must measure its actual notary, link, and reconciliation cost.

**DNA boundary:** reuse of current entries and links permits coordinator-only work.
If required validation cannot be expressed safely with those primitives, explicitly
review an integrity change and its DNA/network migration before claiming neutrality.
Coordinator return values remain native action/entry hashes; descriptor CIDs are
separate addresses, with witnessed mappings. A CID is not an ActionHash alias.

**SDO/RWA test:** a hostile holder must not gain a global membership/activity graph,
private counsel context, raw recovery evidence, or key material from a mandate.
Proofs disclose only what the action's reach permits; intimate relationships stay
within their holon. Cross-namespace correlation requires an authorized crossing act.
Counter-evidence and constitutional protections survive adverse standing.

## 3. Four relationships that must remain distinct

1. **Recognition:** relationships back the continuing Human identity and its recovery
   policy. Self, spouse, family, a church, library, rights organization, government,
   hospital, or association can have different roles; no universal list or quorum
   makes every person's circumstances identical.
2. **Standing:** a person or collective has a justified role concerning a subject,
   such as course authorship or stewardship. Evidence and applicable policy determine
   that standing. A device binding alone does not establish content stewardship.
3. **Mandate:** an authorized party permits particular actions on particular subjects,
   under named conditions and a versioned policy. Delegation cannot enlarge that scope.
4. **Exercise:** a particular runtime, using its actual signing key, performs an action
   under that mandate. The receipt preserves both human authorization and execution.

The demonstration may use fixture humans as authors of record and replicating peers.
Matthew's stewardship can justify their scoped delegation, but the current native
root-author check still requires each immutable root author's signature. A collective
stewardship claim cannot silently replace that check. The bridge from social standing
to native authorization must be explicit and verifiable.

## 4. Credential descriptor and wallet

The public-facing descriptor reuses EPR Delegation, Attestation, Claim, Manifest, and
Commitment semantics as appropriate. Do not mint a new Key kind merely because the
descriptor describes a key. A descriptor is trusted through its proofs and policy,
not because its kind or title says "credential."

Its inspectable contract identifies:

- The recognized Human or collective, issuer, accountable steward, delegate key, and
  actual runtime/cell/network context.
- The purpose or story, immutable subjects/content roots, permitted operations,
  audience/reach, delegation limits, and excluded authority.
- The supporting claim references and pinned policy/lens version; private evidence
  is represented by authorized outcome attestations, not disclosed wholesale.
- Activation, expiry, renewal, withdrawal, supersession, and challenge conditions.
- Applicable value obligations or economic Commitments, if any. An author's ordinary
  correction need not invent payment or collateral to become legitimate.

The wallet holds private signing material and references to descriptors, proofs,
custody arrangements, and receipts. It explains "who can do what, why, until when,
and how to withdraw it." Public inspectability does not expose secrets or require
global visibility of the backing social graph. Raw recovery shares are private.

PAT/JWT-like credentials can be transport conveniences for this contract. Their
issuer/audience/expiry and proof binding must be verified; token possession is not
a substitute for subject scope. No new token format is selected by this draft.

## 5. Native and remote execution

**Che as a native device:** Che keeps its own conductor, keystore, and history. Its
verified binding resolves to the existing real Matthew. Existing hub/device and
browser participation remain valid. Missing local identity state requires explicit
reenrollment, not silent replacement of Matthew or copying another device's key.

**Che through another runtime:** distinguish the requesting key from the callee's
signing key. The peer runtime acts under a bounded mandate; its signature must not
launder arbitrary requests into apparent root-author consent. Record the requester,
executing agent, mandate proof, and resulting native action together.

At the native authority boundary, compare authenticated call provenance with the
authorized requester and enforce payload scope before signing. An admin login or a
function allowlist cannot authorize every root the callee happens to have authored.
Ordinary publishing must not silently mint further signing or delegation grants.

For the demo, issuance remains with the root authors unless a native-enforced,
exact-subject issuance mandate is implemented and proven. The seven previously
proposed persistent function-scoped credentials are superseded by this contract;
they were not issued, and do not constitute approval of a broader signing mandate.

## 6. Deterministic verification and contextual judgment

The evaluator walks a bounded proof chain: recognized identity/key association;
applicable issuer standing; signed mandate; permitted subject/action/context;
conditions and effective lifecycle state; then exact action and acceptance evidence.
Cycles, excessive depth, ambiguous policy successors, and unavailable prerequisites
produce an explicit unresolved result. Inspectability alone never proves validity.

The deterministic floor checks signature integrity, proof possession where required,
network/audience binding, scope containment, policy authority, and witnessed ordering.
For canonical publication, the receiving conductor independently integrates and
validates required ancestry, grant, acceptance, and election before serving the head.
Carried history and a caller-supplied past timestamp do not replace that evidence.

Contextual judgment can affirm ordinary work, request clarification, narrow reach,
seek another witness, or escalate a consequential decision. Familiar stewardship
should make routine course updates easy. Recovery, custody changes, and authority
issuance require stronger deliberation than correction of one's own course.

Policy can allow bounded, optimistic local activity while affirmation is pending.
Offline drafts and authored-but-undeclared versions remain available and honestly
pending. This does not make a receiver serve an unverified canonical version.
Unavailable AI does not prohibit actions already justified by the deterministic
floor and applicable policy. Greater compute deepens context; compute poverty does
not diminish personhood or require every ordinary action to run an LLM.

An AI affirmation or challenge is itself an attributable, bounded claim: identify
its mandate, policy, relevant evidence, and reasons, with an appeal path. AI does
not become an ambient administrator. Each holon retains its policy surface and
negotiates context with neighboring holons; there is no single global trust score.

## 7. Challenge, revocation, expiry, and repair

Anyone with appropriate access can recheck evidence and raise a concern. A challenge
does not automatically become a revocation: the governing scope and policy identify
who may suspend, withdraw, adjudicate, or repair the affected authority. Responses
remain attributable and proportionate, with subject voice and appeal protected.

Device-binding revocation, invocation-capability revocation, and content-delegation
revocation are different acts. Each stops the corresponding future exercise under
its effective conditions; revoking one does not silently revoke every other layer.
If incident response needs several withdrawals, record each explicitly.

Ordinary expiry stops acceptance of new versions. Resuming declaration of the exact
version already witnessed before expiry remains valid; that resume is not another
exercise of expired authority. Expiry does not demote a version validly accepted
earlier. Preserve the exact grant, policy version, and native acceptance
witness that established historical authority. Explicit invalidation of an accepted
version requires a distinct authorized act and correction/election evidence; it
cannot be smuggled in as expiry or a silent database edit.

Repair preserves recoverable content, identity history, and accountability. Keys,
standing, and mandates can change without deleting the person. Community-grounded
recovery must support people acting with or through others; a lost key must not
become loss of dignity. Full social recovery is outside this demo implementation.

## 8. Current implementation and gaps

At the audited revision, additive bindings use existing notarized Commitments,
controller-policy authorization, device proof of possession, and network binding.
The bootstrap is explicitly development-network authorization, not broad social
affirmation. Content delegation provides root-author signatures and exact native
acceptance witnesses that preserve historical authorization after ordinary expiry.
The `steward-publish` preflight independently verifies the Mishpat device binding;
native content-delegation functions do not themselves inspect that binding. This
draft requires explicit composition at the authority boundary, not a claim that
device revocation already withdraws every content grant.

The remaining credential gap is significant: `grant_head_delegation` uses the
callee's `agent_info()` authority. Native invocation grants restrict functions,
not the roots or expiry in their inputs, and the assigned invocation capability
itself has no TTL. CLI selection of 59 roots therefore does
not constrain what a stolen issuer invocation credential can request. Enforcement
must happen before native issuance/acceptance, not solely in the wallet or CLI.

Collective stewardship recognition, the complete negotiated recovery holon,
claim-backed invocation mandates, and continual AI affirmation are design work.
Existing browser localStorage signing credentials and unencrypted local recovery
share storage are custody limitations, not evidence of a completed secure wallet.
This spec changes no live credentials, identities, or recovery policy.

## 9. Acceptance contract for the existing story

Extend the existing story and its `@concern:`-bound checks rather than creating a
parallel permission test registry. Implementation acceptance requires:

- Two independently keyed participants resolve to one real Matthew; restart preserves
  Che's key/history; withdrawing Che leaves Matthew, other devices, and browser access.
- Unknown, forged, wrong-context, revoked, and out-of-scope proofs are refused. A stolen
  invocation credential cannot issue grants for an unlisted root, change recipients,
  extend expiry, administer identity, or delegate onward beyond its mandate.
- Native and remote exercises identify requester, executor, Human, subject, and proof.
  Ordinary publication creates no implicit capability. Missing proofs remain pending.
- Successive updates preserve the update-then-declare workflow and resume the exact
  authored version after interruption. Offline work survives; seeding preserves heads.
- Expiry prevents acceptance of another version but permits resuming declaration of
  the exact previously accepted version and preserves its witnessed acceptance.
  Explicit revocation and invalidation are tested separately, including stale caches.
- The selected 94-item closure publishes dependencies before composition. Adam and
  Jessica independently integrate required history/election, and both doorways serve
  the accepted version within one shared 75-second deadline for each publication.
- Contextual affirmation cannot manufacture authority, erase counter-evidence, expose
  private supporting relationships, or silently turn a challenge into network removal.

The habit stays red until its runnable evidence proves the required behavior. A
written spec, green source tests, and a successful deployment are distinct from
successful publication under this contract.

## 10. Scoped implementation candidate — 2026-10-01

The candidate on `codex/claim-backed-fct-credentials` is based on the audited
`e763b68` implementation, in an isolated worktree. The shared development tree
was not replaced. This remains a draft: no live credential or FCT head was changed.

The peer runtime now checks the conductor-selected, author-signed Assigned/Listed
CapGrant before remote issuance, acceptance, earned preflight and declaration.
Its private tag binds actual requester, executing issuer, DNA, exact content IDs
and immutable roots, delegate, existing device binding, operations, policy and
expiry. A separate exact-payload mandate guards identity signing ceremonies.
The capability secret is refreshed into a separate custody profile while the
existing signing key is reused, so an overlapping old grant is not selected by
the same secret. No zome function mints arbitrary invocation mandates.

Signed content grants retain the device binding and invocation exercise. Existing
v2/v3 bytes are preserved when the new fields are absent. Historical recovery
uses the exact native accepted head and original witness time; controller policy
is evaluated at that time, and later visible withdrawal does not erase history.
Publication receipts identify requester, executor, Human and binding. A peer can
notarize and verify a device affirmation using the existing Mishpat Commitment;
this evidence alone grants no publication or collective admission authority.

`genesis/a2o/scripts/steward-credential.ts` supplies explicit `issue`, `grant-heads`,
`exercise-ceremony` and `refresh-hosted` commands. `steward-publish --connection`
reuses the resulting connection descriptor. Issuance requires owner admin custody,
an exact installed cell and an existing signing profile. Its plan contains:

```json
{
  "connection": {
    "adminWs": "ws://localhost:4444",
    "appWs": "ws://localhost:4445",
    "appId": "existing-owner-app",
    "role": "lamad",
    "zome": "content_store",
    "expectedAgent": "<native-root-author-key>",
    "expectedDna": "<content-DNA>",
    "signingCredentialsDir": "<existing-private-custody-directory>"
  },
  "outputDir": "<new-private-scoped-profile-directory>",
  "mandate": {
    "delegate": "<workspace-peer-key>",
    "subjects": [{ "id": "<exact-course-item>", "root": "<verified-native-root>" }],
    "operations": ["grant_head_delegation", "stage_delegated_head_acceptance", "accept_delegated_head", "get_accepted_delegated_head"],
    "valid_until": 0,
    "binding": "<existing-Matthew-device-binding>",
    "policy": "fct-commons-v1",
    "exact_payload_json": null
  }
}
```

Replace every placeholder and set `valid_until` to a future Unix time in
microseconds before use. A publishing-device plan instead permits only
`preflight_head_publication` and `declare_earned_canonical_head` for its exact
roots. The owner must explicitly issue each profile. Secrets remain in the
private profile, never in the public grant. Lost issuance responses require
explicit custody reconciliation; re-running must not silently replace a grant.

Doorway `POST /hc/connect` accepts explicit `reuseCredentials: true`: it resolves
the authenticated existing peer and issues transport access without creating a
native capability. The same scoped native credential remains the authority.
Transport refresh requires the saved issuing-origin session and exact app/cell.

**Exact publication witness — 2026-10-01 follow-through:** device-backed acceptance now
requires a controller-authored `authorizes-device-publication` Commitment for the
exact device binding, content DNA, immutable root and authored head. The receiver
loads every consecutive signed action from that witness to the existing authority
checkpoint and inspects public Commitment writes directly. Missing lifecycle
links cannot hide withdrawal on that history. Missing actions/entries or more
than 4096 actions leave verification pending. The root first stages provisional approval. The controller witness includes that
signed native root action and follows it. Complete controller history through the
witness must contain no prior withdrawal. Thus withdrawal between provisional
approval and reconciliation also refuses. Only both facts together authorize the
exact version; later withdrawal preserves this completed earlier exercise. Final
root receipt reconstruction retains the original provisional approval time and
cannot refresh election priority.

This bounded first implementation supports the original single-controller
bootstrap policy. Rotated policies and multi-controller quorum checkpoints are
refused; their reconciliation is not implemented. This is A2 evidence on an
existing binding, reused through Mishpat's existing Commitment type, with immutable
ActionHash addressing, no new content head, no HTTP route or Automerge projection,
and no integrity/DNA-hash change. At 94 initial items and an illustrative monthly
full revision, 1128 witness records/year create no independent content-head sweeps;
verification cost is bounded per exercised publication. Public evidence exposes
only the scoped authorization and existing controller action headers, never
private entry bodies, credential secrets or raw private supporting evidence.

Explicit `stage-head DESCRIPTOR ID HEAD` records provisional root approval;
`exercise-ceremony` with the `witness_device_publication` operation emits
the controller action; `steward-publish --device-witnesses FILE` consumes a private
map `id -> {head, witness}`. Ordinary publication never mints that authority.
The signed root acceptance and portable receipt retain the controller action,
so crash recovery remains exact and does not refresh authorization.

**Verification:** 128 content-coordinator unit regressions, 72 Mishpat unit
regressions, two mandate regressions and the doorway transport regression passed.
All nine focused steward TypeScript test files, the owning A2O gate, schema/DNA
and manifest gates, sweettest compilation and all five DNA extern lints passed.
Both changed coordinator crates compile for WASM. The real three-device native
ceremony passed: selected scoped capability issues exact authority, broad
function access and expanded scope refuse; controller proofs survive later
withdrawal while new proofs after withdrawal refuse even without discovery links.
Historical recovery also refuses a valid public acceptance when the selected
private mandate names a different binding. A local 32 MiB proof stack and two worker threads keep this regression runnable
under libtest and CI without changing peer runtime presets.

Live owner preflight found four original root-owner cells intact and Adam's
conductor blocked by SQLite disk I/O error (code 4618). Private requester custody and scoped enrollment plans now cover 55 foreign roots
on the four healthy cells; those native grants are not yet issued. The workspace
received and exercised its native invocation credential for the exact 94 roots,
existing device binding and two publication operations. No deployed
foreign-owner grants, independent live receiver adoption or full 94-item release
is claimed. The owning habit remains red.

## 11. Contextual reconciliation boundary

Witnesses and affirmations support a credential's claims from the peer runtime.
A different holon of peers may inspect those claims and supporting evidence, then
affirm the relationship and issue its own bounded authorization for its governance,
values and current standing. Inspectable evidence can cross an authorized boundary;
authority does not transfer automatically. Existing mandates are DNA/network bound.
No cross-network admission, community scoring or universal witness quorum is
implemented by this candidate.

## 12. Authority by standing — operator ruling, 2026-10-04

> **Status: failed review, do not implement.** A second-model adversarial review on
> 2026-10-04 judged 12.2-12.5 unsound. The ruling and the planes (12.1) stand. The
> findings are in 12.7 and the records and election must be redesigned against them.

**Ruling.** The native rule that only the key which authored a content root may
grant or accept a head is ownership by key. It violates stewardship. The network
holds the root; anyone the network's governance recognizes as having authority may
author to it. The rule is replaced, not kept beside its replacement: no
compatibility arm, no interim bridge. This supersedes §2's "Publishing mandate and
acceptance" row, the "current native root-author check" paragraph of §3, and the
root-author issuance paragraph of §5.

### 12.1 Four planes that one check must never fuse

| Plane | Question | Carried by |
|---|---|---|
| Provenance | Which device produced these bytes, and when | The device's cryptographic root signing what it produces; action signatures; root authorship |
| Recognition | Whose device that is | The identity's controller policy and device bindings |
| Standing | Who may change this subject | A standing claim (12.3) |
| Reach | Who may see or receive it | Existing reach machinery |

Provenance is evidence and attribution. It confers no authority, and no change of
standing alters it. Recognition says which person a key acts for; it confers no
authority over any subject. Standing is held by a person or a collective, never by
a key. Authorization does not read reach. A check that reads two planes in one
comparison is the defect this section removes. The planes in §3 (recognition,
standing, mandate, exercise) are relationships; these are the planes they live on.

**A further plane, named and not designed here (operator, 2026-10-04).** The four
above describe a persistent record, which changes rarely. There is also a live
plane: peers online together, merging one another's changes as they happen, the
way a CRDT does. That is where an interactive witness occurs, because a peer who
is present can sign what it sees as it sees it. The witness ladder already has a
rung for it: recorded, then converged, then corroborated, then notarized. The
persistent record holds what was settled; the live plane is where peers come to
agree on what is current. This bears on finding 2 in 12.7: whether a claim is
still current may be a question for the live plane, not one a carried record can
answer. Nothing in this section depends on the live plane yet, and nothing in it
should prevent a claim being witnessed there.

### 12.2 Two properties every authority record has

1. **The base case is one person with one device.** That device is the identity's
   controller, is first steward of what it creates, and authors with nobody else
   present. Mandates, acceptance and witnesses appear only when a second device or
   another person is involved. They strengthen a claim. They are never a
   precondition for authoring.
2. **Authority is checkable from carried bytes.** Each record is a
   content-addressed claim with detached signatures. A head declaration carries
   the claims it rests on, and a receiver verifies signatures and chains from what
   it was handed. Local lookups answer only freshness: revoked or superseded here.
   A network that has never seen the author can therefore inspect the same claims
   and decide for itself (§11); authority still does not transfer automatically.

A claim born alone is recorded. One signed by others at birth is witnessed. Either
can be affirmed later without changing its address, because the address excludes
the proofs. A receiver prices how much checking a claim needs by what is at stake;
delegations and attestations stay on the floor that is never priced down.

### 12.3 The records

**Standing claim.** States that a person or collective stewards a subject (one
content root; a corpus later), for a scope and a period, how it was constituted,
and its predecessor.

- *First standing* is recorded in the act that creates the content, as a link on
  the root naming the steward. It adds no head. The steward is the identity the
  creating key is recognized for, or a collective that identity is a Steward of.
- *Sharing and succession* are governance commitments signed by the current
  holders, meeting the holder identity's own controller policy.
- *Recovery* is a governance commitment signed by the network's witness quorum.
  It always exists. Today that quorum is the network's founding authority.
- A fork in a standing chain leaves no holder able to move the head until a
  recovery claim resolves it. Provenance and already-accepted heads are untouched.
- Roots that exist today receive their first standing in one pass per network.
  There is no rule that falls back to the root author's key.

**Mandate.** A standing holder permits one recognized device to author heads, for
every subject the holder stewards or a named subset, until a time. One mandate per
device, not one per subject. A device binding alone never confers this.

**Acceptance.** A standing holder's controller signs that a specific head by a
mandated device is accepted. Not required when the declarer is itself a controller
of the holder.

### 12.4 The election

A declarer stands in the earned tier when the carried claims show, for the
subject's root: a valid standing chain ending in holder H; and either the declarer
is a controller of H, or the declarer is a device bound to H, holding H's mandate,
with H's acceptance of this head. A mandated device's head without acceptance is a
valid candidate below the earned tier. The founding authority no longer declares
heads directly; it acts only through a recovery claim.

### 12.5 Design gate

| Record | Class | Address | Home | Cost |
|---|---|---|---|---|
| First standing | Linked (A2) | Link on the root; anchor is the root's action | Content DNA coordinator, existing link type | One link per root, no new head |
| Shared, succeeded or recovered standing | Notarized (A), existing type | Commitment entry hash | Governance DNA coordinator, new action name | Rare: a handful per subject per year |
| Mandate | Notarized (A), existing type | Commitment entry hash | Governance DNA coordinator, new action name | One per device, renewed on expiry |
| Acceptance | Linked (A2) | Link on the root to the head | Content DNA coordinator, existing link type | One per accepted head, as today |

All of it is coordinator-only; no entry or link type is added, so no DNA hash
moves. Every record is floor-protected at every network stage. A hostile holder of
any of them learns that a device acts for an identity on a subject it could already
see; device labels and the consent record that produced a mandate are not notarized.

### 12.6 Open, and named

- Vote-constituted standing waits on a tally that cannot be satisfied by one write.
- Collective first standing needs the membership check to be callable from the
  content DNA.
- Corpus-scoped standing, to avoid one sharing commitment per root when a whole
  corpus changes hands.
- The witness quorum beyond the founding authority.
- How a holder accepts many heads without approving each by hand.
- Whether an integrity-level check is needed so a forged first-standing link is
  refused at validation and not only at election; that would move the DNA hash.

### 12.7 Review findings to design against

1. **First standing is forgeable.** The content integrity zome validates no links,
   so anyone can write a first-standing link, and anyone can create a second root
   for an existing content id (the canonical root is only the earliest retrievable
   one). First standing needs integrity-level validation and a provably unique
   subject. The operator accepts the DNA hash move this requires.
2. **Authentic is not current.** Carried claims prove a claim was made, not that
   it still holds. A succeeded holder or a revoked device can replay old claims to
   a peer that has not seen the change. Claims must name a standing epoch, and
   receivers need a stated freshness policy and a pending outcome when they cannot
   tell.
3. **A fork must not lock everyone out.** One finalized successor per predecessor,
   with sharing distinguished from conflicting succession.
4. **Recovery cannot rest on the founding authority alone.** It needs a quorum
   constituted independently of that authority, bounded grounds, and appeal.
5. **Creator-as-first-steward is the ownership defect in an identity wrapper**
   unless governance states it as an explicit, transferable initial-stewardship
   policy over a genuinely new subject.
6. **Carried proof does not fit the 1,000-byte link tag.** It needs a separate
   bounded proof bundle with its digest in the tag.
7. **Two reuse traps.** The k-of-n lineage check accepts a supplied threshold and
   signer set without authenticating eligibility. A governance commitment's
   address includes its proofs, so it cannot be re-signed by later witnesses
   without changing its address.
8. **Unspecified:** controller quorum for acceptance, ordering across chains,
   proof size limits, deterministic encoding, and uniform verification across the
   staging, earned and carried paths. The generic declarer gate is open today.

The one-device base case and carried positive proof were judged achievable.

### 12.8 Second design, against the findings

> **Status: failed review, do not implement.** This replaced 12.3-12.5. A second
> adversarial review on 2026-10-04 judged it unsound; see 12.9. Review is capped
> at these two rounds until the operator rules on finality.

**The subject is a root, never a name (finding 1).** Standing attaches to a
content root's action hash. A second root made under the same content id is a
different subject and has no bearing on the first. Which root a name means is a
separate question, answered by the steward of the namespace that binds the name,
as a declared dependency; "earliest root retrievable" is not an answer and is
removed with the rest. Name binding is a dependency of this design, not part of it.

**First standing applies a stated policy (findings 1, 5).** Every network carries
an initial-stewardship policy in its constitution: the recognized identity that
creates a new subject is its initial steward, and that stewardship is transferable
and recoverable. The first-standing link on a root cites that policy. It is valid
at the integrity level only when its author is the author of the root it is placed
on. That check moves the DNA hash, which is accepted. If two first-standing links
appear on one root, the one with the lower action hash stands; nothing is frozen.
First standing is therefore governance applying a rule to a fact of provenance, not
provenance acting as authority.

**A standing chain has epochs, and the election reads them first (findings 2, 3).**
Each standing record names its predecessor and an epoch one higher. A mandate, an
acceptance and a head declaration each name the standing record and epoch they act
under. The election orders candidates by epoch before anything else. A head
declared under a higher epoch beats every head under a lower one, so a holder who
was succeeded, or a device whose mandate rested on an earlier epoch, loses as soon
as the current holder declares or re-affirms a head. A peer that has not yet seen
the succession is behind, not wrong: it converges when the record reaches it.
Nothing accepted earlier is erased from history.

- Sharing and succession are both successor records; sharing keeps the earlier
  holders in the new set.
- A successor needs the outgoing holders' signatures, checked against each
  identity's own authenticated controller policy with the quorum check the device
  binding already uses (`verify_quorum`), not the lineage check (finding 7).
- Two valid successors of one predecessor are a conflict, not a freeze: the one
  with the lower record address stands. An identity that lets one controller sign
  alone accepts that one controller can hand its standing on; a quorum policy is
  the protection.

**Revocation is a successor, not a lookup (finding 2).** Withdrawing a device's
mandate, or its binding, is recorded as a successor epoch of the standing chain or
of the identity's authority. Heads the device declares afterwards name a stale
epoch and lose the election. A mandated device's head still needs the holder's
acceptance to reach the earned tier, which is what stops a withdrawn device
backdating a head.

**Recovery belongs to a body the subject names (finding 4).** A standing record
names its recovery body and threshold. The network's constitution names a witness
body as the last resort, and the declared network stage says how it may be
constituted: at the bootstrap stage it may be the founding authority, stated
openly as such; at the coordinated and enforced stages it must be a quorum
independent of that authority. A recovery record states its grounds from a closed
list and takes effect after a contest window in which the current holders may
object. A person alone on a network of one has no one to recover them, and the
design says so.

**Claims are addressed apart from their notarization (finding 7).** A standing
record, a mandate and an acceptance are each a canonical claim body with its own
content address. The governance commitment that notarizes one carries the claim's
address and the constituting signatures. A later witness adds an affirmation that
cites the claim's address. The claim's address never changes.

**Carried proof is a bundle (finding 6).** A head declaration's link tag carries
the digest of a proof bundle: the standing chain from first standing to the epoch
acted under, each holder's authority and controller policy, the mandate, the
device binding and the acceptance. The bundle is a content-addressed blob with a
stated ceiling on size and on chain length. A receiver verifies it from its bytes.
A network that has never seen the author can do the same.

**One verification, every path (finding 8).** A single function decides authoring
standing from a bundle, a subject, a declarer and a head. The staging, earned and
carried paths all call it. Proposing a head (the staging tier) requires only
recognition: who is proposing. Earning requires standing.

**The live plane carries currency.** Peers online together exchange, per subject,
the highest epoch each has seen. That is how a peer that is behind catches up
quickly. It adds speed; the epoch ordering above is what makes the outcome the
same without it.

**The one-device case.** One person with one device is the sole controller of
their identity, initial steward of what they create, and declares heads with no
mandate, no acceptance and no one else. Every other record in this section appears
only when a second device or another person does.

**What moves the DNA hash:** the first-standing link's author check, and the
removal of the root-author rule's link conventions. Everything else is coordinator
logic in the content and governance zomes.

**Still open:** name binding; vote-constituted standing; corpus-scoped standing;
the closed list of recovery grounds and the contest procedure; how a holder accepts
many heads without approving each by hand; the bundle's size ceiling.

### 12.9 Second review, and the question it leaves

Of the eight findings, one was judged resolved (5, the explicit initial-stewardship
policy), six partly resolved, and one not resolved (2, currency).

What 12.8 got wrong:

- **Epochs order heads; they do not make a change current.** A succeeded holder
  still wins wherever no higher-epoch head has appeared, even where the succession
  is known. A withdrawn device is not stopped at all when the withdrawal is an
  identity-authority change, because that does not advance the subject's epoch.
- **"Lower address wins" is not finality.** An outgoing holder can sign a
  legitimate succession and a competing one that keeps itself, grind the second to
  a lower address, and extend it. Repeated self-successions also inflate epochs.
- **Naming the founding authority's recovery power does not limit it.**
- **Still unspecified:** how a receiver authenticates the constitution it is shown;
  acceptance quorum; bundle delivery, since a digest authenticates bytes and does
  not deliver them; what replay binding each claim signs over; migration of
  existing roots; and keeping a higher-epoch proposal from displacing an earned head.

What held: the integrity author check stops a stranger's first-standing link; a
counterfeit root gains nothing over the original; one device authoring alone works;
carried verification is achievable once the bundle travels with the declaration.

**The open question is finality, and it is the operator's to rule on.** A network
with no global order cannot make a change of standing current everywhere at once.
Either a change becomes final when a named body witnesses it, and is provisional
until then; or changes are never final, only eventually agreed, and the design
bounds what a stale party can do in the meantime. The first matches the model the
operator described (born locally, stronger when witnessed, affirmed later) and
places finality on the live plane, where peers present together witness a change.
It needs an answer to who the witnesses are for a subject, and for one person alone.
