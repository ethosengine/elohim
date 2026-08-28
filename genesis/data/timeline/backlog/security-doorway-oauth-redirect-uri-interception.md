---
id: "backlog-security-doorway-oauth-redirect-uri-interception"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "GET /auth/authorize issued real authorization codes to ANY https redirect_uri — the wildcard matcher compared only the text before the first * and after the last one, so https://*.elohim.host/* accepted every https URI"
slug: "security-doorway-oauth-redirect-uri-interception"
written: "2026-08-28"
author: "opus (rung-1 of the elohim-native OAuth ladder; proved and cured on the local mesh)"
status: "wip"
priority: "critical"
area: "doorway/auth"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "concern:oauth-authorization-code"
cites:
  - genesis/data/timeline/backlog/security-doorway-devmode-auth-bypass.md
  - genesis/data/timeline/backlog/security-doorway-auth-required-unenforced.md
tags: [security, doorway, auth, oauth, rfc6749, redirect-uri, code-interception, critical, local-mesh-proven]
---

# An authorization code is a bearer credential for a human's identity, and any https URI could receive one

**Root fact.** `matches_uri_pattern` (`doorway/doorway-service/src/db/schemas/oauth_session.rs`)
split a registered pattern on `*` and then compared **only** `pattern_parts[0]` as a prefix and
`pattern_parts.last()` as a suffix. For a pattern ending in `*` the last element is the empty
string, so the suffix check was skipped — and every literal *between* the wildcards was never
compared at all.

`elohim-app` registers `https://*.elohim.host/*`. That splits to
`["https://", ".elohim.host/", ""]`, so the whole pattern degenerated to
**"starts with `https://`"**. The literal `.elohim.host/` was never checked.

## Consequence

`GET /auth/authorize` validates `redirect_uri` against the client's patterns and, for an
already-authenticated human, mints a single-use authorization code and hands it to that URI.
With the matcher broken, **any `https://` URI was an accepted callback for `elohim-app`**.

The attack is the textbook RFC-6749 authorization-code interception: send a logged-in human to
`/auth/authorize?client_id=elohim-app&redirect_uri=https://attacker.tld/cb`, receive a valid
code, exchange it at `POST /auth/token` for a full access token bearing that human's
`human_id`, `agent_pub_key` and `identifier`. No consent screen stands in the way —
both registered clients are `trusted: true`, and `trusted` is never read anywhere in
`doorway-service`.

`http://` and `ftp://` callbacks were correctly refused, which is precisely what made the guard
look like it worked.

**PROVEN (2026-08-28, local mesh, non-mutating).** Against a doorway built from HEAD:

```
https://attacker.tld/steal        -> {"redirect_uri":"https://attacker.tld/steal?code=6ddc935f…&state=s9"}
https://a.b.c.evil.co.uk/x?q=1    -> {"redirect_uri":"https://a.b.c.evil.co.uk/x?q=1&code=a6d750ac…&state=s9"}
http://evil.example/cb            -> {"error":"invalid_redirect_uri"}   (refused — masked the hole)
```

The same code path runs on every deployed doorway. **The fleet was not probed for this**;
the local proof and the source trace are the evidence.

## The cure (landed, locally verified)

Redirect-URI safety is a property of **scheme + host + port**, not of string shape, so the
matcher no longer pattern-scans the whole URI. `split_uri` parses `scheme://host[:port][/path]`
and the comparison is now structural:

- **scheme** — exact, case-insensitive. No wildcard, so an attacker cannot downgrade https to http.
- **host** — a literal (exact, case-insensitive) or a single leading `*.` label wildcard. The
  wildcard part comes from the *parsed* host, so it cannot contain `/`, `:` or `@`.
- **port** — `*` admits any; otherwise exact, and an absent pattern port requires an absent request port.
- **path** — glob-matched, and `glob_match` now anchors the first literal at the start, the last
  at the end, and requires every interior literal in order.
- **userinfo rejected** — `https://elohim.host@evil.tld/` must never read as the `elohim.host` authority.

## Evidence

