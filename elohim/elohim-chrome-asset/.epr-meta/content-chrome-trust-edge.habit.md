---
epr-habit-version: 1
id: content-chrome-trust-edge
invariant: >
  When EPR content reaches a person through a browser, the chrome loads first and the content
  loads inside it, in an origin apart from the chrome and the person's sign-in, by default. A
  quiet chip opens a minimal omnibar, whose trust mark opens page info. Page info's readings are
  Source, Delivery, Holders, Steward, Claims, Permissions, plus Yours when signed in. They come
  from the content-chrome facing, which is folded from the p2p dataplane and the holons that
  govern the content; nothing is read from a doorway's own configuration. Every card ends at the
  content's EPR page, carrying the delivery path that reached the person. The frame is given
  what the record declares, within what its gates admitted, as the person grants it, and nothing
  undeclared; undeclared behaviour raises a signal against the content's address. The witness
  reads amber while the DHT has not yet anchored the record and green once it has; it is derived
  and is not written, and delivery does not wait for it. The same content reads the same
  through any doorway, and on the person's own peer with no doorway at all. The chrome comes off
  only inside a wrapper whose record declares that it presents content and fulfils these claims.
status: red
active: false
checks:
  - "a2o @concern:content-chrome-trust-edge (genesis/a2o/features/protocol/content-chrome-trust-edge.feature — @act:i @requires:owned-substrate, household lane: `just test mesh features/protocol/content-chrome-trust-edge.feature` against binaries built from the tree under test (`just mesh build doorway` and `just mesh build storage` first). Nine scenarios: page info and its EPR links, content isolated from the chrome and sign-in, amber then green, declared capabilities only, the wrapper-only exception, the signed-in Yours reading, the undeclared-behaviour signal, plural delivery across both doorways, and the person's own peer with every doorway gone. A run with undefined, pending or skipped steps discharges nothing.)"
  - "a2o Station 5 of genesis/a2o/features/dataplane/epr-app-deliverability.feature (the returning visitor whose apps worker is installed still lands in the chrome; navigation preload). Shared with epr-app-deliverability's own register; it is listed here because the chrome's load-first clause depends on it."
  - "unit: elohim/elohim-chrome-asset cargo tests plus node tests including tests/validate-island.mjs (the island validates against elohim/sdk/schemas/v1/views/chrome-context.schema.json); elohim/elohim-facings folds::content_chrome; the doorway's apps.rs routing tests; storage's apps_chrome.rs origin-pair tests."
refs:
  - "genesis/docs/superpowers/specs/2026-10-14-content-apps-wear-chrome-design.md (Decided 2026-10-15: page info, authority from the dataplane, trustful and trustless modes, the public-name apps origin, plural delivery, the VSM reading)"
  - "genesis/data/timeline/backlog/2026-10-15-projection-inspection-load-from-a-holding-peer.md (the System 3* audit past the projection; later work)"
retire-when: >
  when the person's own client — a native protocol browser or the peer-native shell on their own
  device — is how most people reach EPR content, and web2 doorways carry only residual traffic,
  so the chrome's browser-edge contract is a compatibility surface rather than the trust edge.
---
BORN red 2026-10-15. The operator's direction is that the chrome is the trust carrier for
content reaching legacy browsers, so it must load first and be inspectable down to the EPR page.
The design is approved, and its story has passed ten blind-reader rounds (READY). These are
built in worktree wt-sw-preload and are uncommitted:
- the SDK schemas (chrome-context, frame-capability, the html5-app body);
- the content-chrome facing and its route;
- storage's peer-native origin-pair shell;
- the doorway's public-name apps origin and signal relay;
- the element's page info.
Nothing has been measured on the household yet, so the status is red with unmeasured checks.
DELTA 2026-10-16 (RED preserved; first household measures, uncommitted tree wt-sw-preload): chrome lane run 7 (00:56–00:59Z, doorways a/b/c on this tree's doorway, storage on this tree's mesh build) passed 7 of 9 scenarios, 102 of 109 steps; runs 1→7 read 0, 0, 5, 6, 7, 6, 7. Green now: page info and its EPR links, isolation from the chrome and sign-in, declared capabilities only, signed-in Yours, the undeclared-behaviour signal (relayed with document_match true after slugs resolve to their CID), plural delivery, own-peer with no doorway. Two reds: (1) amber-then-green — an authored record gets its dht_anchor_hash at once, so it reads green before any validator has seen it; the witness rule awaits the operator's decision (validation receipts on the author's peer, authority arrival elsewhere); (2) the wrapper exception's lamad step — the household serves a lamad bundle older than this branch, whose viewer has no app-frame; it needs this branch's lamad and landing builds staged. Station 5 (epr-app-deliverability rerun at 01:21Z, doorway binary c1b9bbd5 on all three before and after): 6 of 7 scenarios; its red is step glue that asserted an apps.<name> frame on an unpaired name and hid the visit's errors, fixed by branching on isolation.boundary and printing them, not yet rerun. A deliverability run at 00:59Z is void: another session restarted the doorways on its own binary mid-lane (the restart arms now refuse under another session's lease, integrate 89c41e696). Reports: genesis/a2o/reports/sprint-report-household-20261016T012120Z-db4f7634.md and lane logs under genesis/local-dev/chrome-trust-edge/. NO status change.
DELTA 2026-10-29 (RED preserved; port, no household measure): the snapshot of wt-sw-preload (47ba8da76) is being ported to dev in layers on port/chrome-trust-edge, dormant by default. Landed: the SDK schemas and codegen, lamad-types' html5-app body parser and vocabulary, the elohim-views facing structs, the elohim-facings content_chrome fold, storage's GET /api/v1/epr/{id}/chrome and POST /api/v1/epr/{id}/signals behind ELOHIM_CONTENT_CHROME (off unless 1/true/yes/on), and elohim-chrome-asset's v1 context types and render_content_shell (pre-v1 islands byte-identical). Deferred with exact plans: the coordinator extern get_record_witness_facts (backlog record-witness-coordinator-extern; until it lands the witness reads amber), and the element, storage's peer-native /apps shell, the doorway apps origin and CSP relay, the apps worker, the Angular surfaces and this habit's story (backlog content-chrome-element-and-app-port). The story file named in checks is not on dev yet, so the first check cannot run. NO status change.
