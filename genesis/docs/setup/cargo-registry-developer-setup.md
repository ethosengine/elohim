# Cargo Registry — Developer Setup

The Elohim Protocol monorepo routes Cargo traffic through Nexus at
`nexus.ethosengine.com`:

- `cargo build` reads crates.io packages through the **group** endpoint
  (`/repository/cargo/`), which transparently caches upstream. No per-developer
  setup needed for *reads*.
- `cargo publish --registry elohim` publishes internal crates to the
  **hosted** endpoint (`/repository/cargo-internal/`). Nexus only accepts
  publishes on hosted repos — group endpoints are read-only.
- Consumers of internal crates declare `registry = "elohim"` in their
  `Cargo.toml` and resolve directly against `cargo-internal`.

The repo-level `.cargo/config.toml` (checked in at the repo root) configures
both. You do NOT need to modify your global cargo config.

## When you need credentials

You only need a credential for `cargo publish`. `cargo build` and `cargo fetch`
work anonymously against the group endpoint.

## Credential setup

Three options, in order of preference:

### Option A — Use your existing Nexus npm token (preferred)

If you already have `NPM_TOKEN` set (any contributor working with the existing
Nexus instance does), the same token works for Cargo — Nexus uses the same
user-token system across all repository formats.

Set `CARGO_REGISTRIES_ELOHIM_TOKEN` in your shell:

```bash
export CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer ${NPM_TOKEN}"
```

Persist by appending to your shell rc file. Recommended because it doesn't put
a file on disk.

### Option B — File-based credentials

If you prefer the cargo-standard file-based approach:

```bash
CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
mkdir -p "$CARGO_HOME"
cat > "$CARGO_HOME/credentials.toml" <<EOF
[registries.elohim]
token = "Bearer ${NPM_TOKEN}"
EOF
chmod 600 "$CARGO_HOME/credentials.toml"
```

### Option C — Get a fresh token

If you don't have NPM_TOKEN:

1. Browse to `https://nexus.ethosengine.com`
2. Sign in
3. Top right avatar → **My Account → User Token → Access User Token**
4. Authenticate with your Nexus password
5. Combine `NameCode` and `PassCode`, base64:
   ```bash
   echo -n 'NameCode:PassCode' | base64
   ```
6. Set:
   ```bash
   export CARGO_REGISTRIES_ELOHIM_TOKEN="Basic <base64-output>"
   ```

## Verifying setup

```bash
curl -sk -o /dev/null -w "HTTP %{http_code}\n" \
  -H "Authorization: $CARGO_REGISTRIES_ELOHIM_TOKEN" \
  https://nexus.ethosengine.com/service/rest/v1/status
```

Expected: `HTTP 200`. If 401, your token is wrong or expired.

## Publishing an internal crate

```bash
cd elohim/<your-crate>
cargo publish --registry elohim --dry-run  # always dry-run first
cargo publish --registry elohim            # actual publish
```

The crate's `Cargo.toml` must declare `publish = ["elohim"]` (a whitelist that
prevents accidental crates.io publishes) and the required `[package]` fields
(`description`, `license`, `repository`).

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `cargo build` fails on `serde` fetch | Proxy is down; check Nexus dashboard |
| `cargo publish` returns 401 | Token expired or not authorized for `cargo-internal` |
| `cargo publish` returns 409 (Conflict) | Version already published; bump version |
| Cargo fetches from `crates.io` directly | `.cargo/config.toml` not picked up; check you're in the repo |
| `cargo search` returns 404 | Nexus's cargo plugin doesn't implement the search API endpoint; this is expected, build/fetch still work via the sparse-protocol endpoints |
| `\u{0}` token errors or "unclosed delimiter" in builds | sccache cache corruption — see `.claude/memory/feedback_sccache_cache_corruption_recovery.md` |