- Rust unit regressions: `wildcard_pattern_does_not_accept_a_foreign_host`,
  `origin_wildcard_does_not_span_a_path_separator`, `legitimate_redirect_uris_still_match`
  (`oauth_session.rs`). Both security tests were written first and observed to FAIL against the
  old matcher. Full doorway lib suite: **1132 passed, 0 failed**.
- a2o regression story: `genesis/a2o/features/auth/oauth-authorization-code.feature`
  (`@concern:oauth-authorization-code`), Scenario Outline "A hostile redirect_uri is refused
  even for a registered client", backed by `genesis/a2o/steps/auth/oauth-code-flow.steps.ts`.
- Local mesh run: **7 scenarios / 39 steps, all passed**; sprint report
  `summary.byConcern["oauth-authorization-code"] = {passed 7, failed 0, pending 0, skipped 0}`
  — the first auth concern in the repo able to discharge its commitment.

## Residue — NOT closed by this fix

1. **No consent screen exists.** Both registered clients are `trusted: true` and the field is
   never read (`get_registered_clients()`, a hardcoded vec of two). The endpoints the portal
   posts to — `/auth/authorize/prepare|grant|decline` — exist in no Rust and in no design doc.
2. **`/auth/authorize` issues a code it may never have stored.** The Mongo write sits under
   `if let Some(mongo)` and `if let Ok(collection)`; both fall through silently and the redirect
   is returned unconditionally. Only an `insert_one` failure returns 500. On an archive-less
   doorway — the local-mesh default, and any fleet Mongo outage — authorize reports success and
   `/auth/token` can never redeem the code. A C4 honest-absence violation; filed here rather
   than fixed, because the honest answer (refuse, or degrade explicitly) is a contract decision.
3. **Client identity is DNS-shaped.** `client_id` is a bare string and redirect validation is a
   hostname comparison. The substrate already carries key-addressed alternatives that OAuth uses
   none of — `did:elohim:<agent key>` (`bridges/did/`), pkarr endpoint records
   (`bridges/pkarr/`), and `PortalHost` as a witnessed "which URL speaks for this agent" claim.
   This fix hardens the web2 projection and is explicitly **not** progress toward peer-native
   client identity. Per `bridges/CLAUDE.md`, RFC-6749 is a *bridge* concern (an external web2
   protocol translated to and from the substrate), yet it lives inside `doorway-service` as if
   it were substrate — the dependency arrow is backwards, and every increment here deepens the
   assumption that identity is a hostname.

   **Corrected 2026-08-28** — an earlier revision of this row said the native path is
   DNA-hash-moving. That is true only of the REMOTE trusted-peer case. The foundation is
   cheaper, and it needs no integrity change at all:

   | Case | Authority | Needs | DNA-hash |
   |---|---|---|---|
   | Self, own device | own runtime | key custody in storage; `did:key` (already resolves offline, no I/O) | neutral |
   | Guest on a trusted peer's runtime | their runtime | custodial-key activation + `DevicePolicy` | neutral |
   | Remote peer over a network | their runtime | `PortalHost` + the handoff already built | **moving** |

   The shared primitive is *activate my key on a runtime I do not own, under a policy its owner
   authored, for a bounded session*. All three parts exist: `custodial_keys` (password-derived
   key encryption) — but **only in `doorway-service/src/custodial_keys`, absent from
   elohim-storage**, so the primitive enabling a guest profile on a peer runtime is currently
   doorway-only; `DevicePolicy` (imagodei `stewardship.rs:178` — `subject_id` + `device_id`,
   `author_tier`, `inherits_from`, `session_max_minutes`, `time_windows_json`,
   `disabled_features_json`), which is already a guest-profile policy object for a shared family
   device; and the nonce challenge (`signal/mod.rs:214-259`). Two are in the wrong process and
   one is unwired to identity. `local_sessions::create_session` already deactivates all other
   sessions (`:124-127`), so the single-active profile switch is built — guest means N profiles
   with an eviction policy, not a new model. `PortalHost` cannot express the local case (its
   integrity validator mandates `https://` and the pure-native case has no URL at all); do not
   stretch it to cover both.
4. **No `seam-registry.yaml` entry** covers the OAuth decision points — consistent with this
   surface never having passed the p2p-design-gate.
