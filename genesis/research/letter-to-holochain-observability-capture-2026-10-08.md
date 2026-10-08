---
title: "A Letter to Holo and Holochain Core: On the Prometheus Question, and What Is Underneath It"
id: letter-to-holochain-observability-capture-2026-10-08
status: Capture
date: 2026-10-08
sovereignty-frame: bridge-legibility
---

# A letter to Holo and Holochain core: on the Prometheus question, and what is underneath it

*Written in reply to a one-paragraph email, because the honest answer is longer than the question.*

Dear Rob, and whoever at core reads this after you,

You asked why we chose Prometheus and whether it makes sense for Holochain. The short answer is yes, with one condition that matters more than the exporter. The long answer is the rest of this letter, because the question turned out to be a thread, and pulling it reached the thing both of our projects have not solved yet.

## 1. The question, restated

What we did is small. The conductor already keeps a set of internal gauges through OpenTelemetry, the standard instrumentation library: how long zome calls take, how long a lookup across the network takes (the cascade), how busy the database write path is, how far validation is behind, uptime, integrated ops. Out of the box, the only way to read them is InfluxDB, a time-series database: a file in its format for a collector called Telegraf, a copy of InfluxDB the conductor downloads and launches itself, or an InfluxDB you already run. We added a fourth option, a switch (`HOLOCHAIN_PROMETHEUS_LISTEN=<addr:port>`) that serves those same gauges on `GET /metrics` in the format Prometheus scrapes. When the switch is off, nothing binds. About five hundred lines, one crate, no new instruments.

We did it because without it we were blind inside the conductor on a fleet that already had Prometheus watching everything else. With it, we found and fixed real slowdowns within days, with evidence: a permission check that re-read a participant's whole history on every call, so the app got slower the longer someone had used it; a validation loop (sys-validation) that asked for a record nobody held every ten seconds, forever; a publish loop that re-sent every record between sweeps. Each of those is on our fork with a measurement attached.

## 2. Our read on how Holochain observes itself today (an invitation to correct us)

Everything here is read from the conductor tree at our pin and from the public Wind Tunnel repository on 2026-10-08. Our fork's only remote is our own, so anything core landed after the 2026-08-03 release is invisible to us. Please correct what is stale.

**The metrics crate serves Wind Tunnel.** `holochain_metrics` describes itself as "integration of opentelemetry metrics and InfluxDB". As far as the public tree shows, its changelog records no user-visible change from 0.6.0-dev.1 through 0.7.0, and the commits touching it since April are housekeeping. Wind Tunnel's seventeen dashboard templates are InfluxDB JSON, four of them reading the conductor's own instruments (`holochain_conductor`, `holochain_database`, `holochain_p2p`, `host`). The shape of the exporter is the shape a load-test bench wants: self-contained, random-port UI, no scraper assumed.

**Wind Tunnel is a real cluster, briefly.** A Nomad cluster of your own, results to object storage, and, per the public workflows, an optional ThreeFold node pool excluded by default. The canonical `dht_sync_lag` run is thirty nodes for thirty minutes, twenty-five writers and five observers measuring lag; the scaled variant declares 150 nodes. Weekly by the published schedule, then gone. Each node is one conductor and one scenario binary; agents are fixtures.

**The operator's surface is the admin API.** `DumpState`, `DumpFullState`, `DumpConductorState`, `DumpNetworkMetrics`, `DumpNetworkStats`, plus tracing. Point-in-time, pull-by-hand, no history.

**Labels were never audited for exposure.** Zome-call duration carries `dna_hash`, `zome`, `fn` and, whenever a source chain is present, the caller's `agent` public key. Signal emission carries `cell_id`. On a bench those are fixture ids. On a hosting fleet they are participants.

Our theory of the gap, in the order we would bet on it: the instruments were built for a developer's laptop and a Thursday bench, not an operator's fleet; the Rust ecosystem discontinued the OpenTelemetry Prometheus exporter crate and the Influx path core already owned stayed; fleet telemetry was, we assume, left to the host side; and the data-model rewrites of the last year (v2 actions, the unified DHT store, source chains into the per-DNA database) rightly took priority. None of that is a criticism. It matches what the tooling was built for.

