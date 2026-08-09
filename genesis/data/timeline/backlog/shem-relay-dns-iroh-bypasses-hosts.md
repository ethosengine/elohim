---
id: "backlog-shem-relay-dns-iroh-bypasses-hosts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "shem conductors' iroh home relay: hickory resolver bypasses /etc/hosts — hairpin needs a CoreDNS-level answer (operator action)"
slug: "shem-relay-dns-iroh-bypasses-hosts"
written: "2026-08-09"
author: "convergence-serve-path shift"
status: "backlog"
priority: "high"
tags: [shem, iroh, relay, dns, coredns, hairpin, operator-action]

relatedNodeIds:
  - "backlog-susan-kitsune2-gossip-never-attempts"
  - "backlog-shem-conductors-signal-hairpin-suspect-dht-silent"
---

# The /etc/hosts hairpin class is inert for iroh — mechanism confirmed at source

Source-verified 2026-08-09 (iroh-holochain 0.95.1, iroh-relay-holochain
0.95.1, kitsune2_transport_iroh 0.4.1, fetched at the fork's exact pins):

1. "Home relay URL is known" is set ONLY when iroh net_report's HTTPS probe
   (GET <relay>/relay/probe; ProbePlan::initial schedules Https only —
   quic:None relays never get a QUIC/STUN probe) completes and names the
   relay preferred → `home is now relay …` (actor.rs:994). No other writer.
2. BOTH the probe and the relay client dial resolve the hostname through
   iroh's own hickory DnsResolver built from /etc/resolv.conf ONLY
   (dns.rs:158-203; reportgen.rs:805 "Use our own resolver rather than
   getaddrinfo") — **/etc/hosts and hostAliases are structurally invisible
   to iroh**. The #1328 hairpin alias (db5fb585b) is inert for the
   conductor relay plane (harmless; still valid for glibc-path lookups).
3. Live contrast: matthew (operations premise, network hairpins fine)
   logs `home is now relay https://relay.alpha.elohim.host./` ~2s after
   insert; shem pods run the identical probe cycle every ~20-27s forever
   with zero successes (probe's underlying error is logged below WARN —
   not recoverable from Loki).

## The fix is DNS-level, cluster-side (operator seat)

Make relay.elohim.host (and the sibling hairpinned apex names) resolve
inside the cluster to the shem ingress without leaving the premises —
CoreDNS hosts-plugin block (mirrors the existing podSpec hairpin aliases,
but at the layer iroh's resolver actually consults):

    # kube-system/coredns Corefile, inside the .:53 block, BEFORE forward:
    hosts {
        10.99.0.2 relay.elohim.host elohim.host signal.elohim.host
        fallthrough
    }

Blast-radius note: a malformed Corefile breaks cluster-wide DNS — operator
applies, not CI. Alternative at the network layer: fix shem router NAT
hairpin for the WAN IP (would also cure the class for non-pod clients).
Post-fix confirmation: `home is now relay https://relay.elohim.host./`
INFO line on shem conductor pods within seconds of restart; kitsune2
"home relay URL is known" WARN rate falls to ~zero; gossip rounds begin.
Residual diagnostic if the probe STILL fails after DNS answers in-cluster:
capture iroh::net_report at debug on one pod (TLS/SNI vs timeout).

## RESOLVED at the resolver that actually answers (2026-08-09, operator)

Delivery-point corrections from the operator's devops session (the
diagnosis held; the backlog's named ConfigMap was wrong):
- kube-system/coredns has been scaled 0/0 since the 2026-06-10 DNS
  migration (inert); live CoreDNS is coredns-ha — and pods never reach
  either for external names: every pod resolves via 169.254.20.10
  (NodeLocal DNSCache, DIRECT mode) forwarding to the node resolv.conf.
- The hosts block was applied to kube-system/node-local-dns instead:
  ethosengine-devops e4cb2a2 (gitops/infrastructure/node-local-dns/
  node-local-dns.yaml), ArgoCD Synced/Healthy, 7/7 pods Ready, reload
  in place. Verified from a clean pod on shem via 169.254.20.10:
  relay/apex/signal → 10.99.0.2; example.com still public (fallthrough).
- CAVEAT for the residual diagnostic: a 404 on /relay/probe is NORMAL
  (the known-good relay.alpha returns 404 there too) — never treat it
  as probe failure.
- CORRECTION: the WAN path was never a timeout — from a shem pod,
  136.50.16.133:443 hairpins fine at L3 in ~140ms and lands on the
  WRONG BOX (self-signed cert). A router-level fix would be NAT-hairpin
  DESTINATION, not ports.
Remaining trigger: conductors cache the old answer (hickory TTL) — a
pod restart (any edge deploy) picks up the fixed DNS; then expect the
"home is now relay https://relay.elohim.host./" INFO per shem pod, WARN
rate → ~0, gossip rounds, caughtUp=true, shed lifts, divergentAnchor
drains.
