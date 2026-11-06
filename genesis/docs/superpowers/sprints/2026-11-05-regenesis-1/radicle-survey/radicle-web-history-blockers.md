# Radicle / Heartwood prior-art survey (2026-11-05)

Grades: [P] primary source (Radicle's own site/blog/forum), [S] single secondary source, [?] unverified/hearsay.
Method note: search + page-fetch summaries only (a small model summarised each fetched page); no raw page reading. Quotes are paraphrases.

## 1. Lineage and abandoned designs
- Radicle alpha (Feb 2019) built on IPFS; Radicle Link started Oct 2019, modeled on Secure Scuttlebutt, Git for replication, IPFS dropped; Upstream desktop client Dec 2020, sunset Jul 2022 in favour of the web client; Heartwood announced Jul 2022 as third protocol generation; 1.0.0-rc.1 Mar 2024, 1.0.0 Sep 2024, 1.1.0 Dec 2024, 1.2.0 Jun 2025. [P] https://radicle.dev/history
- Why IPFS was dropped: IPFS is a content-addressed store, repos are mutable; also performance/functionality issues. [S, quoted via a fork's docs] https://git.hswro.org/mab122/radicle-heartwood-lfs/src/commit/5d53fab6735fb5ce296d9c8dd5fddc1cc2f3779e/LFS-IPFS.md ; Radicle FAQ phrasing https://radicle.dev/faq
- Why Link was abandoned: NOT FOUND. The history page gives no reason (only a linked GitHub issue, not retrieved). Nov 2022 clients update marks radicle-link/cli/client-services/keystore deprecated, heartwood CLI+node as replacement. [P] https://community.radworks.org/t/clients-community-update-november-2022/3141 ; Aug 2022 update: little Link development. [P] https://community.radworks.org/t/link-august-2022-community-update/3020 . Heartwood "addresses usability and performance concerns of previous iterations" (release framing) [S].
- Oscoin: NOT FOUND (history page does not mention it).
- Link explainer by cloudhead: https://radworks.mirror.xyz/8xAEmNhGSwQDZ-IKMzFEklPzCVXAm751vaEcSKc5xkw [P, not read in full]. Heartwood design doc is a RIP by cloudhead on HackMD: https://hackmd.io/@cloudhead [P, listed in search only].
- No maintainer retrospective ("lessons learned") located. Do not infer one.

## 2. Funding and governance (short)
- Foundation funded core development until May 2023, then split into Radicle (code collab) and Drips (streaming funding); Foundation renamed Radworks; Radicle/Drips/Grants became DAO-funded orgs. [P] https://docs.radworks.org/community/our-story
- Q1 2025: "We are within budget" (no figures); cloudhead stepping down as Org Lead, protocol dev moving under Foundation org/council by 30 Jun 2025. [P] https://community.radworks.org/t/radicle-org-q1-2025-update/3636
- Ethereum-era org/registry feature removal: NOT FOUND in sources. Sustainability read: single-org, DAO-grant dependent, leadership transition in 2025; that is inference.

## 3. Heartwood protocol design (all [P]: https://radicle.dev/guides/protocol)
- Storage: one bare Git repo per Repository ID (RID); every peer's data in the same repo under a Git namespace keyed by Node ID (shared object DB, each object once). Only owner writes its namespace. Rationale: soft-forks, no central host, Git compatibility.
- Sigrefs: on any ref change the node signs the full ref set; blob at refs/rad/sigrefs. Extends verification from commits to all refs incl. COB refs.
- Identity doc: canonical JSON at refs/rad/id; name, description, default branch, delegate DIDs, threshold, visibility. RID = hash of initial doc, stable thereafter. Self-certifying, TUF-inspired.
- Delegates / canonical refs: canonical default branch = commit that threshold-many delegates agree on (e.g. 2-of-3). Only default branch for now.
- Gossip: node announcements, inventory announcements, reference announcements; signed, timestamped, deduped, replayed to rejoining nodes. Gossip is metadata only.
- Fetch: content moves over Git fetch, multiplexed on the same connection as gossip (helps NAT).
- Seeding: per-node seeding policy; public seeds seed everything, community seeds scope to trusted peers.
- COBs: issues/patches/identity as signed commit graphs under refs/cobs; custom types by reverse-domain name.
- No DHT by design: DHTs give nodes little say over what they serve and add latency. [P] https://radicle.dev/faq
- RIPs: the only RIP numbers found: Heartwood itself is a cloudhead RIP (HackMD, number not seen); RIP 2 = storage layout (draft in Nov 2022). [P] https://community.radworks.org/t/clients-community-update-november-2022/3141 . RIP numbers for identity, COBs, canonical refs, and any artifact/release proposal: NOT FOUND (no rips repo located; search for it failed).

## 4. Blockers and criticisms
- Availability: data stays available "as long as at least one seeding node is online"; seed nodes are always-on public nodes that provide availability. [P] https://radicle.dev/faq . Centralization-in-practice critique: LWN commenters say it relies on third-party public seeds [S] https://lwn.net/headlines/966869 (not read directly; reported via search summary).
- NAT: relies on seed nodes to serve NATed peers; hole punching "under development" [P faq]. Iroh adoption will add NAT traversal [P] https://radicle.dev/2026/09/23/disclosure-of-vulnerability-in-network-protocol
- Private repos: visibility=private + DID allow list restricts replication; no encryption at rest. [P guide]. Disclosure 2026-09-23: node transport discards Noise session keys so all traffic (gossip, packs) is cleartext in ALL released versions; second flaw lets a node present another's Node ID to fetch private repos. Advice: treat private repos sent over network as disclosed; `rad block <RID>`; Tor/I2P/VPN not a fix. Signed refs still detect tampering. Fix = replace custom Noise transport with iroh in a breaking major release (network partition vs 1.x, storage layout kept compatible), no date. [P] same URL; corroboration [S] https://www.infoq.com/news/2026/09/radicle-network-vulnerabilities/ (InfoQ says guidance "halt clearnet private operations").
- Gossip/scaling: no source found with numbers. NOT FOUND. Q1 2025 notes work on cloning repos >1GB. [P]
- Discovery/search: "basic search exists, work in progress". [P faq]
- Large files / LFS / releases: FAQ says Git handles moderate binaries; for large data use git-annex or git-lfs. [P faq]. No official LFS or release-artifact feature or proposal found. Third-party prototype: heartwood fork with Git LFS where bytes live on each contributor's IPFS (Kubo) node, pointer replicated through Radicle, CID mapping in git notes at refs/notes/rad-lfs, `rad lfs rekey` needed after granting private-repo access; seeding still pulls full large files. [S, prototype] https://git.hswro.org/mab122/radicle-heartwood-lfs/raw/branch/rad-lfs-ipfs/README.md . Radicle's own release binaries are distributed via a curl installer + tarballs with SHA-256 (not via Radicle). [P] https://radicle.dev/2026/08/05/radicle-1.10.0
- CI: CI broker + adapters (native CI runs broker/node/adapter on same host); each run recorded as a "job" COB (type xyz.radworks.job). [P/S] https://radicle-ci.liw.fi/radicle-ci-broker/overview.html , https://radicle-ci.liw.fi/architecture.html . Trust model for job COBs (who may publish, how verified): NOT FOUND. Operator-run, third-party (liw.fi) docs, not radicle.dev.
- Identity/keys: each device needs its own identity; multi-device being worked on. [P faq]. Key recovery: NOT FOUND. Q1 2025 roadmap chose Identity Management as a focus. [P]. 1.10.0 reworked identity evaluation (revisions with parent/child, 4 states) because ambiguous majorities were possible; known remaining inconsistency under concurrent redact/accept; ~15 of several thousand scanned repos changed. [P] https://radicle.dev/2026/08/05/radicle-1.10.0
- Adoption: NOT FOUND (only: Zulip +80 users in a quarter, 6 non-core contributors, 57 commits since 1.1.0). [P Q1 2025 update]. LWN commenter said "1.0" felt like 0.3 [S, via search summary] https://lwn.net/Articles/989605/

## 5. Comparisons
- Only one found: 2024 Nostr post by a NIP-34 developer: Radicle = ed25519 + custom gossip, seed nodes act like relays, complete v1 with IDE/web UI, but complex, custom protocol outside a wider ecosystem, large maintenance burden; GRASP keeps client-server git but moves auth/trust to Nostr, multiple servers for redundancy. Partisan [S]. Via https://njump.me/ (long nevent URLs in search) and https://wiki.archlinux.org/title/Radicle (web frontend read-only).
- ForgeFed, git-ssb, Pijul/IPFS forge comparisons: NOT FOUND. Data-vs-state separation commentary: NOT FOUND (Radicle itself: git objects are content-addressed and verified, state lives in signed refs, i.e. the separation is built in; inference from guide).

## 6. Recent news
- 1.9.0 (2026-05-19) https://radicle.dev/2026/05/19/radicle-1.9.0 (not read). 1.10.0 "Sunflower" (2026-08-05): identity rework, `rad id cache`, patch labels, configurable fetch timeout (30s default; raise for Tor/I2P); dev moves to master for Radicle 2.0.0, 1.x backports on releases/1. [P]
- Sep 2026 transport vulnerability disclosure and iroh plan (above).
- Radicle Desktop, attachments/releases/mirrors features: NOT FOUND in what was retrieved (1.10 only links a mirroring guide).

## Top 8 lessons for artifact storage on a p2p blob plane
1. Radicle punted on large blobs: FAQ says use git-annex/git-lfs. Blob custody is unsolved there; the only attempt is a third-party IPFS fork. (https://radicle.dev/faq)
2. Availability = whoever chooses to seed; in practice always-on seeds. Plan an explicit custody/pinning policy and a declared floor of holders. (https://radicle.dev/faq)
3. Keep state (signed refs/identity/threshold) separate from data (content-addressed objects): this let Radicle's tampering guarantees survive its own transport failure. (https://radicle.dev/2026/09/23/disclosure-of-vulnerability-in-network-protocol)
4. "Private by replication allow-list" without transport encryption and peer auth is not private; Node-ID-as-credential let an attacker fetch private repos. Auth the channel, and encrypt at rest/in transit if private artifacts are in scope. (same)
5. A hand-rolled transport with no version negotiation forced a hard network partition; build wire-version negotiation in from day one. (same; InfoQ)
6. Threshold-signed canonical state (k-of-n delegates agreeing on a head) is a good release-attestation shape, but identity evaluation had ambiguous-majority bugs fixed only in 1.10. (https://radicle.dev/2026/08/05/radicle-1.10.0)
7. CI results as signed COB job objects exist, but trust of the recorder is undocumented; define who may attest a build and bind it to the artifact digest. (https://radicle-ci.liw.fi/radicle-ci-broker/overview.html)
8. Sustainability/governance risk: one org, DAO grants, lead stepping down, no adoption metrics published; don't depend on upstream roadmap (multi-device, hole punching were "in development" for years). (https://community.radworks.org/t/radicle-org-q1-2025-update/3636)
