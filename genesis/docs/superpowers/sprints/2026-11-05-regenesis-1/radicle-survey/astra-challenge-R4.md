**REFUTED — R4’s separation of historical verification from adoption is sound, but unconditional pinned serving, automatic fallback, and audit retention conflict with the existing election and witnessed-harm contracts.**

1. **High — “Revoke retains for audit; purge never by the release plane” lacks mandatory floor precedence.**

   Keeping release withdrawal separate from subject erasure is reasonable. The contradiction arises because R4 also includes **mishpat rulings against content** within `revoke`, then prescribes retention and reach-limited serving.

   The swarm-custody design already distinguishes **quarantine**, which expires and retains unserved bytes, from **refuse**, which requires local destruction of shards, manifests, inventory rows, and caches. Blind custodians need not inspect or unseal anything: duty attaches when they receive the authorized verdict. These are explicit design requirements, although the issuing council’s position remains separately governed. [Swarm-custody design, lines 343–361](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:343), [actuation contract, lines 413–440](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:413).

   The inalienable-subject spec says consent revocation must at minimum stop further replication and remove serving authority. Its acknowledgement that global deletion cannot be guaranteed does **not** excuse continued service by compliant peers. [Inalienable red-team, lines 265–269](/projects/elohim/genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md:265).

   **Failure scenario:** a release contains prohibited material; a peer receives a terminal refusal but follows R4’s “retain for audit” cell. It preserves precisely the bytes the floor orders destroyed.

   **Fix:** explicitly subordinate release policy and custody commitments to subject-right and witnessed-harm decisions. Preserve permissible decision evidence; never imply that historical verification requires perpetual retention of harmful payloads.

2. **High — “Yank keeps serving pinned peers” makes a security withdrawal ineffective.**

   R4 treats every defect as an ordinary rollback case. An authenticated old pin proves historical selection, not present safety. The existing quarantine design expressly stops serving from **both** blob stores and prevents fetch, push, and salvage from redistributing the object. [Swarm-custody actuator table](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:426).

   This also sits uneasily with P1: the runtime-artifacts design calls silent staleness a harm the network heals and places upgrade stewardship at the constitutional level. It does not grant old pins an indefinite security exception. [Runtime-artifacts §4](/projects/elohim/genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:160).

   **Failure scenario:** a signed release contains a credential-exfiltration defect. After a security yank, a recovering pinned peer downloads the exact vulnerable bytes and reinstalls them. Their correct CID and historical signature do not make that recovery safe.

   **Fix:** distinguish ordinary withdrawal from a scoped security hold. Permit pinned recovery only when current policy allows it. Provide an authorized recovery path to a qualifying release, and report blocked recovery honestly when none exists. Already-held bytes cannot be remotely recalled; future compliant serving can be stopped.

3. **High — “Head moves to the prior elected winner” bypasses present qualification.**

   The election design specifies a **ceremony declaring a prior head canonical**, followed by the same local verification and adoption loop. It does not specify popping a historical winner stack. Verification includes the installed DNA, compatibility envelope, lineage, and discipline. [Runtime-artifacts §6](/projects/elohim/genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:304), [compatibility envelope](/projects/elohim/genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:389).

   R4 also contradicts its own table: superseded releases are “newly adoptable: no,” yet its yank rule makes one the replacement. R2 separately requires current custody evidence before election; a former winner may no longer satisfy that floor.

   **Failure scenario:** A is superseded by B; A subsequently loses qualifying custody or its migration path is revoked. Yanking B selects A merely because it previously won. Adoption then fails, or an implementation skips the checks to honor R4.

   `none` **is representable**: the inspected module documents `HeadTier::None`, separately from an unreachable conductor. This establishes representation, not a proven authorized transition from an existing head to explicit absence. [Release module, lines 87–90](/projects/elohim/elohim-storage/src/services/release_adoption/mod.rs:87).

   **Fix:** require a new authorized election among presently qualifying candidates, or explicit no-eligible-head. Never substitute unreachable, recency, or historical precedence for that decision.

