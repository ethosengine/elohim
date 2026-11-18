---
title: "Unified Attestation and the remote-attestation question — whose word, about what, to whose benefit"
id: unified-attestation-remote-attestation-cross-pollination-2026-11-18
status: Draft
version: 1
date: 2026-11-18
sovereignty-frame: bounded
---

# Unified Attestation and Remote Attestation — Cross-Pollination, November 2026

**[Unified Attestation](https://uattest.net/)** (UA) is Volla Systeme's open-source alternative to Google Play Integrity, announced 2026-03-16. (The project's name is *Unified*, not *Universal*, Attestation.) It lets a banking or wallet app ask "is this phone running an approved, locked, patched Android build?" without Google answering. Two critiques arrived within the week. [lumi](https://lumi.codeberg.page/blog/remote-attestation.html) rejects the whole category: UA "is the same thing, just with a different authority". [GrapheneOS](https://grapheneos.social/@GrapheneOS/116239523775374959) rejects the *shape*: the same hardware API already supports plural certifiers publishing signed data, and UA "solely exists to create a centralized authority".

This note reads all three against what Elohim calls attestation, and against the open question in the integration work: a peer running someone else's code over the mesh.

**The answer in one paragraph.** We are not talking about the same thing, and the difference is worth naming in our vocabulary. Remote attestation in the Play Integrity sense is *hardware testimony about a machine's software state, delivered to a party who can then refuse the machine's owner*. Elohim attestation is *a principal's signed claim about a content-addressed subject, at a reach, weighed by a reader's own policy*. We attest artifacts and acts; they attest machines. UA gives us almost nothing to adopt as a mechanism and three things to adopt as discipline: a direction test for any evidence about a device, GrapheneOS's "signed data, verified locally, plural certifiers" shape (which our derivation design already has), and a short list of privacy details UA gets right. The one place the machine question honestly arises for us is confidential or unverifiable remote execution, and there the answer our own specs already give (evidence over receipts, re-execution over trust, principals over hardware) holds.

**Verification key:** ✅ read in source at the pinned commit, or in our tree at the cited line · ◐ single source, a primary page read through a summariser, or reported by the repo survey and not re-read by the author · ⚠ secondary web, or from the author's prior knowledge with no source fetched this session. Abbreviations: **UAB** = `genesis/research/repos/uattest-backend/` (commit `33008837`), **UAS** = `…/uattest-android-service/` (`0f930b45`), **UAW** = `…/uattest-wiki/` (`37ec6088`), **VBS** = `…/uattest-vbmeta-build-submit/` (`1275155c`).

**Membrane (sovereignty-frame: bounded).** lumi argues from device ownership as bodily autonomy: "everything must run under your rules, and only your rules". We share the refusal and not the ground. In the Elohim ontology a person's standing is not a property of a key or a device they control; it is imago dei, an inviolable dignity backstopped by a community that witnesses it. That makes our objection to Play Integrity narrower and, we think, harder to route around: the wrong is not that someone other than the owner has an opinion about a device, it is that a hardware vendor's opinion about a machine is allowed to stand in for a judgment about a person. A household, a bank and a neighbour may all legitimately decline to rely on a device. None of them may make a machine verdict the condition of a person's participation.

---

## Lineage and org guard (read first)

- **Volla and Holochain.** The Holochain Foundation announced a partnership with Volla Systeme GmbH on 2024-07-22, and the Volla Quintus shipped with a Holochain app (Relay, a chat app built by Holo) pre-installed ◐ ([Holochain blog](https://blog.holochain.org/volla-partnership-announcement/), [Holo press](https://press.holo.host/243118-shifting-gears-to-delivery-volla-phone-apps-powered-by-holochain), via search summaries). Our tree's only prior mention of Volla is Volla Messages as an example of `clone_limit: 100` (`.claude/skills/holochain-hdk-0-7/references/cell-cloning.md:86`) ◐. So the vendor shipping agent-centric apps is the same vendor proposing a certifying backend. That tension is theirs, and it is the reason this survey is worth our time.
- **Who is in it.** Volla initiated; a first working meeting of "several European makers of mobile operating systems and devices" took place in February 2026 ◐ ([Netzwoche, 2026-03-09](https://www.netzwoche.ch/news/2026-03-09/konsortium-entwickelt-open-source-alternative-zu-google-play-integrity)). GrapheneOS names Volla, Murena and iodé ◐ (critic's account). The site names no partners, no foundation and no governance ◐.
- **What was announced versus what exists.** The press described three parts: an OS service, "a decentralized validation service", and an open test suite for certifying an OS on a device model ◐. The code has the OS service and one backend. There is no test suite in the organization's nine repositories ✅ (`gh api orgs/unifiedAttestation/repos`, 2026-11-18), and nothing decentralized in the validation path (see Mechanics).
- **Activity.** Backend last pushed 2026-09-30, Android service 2026-09-03 ✅. Apache-2.0 throughout, except `vbmeta-build-submit`, which declares no licence ✅.
- **Prior coverage in this index:** none. No mention of Play Integrity, SafetyNet, Widevine, key attestation or uattest anywhere in the tree ◐ (repo survey). TPM and secure-enclave appear only as deferred design notes (`genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md:172`; `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md:282`) ◐.

---

## The category, stated without a vendor

Every remote-attestation system has the same four roles (the IETF RATS architecture, RFC 9334, names them ⚠):

| Role | Play Integrity | Unified Attestation | What it holds |
|---|---|---|---|
| **Attester** | the phone's secure hardware | the same hardware (KeyMint) | a key the owner cannot extract, which signs a description of the boot chain and OS |
| **Endorser** | Google's root and key provisioning | UA backend root, OEM intermediate | the certificate chain that says the hardware key is genuine |
| **Verifier** | Google's servers | the UA backend | the policy: which boot keys, builds and patch levels count as acceptable |
| **Relying party** | the bank's server | the bank's server | the consequence: serve or refuse |

Three properties follow from the roles, not from any vendor's intent:

1. **The owner is not a party.** The evidence is about the owner's device, signed by a key the owner cannot use, judged by a policy the owner did not write, for a relying party who can then refuse the owner. This is lumi's "Security for who?", and as a description of the structure it is correct.
2. **It attests state at boot, not behaviour.** A locked bootloader and a known build say nothing about what the software does or whether it is patched against anything in particular. GrapheneOS's complaint that the certifiers "will sign off on their products regardless of the level of insecurity" is a complaint about the verifier's policy, and no hardware fixes policy.
3. **The verifier is where the power is.** Hardware and API are commodity. Whoever maintains the list of acceptable boot keys decides which operating systems may run a banking app. DRM (Widevine L1), anti-cheat and Apple App Attest are the same four roles with a different relying party ⚠.

The hardware is neutral about who verifies. GrapheneOS's own Auditor app uses the same Android key attestation with the *owner* as verifier: one of your devices checks another ⚠. Same primitive, opposite direction. That observation is the hinge of this note.

---

## What Unified Attestation is, mechanically

| Mechanism | How it works | Grade, cite |
|---|---|---|
| **Request binding** | App and app server each compute `requestHash = SHA-256(canonicalRequestString)`. The hardware attestation's challenge is set to that hash, so the evidence is bound to one request. No nonce round-trip, no request id | ✅ UAW `Protocol.md` |
| **Device side** | A privileged system service (signature or privileged permission to bind) generates a per-`(package, backend)` EC P-256 key in KeyMint, obtains the certificate chain, and posts it to the backend. Apps never see the chain. Default backends come from an OEM-provided `/product/etc/unifiedattestation.xml`; a settings screen lets the user add, enable or disable backends | ✅ UAS `README.md`; `ConfigReader.java:16` |
| **Verification** | Backend checks the chain strictly, requires it to end at a root belonging to a registered *authority*, checks challenge equals `requestHash`, enforces `projectId == packageName` from the attestation, checks revocation, then matches verified-boot key, optional boot hash, OS version and patch level against an OEM-registered **build policy** | ✅ UAW `Attestation-Validation.md`; UAB `apps/backend/src/routes/device.ts:147-420` |
| **Verdict** | `{isTrusted, reasonCodes}` with codes such as `DEVICE_UNLOCKED`, `BOOT_STATE_UNVERIFIED`, `BUILD_POLICY_MISMATCH`, inside a backend-signed token, default lifetime 60 s | ✅ `device.ts:398-419`; UAW `Backend.md` |
| **No device id** | The protocol carries no device or request identifier. A `scopedDeviceId = SHA-256(backendId ‖ projectId ‖ SPKI)` is computed for the backend's own logs: a per-app, per-backend pseudonym | ✅ UAW `Home.md`; `device.ts:379` |
| **Own PKI** | UA does not rely on Google's attestation root. The backend holds an RSA and an ECDSA root; an OEM's trust anchor is an intermediate signed by that root; a device keybox is leaf → OEM intermediate → backend root. External authorities can be registered by base URL, with their roots and revocation status fetched and cached for an hour | ✅ UAW `OEM-Trust-Anchors-and-Keybox.md`, `Root-Cert-Generation.md`; UAB `services/attestationAuthorities.ts:3, 60-100` |
| **Build enrolment** | An OEM submits build identities from its build server: verified-boot key hash, vbmeta digest, OS version, patch level, fingerprint | ✅ VBS `README.md` |
| **"Federation"** | A table of other backends' ids and public keys. Only a user with role `admin` on this backend can add, disable or delete an entry. Tokens from a listed backend are verified offline against the stored keys | ✅ UAB `routes/federation.ts:20-24, 67-71, 119-123`; UAW `Federation.md` |
| **Public registry** | Unauthenticated read API listing enabled device families, their certificate serials, and their accepted builds (boot key, boot hash, OS version, minimum patch level) | ✅ UAW `Public-Device-Registry.md` |
| **Client-side verifier** | An npm package that lets an app server verify the raw chain against the public registry, or verify a token against published keys, without calling the backend's verdict endpoints. The wiki is candid: "Neither function removes trust in UA … What changes is *when*" | ✅ UAW `Client-Side-Verification.md` |

Three findings from the source that the site does not state:

- **The backend generates and stores OEM private keys.** OEM anchor generation creates the RSA and ECDSA intermediates server-side, stores the private key PEMs in the database (`OemOrg.rsaRootKeyPem`, `ecdsaRootKeyPem`), and returns an XML containing certificates and private keys; device keybox generation likewise produces leaf private keys on the backend ✅ (UAB `prisma/schema.prisma:75-78`; `routes/oem.ts:138-170`; `services/keybox.ts:103-111`). The party that judges attestations also holds the keys that produce them. A hardware root's entire value is that the endorsing key was provisioned where nobody could copy it.
- **The policy is whatever the OEM enrols.** The verdict's substance is "this boot key and build were registered by an OEM account on this backend". No code or document describes a review of what an enrolled build must meet ✅ (absence, searched in UAB and UAW). GrapheneOS's conflict-of-interest charge is structurally accurate: vendors enrol their own builds.
- **The project moved toward its critic.** The public registry and the client-side verifier post-date the March thread and are exactly the direction GrapheneOS described: publish the roots, revocations and acceptable fingerprints as data and let relying parties verify locally. What remains centralized is who curates the registry ◐ (ordering inferred from push dates, not from commit history: the clones are shallow).

---

## The three positions

**Volla.** "If trust verification itself is controlled by a single market actor, structural dependency results" (Jörg Wurzer ◐). Banks will demand attestation regardless; without a second verifier, every de-Googled Android is locked out of banking and identity apps. This is true, and it is a statement about relying parties' demands, not about what is right.

**GrapheneOS.** Accepts attestation, rejects the central verdict service and the self-certifying membership: "One or more neutral organizations could exist certifying devices and operating systems without providing a centralized API. Those organizations could simply provide signed releases with the roots of trust, revoked keys and operating system key fingerprints. Apps could use multiple different certifying organizations" ✅ (post of 2026-03-16, fetched from the instance API). GrapheneOS also says UA disrupts its regulatory case against Play Integrity ✅ (same thread; their claim, unassessed here).

**lumi.** Rejects the category: "There should not be any external authority able to police what I do on my own devices." Concedes that the technology "might have some use cases" and gives none; offers no technical alternative ✅ (post fetched directly; undated).

Where each is right:

- lumi is right about **direction**. Changing the verifier from Google to a consortium does not change who can refuse whom.
- GrapheneOS is right about **shape**. A verdict API makes one party the judge of every request; signed data verified locally makes the relying party the judge under a policy it chose, with as many certifiers as it likes.
- Volla is right about **the bridge**. A person who needs a banking app today is not helped by a correct argument.

Where each falls short, from where we stand:

- lumi's position leaves no account of the cases where a device's state is legitimately someone else's business because *they are entrusting something to it*: a household deciding which of its machines holds the family archive key, or a requester deciding where to place a job over data that is not theirs to expose.
- GrapheneOS's plural certifiers are still certifiers of machines. Plurality fixes capture and leaves the category in place.
- Volla's bridge has no exit. Nothing in UA makes the relying party's demand weaker over time.

---

## Elohim: is it the same thing?

No. The repo uses "attest" in at least ten places, and none of them is a claim about what software a machine is running. The common shape is: **a principal signs a claim about a content-addressed subject; a reader weighs it under the reader's own policy; independence is counted in principals with standing.**

| Ours | What it claims | Signed by | Grade, cite |
|---|---|---|---|
| DHT attestation (24 `attestation:*` kinds) | "agent X asserts kind K about subject CID S at reach R", with a proof class of witness, audit, proof or confirmation | Holochain agent key | ◐ `elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs:15-28` |
| brit build / validation notes | "agent K built output CID O from closure H"; "check C passed on artifact A" | device or repo Ed25519 key | ◐ `elohim/brit/brit-epr/src/elohim/attestation/build.rs:5-29` |
| Gate evidence | a gate verdict keyed on its input closure; a sibling device's verdict is labelled `independence: sibling` and never counted in a k-of-n | this workspace, or an admitted sibling | ◐ `genesis/orchestrator/gate-evidence.mjs:491-551`; `seam-registry.yaml:73-76` |
| Derivation record | "this output came from this method and these inputs, on this basis". "The record is evidence, never authority" | the deriving device | ◐ `elohim/eprfs/eprfs-core/src/derive.rs:1-11` |
| Device key and roster | one key per OS user per device; a two-signature row binds a device to a human | both devices | ✅ `elohim/eprfs/epr-cli/src/device_key.rs:12-18` |
| Compute receipt | "provider P ran task CID T under grant G and observed these tests" | provider's agent key | ✅ `genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md:552-568` |
| Humanity witness | "I, a present agent, witness this human" | the witnessing agent | ◐ `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:641-653` |

Set beside the four roles:

| | Platform attestation | Elohim attestation |
|---|---|---|
| Subject | a machine's boot state | an artifact, an act, a person's standing |
| Attester | hardware the owner cannot speak for | a principal who can be asked, contested and held to account |
| Root of trust | a manufacturer's key | standing, witnessed by a community |
| What makes it checkable | the certificate chain | the CID: fetch the bytes and hash them; re-run the derivation |
| Counting | one chain suffices | principals with standing; "device, hardware, toolchain lineage and infrastructure are declared dimensions … never counted" |
| Who decides | the verifier's policy, for everyone | each reader's `TrustPolicy`, for itself |
| Failure it prevents | a modified client talking to a server | a claim travelling further than the evidence for it |

Three of our own sentences already say what lumi and GrapheneOS are reaching for:

- "A device key makes a claim portable and self-verifying. It never makes it a credential" ✅ (`device_key.rs:12`).
- The delegation substrate "makes the claim attributable, immutable, bounded and revocable; it does not make it true" ✅ (peer-executed-stage design, `:563-564`).
- Ruling D4: "independence counts principals with standing, never keys or devices" ✅ (`genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:330-340`).

And one of our own checks is, structurally, owner-direction attestation already: `just status device` compares the running `epr` executable's CID with the device's bin-store head and refuses on mismatch ◐ (`elohim/eprfs/epr-cli/src/device/arrival.rs:600-650`). The device reports its software state to its own human. That is the GrapheneOS Auditor direction, done with content addressing instead of a hardware key.

**A vocabulary ruling this suggests.** Keep *attestation* for testimony by a principal. When we mean machine-state evidence, say **platform evidence**, and treat it as one more declared dimension. The words matter because a reader arriving from the Android world will hear "device-health attestation" and "attestationCapabilities" (`genesis/data/devices/*.md:31` ◐) as Play Integrity.

---

## Where the machine question honestly arises: remote execution

Play Integrity answers "can this server trust that client?" Peer compute raises the mirror pair:

1. **Provider's question: should I run this stranger's code?** Here our design is the *inverse* of attestation, and that is the right way round. The owner of the device constrains the code, through an explicit consent grant with a closed scope list, a reach ceiling, a rate and a rotation ◐ (`elohim/elohim-storage/src/api/compute_grants.rs:46`), a content-addressed task whose `binary` CID is the definition of what runs, and the device's own envelope and guards (ark is the only executor, ruling D8 ◐). In DRM the publisher's code constrains the owner's machine. In our compute plane the owner's machine constrains the visitor's code. Nothing to borrow from UA here.
2. **Requester's question: did you run what I asked, and did you keep what I sent private?** This is the question platform attestation exists for, and our specs are honest that we do not answer it by trust: "What it does not check: that the provider executed anything … both storage and the zome will accept" a forged `passed` receipt ✅ (peer-executed-stage design, `:560-563`). The answer on integrity is the graduated-rigor ladder: witness (the provider signs), audit (anyone re-executes), proof, confirmation ◐ (`genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md:20-75`), with the verdict read from the evidence the run leaves, never the receipt's status. Re-execution on a second principal's hardware is strictly stronger than a hardware quote for *integrity*, because it does not depend on any manufacturer.

What re-execution cannot give is **confidentiality from the provider**: a requester with data the provider must not read. A trusted execution environment's whole offer is that case. Our position should be explicit rather than accidental:

- The first answer is **reach, not hardware**: confidential work is placed on devices whose humans already hold reach to the data (own device, household, R0 to R2 of the compute ladder ◐). Trust that already exists is cheaper and more honest than trust manufactured by a chip.
- Ark already refuses to *assume* such features: SGX and 32 other host features it "cannot observe soundly" refuse as `unobserved` ✅ (`genesis/docs/superpowers/sprints/2026-11-11-host-compatibility/journal.md:224-225`). That is the correct default: an enclave nobody verified is a string in a manifest.
- If a provider ever *offers* platform evidence, it is a `Provides` clause the provider chose to publish and the requester's `Requires` may ask for. It affects **placement** only. It never enters D4's count, never substitutes for a second principal, and its absence costs a provider nothing except that one job.

---

## Verdicts

### TAKE

1. **The direction test.** For any evidence about a device's state, ask three questions: *who verifies*, *who chose the policy*, and *what can the relying party do to the device's human on a negative answer*. Admissible: the owner or household verifies their own devices; a provider volunteers evidence to win a placement. Refused: any design where a machine verdict gates a person's identity, standing, reach, or access to their own data. This is lumi's argument kept as a rule, and it is the piece of this survey that is not already written down in our tree. **Minted:** row 54 of [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog).
2. **Signed data, verified locally, plural sources; never a verdict API.** GrapheneOS's correction to UA, and UA's own later drift toward it. Ours already: `TrustPolicy` is reader-side and a derivation record is evidence, not authority. The take is a guard on what we build next: release channels, the artifact store's "acceptable" sets and any future compatibility registry publish *signed records and the policy that reads them*, and no component answers "is this trusted?" on another's behalf. Carried in row 54; no separate row.
3. **Vocabulary: *attestation* is a principal's testimony; machine state is *platform evidence*.** Costs nothing now and prevents a category error with every reader who knows Android. Carried in row 54.

### STUDY

4. **Scoped pseudonyms for outward-facing device evidence.** UA's `scopedDeviceId = H(backend ‖ project ‖ key)` gives each relying party an unlinkable handle. Our device key is one stable `did:key` that signs every claim the device makes, so anything a device signs outward is linkable across contexts by construction. Right for attribution inside a household; wrong default for evidence handed to a stranger. Belongs with the confidentiality plane and the continuity-witness design, when either reaches the point of emitting anything outward. Not minted: no consumer yet.
5. **Request-bound freshness for step-up evidence.** UA binds evidence to `SHA-256(canonical request)` and expires it in 60 seconds, which removes a nonce round-trip and a replay window together. Our compute receipts already bind task, request and grant ✅. The unbuilt continuity witness and any device-health evidence used for a step-up decision should bind to the act they authorize in the same way. Not minted: the continuity witness has no spec.
6. **Platform evidence as a declared `Provides` dimension.** Only if a real confidential-compute need appears that reach cannot serve. The shape is fixed above (placement only, never counted). Not minted.

### LEAVE

7. **A certifying backend, of any membership.** One verifier, an admin-curated federation list, OEMs enrolling their own builds. Plurality by invitation is not plurality.
8. **Server-generated endorsement keys** ✅. The judge holding the witnesses' keys.
9. **Hardware as a root of standing.** Contradicts D4 and the constitutional floor. A manufacturer's key says a chip is genuine; it cannot say anything about a person.
10. **The bridge without an exit.** UA makes it easier for relying parties to keep demanding machine verdicts. When we meet a legacy relying party that demands one, that is bridge-seam work scoped to that party, and it must not leak a machine verdict into the substrate.

### WATCH

11. **Whether UA finishes moving** to published signed data with plural, non-vendor certifiers, and whether the announced test suite appears. If it does, it becomes a decent external example of take 2.
12. **Regulators and Play Integrity** ◐ (GrapheneOS reports ongoing conversations), and whether any European identity-wallet rule makes platform attestation mandatory for citizens ⚠. That would turn lumi's worry about desktops into a present concern for anything we ship to a phone.
13. **Volla as a Holochain shipper.** If Relay-class apps on Volla devices ever sit behind UA, the two halves of that vendor's stack will have met, and what they decide will be informative.

---

## What matters more than any of this

The survey's most practical finding is about us. A hardware root would be a costume over integrity debts our own backlog already names, each of which lets a claim travel further than its evidence today:

- DNA issuer authorization accepts every issuer: floor 2 is a TODO ✅ (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs:67-70`).
- Replicated attestations are re-authored under the local key, losing the issuer ◐ ([security-attestation-issuer-relaundering](epr:security-attestation-issuer-relaundering)).
- Release promotion counts agent strings, not principals ◐ (`elohim/elohim-storage/src/services/release_attestation.rs:859-861`).
- `steward/node`'s updater accepts unsigned downloads ✅ ([security-steward-node-updater-accepts-unsigned](epr:security-steward-node-updater-accepts-unsigned)).

D5 already gates graduation on the first three. This note adds no rows for them and changes no priority; it records that the comparison with Play Integrity is flattering to our design and unflattering to our enforcement, and that the second is where the work is.

---

## Open questions / frontier

- **Does a household want owner-direction platform evidence?** A parent checking that a child's tablet runs the build the household chose is the Auditor case. It passes the direction test only while the household, not a vendor, holds the policy, and only as care within a relationship, not as a gate. Unexplored; belongs to the device-recognition work if anywhere.
- **What do we say to the bank?** A person on the network will still need an app that demands Play Integrity. **Operator's proposed posture (Matthew, 2026-11-18): a mud-room.** A bridge crate keeps a quarantine at the edge of the network where the banking app lives. It is legible to the network, and it is the one place where we reckon with Play Integrity: when value enters the network, it enters there. This passes the direction test (the bank can refuse the bank's service; the verdict reaches nobody's standing, reach or data) and matches the monetary-posture note's selective legibility: legible where the concern is legitimate, uncapturable where it is acquisitive. Three conditions, the author's, not yet ruled:
  - **The verdict stays in the mud-room.** What crosses the threshold is the economic event, as the person's own witnessed claim. It never arrives as "attested by Google"; a machine verdict must not raise the trust of anything inside.
  - **The mud-room is partly physical.** A bridge crate cannot itself satisfy Play Integrity; the app still needs a Google-certified device or profile, which the person chooses to keep. This contains lumi's complaint and does not solve it.
  - **Legibility likely comes from beside the app, not through it** ⚠. Banking apps expose nothing to other apps; the bridge would read an open-banking API or a statement export, or take the person's entry. Whether those paths ever demand platform attestation is unchecked.
  
  Nothing minted: there is no bridge crate to build against yet.
- **Is there any honest use of a TEE that reach cannot serve?** The candidate is computation over data pooled from people with no relationship to each other. The commons data-pools note met SGX there once (`genesis/research/commons-data-pools-hot-path-and-external-tooling-2026-09-11.md:213` ◐). Unresolved.

---

## Method + credit

One author (Claude Fable 5.2), one session, 2026-11-18, operator-directed by Matthew. External side: uattest.net and Netzwoche read through a summariser; the lumi post, the GrapheneOS thread and Volla's announcement fetched as text directly; five UA repositories and the wiki cloned shallow and read in source at the commits above (wiki pages for protocol, validation, federation, trust anchors, registry and client-side verification; backend federation, authority, root-anchor, keybox and device-verdict code by targeted read, not line by line). Internal side: one read-only repository survey by an Explore subagent, whose citations are graded ◐ here unless re-read; the six load-bearing ones were re-read and are graded ✅.

**Not done:** no challenge pass and no blind-reader pass; the backend's `oem.ts` (1,759 lines) was read only around key generation; UA commit history was not examined (shallow clones); RATS, Widevine, App Attest, Auditor and Web Environment Integrity claims rest on prior knowledge and are graded ⚠; Korben's article returned 403 and was not read. Credit to lumi and to GrapheneOS, whose arguments this note mostly rearranges.

## Outputs

- **Minted:** [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) row 54 (takes 1 to 3).
- **Cited, not re-minted:** [security-attestation-issuer-relaundering](epr:security-attestation-issuer-relaundering), [security-steward-node-updater-accepts-unsigned](epr:security-steward-node-updater-accepts-unsigned), [arch-authority-in-integrity-backlog](epr:arch-authority-in-integrity-backlog).
- **Manifest clones added:** `uattest-backend`, `uattest-android-service`, `uattest-android-sdk`, `uattest-vbmeta-build-submit`, `uattest-website` (the wiki is cloned beside them as `uattest-wiki`, outside the manifest).
- **Index:** Research Index, *The Machine-Witness Problem — Unified Attestation and remote attestation*.