## 3. Why we are on Kubernetes, and what that lets us measure

We run two things. A household mesh: three conductors, three storage peers, two doorways (our web-facing gateways) and a relay on one machine, started and stopped by one command, where every protocol scenario runs first. And an alpha neighbourhood: seven long-lived pods on a cluster we operate, with real network paths, hosted accounts on the doorways (the hosting path is real; the people in it are still our test fixtures, because we are designing for hosted people before anyone real signs in), a dataplane above the conductor (our own storage peers on libp2p and iroh with SQLite, and doorways serving it to the web), and months of accumulated state.

Why Kubernetes is a personal history, so this paragraph is in the first person. I never had a HoloPort. I learned Kubernetes as a developer hoping that its whole model of scaling containers would solve the reliability problems of self-hosting, if I could only work out how it worked. Once I had, I saw that it is built for hyperscalers, not for self-hosters or for ordinary people behind a dynamic DNS address: it is superpowers of scale for a datacenter. When this project started, the edge-node release was the first point where I could begin from what I already knew, cloud-native as a homelab hobbyist and a corporate web-app developer by day, bring Holochain into my cloud-native workspace, and work from the other direction: use those datacenter superpowers to give peer-to-peer technology, Holochain included, the powers a hyperscaler has. I never seriously considered Wind Tunnel or the hc tooling, because Kubernetes solved that problem more or less out of the box; I only needed to make it legible to my own bench. The protocol's own design gate names the trap in so many words: modeling a protocol concern in the Kubernetes plane is an anti-pattern, because Kubernetes describes compute and hardware only. The cluster is our bench. It is not the network. The goal of the whole architecture is the inversion: bring hyperscaler superpowers to everyday peer-to-peer, and with them subsume and escape the internet's existing capture by the corporate datacenter.

What it lets us measure that a Thursday run cannot: what a conductor looks like after a month. Every serious defect we found this quarter lived there. A zome call whose fixed cost grew with the caller's chain length (a no-op went from 7 ms at about 120 actions to 469 ms at about 18,300). A sys-validation loop that spun on an unfetchable dependency. A publish-time write lock held across a sweep. Dead anchors that a reconcile sweep never counted as divergent. Heads re-declared with the same body that never reached peers already holding the row. None of those appear on a fresh network under synthetic load; all of them appear on a network that has been alive.

What Wind Tunnel measures that we cannot: scaling laws on a fresh network. How sync lag grows with 25 versus 127 writers. Reads from peers that hold none of the data versus all of it (zero-arc versus full-arc). Countersigning under load. We have seven pods; you have 150 for half an hour. Breadth is yours, duration is ours, and neither substitutes for the other.

The two artefacts are the same thing in different trees: your `summariser` uploads to object storage; our test reports land as dated evidence lines in a register of the things the system must reliably do, where nothing is marked green on intention, only on a run. We would be glad to read yours.

## 4. What we think core should take, and why

Take the exposition as a fourth backend beside the three Influx ones, behind the same environment-variable convention, with Prometheus taking precedence when both are set, or a documented rule either way. Reasons:

- Most operators run the conductor inside something that already scrapes. Pull needs no process, no download, no push target.
- The instruments stay OpenTelemetry. This is not Prometheus versus OpenTelemetry; it is one more reader of what core already pays to maintain.
- It is one crate change, opt-in, off by default, and it has been proven in one place for a week. "Proven everywhere" is not a claim we make.

**The condition that matters more:** audit the labels before any exposition ships. The `agent` and `cell_id` labels must not leave the process by default, and `dna_hash` deserves a decision, because it says which networks a node participates in. This is not our preference; it is the risk for every production user of Holochain today, with or without our exporter. Any operator who turns on the Influx child service, or points Telegraf at the line-protocol file, is already building a per-participant activity ledger they did not mean to build. A Holo host collecting port metrics holds one for every hosted participant. The exporter did not create that; it made it one scrape away. The fix is at the exposition: drop or hash those labels unless the scraper is the peer itself.

