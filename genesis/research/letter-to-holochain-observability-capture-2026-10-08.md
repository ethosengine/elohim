---
title: "A Letter to Holo and Holochain Core: Why We Added a Prometheus Endpoint to the Conductor"
id: letter-to-holochain-observability-capture-2026-10-08
status: Capture
date: 2026-10-08
sovereignty-frame: bridge-legibility
---

# A letter to Holo and Holochain core: why we added a Prometheus endpoint to the conductor

Dear Rob, and whoever at core reads this after you,

You asked why we chose Prometheus and whether it makes sense for Holochain. The short answer is yes, with one condition that matters more than the exporter. The long answer is the rest of this letter. It runs long because answering it led us into upgrade safety and into who gets to keep monitoring data about people.

## 1. The question, restated

What we did is small. The conductor already keeps a set of internal gauges through OpenTelemetry, the standard instrumentation library: how long zome calls take, how long a lookup across the network takes (the cascade), how busy the database write path is, how far validation is behind, uptime, integrated ops. Out of the box, the only way to read them is InfluxDB, a time-series database: a file in its format for a collector called Telegraf, a copy of InfluxDB the conductor downloads and launches itself, or an InfluxDB you already run. We added a fourth option, a switch (`HOLOCHAIN_PROMETHEUS_LISTEN=<addr:port>`) that serves those same gauges on `GET /metrics` in the format Prometheus scrapes. When the switch is off, nothing binds. It's about 500 lines in one crate and adds no new metrics.

We did it because without it we were blind inside the conductor on a fleet that already had Prometheus watching everything else. With it, we found and fixed real slowdowns within days, with evidence: a permission check that re-read a participant's whole history on every call, so the app got slower the longer someone had used it; a validation loop (sys-validation) that asked for a record nobody held every ten seconds, forever; a publish loop that re-sent every record between sweeps. Each of those is on our fork with a measurement attached.

## 2. Our read on how Holochain observes itself today (an invitation to correct us)

Everything here is read from the conductor tree at our pin and from the public Wind Tunnel repository on 2026-10-08. Our fork's only remote is our own, so anything core landed after the 2026-08-03 release is invisible to us. Please correct what is stale.

**The metrics crate serves Wind Tunnel.** `holochain_metrics` describes itself as "integration of opentelemetry metrics and InfluxDB". As far as the public tree shows, its changelog records no user-visible change from 0.6.0-dev.1 through 0.7.0, and the commits touching it since April are housekeeping. Wind Tunnel's seventeen dashboard templates are InfluxDB JSON, four of them reading the conductor's own instruments (`holochain_conductor`, `holochain_database`, `holochain_p2p`, `host`). The exporter is shaped for a load-test bench: self-contained, a UI on a random port, no scraper assumed.

**Wind Tunnel runs on a real cluster, for short runs.** A Nomad cluster of your own, results to object storage, and, per the public workflows, an optional ThreeFold node pool excluded by default. The canonical `dht_sync_lag` run is thirty nodes for thirty minutes, twenty-five writers and five observers measuring lag; the scaled variant declares 150 nodes. It runs weekly per the published schedule and is torn down afterward. Each node is one conductor and one scenario binary; agents are fixtures.

**The operator's surface is the admin API.** `DumpState`, `DumpFullState`, `DumpConductorState`, `DumpNetworkMetrics`, `DumpNetworkStats`, plus tracing. Each dump is a snapshot you request by hand, and nothing keeps history.

**Labels were never audited for exposure.** Zome-call duration carries `dna_hash`, `zome`, `fn` and, whenever a source chain is present, the caller's `agent` public key. Signal emission carries `cell_id`. On a bench those are fixture ids. On a hosting fleet they are participants.

Our theory of the gap, in the order we would bet on it: the instruments were built for a developer's laptop and scheduled load tests, not an operator's fleet; the Rust ecosystem discontinued the OpenTelemetry Prometheus exporter crate and the Influx path core already owned stayed; fleet telemetry was, we assume, left to the host side; and the data-model rewrites of the last year (v2 actions, the unified DHT store, source chains into the per-DNA database) rightly took priority. That all makes sense for what the tooling was built to do.

