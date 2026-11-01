---
id: "backlog-projection-inspection-load-from-a-holding-peer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Projection inspection: from the chrome's Delivery card, load the same content directly from a holding peer by its CIDv1"
slug: "projection-inspection-load-from-a-holding-peer"
written: "2026-10-15"
author: "matthew"
status: "envisioned"
priority: "medium"
relatedNodeIds:
  - "genesis/a2o/features/protocol/content-chrome-trust-edge.feature"
  - "genesis/docs/superpowers/specs/2026-10-14-content-apps-wear-chrome-design.md"
tags: ["chrome", "doorway", "projection", "plural-delivery", "trust-edge", "network-hardening", "peer-auth"]
shift_objective: |
  From a content shell's page-info Delivery card, a person can choose "load this directly from a
  holder": the person presents themselves to a holding peer through Elohim-native peer-authenticated
  login, and if the peer admits the visit the chrome loads the same bytes from that peer by the
  content's CIDv1, instead of the doorway's projection, verifies them against the record's content
  address, and shows the result side by side with what the doorway delivered (match, or the
  difference). Done when an a2o scenario on the household opens content through doorway
  "alpha-A", asks for a direct load from a holding peer, and sees the same content address
  verified from the peer, with the Delivery card naming the peer reached directly.
---

## What the operator asked for (2026-10-15)

"Projection inspection: you get the projection by default, but you could, in theory, ask to load
the same content directly from a peer, using some raw CIDv1-based peer address. It may be less
performant, but you could actually visit and load from the source."

## Why it belongs to the chrome

The content chrome (spec `content-apps-wear-chrome-design`, revision 2026-10-15) already makes
the delivery path inspectable. Its Delivery card says which public name, which doorway and which
peer's storage brought the bytes, and every EPR link carries `?via=<doorway>&peer=<peer>`. The
doorway's answer is a **projection**: a cached, web2-shaped copy of what the network holds.
Today a person can inspect the projection's provenance but cannot step around it.

Projection inspection is the next rung. The person goes from "this doorway says it got these
bytes from that peer" to "I fetched them from a holder myself and they match". Delivery stays
plural, and the projection becomes something a person can check rather than something they have to
take on faith. That is the same reason trust lives in the record and not the doorway.

## Shape to work out (not decided)

- **Addressing a holder.** A CIDv1 names the content; a holder needs a peer address reachable from
  a browser. Candidates are a peer's own HTTP surface (where a peer runs a hub-side web seam,
  `elohim/elohim-hub/`), a relay route, or a browser-native transport (WebTransport or iroh) once
  one exists. The doorway must not choose the peer for the person (it is single-target dispatch
  and never fans out; see `doorway/CLAUDE.md`); the holder set comes from the network's own
  holder records, the same ones the Holders card reads.
- **Direct access is a privilege, not a right.** A person does not simply reach into a peer;
  they present themselves first, as anyone would before walking up to someone in person. The
  direct load therefore happens under Elohim-native, peer-authenticated login: the person's agent
  introduces itself to the holding peer, and the peer decides whether to answer. A peer may
  decline, and the card says so plainly ("This holder did not admit a direct visit") rather than
  treating it as a failure. Projection through a doorway stays the open default; going direct is
  an admitted visit.
- **When the peer is not home.** A holder may be offline, asleep or away. Instead of an error,
  the card shows the peer's door sign, a small digital sign like the one on a shop door: "Away,
  last seen this morning, come back later", or a note the peer's steward left ("Back Sunday").
  The doorway projection still serves the content meanwhile, because other holders and the
  doorway's cache carry it. The person can try another holder, or ask to be told when this one
  is back.
  - The sign reads the network's own presence for the peer (delivery peers already carry
    `lastSeen`), plus anything the steward chose to say. The doorway decides none of it.
  - "Last seen" tells an outside observer when someone is active, and the protocol treats
    participation as sensitive (`genesis/docs/architecture/private-thought-governed-fruit.md`
    §4, boundary 6). So the steward chooses what the sign says and at what reach. The default is
    coarse ("Away", "Usually back by evening"), never a precise timestamp, and a peer may show no
    sign at all.
- **The holder may be a collective, not a house.** Content can be held by a commons pool: a
  collective, a shop, a business or a group whose members' peers hold it between them. A direct
  visit then resolves to the pool, and any member peer that is in may answer. The door sign
  belongs to the collective's own door, like a note on a shop or a group's meeting hall ("Closed
  for the holiday, open Monday"), written by whoever the collective's own governance allows. A
  person presents themselves to the collective, not to one member's house, and the collective's
  terms decide whether to admit the visit. The Delivery card names the collective, and names the
  member peer only if the collective chooses to.
- **Verification.** The chrome, or the apps worker, hashes what arrives and compares it with the
  record's content address. A match is shown as such. A mismatch is a signal relayed to the
  holding peers, like any other undeclared behaviour.
- **Presentation.** A second frame, or a toggle that reloads the frame from the peer, with the
  Delivery card switching to "Loaded directly from <peer>". It may be slower, and the card says so.
- **Plural, not authoritative.** A holding peer is one witness among many, not an authority. The
  record and the DHT witness stay the source of truth, and the direct load is a cross-check.

## Readiness

- Ready: the Delivery card, delivery-aware EPR links, the holder reading, the content address on
  the record, and the apps worker's verify-before-cache path (`apps-sw.ts`).
- Blocking:
  - a peer address a browser can reach without the doorway in the path. This is the open web-seam
    question (hub side);
  - Elohim-native peer-authenticated login, so a person can present themselves to a holding peer.
    This is planned as network-hardening work (operator, 2026-10-15) and is a prerequisite here.