Wind Tunnel does not compete with any of this. The clean statement is: Wind Tunnel keeps Influx for the bench; operators get a scrape endpoint for the fleet; same instruments, two readers.

## 5. Upgrade and rollback: the defence of the DHT that has to be a solved problem

This is where the thread leaves metrics. The reason we could ship a conductor patch, measure it, and roll it back without losing a network is a set of rules we had to build, and we think they belong to Holochain's story more than to ours.

Some vocabulary first. A hApp's rules live in two kinds of code. Integrity code defines what is valid, and it is hashed into the network's identity (the DNA hash): change it and you have a different network. Coordinator code is the behaviour on top, and it can be replaced in place (hot-swap). So a conductor patch that changes only coordinator code does not move the DNA hash and heals by hot-swap (`update_coordinators`) with no re-key and no DHT churn. A change to integrity code moves the hash, and a reinstall mints a new agent key, which on a live network is a migration with lineage, never a wipe. We learned the hard way that forcing a reinstall on some peers and not others lands them on different DHTs and partitions the network, so the genesis pair must move together. We guard the packed DNA hashes in CI against a committed baseline and refuse a build that moves them without a tagged migration. The release itself is a manifest (artifacts by content id and sha256, a role binding naming the DNA hash, coordinator wasm hashes, what it migrates from, its lineage, a pointer to the governing rules, full build provenance, and an adoption discipline of soak time, attestation threshold and canary order), with the rule that a builder's own attestation never suffices to earn its release. Soak attestations ride an existing attestation kind so that adding them did not itself move the hash.

Why this matters to the DHT: integrity is the thing a DHT is for, and the only honest way to change integrity rules is a closed set of primitives that every peer can verify: what hash am I on, what did I migrate from, who attested the release, how long did it soak, in what order do peers adopt. Subjecting upgrade and rollback to those primitives is the right defence of the network. It is also not optional, because a network that cannot upgrade safely will eventually be upgraded unsafely. Today it is partly solved on our side, and we could not find a core-level equivalent beyond the DNA-hash and coordinator-update mechanics themselves; if one exists we would like to know. We would rather this were shared than forked.

## 6. The scaling story: sharding there, a dataplane here

The other thread we think you are pulling is sharding. Wind Tunnel's scenario names say so: zero-arc, mixed-arc, full-arc. Arc coverage is Holochain's answer to "not every peer holds everything", and our fleet runs with an arc factor set deliberately, because we hit the gossip wedge at roughly 120k ops where full-arc rounds stop completing.

Our answer is complementary, not competing. Above the conductor we run a dataplane the DHT was never meant to be: content bytes in a peer-owned blob store (SQLite behind libp2p and iroh), inventory exchanged as metadata so that notarisation and byte-availability are deliberately decoupled, data shards placed by a rule that reads each peer's declared posture, heads elected and carried with their content, and doorways that serve all of it to the web without owning any of it. The DHT stays small and stays the notary; the heavy bytes move on a plane built for them.

The commons question sits on top of both. We have a research line on commons data pools (shared indexes, shared models, shared compute on committed devices) whose design decision is that a pool is a collective holding a resource, with contributions as notarised commitments and draws admitted by membership and reach. Sharding decides who holds which bytes; the pool decides who may draw on them and under what standing. Holochain's arcs and our pools are two halves of one scaling story, and we think the halves should be told together.

## 7. Capture: the problem neither bench solved

Here is the finding that made this letter long.

Our exporter did not just expose the conductor to Prometheus. It exposed us, raw, to Kubernetes as a trusted consumer. Whoever holds the collector is a steward of what it captures, and on our fleet that steward was undeclared: the cluster's Prometheus held per-agent series for every hosted account, with no authority basis, no scope, no expiry, no review, and nobody a subject could appeal to. Those accounts are still test fixtures, so nobody was harmed; but the ledger was being built all the same, and it would not have changed shape on the day a real person signed in. A Holo host holds the same for every hosted participant on a port. We did not declare it, and we suspect no host has had reason to yet. A bench never has this problem because a bench has no subjects; Wind Tunnel's agents are fixtures, so there is nobody to be steward for. Our fleet runs the hosting path with fixture people in it, which is the moment to notice: before anyone real arrives.

