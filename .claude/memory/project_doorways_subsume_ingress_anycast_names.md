---
index: false
name: project-doorways-subsume-ingress-anycast-names
title: "Doorways subsume the ingress — anycast names"
description: "Doorways are host-aware and stand anycast-style behind a name's DNS records, failing over for each other; the k8s ingress goes away — bites on any serving-edge design."
metadata: 
  node_type: memory
  title: "Doorways subsume the ingress — anycast names, mutual failover"
  type: project
  originSessionId: e3b37899-a842-43df-9853-12981fc438ed
  modified: 2026-09-19T14:39:55.019Z
---

Operator direction, 2026-09-19 (pipeline-shakeout shift, after I described the alpha doorway pair as "the ingress pins each name to one doorway"):

Doorways CAN be hostname-aware (the EprRouter already dispatches on `(host, path)`; a contract bound to the asked name beats an any-host one). But their **federation** capability is the larger goal: doorways construct an **anycast-style relationship to a name's DNS records** — several doorways stand behind one name, membership is published and withdrawn (the beacon lane, see [[project-two-premises-dns-beacon-owned]]), and they **serve failover for each other**. The trajectory is to **subsume every concern the k8s ingress carries today** (host routing, TLS termination, cross-premise forwarding, health-based withdrawal), because the ingress will ultimately go away — the same ruling as [[k8s-is-not-the-architecture]], applied to the serving edge.

**Why:** a name pinned to one doorway by an ingress rule is per-host luck, which is exactly what the doorway-failover habit exists to retire; and a doorway that only ever answers one name cannot carry different reach/trust agreements per name at the point of projection ([[feedback_doorway_thin_plural_projection_not_trusted_tier]], [[feedback_doorway_projection_is_commons_privilege]]).

**How to apply:**
- Never name the ingress as the thing that should route a name to a doorway, and never propose "fix the ingress" as a cure. Ask what the doorway pair must do for itself: answer for any name it is a member for (own contract, or relay to the holder with `x-elohim-served-by`), and withdraw itself from the name's records when it sheds.
- When measuring, separate what the ingress is doing from what the doorways do. Measured 2026-09-19 on alpha: public DNS holds ONE A record per name (`elohim.host` → shem WAN, `alpha.elohim.host` → ops WAN, TTL 60); either premise's ingress forwards `elohim.host` to doorway B and `alpha.elohim.host` to doorway A; every live projection contract is any-host. So today's cross-name reachability is the ingress's, not the doorways'.
- The invariant to test is "the same name and contract resolves the same head wherever it is asked", not "both doorways serve one head".
