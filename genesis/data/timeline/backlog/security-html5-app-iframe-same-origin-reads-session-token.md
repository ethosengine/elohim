---
id: "backlog-security-html5-app-iframe-same-origin-reads-session-token"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content framed by lamad's iframe renderer runs same-origin with allow-scripts + allow-same-origin, so any html5-app author's script can read the viewer's session token (renewable indefinitely) and, in some connection modes, their stored signing key"
slug: "security-html5-app-iframe-same-origin-reads-session-token"
written: "2026-10-29"
author: "claude (found by a read-only prior-art pass comparing napplet NAP specs; chain re-verified on disk; adversarial review by Codex gpt-6-astra, 74 claims checked, 19 corrected)"
status: "open"
priority: "high"
area: "app/lamad"
domain: "protocol"
jobs: [elohim-app, elohim-edge]
relatedNodeIds:
  - "concern:doorway-portal-login"
tags: [security, html5-app, iframe-sandbox, credential-exfiltration, session-token, signing-credentials, epr-app-bundle, lamad, doorway]
---

# A content author's script can act as whoever views their app

**One line.** lamad's iframe renderer frames content on the app's own origin with
`sandbox="allow-scripts allow-same-origin …"`. Script inside the frame therefore has the shell's origin.
It can read the doorway bearer token from `localStorage` and renew it indefinitely through
`/auth/refresh`. In connection modes that persist signing credentials, it can also read the agent's
**private key and capability secret**, whose validity is not bounded by the token at all.

Sibling defect, same class, worse sink:
[security-markdown-renderer-raw-html-bypasses-sanitizer](epr:security-markdown-renderer-raw-html-bypasses-sanitizer).
Markdown content executes in the shell document directly, with no frame involved.

## The chain (each link read on disk, 2026-10-29; paths from the repo root)

| # | Link | Where |
|---|---|---|
| 1 | One static sandbox attribute for every mode of the renderer: `sandbox="allow-scripts allow-same-origin allow-forms allow-popups"` | `app/lamad/src/app/renderers/iframe-renderer/iframe-renderer.component.ts:57-65` |
| 2 | **Four** framing modes all pass through `bypassSecurityTrustResourceUrl`: html5-app (`${doorwayUrl}/apps/${slug}/${entryPoint}`), direct `http…` URL, legacy `metadata.embedUrl`/`url`, and a final fallback that accepts **any non-empty string with no scheme check** | same file `:27-30`, `:118`, `:126`, `:136`, `:140-145`, `:154` |
| 3 | `resolveDoorwayUrl()` returns `''` in every environment **on purpose**, so html5-app bundles load from the shell's own origin. The stated reason is that the service worker can then intercept `/apps/` for ZIP delivery on a cold cache. It ignores the native `storageUrl` too (`app/elohim-app/src/environments/environment.native.ts:27`) | same file `:165-177` |
| 4 | The session token, its expiry and the identifier are stored in that origin's `localStorage` (`elohim-auth-token`, `AUTH_TOKEN_KEY`) | `app/elohim-app/src/app/imagodei/services/browser-session-token.store.ts:60,85-98`; key at `app/elohim-app/src/app/imagodei/models/auth.model.ts:18` |
| 5 | **Signing credentials are also in `localStorage`** when the connection strategy supplies them: the agent key pair and the capability secret, base64-serialized | `app/elohim-app/src/app/elohim/services/holochain-client.service.ts:389-391,1048-1058` |
| 6 | The token is renewable without re-authentication. `handle_refresh` accepts any valid bearer and issues a new token with renewed expiry, preserving `session_id`, `permission_level`, `is_steward` and `conductor_id`. The one-hour default (`JWT_EXPIRY_SECONDS=3600`) is an issuance default, **not a ceiling on the attack** | `doorway/doorway-service/src/routes/auth_routes.rs:2634-2680`; `doorway/doorway-service/src/config.rs:168-169` |
| 7 | The bearer reaches storage **as the person**: a hosted session is forwarded with the agent key as `X-Agent-Id`, which `resolve_account_caller` takes as the caller, plus the account id as `X-Agent-Cid`. Account, identity, hosting-account, profile-update and session-handoff requests all authorize on it | `doorway/doorway-service/src/routes/identity.rs:410,744-760`; `elohim/elohim-storage/src/api/account.rs:1063`, `api/identity.rs:175,206`; clients `app/elohim-app/src/app/imagodei/services/identity-api.service.ts:100-105`, `hosting-account.service.ts:67`, `app/elohim-app/src/app/account/services/handoff.service.ts:84` |
| 8 | `/apps/` responses carry `Content-Type`, `X-Cache`, CORP `cross-origin`, COEP `credentialless` and caching headers, but **no** `Content-Security-Policy` and no frame restriction. Nothing in `doorway/` or `genesis/manifests` (ingress) adds them. None of the headers present isolate a frame from a same-origin parent | `doorway/doorway-service/src/routes/apps.rs:721,821` |

