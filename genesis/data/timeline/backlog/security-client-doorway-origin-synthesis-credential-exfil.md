---
id: "backlog-security-client-doorway-origin-synthesis-credential-exfil"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The login client SYNTHESIZES a doorway origin from user-typed text, persists it, and POSTs the plaintext password there ahead of every configured value — the redirect_uri-interception class, inverted across the wire"
slug: "security-client-doorway-origin-synthesis-credential-exfil"
written: "2026-08-28"
author: "opus (red-team lens of the auth-client-module design pass; chain re-verified on disk)"
status: "resolved"
priority: "critical"
area: "app/imagodei"
domain: "protocol"
jobs: [elohim-app]
relatedNodeIds:
  - "concern:doorway-portal-login"
cites:
  - genesis/data/timeline/backlog/security-doorway-oauth-redirect-uri-interception.md
tags: [security, auth, client, credential-exfiltration, open-redirect, origin-synthesis, critical]
---

# A host the human never typed receives their password

**Root fact.** `resolveGatewayToDoorwayUrl` fabricates an origin when nothing matches:

```ts
// 2. Convention: alpha.elohim.host → https://doorway-alpha.elohim.host
if (parts.length >= 3 && !firstSubdomain.startsWith('doorway-')) {
  return { ok: true, doorway: { url: `https://doorway-${gatewayDomain}` } };
}
// 3. Fallback: domain may be the doorway itself
return { ok: true, doorway: { url: `https://${gatewayDomain}` } };
```
(`app/elohim-elements/elohim-imagodei/src/federated-identifier.ts:133-139`; the file header
concedes "all resolution is currently convention-based and synchronous".)

There is no allowlist, no verification, and no server round-trip. `ok: true` is returned for
any input.

**It then outranks configuration.** `getAuthBaseUrl()` returns the registry selection FIRST —
ahead of the workspace doorway, ahead of `environment.holochain.authUrl`, ahead of the
adminUrl derivation (`app/elohim-app/src/app/imagodei/services/providers/password-auth.provider.rs`
— TS at `providers/password-auth.provider.ts:60-63`). The login POST is built from it, carrying
the plaintext password.

**And it persists.** The selection is written to localStorage `elohim-doorway-url`
(`doorway-registry.service.ts:492 persistSelection`) and restored on every boot
(`:515 restoreSelection`), so one poisoned entry re-points every later sign-in.

## The chain

A human types `foo.evil.tld` into the federated-identifier field (or is induced to — the field
exists precisely to accept an unfamiliar doorway). The client synthesizes
`https://doorway-foo.evil.tld`, selects it, persists it, and POSTs `{identifier, password}`
there. The OAuth path is worse: `oauth-auth.provider.ts` sends the browser to
`${doorwayUrl}/auth/authorize` and stores `doorwayUrl` in the OAuth `state`, then exchanges the
code at `${doorwayUrl}/auth/token` — the `state` check guards CSRF, never the destination.

This is the same class as `security-doorway-oauth-redirect-uri-interception`, reflected. There,
a lax server-side matcher let the doorway send an authorization code to an attacker URI. Here a
matcher-free client synthesizes an origin and sends the credential itself.

## The fix, and why it is now cheap

**Discovery must RESOLVE, never SYNTHESIZE.** As of 2026-08-28 the doorway serves
`GET /.well-known/elohim-auth` (`doorway/doorway-service/src/routes/auth_discovery.rs`) — an
unauthenticated JSON document whose every value is an origin-RELATIVE path, so it is
structurally incapable of naming another origin. That is the verified-fetch anchor this fix
needs:

1. **Delete both synthesis branches.** Return a not-resolved result instead of a fabricated URL;
   `ok: true` for arbitrary input is the defect.
2. **Resolve by exact match** against the known-doorway set, **or** by fetching
   `https://<exactly what the human typed>/.well-known/elohim-auth` and requiring a valid
   document. A host that does not serve one is not a doorway. Never derive a *different*
   hostname (`doorway-` prefixing) — a secret must never go to a host the human did not type.
3. **Invert the precedence** in `getAuthBaseUrl()`: a runtime-resolved origin must rank BELOW
   configured/pinned values, not above them.
4. **First use of a new doorway is an explicit TOFU consent**, showing the un-truncated origin
   before any credential is sent — mirroring what storage already does server-side
   (`services/session_exchange.rs`, the doorway allowlist).

## Residue on the discovery document itself

`/.well-known/elohim-auth` meets three of the red-team's four requirements as shipped: it lives
under the reserved `/.well-known/` prefix so an unknown path 404s honestly rather than being
swallowed by the SPA fallback (verified: `/auth/config` returns the SPA shell, which hands a
client a JSON parse error instead of an absence signal); it emits only relative paths, which
*exceeds* "constrain and reject cross-origin values at the client" by making them
unexpressible; and it deliberately omits `portalHostUrl`, which is per-human session state that
must be attested in that human's own record rather than advertised publicly.

**Not yet met: requirement 3 — the document is unsigned.** The Ed25519 key is already published
at `/.well-known/doorway-keys` (`routes/federation.rs:173`), so signing the body and having the
client verify against that JWKS is the remaining work, and it is what closes a MITM or
stale-DNS answer rewriting the document.


---

## RESOLVED 2026-08-29

Landed in `e73869bd6`. 1199 imagodei tests pass; `just gate elohim-app` green
end to end (0 lint errors, AOT build, 220 files / 4612 tests).

Four changes, each removing a way for typed text to become a request origin:

1. A doorway DECLARES the gateway it serves (`gatewayDomain` on `DoorwayInfo`).
   `resolveGatewayToDoorwayUrl` is a lookup against declarations and returns
   `string | null`; both synthesis fallbacks are deleted. The old matcher also
   used `.includes`, so `alpha.elohim.host.evil.tld` matched the real alpha
   doorway — host equality closes that.
2. Adopting an unknown host requires `probeDoorway`, which GETs
   `/.well-known/elohim-auth` on the typed host's OWN origin and selects exactly
   what answered, never a `doorway-`-prefixed derivative.
3. `DoorwaySelection.verified` gates use, and is RECOMPUTED on restore rather
   than read back from storage. This was the subtle half: fixing the write path
   while trusting the restore path fixes nothing for anyone already poisoned,
   because localStorage is attacker-writable and a selection persisted before
   the rule existed would otherwise survive it.
4. Precedence inverted — the environment is the default; a selection is an
   override that must carry proof.

**One thing this investigation found that the row did not name.**
`handle_login` re-qualifies a submitted identifier's local part with the
doorway's OWN gateway domain, so POSTing `alice@other.host` at a doorway serving
`alpha.elohim.host` authenticates ALPHA's alice on a password collision —
silently, as the wrong human. The wire cannot express the distinction, so login
refuses a foreign identifier before the request exists.

**The tests are the deliverable as much as the fix.** Four in
`doorway.model.spec.ts` previously asserted the SYNTHESIS behaviour — they
encoded this vulnerability — and now assert the refusal, alongside four
hostile-input vectors. The registry spec's "should create minimal entry for
unknown URL" likewise asserted the permissive path.

**Caught late, worth recording:** lint and 4611 tests passed on a version where
`login.component` still called the trusted-only setter with the probed URL,
which would have silently refused every legitimately-probed doorway and broken
federated sign-in. Only the AOT build surfaced it (`TS2345`), via the nullable
return type. The security fix and the regression it nearly caused were separated
by one `ng build`.