## 3. Why we are on Kubernetes, and what that lets us measure

We run two things. A household mesh: three conductors, three storage peers, two doorways (our web-facing gateways) and a relay on one machine, started and stopped by one command, where every protocol scenario runs first. And an alpha neighbourhood: seven long-lived pods on a cluster we operate, with real network paths, hosted accounts on the doorways (the hosting path is real; the people in it are still our test fixtures, because we are designing for hosted people before anyone real signs in), a dataplane above the conductor (our own storage peers on libp2p and iroh with SQLite, and doorways serving it to the web), and months of accumulated state.

This part is personal, so I'll write it as me. I never had a HoloPort. I learned Kubernetes as a developer hoping that its whole model of scaling containers would solve the reliability problems of self-hosting, if I could only work out how it worked. Once I had, I saw that it is built for hyperscalers, not for self-hosters or for ordinary people behind a dynamic DNS address: it is superpowers of scale for a datacenter. When this project started, the edge-node release was the first point where I could begin from what I already knew, cloud-native as a homelab hobbyist and a corporate web-app developer by day, bring Holochain into my cloud-native workspace, and work from the other direction: use those datacenter superpowers to give peer-to-peer technology, Holochain included, the powers a hyperscaler has. I never seriously considered Wind Tunnel or the hc tooling, because Kubernetes solved that problem more or less out of the box; I only needed to make it legible to my own bench. To be clear, we use Kubernetes only for compute and hardware; we don't model protocol behaviour in it. It's our test bench, not the network. The goal of the whole architecture is the inversion: bring hyperscaler superpowers to everyday peer-to-peer, and with them subsume and escape the internet's existing capture by the corporate datacenter.

What it lets us measure, and a 30-minute run can't, is how a conductor behaves after a month. Every serious defect we found this quarter lived there. One was a zome call whose fixed cost grew with the caller's chain length (a no-op went from 7 ms at about 120 actions to 469 ms at about 18,300). Others were a validation loop retrying a record nobody had, a write lock held for an entire publish sweep, and two sync bugs in our own storage layer. None of them showed up until the network had been running for weeks.

What Wind Tunnel measures that we can't is scaling on a fresh network: how sync lag grows with 25 versus 127 writers, reads from peers that hold none of the data versus all of it (zero-arc versus full-arc), countersigning under load. We have seven pods; you have 150 for half an hour. You can test at a scale we can't, and we can run for longer than you do. We need both.

Our test reports do the same job as your `summariser` output. Yours uploads to object storage; ours land as dated evidence lines in a register of the things the system must reliably do, where nothing is marked green until a run shows it. We would be glad to read yours.

## 4. What we think core should take, and why

Everything above assumes something core does not: that the operator and the person are different. Holochain's conductor is built for one person on their own device, and that person is the operator. The admin interface binds localhost, checks origins and mints the tokens; every operator read (the dump requests, network metrics, network stats) is a local call by the device's owner. That is the right model for that person, and we are not asking core to change it.

We run a different shape, and so does Holo: one operator running many conductors for people who are not the operator. At that scale the admin interface stops fitting. You cannot port-forward an admin websocket per conductor and dump state by hand across a fleet, so you add a scrape endpoint. We did. Ours binds every interface, plain HTTP, no auth, and is safe only because our cluster has a network policy the conductor knows nothing about. On a HoloPort or a home machine behind dynamic DNS, that is a public port. It found real bugs for us. It is not the shape we are asking core to adopt.

What we would ask instead:

