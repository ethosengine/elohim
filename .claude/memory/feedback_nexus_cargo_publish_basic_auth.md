---
name: nexus-cargo-publish-basic-auth
description: "Nexus 3.x Cargo hosted publish requires HTTP Basic auth, not the Bearer/NpmToken format cargo's default credential provider sends. NPM_TOKEN reads succeed; PUT /api/v1/crates/new returns 401 with www-authenticate: BASIC realm. Resolving needs either the Nexus user-token Pass Code (so Basic auth can be constructed) or a Nexus-side config change to accept token-format publishes. Verified 2026-05-18 on cargo-internal."
metadata:
  node_type: memory
  type: feedback
---

When publishing to a Nexus 3.x Cargo hosted repository, `cargo publish` will fail with HTTP 401 even though the same token reads the sparse index successfully. This memory captures the specific shape of the failure and the recovery paths so future attempts don't re-walk the same wall.

## Verified findings

### Symptom

```
$ CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer $NPM_TOKEN" cargo publish --registry elohim
...
   Packaged 37 files, 141.4KiB ...
error: failed to publish to registry at https://nexus.ethosengine.com/repository/cargo-internal/

Caused by:
  the remote server responded with an error (status 401 Unauthorized): the server returned an unexpected response
```

The `www-authenticate` header on the 401 reads `BASIC realm="Sonatype Nexus Repository Manager"` — Nexus is asking for HTTP Basic auth, not a Bearer/Token header.

### Read vs write asymmetry

The same NPM_TOKEN that authenticates Nexus npm/cargo *reads* does NOT authenticate Cargo *publish*:

| Operation | Endpoint | Header that works |
|---|---|---|
| Sparse-index read | `GET /repository/cargo-internal/el/oh/elohim-epr` | `Authorization: NpmToken.<token>` (raw, no Bearer prefix) |
| Crate publish | `PUT /repository/cargo-internal/api/v1/crates/new` | `Authorization: Basic base64(username:passcode)` |

Nexus's user-token system gives two parts: a Name Code (substitute username) and a Pass Code (substitute password). Reads accept the Pass Code as a raw NpmToken header; publishes require both parts encoded as Basic auth.

### What cargo's default credential providers send

| Provider | Sends |
|---|---|
| `cargo:token` (default) | `Authorization: Bearer <token-from-credentials.toml>` |
| `cargo:libsecret` | same (Bearer) but pulled from keyring |
| `cargo:basic-auth` | NOT a built-in; would need a custom provider |

There is no built-in cargo credential provider that emits Basic auth. So `cargo publish --registry <nexus-cargo-hosted>` cannot succeed against an out-of-the-box Nexus 3.x install without one of the workarounds below.

### Recovery procedures

**Option A — Construct Basic auth in `~/.cargo/credentials.toml`** (operator must supply the Name Code)

```toml
[registries.elohim]
token = "Basic <base64(NameCode:PassCode)>"
```

Cargo passes the `token` value through as the literal `Authorization:` header value — so the `Basic <base64>` form is sent verbatim. This works if and only if the operator can supply the Nexus user-token Name Code. (The Pass Code alone, as raw NpmToken, is what the NPM_TOKEN env var holds — that's the read-side credential.)

To retrieve both parts: in the Nexus UI, log in as the publishing user → top-right user menu → "Account" → "User Token" → "Access user token". The dialog shows `Name Code` and `Pass Code`. Then:

```bash
echo -n "$NEXUS_NAMECODE:$NEXUS_PASSCODE" | base64 -w0
```

**Option B — Nexus admin enables a Cargo-publish-accepting auth realm** (operator-side, not local)

Nexus 3.x has an experimental "Cargo Bearer Token" realm in some recent versions. Enabling it in Administration → Security → Realms allows `Authorization: Bearer <token>` for publish too. This is a one-time cluster-wide change.

**Option C — Defer publishing entirely; use path-deps until publish-auth is resolved**

For workstreams that don't strictly need a published crate (workspace dep unification, dedupe, baseline measurements, etc.), continue using `path = "../epr"` declarations and skip the registry hop. This is what T10 (switch one consumer to registry dep) would validate — but T10 has no hard dependency on T11–T15 below it in the plan, so the rest of Phase 1 can proceed.

## What does NOT work (verified)

- `CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer $NPM_TOKEN"` → 401 (Bearer not accepted on publish)
- `CARGO_REGISTRIES_ELOHIM_TOKEN="$NPM_TOKEN"` (raw, no prefix) → 401 (still sent as Bearer by cargo:token provider)
- `cargo:token` credential provider in any configuration → always Bearer, never Basic
- Setting `Authorization: Token <token>` via custom env hooks — `cargo publish` ignores arbitrary env vars; only the `CARGO_REGISTRIES_<NAME>_TOKEN` env var feeds the provider

## How to apply

1. **When you see `401` + `www-authenticate: BASIC realm` on cargo publish to a Nexus hosted repo**, stop trying token-format variations — the realm itself rejects Bearer.
2. **Default workaround**: ask the operator for the Nexus user-token Name Code, then construct Option A's Basic-auth `credentials.toml` entry.
3. **Long-term**: ask the Nexus admin to enable the Cargo Bearer Token realm (Option B) so the default `cargo:token` provider works for every contributor without a manual Name+Pass dance.
4. **Don't memorialize speculative fixes**: there is no known way to make `cargo publish` against Nexus 3.x send Basic auth without either (a) the Name Code in hand or (b) the admin-side realm change. Both require operator action.

Related: T9/T10 of `genesis/docs/plans/2026-05-17-cargo-registry-and-compilation-load-reduction.md` deferred 2026-05-18 pending this auth resolution. `[[sccache-cache-corruption-recovery]]` for the sibling sccache wall hit on the same plan.