With `allow-scripts` and `allow-same-origin` on a same-origin document, the framed script can:

- read `localStorage` (token, expiry, signing credentials), `sessionStorage`, IndexedDB and readable cookies;
- reach `window.parent` and the shell's DOM. From there it can also hook the shell's own later requests,
  so moving the token into memory alone would not stop it;
- remove its own `sandbox` attribute, effective on its next navigation;
- send the token anywhere, since there is no CSP and so no `connect-src`. Note that a bare
  `fetch` does *not* attach the bearer automatically: the shell adds `Authorization` explicitly. The
  attacker reads the token and attaches it, which this defect allows.

## Impact, precisely stated

- **Proven:** the viewer's authenticated session is taken over and kept alive by refresh. Storage
  then answers as that person for account, identity, hosting-account and profile writes and for session
  handoff (link 7).
- **Proven where the connection mode stores them:** the agent's private key and capability secret
  (link 5). A token expiry or a session revocation does nothing for these.
- **Not established:** that a stolen *doorway bearer* alone yields unrestricted conductor signing for
  a hosted person. The forwarded identity is proven; a signing-operation trace from a bearer has not
  been read. Do not overstate this in a fix PR.
- **Persistence through the app cache:** the service worker's `CacheTier` (`eprfs-objects`) re-verifies
  every hit (`app/elohim-app/src/apps-sw.ts:109-134`). The **`apps-v3` cache that actually serves these
  frames returns CID and original-URL hits without verification** (`apps-sw.ts:430,451`), and files
  fetched from another peer are served without being checked against the bundle address. The worker's
  own comment says so (`apps-sw.ts:402,475`). So same-origin script can poison later app loads, and *"the
  doorway hash-checked the bundle"* does not prove which bytes ran in the browser.

## Who can trigger it, and who is exposed

- **Trigger:** anyone whose content reaches a viewer through this renderer: an html5-app bundle, or a
  ContentNode whose content or legacy metadata names a same-origin path, or any string reaching the
  scheme-less fallback (link 2). The FCT course is a real html5-app (`genesis/data/lamad/content/fct-course.json:11`).
  Sophia quizzes are *not* html5-apps: they render through `sophia-quiz-json` and the Sophia element in the
  shell document (`app/lamad/src/app/content-io/plugins/sophia/sophia-format.plugin.ts:66`,
  `sophia-element-loader.ts:123`). That is a separate trusted-code surface.
- **Exposed:** every signed-in person who opens such content in the browser app, whether a doorway serves
  it or the dev proxy does. The checked environment and proxy configurations provide no separate origin
  anywhere.
- **Silent:** nothing in the UI changes.

## Not affected, and not yet known

- **Not affected:** `external-embed` (`app/elohim-app/src/app/elohim/components/external-embed/`) uses the
  same flags but only loads allow-listed third-party hosts (`www.youtube.com`, `www.youtube-nocookie.com`).
  A cross-origin frame keeps its own origin and cannot read ours.
- **Unverified (treat as affected until checked):** the Tauri steward. Storage is reachable directly
  (`elohim/elohim-storage/src/http.rs:458`), but this renderer still returns a relative `/apps/...` URL in
  every environment (link 3), so the frame's origin there has not been established.
- **Unverified:** whether doorway sessions can be revoked server-side, which would bound the refresh chain.

## Fix shapes (a design fork; this entry does not choose)