The claim we are now building on, and would like tested by someone who runs a different fleet: **observability scales with relationship, and capture is a stewardship relationship, not a plumbing decision.** One fleet's label exposure is a small base for a principle; it is the base we have. Our identity layer already has the object for it: a stewardship grant, with a steward and a subject, an authority basis, evidence and a verifier, capability flags (one of them is literally activity monitoring), a mandatory expiry and review date, delegation depth, and an appeal. What it lacks is scope: which observation kinds, which labels, at which reach, retained how long. That is one field, not a new primitive.

With scope on the grant, the ladder writes itself:

- **Self.** A peer reads its own instruments. No grant, no cost, nothing leaves.
- **Declared.** A steward of capture holds a grant for a subject: a hub for its members, a gateway for the people it hosts, an operator for the fleet they run, a Holo host for the ports it serves. The grant names scope, reach and expiry; the subject sees it in their activity log and can appeal. Series carry only the labels the scope allows.
- **Earned.** Aggregation across households and regions happens only over attested outcomes (a health transition closed with a period, backed by a diversity of independent observers), never over raw rows.
- **Nothing.** An unscraped port. Silence reads as "unobserved", which is the honest answer.

How this graduates in our tree, each step useful alone: label hygiene at the exposition; a peer folds its own scrape into a self-observation on the observation log we already run (signed, append-only, one per observer, shared only at the declared reach); neighbours sign what they see of each other; transitions attest by diversity threshold instead of alert rules firing; each group's always-on node (a hub) holds its own pool of its members' observations, so there is no central server to become a dragnet; the wider network rolls up attested outcomes only; and finally the fleet's Prometheus becomes a reader of the operator's own group view rather than the thing peers push to. At that point Prometheus is a display at one edge, declared and scoped, and the network is what observes itself.

Wind Tunnel already knows the answer is observers. Its `dht_sync_lag` run splits the fleet 85/15 into writers and a minority whose only job is to watch what the writers did. The protocol makes that a standing role under a declared relationship instead of a Thursday job.

## 8. The design brief, and the jelly and the peanut butter

Everything above is one brief. Holochain gives us the technical substrate: agent-centric source chains, a DHT that notarises, integrity zomes that every peer validates, a conductor that can be upgraded by hot-swap or by lineage. That is the peanut butter: it holds, it is structural, it is what makes any of this real.

Elohim adds the information-philosophical layer on top, the jelly, which we call EPR. Its commitments, one per line:

- Every durable thing is a content-addressed atom, and which version applies is a declared dependency, never "latest".
- Every atom is coupled to three concerns, knowledge, value and governance, so no artefact can claim more than the thing it came from.
- Reach is earned by what a thing produces over time, under a steward's standing, not granted by holding a key.
- Identity is a right backstopped by community, not a self-asserted cryptographic primitive.
- The system's state is a register of habits that flip only on evidence.
- Observation is witness, not surveillance: held by the group it describes, graduated by diversity of observers, and now, we think, governed by the same stewardship grant that governs every other act one person takes on behalf of another.

The conductor exporter is a tiny instance of that brief: keep the sensor Holochain built, declare who holds the collector, bound what leaves. The upgrade manifest is another: keep the DNA hash Holochain mints, declare the lineage and the soak. The dataplane is another: keep the notary, move the bytes. None of it replaces Holochain. All of it needs Holochain to be true.

## What we ask

Three things, sized to a call.

1. Tell us what we cannot see: whether core has a Prometheus or OTLP exporter in flight (OTLP is OpenTelemetry's own wire format), and how a Holo host collects port-side metrics today.
2. Take the exposition with the label audit, as one clean pull request from our side if you want it.
3. A 45-minute call in the next two weeks with one engineer who owns host-side metrics at Holo. We bring a one-page draft of a scoped capture grant (what a host may collect about a hosted participant, at what reach, for how long, reviewable by whom); we leave with a yes, a no, or a counter-shape.

We are a small project whose own register of what works has more red than green, and says so. We will say what we have not proven. We would like to compare notes with people who run the same conductor at a different scale.

With respect,
Matthew Dowell, for the Elohim Protocol