4. **High — The authority and code evidence do not establish an executable withdrawal contract.**

   R4’s factual premises partly survive inspection. `Recomputed { matched: false }` exists, and §10.1 preserves derivation records. `holds.rs` really does protect another peer’s placement until withdrawal; its test demonstrates that behavior. But this is a reclamation rule, not proof of authenticated release-yank authority. [Derivation basis](/projects/elohim/eprfs/eprfs-core/src/derive.rs:147), [holds implementation](/projects/elohim/elohim-storage/src/services/holds.rs:119), [withdrawal test](/projects/elohim/elohim-storage/src/services/holds.rs:1160).

   Neither the inspected release module nor release schema defines the proposed `Dismiss` release-yank operation. An intent-dismissal convention cannot establish who may withdraw a release, revoke a signer, or issue a harm ruling.

   Conversely, claiming **all** authority machinery is absent would be false: `PathEvidence` already carries migration commitment status, revocation, signers, and conductor-read roster evidence. That specialized mechanism does not establish authority for R4’s operations. [PathEvidence](/projects/elohim/elohim-storage/src/services/release_adoption/mod.rs:357). The native-dev design explicitly says the general standing design failed review and blocks F6. [§13](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:688).

   **Fix:** name separate authorities and checks for election, withdrawal, standing revocation, subject consent, and harm verdicts. Signer revocation must state its effective scope; ordinary later loss of standing is not evidence that every historical artifact is compromised. One current member is not an authorization check.

5. **High — Adding restrictive table cells as ordinary manifest defaults violates D7.**

   The release schema currently declares soak, attestation threshold, canary ordering, and migration-path information—not R4’s lifecycle matrix. The Rust manifest explicitly tolerates added fields. [Schema](/projects/elohim/rakia/schemas/v1/release-manifest.schema.json:371), [Rust manifest](/projects/elohim/elohim-storage/src/services/release_adoption/mod.rs:469).

   D7 expressly forbids adding restrictions as ignorable extensions: meaning-changing fields require an epoch change, and unknown epochs cannot inform decisions. [D7](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:410).

   **Fix:** version restrictive semantics and prove mixed-version refusal. Otherwise, thousands of older peers can correctly parse a new manifest while silently ignoring its security restriction. Retrofitting enforcement later requires coordinated reader upgrades and policy migration.

6. **Medium — Three labels omit expiry, deprecation, and commitment termination, creating generational ambiguity.**

   These need not become six competing release verbs. They must nevertheless be modeled: deprecation is advisory; quarantine is expiring; refusal is terminal; custody release is a separate settlement. The custody design explicitly allows a withdrawing device’s capacity floor to reach zero. [Custody commitments](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:250).

   “Retain: yes” must mean **while a valid obligation remains**, not forever. Existing device rulings keep ten releases on hubs and current-plus-neighbors on personal devices. [Device-footprint §1](/projects/elohim/genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md:54).

   Illustratively, retaining one distinct 454 MB binary daily for ten years costs approximately **1.66 TB per complete copy**, before parity or other artifacts; a thousand such holders approach 1.66 PB. This is an assumed workload, not a measurement. Bounded custody plus durable authority evidence avoids later mass-retention migrations without conscripting peers into permanent archives.

**The ruling I would record instead**

Release lifecycle, current eligibility, serving authorization, and custody retention are separate decisions composed under the constitutional floor.

**Supersede** records that an authorized election selected another release. It ends the former release’s current-head status without destroying its history. A superseded release may become eligible again only through current qualification and a new authorized election.

**Yank** is a channel-scoped withdrawal issued under an explicitly verified delegation or the channel’s standing rule. It prevents new adoption within its scope. It neither elects a replacement nor grants continuing serving authority. Ordinary pinned recovery remains permitted only under current reach, grant, and safety policy.

**Standing revocation** records whose authority ended, when, and for which acts. Historical signatures and publication-time standing remain independently assessable from retained evidence. Prospective standing loss alone does not invalidate every prior release; compromise findings must identify affected releases or intervals.

**Security and harm decisions override those lifecycle defaults.** Reuse the existing distinction between expiring quarantine and terminal refusal. Quarantine blocks affected adoption, serving, and redistribution while retaining bytes under its authorized hold. Terminal refusal actuates the mandated local destruction. Subject consent withdrawal removes further replication and serving authority. No channel policy, pin, or custody commitment overrides these floors. Global recall is never promised.

**Heads move only through the authorized election mechanism.** Replacement candidates must satisfy current admission and custody requirements; each adopting peer verifies compatibility against its installed reality. No qualifying candidate yields explicit no-eligible-head. An unavailable authority yields unreachable, never absence. Cross-DNA recovery requires the applicable migration ceremony.

**Custody follows explicit, bounded obligations.** Release status alone neither conscripts retention nor orders purge. Termination and withdrawal reconcile commitments, advertisements, providers, stores, and caches. Retain permissible audit evidence separately from payload availability. Deprecation is advisory; expiry and review transitions are explicit.

Every restrictive act binds its subject, channel or other scope, issuer, authority evidence, effective interval, reason, and policy epoch. General network enforcement remains gated on the missing standing design and adversarial receipts. Existing lineage-roster checks are reusable evidence, not blanket authorization.

This alternative costs more explicit state and enforcement work now, but preserves the three planes, bounded custody, and historical verification without making unsafe serving or automatic rollback foundational behavior.