| Shape | What it closes | What it costs or leaves |
|---|---|---|
| **A. Opaque origin.** No `allow-same-origin`. Load verified bundle bytes into a `srcdoc` or `blob:` frame with `sandbox="allow-scripts"`, the napplet NAP web projection. The shell binds every message to the bundle's verified CID through `MessageEvent.source` | Shell storage, DOM, cookies and sandbox removal | The service worker no longer controls the frame, so the shell must fetch, **verify against the bundle CID** (today's `apps-v3` path does not), resolve relative assets and hand the bytes in. Offline needs a complete verified bundle already held, and `apps-v3` holds extracted files, not a verified archive, so migration is needed. A fetched response's CSP does not carry into `srcdoc`/`blob:`. Bundles that use their own storage need a shell-provided store. An opaque frame can still make network requests, so the policy for those is a separate decision |
| **B. Separate apps origin** (for example `apps.<doorway-host>`), keeping the sandbox flags | The shell's origin: its storage, DOM and token | Apps on that origin share **one another's** storage and cookies. A sibling subdomain is usually *same-site* and may share parent-domain cookies, so cookie scoping needs care. Contradicts the same-origin reason in `resolveDoorwayUrl()`, and needs DNS, TLS and a second service-worker scope per doorway |
| **C. Take the secrets out of script reach.** Token in memory or an `HttpOnly; SameSite=Strict` cookie, and signing credentials out of `localStorage` | Direct reads of these credentials | **Not sufficient alone.** Same-origin script can still drive the shell and use the session it carries. It does not need to read a secret it can ride |
| **D. `Content-Security-Policy` on `/apps/`** | A response that carries the header | Covers only doorway responses. SW-synthesized ZIP-extraction responses carry only `Content-Type` (`apps-sw.ts:455,610`), and cached responses bypass the doorway. `CSP: sandbox allow-scripts` forces an opaque origin even on B's separate host, which cancels B's storage benefit. `/apps/` also serves trusted `spa-bundle` apps (`elohim/elohim-storage/src/http.rs:2029-2034`), so the policy needs a per-kind scope decision |

Through our lenses:

- **Local-first.** A keeps an offline path *only if* the shell holds a complete, verified bundle. B
  adds a host-shaped requirement (a second origin) that a person's own node must also provide.
- **The two-portals rule.** An app is a relying party and never holds the person's credentials. A with a
  capability-scoped `postMessage` contract is that rule at app scale: identity is read-only, publishing
  goes through the shell, and storage sits under runtime policy (napplet NAP-IDENTITY / NAP-RELAY; NAP-FS
  is an open proposal, PR #88).

Prior art: the [Nostr Compass survey](epr:nostr-compass-ecosystem-cross-pollination-2026-10-29), Part II
napplets and N7. The `napplet/naps` clone is pinned at `5aadf1b`; line citations there are to that
revision.

## Acceptance (what "fixed" means)

The test is **the absence of shell secrets and of unauthorized server effects**, not that every API
throws.

1. A fixture app, run in **every** renderer mode (html5-app, direct URL, legacy metadata URL, fallback
   string) with a same-origin target in each, attempts the following. None may yield a shell secret or
   change server state:
   - read `localStorage` for the session token and for the stored signing credentials;
   - read `document.cookie`;
   - touch `window.parent.document`;
   - `frameElement.removeAttribute('sandbox')`, then self-navigate and re-read storage;
   - a credentialed request to a real authenticated endpoint (`/api/v1/identity/me`), and the same
     request with an explicitly attached bearer taken from anything the frame can reach.
2. A **positive control** in the same run proves the shell session itself is authenticated, so a pass
   cannot come from a broken session.
3. The test covers every delivery path: doorway-served, SW ZIP-extracted, `apps-v3` cache hit, offline,
   and direct navigation. It is **eyes-on in a real browser** (`pnpm look` or an a2o browser scenario).
   Sandbox semantics are browser behaviour that Angular test hosts do not reproduce.
4. Bundle bytes reaching the frame are verified against the bundle CID on every path, including peer
   fetch and cache hit.
5. A real html5-app (the FCT course) still loads cold and offline, or each bundle that needs shell
   storage is named with its migration.
6. If D is adopted, a contract test pins the header on doorway responses, and the SW-synthesized
   responses get an equivalent guarantee.

## Interim

The sandbox attribute is one static string shared by all four modes, so "drop `allow-same-origin` for
html5-app only" is a restructuring, not a one-line change. Dropping it everywhere breaks SW cold delivery
and any app using its own storage. Measure which seeded content breaks first: a stopgap that silently
breaks the FCT course is its own incident. The fastest true reduction is to stop persisting signing
credentials in `localStorage` (link 5), because those survive any token fix.
