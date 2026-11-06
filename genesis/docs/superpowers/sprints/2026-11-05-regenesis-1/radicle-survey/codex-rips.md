✅ marks explicit source statements, including named patch/issue records; ◐ marks inference or conclusions from missing evidence. Citations abbreviate [0001-heartwood.adoc](/projects/elohim/genesis/research/repos/radicle-rips/0001-heartwood.adoc) as R1, [0002-identity.adoc](/projects/elohim/genesis/research/repos/radicle-rips/0002-identity.adoc) as R2, [0003-storage-layout.adoc](/projects/elohim/genesis/research/repos/radicle-rips/0003-storage-layout.adoc) as R3, and [general-uri-scheme.adoc](/projects/elohim/genesis/research/repos/radicle-rips/0004-general-uri-scheme/general-uri-scheme.adoc) as R4. Revision IDs below identify entries in the supplied [patch JSON](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3trNYnLWS11cJWC6BbxDs5niGo82.json); issue titles identify entries in the supplied [issue JSON](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3trNYnLWS11cJWC6BbxDs5niGo82.json).

**1. Design rationale and rejected alternatives**

✅ **Git supplies replication, integrity, history and deduplication.** R1 chooses it for replication, then follows that choice into storage because the two are tightly coupled. R3 adds that Git fits the exchanged data, saves disk space, deduplicates shared objects and fetches missing objects. R2 uses hash-linked commits to preserve identity history and expose omitted intermediate changes. This is an explicit reuse argument, not a comparison proving Git superior for arbitrary artifacts. (R1, lines 56–68, 490–495; R2, “Identity Storage”; R3, lines 35–42.)

✅ **One object database per project, multiple peer namespaces.** “One storage” does not mean one global repository: Heartwood rejects its earlier monorepo arrangement in favor of separate bare repositories, citing privacy, project-level locking, object sharing and per-project settings. Within each repository, peer namespaces share an object database, avoiding duplicate objects across forks. Single-writer forks accommodate offline work without a protocol-wide ordering consensus. (R1, lines 536–556; R3, lines 23–33, 120–137.)

✅ R3 explicitly rejects organizing these forks under `refs/remotes/<nid>`: ordinary abbreviated fetches fail to find the expected branch, and tag pushes target Git’s special tag namespace unless overridden. Native Git namespaces preserve normal reference behavior. Merely configuring namespaces is insufficient authorization; the remote helper rejects pushes to another Node ID. (R3, “Alternative Designs,” lines 365–422; “Authorization,” lines 346–355.)

✅ **Signed refs provide authenticity beyond object integrity.** Git can verify object integrity without establishing that Alice published those objects. Requiring direct retrieval from Alice would require simultaneous availability and create a single point of failure. Signed reference mappings let intermediaries serve verifiable repository state; including the identity document binds source history to repository identity. (R1, lines 59–68, 95–104, 127–139.)

✅ **Delegates and thresholds express repository authority.** Delegates are trusted signing keys representing a project; R1 explicitly says they need not map one-to-one to maintainers. R2 requires every initial delegate to sign the root identity commit, while subsequent updates require the previous document’s threshold of previous delegates. This prevents a proposed replacement authority from authorizing itself. (R1, lines 108–119; R2, “Verification,” lines 389–408.)

✅ **The initial identity document anchors continuity.** Mutable source contents alone cannot identify a repository whose views diverge and later converge. R2 explicitly rejects a merely random identifier because it leaves legitimacy entirely to users. Hashing the initial canonical identity document supplies a stable, self-certifying identifier; using its Git blob OID also makes retrieval convenient. Identical source histories can consequently have different project identities. (R2, “Overview,” lines 31–47; “The Repository Identifier”; R1, lines 121–133.)

✅ **Seeding is voluntary; gossip locates willing holders.** R1 rejects deterministic DHT-style placement because operators must choose what they host. Inventory gossip builds routing knowledge; reference announcements trigger interested peers’ fetches. It declines routing-table partitioning “for the time being,” using an estimated 244 MB for one million projects as justification. That is a scale assumption, not a universal scalability result. (R1, “Topology,” “Routing,” “Reference Announcements.”)

✅ R1 also rejects indiscriminate replication of every contributor’s branches as readily abused. Delegate-only tracking is safer but leaves offline contributors’ patches potentially unavailable; the text admits no satisfactory protocol answer and delegates policy to seed operators. (R1, “Project Tracking and Branches,” lines 418–441.)

**2. What the open proposals actually establish**