- Keep the read behind the admin interface. A `DumpMetrics` admin request that returns Prometheus text rides the policy core already has. A sidecar bridges it to a scraper, and the sidecar is the host's problem, ours or Holo's, not core's. The metrics stay OpenTelemetry; this is one more way to read them.
- If a listener is added at all, default it to 127.0.0.1 with a `danger_` override, matching core's own convention.
- Audit the labels before either ships. `agent` and `cell_id` must not leave the process by default. `dna_hash` is an exposure decision, not a cardinality one: it tells a reader which networks a node is on and, with the per-function timings, when its person is active. This affects every production user today. Anyone who turns on the Influx child service or points Telegraf at the line-protocol file is already building a per-participant activity ledger, and a Holo host collecting port metrics holds one for every hosted participant. Our exporter didn't create the problem; it made the data much easier to collect.

Wind Tunnel does not compete with any of this. It keeps Influx for the bench; hosts get a fleet-shaped read under the admin interface's policy. Both read the same metrics.

## 5. Upgrade and rollback

This part isn't about metrics. The reason we could ship a conductor patch, measure it, and roll it back without losing a network is a set of rules we had to build, and we think they belong in Holochain more than in our fork.

Some vocabulary first. A hApp's rules live in two kinds of code. Integrity code defines what is valid, and it is hashed into the network's identity (the DNA hash): change it and you have a different network. Coordinator code is the behaviour on top, and it can be replaced in place (hot-swap). So a conductor patch that changes only coordinator code does not move the DNA hash and heals by hot-swap (`update_coordinators`) with no re-key and no DHT churn. A change to integrity code moves the hash, and a reinstall mints a new agent key, which on a live network is a migration with lineage, never a wipe. We learned the hard way that forcing a reinstall on some peers and not others lands them on different DHTs and partitions the network, so the genesis pair must move together. We guard the packed DNA hashes in CI against a committed baseline and refuse a build that moves them without a tagged migration. The release itself is a manifest (artifacts by content id and sha256, a role binding naming the DNA hash, coordinator wasm hashes, what it migrates from, its lineage, a pointer to the governing rules, full build provenance, and an adoption discipline of soak time, attestation threshold and canary order), with the rule that a builder's own attestation never suffices to earn its release. Soak attestations ride an existing attestation kind so that adding them did not itself move the hash.

Why this matters to the DHT: integrity is the thing a DHT is for, and the only way to change integrity rules safely is a closed set of primitives that every peer can verify: what hash am I on, what did I migrate from, who attested the release, how long did it soak, in what order do peers adopt. If there's no safe way to upgrade, someone will eventually do it the unsafe way. Today it is partly solved on our side, and we could not find a core-level equivalent beyond the DNA-hash and coordinator-update mechanics themselves; if one exists we would like to know. We'd rather build this with core than keep it on our fork.

## 6. The scaling story: sharding there, a dataplane here

We think you are also working on sharding. Wind Tunnel's scenario names say so: zero-arc, mixed-arc, full-arc. Arc coverage is Holochain's answer to "not every peer holds everything", and our fleet runs with an arc factor set deliberately, because we hit the gossip wedge at roughly 120k ops where full-arc rounds stop completing.

Ours sits alongside it. Above the conductor we run a separate storage layer for things the DHT isn't designed to carry: content bytes in a peer-owned blob store (SQLite behind libp2p and iroh), inventory exchanged as metadata so that notarisation and byte-availability are deliberately decoupled, data shards placed by a rule that reads each peer's declared posture, heads elected and carried with their content, and doorways that serve all of it to the web without owning any of it. The DHT stays small and keeps validating. Large files move over our storage network.

The commons question sits on top of both. We have a research line on commons data pools (shared indexes, shared models, shared compute on committed devices) whose design decision is that a pool is a collective holding a resource, with contributions as notarised commitments and draws admitted by membership and reach. Sharding decides who holds which bytes; the pool decides who may draw on them and under what standing. We think the two fit together.

## 7. Capture: the problem neither bench solved

Our exporter did not just expose the conductor to Prometheus. It also meant our cluster's Prometheus was collecting per-person data, and nobody had decided it should. Whoever holds the collector is responsible for what it captures, and on our fleet nobody held that responsibility: the cluster's Prometheus held per-agent series for every hosted account, with no record of who allowed it, what it covered, how long it was kept, or how someone could object. Those accounts are still test fixtures, so nobody was harmed; but the data was being collected all the same, and it would not have changed shape on the day a real person signed in. A Holo host holds the same for every hosted participant on a port. We did not declare it, and we suspect no host has had reason to yet. A bench never has this problem because a bench has no subjects; Wind Tunnel's agents are fixtures. Our fleet runs the hosting path with fixture people in it, so now, before real people sign in, is when to fix it.