✅ **The capture does not contain several requested problem statements.** The initial descriptions for threshold semantics, Identity 2, SHA-256 RIDs and Network Configuration are empty. Later descriptions are mostly revision summaries; the JSON contains commit identifiers, not the proposed file contents. Consequently, quoting complete proposal rationales from this capture would invent evidence.

◐ **Threshold ambiguity concerns scope more clearly than transition arithmetic.** R2 says delegates sign “all updates to a repository,” including code and identity changes, but its executable verification narrative specifies identity-document transitions. R1 separately describes single-key branches and optional social/quorum canonicity; R3 postpones canonical-reference verification. This leaves unclear how one threshold governs identity changes versus ordinary or canonical ref updates. The open patch’s title corroborates a concern, but its descriptions—“Edit for clarity,” “Review,” “Address Lorenz’s feedback”—do not establish its exact resolution. (R2, lines 104–114, 398–408; R1, “Canonicity”; R3, “Future Work”; “Identity document threshold semantics,” revisions `317fd631`, `c176154d`, `d08a9523`.)

✅ **Identity 2 visibly addresses schema evolution and canonical references.** Revision `2180a517` says:

> “Add version field to identity document schema”

and narrows “everything else” to “git refs,” explicitly excluding COB actions from `xyz.radicle.crefs`. Initial-revision discussion proposes moving default-branch specification into `xyz.radicle.crefs.symbolic.HEAD`. Discussion on revision `69c40536` says:

> “we should not assume the cref payload is required.”

Its examples include invite-tree, agent and bare repositories. These are concrete concerns about separating repository identity from a mandatory code-project shape. (“Identity 2,” revisions `d5604e96`, `2180a517`, `69c40536`.)

◐ **Key rotation and multi-device fixes cannot be attributed to Identity 2 from this capture.** The underlying problem is visible: R2 equates nodes, users and public keys; R4 calls namespace/user identity ambiguous and explicitly postpones multi-device IDs. R2 permits authorized delegate replacement, but that does not specify continuity of a person across devices or key changes. These are relevant design gaps, not evidence that the unseen Identity 2 text solves them. (R2, lines 64–83, 398–408; R4, lines 100–111, 761–768.)

✅ **SHA-256 RIDs expose format coupling.** R2 defines an RID from a raw 20-byte SHA-1 Git OID; R4 reproduces fixed-length RID grammar. The patch’s substantive revision states:

> “Needed so RIDs stay self-describing as SHA-256 repositories appear”

It proposes accepting legacy bare SHA-1, CID-wrapped SHA-1 and CID-wrapped SHA-256, plus 64-hex Git object IDs and new parser tests. (“SHA-256 Repository Identifiers,” revision `a82c2e55`; R2, lines 265–305; R4, lines 159–166.)

◐ The demonstrated migration cost spans identifiers, URI grammar, parsers, tests and legacy compatibility—not just swapping a hash function. Whether existing repositories require re-identification or object rewriting is **not established** by these descriptions. Review also requests preserving RID/NID distinguishability. (“SHA-256 Repository Identifiers,” revisions `a82c2e55`, `31cfefd9`, discussion `422a10ea`.)

✅ **Network Configuration remains opaque here:** its only revision, `c260a06e`, has an empty description and no discussion. R4 independently labels network-specification syntax future work; it does not supply this proposal’s design. (R4, lines 763–768.)

**3. Artifacts, large files and availability**

◐ **Absent from the inspected RIP text:** a large-file protocol, binary-attachment model, artifact manifest, external blob-store integration, chunking scheme, custody commitments, durability targets or enforced replication factor. This finding concerns these documents, not Radicle’s entire implementation ecosystem.

✅ There are adjacent concepts: R3 illustrates a canonical release tag; R4 addresses Git blobs and suggests a raw-download query; R2 cites TUF as inspiration. None specifies release binaries or artifact delivery. (R3, line 183; R4, lines 375–407; R2, lines 49–51.)

✅ Availability is addressed at repository level: retrieval should work while at least one hosting node remains, and stale inventory entries expire without refresh. The “three nodes per project” number is an assumption in routing-memory arithmetic, not a prescribed replication factor. (R1, lines 30–39, 207–222, 299–313.)

✅ The issue “Add missing spec for Collaborative Objects” names a `radicle-artifact COB` discussion while explaining that implementers currently consult implementations and discussions. It establishes artifact-related work exists, but supplies no artifact design. (“Add missing spec for Collaborative Objects,” opening comment `0f06c1cf`, 2026-09-01.)

**4. State versus data—and the fusion points**

✅ Radicle separates gossip metadata from Git-object transfer, and peer-owned refs from shared immutable objects. Identity and sigrefs histories are disjoint from source-code history, although stored in the same repository. (R1, lines 394–408, 575–585; R3, lines 135–137.)

◐ Describing all state as “signed, thresholded” overstates the specified boundary. A peer owns its namespace; identity transitions have explicit threshold rules; collective canonical refs remain incompletely specified. Possessing or serving objects conveys no authority to publish another peer’s refs. (R3, lines 23–33, 168–173, 346–361; R2, “Verification.”)

✅ The fusion is deliberate: Git stores both code and authority history; the repository identifier inherits Git’s object hash; public keys identify nodes, authors and namespaces. R1 even proposes resolving forks in sigrefs history by the latest timestamp. R2 admits that valid but stale identity history can be served until an honest peer reveals newer history. (R1, lines 226–235, 487–488; R2, lines 265–270, 463–472.)

◐ For your plane, authenticity therefore cannot substitute for elected channel authority, sequence ordering, freshness or custody evidence.

**5. Process governance and elapsed time**

✅ The supplied corpus contains **four numbered RIP documents: three marked Final, one Draft**. README contains only the repository title and “Radicle Improvement Proposals.” There are **seven open patch objects, all with empty `merges`**, including editorial work—not seven new protocol RIPs. Historical merge totals cannot be verified without history or merged-patch records. (R1–R3, line 4; R4, line 6; README, lines 1–3; patch JSON.)

✅ First/latest revision dates below come from patch timestamps. ◐ Ages are calendar-day calculations to **2026-11-05**, not proof these captured-open proposals remained open until that date.

| Patch title / first revision | First revision | Latest revision | Age |
|---|---|---|---:|
| Identity 2 / `d5604e96` | 2025-09-23 | 2026-09-16 | 408 days |
| Network Configuration / `c260a06e` | 2026-03-22 | same | 228 |
| SHA-256 RIDs / `31cfefd9` | 2026-06-23 | 2026-07-01 | 135 |
| Threshold semantics / `4c4bb9ba` | 2026-07-31 | 2026-08-14 | 97 |
| RIP Purpose and Guidlines / `5a29f38b` | 2026-06-15 | 2026-08-11 | 143 |
| Glossary / `89b53a8c` | 2026-08-11 | same | 86 |
| Remove ToCs / `95c5cca` | 2025-08-29 | 2026-04-25 | 433 |

✅ Process governance itself is under proposal: RIP-0000 requests community feedback and “a RIP delegate to volunteer to shepard it through the process.” Its latest description says “Transition to ‘Review’.” Separately, “Signed References” admits documentation debt and an intended move toward signed push certificates. (“RIP-0000: RIP Purpose and Guidlines,” revisions `5a29f38b`, `f9ec1950`; “Signed References,” opening comment `801ff0c3`.)

◐ This supports a picture of specifications evolving alongside implementation, with long review intervals and unfinished foundational contracts. It does **not** prove team size caused delays or that open proposals are abandoned. For a small team, the transferable warning is to give essential invariants executable acceptance criteria and a named owner rather than leave them indefinitely to a future specification.

**Lessons for an artifact plane**

- ◐ **TAKE — Separate shared bytes from publisher state:** deduplicate CID chunks independently of each publisher’s signed declarations. (R3, lines 120–137.)
- ◐ **TAKE — Preserve voluntary placement:** inventory should locate consenting custodians, consistent with R1’s rejection of compulsory deterministic hosting. (R1, “Topology.”)
- ◐ **LEAVE — Git-specific storage coupling:** its rationale fits source histories; your artifact bytes already have a content-addressed substrate. (R3, lines 35–42.)
- ◐ **LEAVE — Key equals person equals node:** retain human and collective standing independently of device provenance. (R2, lines 81–83.)
- ◐ **LEAVE — Timestamp conflict authority:** retain declared, elected heads and per-device sequences rather than importing sigrefs’ proposed timestamp recovery. (R1, lines 487–488.)
- ◐ **TAKE — Self-describing identifiers:** preserve CID version/hash information across manifests and links to avoid RID-style grammar migration. (“SHA-256 Repository Identifiers,” revision `a82c2e55`.)
- ◐ **WATCH — Availability requires separate evidence:** holdings advertisements and valid signatures do not establish custody duration or sufficient replication. (R1, “Pruning” and “Project Tracking and Branches.”)
- ◐ **WATCH — Specify authority transitions before shipping:** distinguish provenance, channel election, delegation changes and threshold scope with executable checks. (R2, “Verification”; R3, “Future Work.”)