The claim we are building on, and would like tested by someone who runs a different fleet: whoever collects monitoring data about a person should hold an explicit, limited permission for it, the same way anyone acting for someone else does in our system. That's based on one fleet, so treat it as a hypothesis. Our identity layer already has the object for it: a stewardship grant, with a steward and a subject, an authority basis, evidence and a verifier, capability flags (one of them is literally activity monitoring), a mandatory expiry and review date, delegation depth, and an appeal. What it lacks is scope: a declaration of what the grant covers, which the grant names by its hash. The declaration answers, for each kind of observation: which labels may leave; which planes are covered, with a separate answer for the raw rows, the summaries, and the references to them; who may hold them and how many holders; how stale a read may be; what holding or viewing them reveals about the subject; who pays to carry them; and how long they are kept. "How widely shared" is not one dial. It is a different answer per plane, so the grant points at a declaration rather than picking a level. That is one field, not a new primitive.

With that field added, the relationships look like this. They are examples of declarations a grant could name, not rungs on a ladder:

- **Self.** A peer reads its own instruments. No grant, no cost, nothing leaves.
- **Declared.** A steward of capture holds a grant for a subject: a hub for its members, a gateway for the people it hosts, an operator for the fleet they run, a Holo host for the ports it serves. The grant names scope, reach and expiry; the subject sees it in their activity log and can appeal. Series carry only the labels the scope allows.
- **Earned.** Aggregation across households and regions happens only over summaries that several independent nodes agree on, never over raw per-person data.
- **Nothing.** An unscraped port. If nobody scrapes it, the data correctly says nobody was watching.

How this lands in our tree, each step useful alone. First, strip the per-person labels at the endpoint. Second, each peer keeps its own signed, append-only log of what it and its neighbours observe, shared only as widely as its permission allows, with each group's always-on node holding the group's log so there is no central server collecting everyone's. Third, status changes are confirmed when enough independent observers agree, instead of alert rules firing, and only those confirmed summaries roll up to the wider network. At that point Prometheus is a dashboard for the operator's own group, not a central collector.

Wind Tunnel already uses observers. Its `dht_sync_lag` run splits the fleet 85/15 into writers and a minority whose only job is to watch what the writers did. We want that observer role to be permanent and permission-based, not a weekly test.

## 8. How it fits together

Holochain is the foundation: agent-centric source chains, a DHT that notarises, integrity zomes that every peer validates, a conductor that can be upgraded by hot-swap or by lineage. On top of it we build a layer we call EPR, with these rules:

- Every durable thing is addressed by its content hash, and which version applies is declared, never "latest".
- Every record is tied to the knowledge, value and governance it came from, so nothing can claim more than its source.
- How widely a thing is shared is earned by what it produces over time, under someone's standing, not granted by holding a key.
- Identity is a right backstopped by community, not a self-asserted key.
- The system's state is a list of the things it must reliably do, each marked green only on evidence.
- Observation is witness, not surveillance: held by the group it describes, confirmed by several observers, and governed by the same permission that governs every other act one person takes on behalf of another.

The exporter is a tiny instance of that: keep the sensor Holochain built, declare who holds the collector, limit what leaves. The upgrade manifest is another: keep the DNA hash Holochain mints, declare the lineage and the soak. The storage layer is another: keep the notary, move the bytes. The pattern is the same each time. Holochain supplies the part that has to be true: the sensor, the hash, the notary. We add the part that says who holds it, who may change it, and how far it travels. Neither project does the other's half. That is why the Prometheus question matters to us more than an exporter should: it is the first place the two halves meet on something core ships.

With respect,
Matthew Dowell, for the Elohim Protocol
