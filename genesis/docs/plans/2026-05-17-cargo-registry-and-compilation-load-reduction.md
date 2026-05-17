# Cargo Registry + Compilation Load Reduction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repave the poisoned sccache bucket, stand up the cargo client for the existing Nexus registry trio (`cargo-proxy` / `cargo-internal` / `cargo`), publish the first internal crate (`elohim-epr`) through it, switch one downstream consumer to versioned dependency, then execute workspace dependency unification + transitive dedupe to bring the recurring 118G PVC pressure under control.

**Architecture:**
- **Nexus cargo trio already exists** at `nexus.ethosengine.com`: `cargo-proxy` (caches `https://index.crates.io/`), `cargo-internal` (empty, ready for publishing), `cargo` (group fronting both). Smoke-tested pre-plan — anonymous read works, `Authorization: Bearer $NPM_TOKEN` authenticates.
- **sccache poisoned first** because every subsequent build phase pays the corruption tax until it's repaved. Doing this in Phase 1 means later phases benefit immediately.
- **Cargo client config** lives at the repo root (`.cargo/config.toml`) — every developer + every CI runner uses the proxy automatically via `[source.crates-io] replace-with`. Credentials never get committed; they use `$CARGO_HOME/credentials.toml` (per-user) or `CARGO_REGISTRIES_ELOHIM_TOKEN` (CI env var).
- **First publish:** `elohim-epr` is the clean candidate — already extracted (`elohim/epr/`), small (~700 LOC), tests passing, clear downstream consumers to validate against.
- **Workspace dep unification** is the biggest single lever for compile-load reduction — `elohim/Cargo.toml`'s `[workspace.dependencies]` table is currently empty; populating it forces version unity within the workspace and eliminates separately-compiled dupes.

**Tech Stack:** Cargo 1.78+ (sparse registries), Nexus Repository 3.83+ (native Cargo format), `sccache` with S3 backend, Rust 1.78+, `cargo tree`.

---

## File Structure

This plan creates/modifies:

- **Create:** `.cargo/config.toml` (repo root) — Cargo client config; redirects crates.io traffic through the Nexus proxy + declares the `elohim` registry for publishing.
- **Create:** `genesis/docs/setup/cargo-registry-developer-setup.md` — one-page developer onboarding doc.
- **Create:** `.claude/memory/feedback_sccache_repave_procedure.md` — recovery procedure as durable feedback memory.
- **Create:** `genesis/docs/measurements/2026-05-17-compilation-load-baseline.md` — baseline numbers that the structural refactor sprint will measure against.
- **Modify:** `elohim/epr/Cargo.toml` — `[package]` metadata required for publishing.
- **Modify:** `elohim/Cargo.toml` — populate `[workspace.dependencies]`.
- **Modify:** Each workspace member `Cargo.toml` — switch common deps to `{ workspace = true }`.
- **Modify:** One downstream consumer's `Cargo.toml` (chosen in Task 9) — switch path-dep to registry-dep.

Multi-workspace unification (bringing `elohim-storage`, `elohim-cache-core`, `elohim/holochain`, `rust-ipfs`, `sdk` into the elohim/ workspace) is OUT OF SCOPE — see `2026-05-17-structural-refactor-sprint.md` for that work.

---

## Phase 1 — sccache Repave

The cache is currently producing null-byte output (caught yesterday on the graph-native push and again on the elohim-epr gate). Every build in subsequent phases pays this tax until the bucket is repaved. Doing this first makes everything that follows faster.

### Task 1: Confirm sccache is configured

**Files:** none (verification)

- [ ] **Step 1: Check the sccache config**

Run:
```bash
echo "SCCACHE_ENDPOINT=$SCCACHE_ENDPOINT"
echo "SCCACHE_BUCKET=${SCCACHE_BUCKET:-<unset>}"
echo "SCCACHE_REGION=${SCCACHE_REGION:-<unset>}"
sccache --show-config 2>&1 | head -10
```

Expected: S3 endpoint and bucket are set. If unset, sccache won't actually cache to S3 and there's nothing to repave — surface the gap to the operator before proceeding.

- [ ] **Step 2: Capture pre-repave stats**

Run:
```bash
sccache --show-stats 2>&1 | grep -E "Cache hits|Cache misses|Cache size|Errors"
```

Note the numbers. The "Cache errors" count may be elevated — that's the poisoning we're about to fix.

### Task 2: Repave the bucket

**Files:** none (network operation)

- [ ] **Step 1: Force-rewrite cache entries on a substantial build**

`SCCACHE_RECACHE=1` tells sccache to ignore read cache and write fresh entries, replacing any poisoned blobs at the same content-addressed keys.

Run:
```bash
cd /projects/elohim/elohim && \
  SCCACHE_RECACHE=1 RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build --workspace --release 2>&1 | tail -10
```

Expected: a slow build (no cache reads) but every compile unit produces a fresh cache entry. Budget 20-40 min depending on machine spec.

- [ ] **Step 2: Confirm fresh entries are flowing during the build**

Run periodically while the build is going (or after it finishes):
```bash
sccache --show-stats 2>&1 | grep -E "Compile requests|Cache hits|Cache misses|Cache writes"
```

Expected: `Cache writes` is climbing steadily. `Cache misses` is high (because RECACHE ignored reads). After the build, this is the new baseline.

- [ ] **Step 3: Verify read-back works**

Run a second build without RECACHE to confirm the new entries read cleanly:
```bash
cd /projects/elohim/elohim && \
  cargo clean -p elohim-epr && \
  RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build -p elohim-epr 2>&1 | tail -5
sccache --show-stats 2>&1 | grep -E "Cache hits|Cache misses"
```

Expected: build is fast; `Cache hits` climbs. If the build produces null-byte errors again, the poisoning hasn't been fully cleared — re-run Step 1 with a wider scope (full release, all features).

### Task 3: Capture the recovery procedure as feedback memory

**Files:**
- Create: `/projects/elohim/.claude/memory/feedback_sccache_repave_procedure.md`

- [ ] **Step 1: Write the memory**

```bash
cat > /projects/elohim/.claude/memory/feedback_sccache_repave_procedure.md <<'EOF'
---
name: sccache-repave-procedure
description: "When sccache cache becomes poisoned (null bytes in compile output, clippy 'unclosed delimiter' errors pointing at cached source positions), repave the S3 bucket via SCCACHE_RECACHE=1 on a full release build. Don't wipe the bucket; force-rewrite. RUSTC_WRAPPER='' bypasses sccache entirely as an in-band escape hatch but doesn't repair the cache."
metadata:
  node_type: memory
  type: feedback
---

When sccache hits cache poisoning — symptoms include null bytes in compile error output, clippy errors of the form `error: this file contains an unclosed delimiter` pointing at cached source positions, or build failures that disappear when `RUSTC_WRAPPER=""` is set — the corrupted cache entries need to be rewritten.

**Recovery procedure:**

```bash
cd <crate-with-broad-coverage>  # e.g. elohim/ workspace root
SCCACHE_RECACHE=1 RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build --workspace --release
```

The `SCCACHE_RECACHE=1` env var tells sccache to ignore read cache and write fresh entries. Every compile unit hit during the build replaces its corrupted cache entry with a clean one.

**Why this works:** sccache's S3 backend is content-addressed by hash of `(toolchain, source, env)`. A poisoned entry has the right key but corrupted content. RECACHE forces a re-upload that overwrites the bad blob at the same key.

**Why NOT to wipe the bucket:** other developers + CI agents are reading from the same bucket. A wipe creates a cold-cache cliff for everyone; a recache only updates the entries you actually compile, keeping the rest warm.

**Escape hatch:** `RUSTC_WRAPPER=""` bypasses sccache entirely for a single invocation. Use this when you need to ship a build NOW and the cache is too compromised to recover from in-band. Pair it with SCCACHE_RECACHE=1 on the NEXT clean build.

Related: 2026-05-17 graph-native sprint where poisoning hid the real elohim-epr fmt drift + RUSTFLAGS gotcha behind unparseable null-byte output. Procedure applied successfully during the cargo-registry rollout.
EOF
```

- [ ] **Step 2: Index in MEMORY.md**

```bash
echo "- [sccache repave procedure](feedback_sccache_repave_procedure.md) — SCCACHE_RECACHE=1 on clean build repaves poisoned entries; RUSTC_WRAPPER='' is escape hatch only; never wipe the shared bucket." >> /projects/elohim/.claude/memory/MEMORY.md
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim && git add .claude/memory/feedback_sccache_repave_procedure.md .claude/memory/MEMORY.md && git commit -m "$(cat <<'EOF'
docs(memory): sccache repave procedure

Captures the recovery procedure for sccache cache poisoning surfaced
during the 2026-05-17 graph-native sprint and applied successfully
during the cargo-registry rollout. SCCACHE_RECACHE=1 on a clean build
repaves the corrupted entries without disturbing the rest of the
shared bucket; RUSTC_WRAPPER="" is the in-band escape hatch.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2 — Cargo Client Configuration

### Task 4: Create the repo-level Cargo config

**Files:**
- Create: `/projects/elohim/.cargo/config.toml`

- [ ] **Step 1: Create the config file**

```toml
# .cargo/config.toml — Cargo client configuration for the Elohim Protocol monorepo.
#
# This config redirects ALL crates.io traffic through the Nexus group endpoint at
# nexus.ethosengine.com/repository/cargo/, which caches upstream crates and also
# serves our internal published crates (e.g. elohim-epr).
#
# Authentication: NEVER commit credentials. Use one of:
#   1. Per-user file: $CARGO_HOME/credentials.toml (chmod 600) — see Task 5.
#   2. CI env var: CARGO_REGISTRIES_ELOHIM_TOKEN="Bearer NpmToken.<rest>"
#
# Anonymous read is enabled on the group, so `cargo build` works without credentials.
# Credentials are only needed for `cargo publish`.

[registries.elohim]
index = "sparse+https://nexus.ethosengine.com/repository/cargo/"

[source.crates-io]
replace-with = "elohim-mirror"

[source.elohim-mirror]
registry = "sparse+https://nexus.ethosengine.com/repository/cargo/"
```

- [ ] **Step 2: Verify the config is picked up by cargo**

Run:
```bash
cd /projects/elohim && RUSTC_WRAPPER="" RUSTFLAGS="" cargo search serde --registry elohim 2>&1 | head -5
```

Expected output (approximate):
```
serde = "1.0.X"           # A generic serialization/deserialization framework
serde_json = "1.0.X"      # A JSON serialization file format
...
```

If you see `error: failed to query registry`, the config path or URL is wrong. If you see results, the proxy is fetching correctly through the new config.

- [ ] **Step 3: Commit the config**

```bash
cd /projects/elohim && git add .cargo/config.toml && git commit -m "$(cat <<'EOF'
feat(cargo): repo-level Cargo config redirecting crates.io through Nexus

Adds .cargo/config.toml at the repo root with:
- [registries.elohim] declaring the Nexus group as a named registry for
  publishing internal crates
- [source.crates-io] replace-with redirecting all crates.io traffic
  through the same Nexus group endpoint (transparent caching)

Anonymous read is enabled on the Nexus group, so `cargo build` works
without credentials. Auth is only needed for `cargo publish`; see
genesis/docs/setup/cargo-registry-developer-setup.md (created in
Task 7) for the credential setup pattern.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5: Set up per-user credentials

**Files:**
- Create: `$CARGO_HOME/credentials.toml` (NOT in repo)

- [ ] **Step 1: Locate CARGO_HOME**

Run:
```bash
echo "CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}"
```

Note the path.

- [ ] **Step 2: Write credentials.toml**

Run:
```bash
CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
mkdir -p "$CARGO_HOME"
cat > "$CARGO_HOME/credentials.toml" <<EOF
[registries.elohim]
token = "Bearer ${NPM_TOKEN}"
EOF
chmod 600 "$CARGO_HOME/credentials.toml"
```

- [ ] **Step 3: Verify auth works**

Run:
```bash
curl -sk -o /dev/null -w "HTTP %{http_code}\n" -H "Authorization: Bearer $NPM_TOKEN" \
  https://nexus.ethosengine.com/service/rest/v1/status
```

Expected: `HTTP 200`. If 401, regenerate the token via Nexus → My Account → User Token.

### Task 6: Verify the proxy is doing real work

**Files:** none (verification only)

- [ ] **Step 1: Clear any local registry cache**

```bash
rm -rf "${CARGO_HOME:-$HOME/.cargo}/registry/cache/index.crates.io-*"
rm -rf "${CARGO_HOME:-$HOME/.cargo}/registry/src/index.crates.io-*"
```

Forces cargo to re-fetch through the Nexus proxy.

- [ ] **Step 2: Run a fresh fetch via the proxy**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER="" RUSTFLAGS="" cargo fetch 2>&1 | tail -20
```

Expected: fetch completes without errors.

- [ ] **Step 3: Confirm the network path goes through Nexus**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER="" RUSTFLAGS="" cargo fetch --verbose 2>&1 | grep -E "Downloading|cargo/|nexus" | head -10
```

Expected: any HTTP fetch lines reference `nexus.ethosengine.com`. If you see `crates.io` directly, the `[source.crates-io] replace-with` redirect isn't taking effect — re-check `.cargo/config.toml` path and contents.

### Task 7: Write the developer onboarding doc

**Files:**
- Create: `genesis/docs/setup/cargo-registry-developer-setup.md`

- [ ] **Step 1: Create the doc**

```markdown
# Cargo Registry — Developer Setup

The Elohim Protocol monorepo proxies crates.io through a Nexus repository at
`nexus.ethosengine.com/repository/cargo/`. This means:

- `cargo build` automatically goes through the proxy — no per-developer setup
  needed for *reads*.
- `cargo publish --registry elohim` publishes internal crates to the
  `cargo-internal` hosted repo behind the same group endpoint.

The repo-level `.cargo/config.toml` (checked in at the repo root) configures
the proxy redirect. You do NOT need to modify your global cargo config.

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
```

- [ ] **Step 2: Commit**

```bash
cd /projects/elohim && git add genesis/docs/setup/cargo-registry-developer-setup.md && git commit -m "$(cat <<'EOF'
docs(setup): cargo registry developer onboarding

One-page guide for contributors covering the three credential options
(NPM_TOKEN reuse via env var, file-based credentials, fresh token from
Nexus dashboard), verification, publish workflow, and troubleshooting.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3 — First Publish: elohim-epr

### Task 8: Prepare elohim-epr/Cargo.toml for publishing

**Files:**
- Modify: `/projects/elohim/elohim/epr/Cargo.toml`

- [ ] **Step 1: Read the current Cargo.toml**

```bash
cat /projects/elohim/elohim/epr/Cargo.toml
```

Note what's already present in `[package]`.

- [ ] **Step 2: Add required publish metadata**

Edit `/projects/elohim/elohim/epr/Cargo.toml` so the `[package]` section contains, at minimum:

```toml
[package]
name = "elohim-epr"
version = "0.1.0"
edition = "2021"
description = "Elohim Protocol canonical EPR codec — CBOR + CIDv1 + Ed25519 for the graph substrate"
license = "Apache-2.0"
repository = "https://github.com/ethosengine/elohim"
readme = "README.md"
keywords = ["elohim", "epr", "cbor", "cid", "p2p"]
categories = ["data-structures", "encoding", "cryptography"]
publish = ["elohim"]
```

Keep any existing fields (e.g. `authors`, `workspace.package` inheritance markers).

Crucial: **`publish = ["elohim"]`** is a safety whitelist — prevents accidental crates.io publishes. The list MUST match the registry name in `.cargo/config.toml`.

- [ ] **Step 3: Verify manifest parses and tests pass**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build -p elohim-epr 2>&1 | tail -5
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo test -p elohim-epr --lib 2>&1 | tail -10
```

Expected: build succeeds, tests pass.

- [ ] **Step 4: Dry-run packaging**

```bash
cd /projects/elohim/elohim/epr && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo package --registry elohim --no-verify --allow-dirty 2>&1 | tail -20
```

Expected: cargo writes `target/package/elohim-epr-0.1.0.crate` and prints the included files. No errors.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add elohim/epr/Cargo.toml && git commit -m "$(cat <<'EOF'
chore(epr): prepare Cargo.toml for publishing to elohim registry

Adds [package] metadata required for cargo publish: description, license,
repository, readme, keywords, categories. Most importantly:
publish = ["elohim"] — a safety whitelist that PREVENTS accidental
publishes to crates.io. First internal release will be 0.1.0.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 9: Publish elohim-epr 0.1.0

**Files:** none (network operation)

- [ ] **Step 1: Final dry-run**

```bash
cd /projects/elohim/elohim/epr && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo publish --registry elohim --dry-run 2>&1 | tail -25
```

Expected: `Verifying elohim-epr v0.1.0`, package contents listed, `Uploading elohim-epr v0.1.0` followed by dry-run-skip. No errors.

- [ ] **Step 2: Real publish**

Ensure `CARGO_REGISTRIES_ELOHIM_TOKEN` is set OR `~/.cargo/credentials.toml` is in place. Then:

```bash
cd /projects/elohim/elohim/epr && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo publish --registry elohim 2>&1 | tail -10
```

Expected: `Uploaded elohim-epr v0.1.0 to registry elohim`. If 401, re-check auth (Task 5).

- [ ] **Step 3: Verify the published crate is fetchable**

```bash
curl -sk https://nexus.ethosengine.com/repository/cargo/el/oh/elohim-epr 2>&1 | head -3
```

Expected: JSON line with `"name":"elohim-epr","vers":"0.1.0",...` and cksum. If 404, the publish didn't land.

- [ ] **Step 4: Verify cargo can resolve it from a clean context**

```bash
cd /tmp && rm -rf cargo-publish-smoke && mkdir cargo-publish-smoke && cd cargo-publish-smoke && cargo init --lib && \
  cat >> Cargo.toml <<'EOF'

[dependencies]
elohim-epr = { version = "0.1.0", registry = "elohim" }
EOF
RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo fetch 2>&1 | tail -5
```

Expected: cargo fetches `elohim-epr v0.1.0`. The `Locking` line mentions it. If `error: failed to select a version`, the registry mapping is wrong.

Clean up: `rm -rf /tmp/cargo-publish-smoke`

---

## Phase 4 — Switch One Downstream Consumer

### Task 10: Identify and switch the cleanest downstream

**Files:**
- Modify: `<chosen-crate>/Cargo.toml`

- [ ] **Step 1: Find all path-deps on elohim-epr**

```bash
cd /projects/elohim && grep -rn "elohim-epr.*path\s*=" --include="Cargo.toml" 2>/dev/null
```

- [ ] **Step 2: Pick the switch target**

Decision rules:
- Prefer a consumer NOT itself heavily depended on
- Prefer a consumer in the same workspace as `elohim-epr` (workspace resolver helps)
- AVOID switching `elohim-storage` first (largest consumer; would multiply build risk)

Likely candidate: a leaf consumer or a sibling test crate. Note your choice here:

The choice is: `_______________________________________________________`

- [ ] **Step 3: Edit the dep**

In the chosen crate's `Cargo.toml`, change:
```toml
elohim-epr = { path = "../epr" }
```
to:
```toml
elohim-epr = { version = "0.1.0", registry = "elohim" }
```

Preserve any features:
```toml
elohim-epr = { version = "0.1.0", registry = "elohim", features = ["..."] }
```

- [ ] **Step 4: Build the consumer**

```bash
cd /projects/elohim/<chosen-crate-dir> && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build 2>&1 | tail -10
```

Expected: cargo downloads `elohim-epr v0.1.0` from Nexus, build succeeds. If `error: failed to load source for dependency`, the registry is misconfigured.

- [ ] **Step 5: Run the consumer's tests**

```bash
cd /projects/elohim/<chosen-crate-dir> && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo test 2>&1 | tail -10
```

Expected: all tests pass. Behavior should be identical to the path-dep version.

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim && git add <chosen-crate-dir>/Cargo.toml && git commit -m "$(cat <<'EOF'
chore(<crate>): switch elohim-epr from path dep to registry dep (v0.1.0)

First downstream consumer to switch from path = "../epr" to
version = "0.1.0", registry = "elohim", validating the Nexus
publish + fetch loop end-to-end.

Tests pass; behavior identical (same crate bytes, different cargo source).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5 — Workspace Dependencies Unification

### Task 11: Audit which deps appear in 2+ members

**Files:** none (research)

- [ ] **Step 1: Extract unique deps per member**

```bash
cd /projects/elohim/elohim
for member in constitution elohim-agent/elohim-agent-service elohim-agent/gate-client elohim-agent/gate-types elohim-agent/gate-client-zome elohim-agent/specialists/defender eae elohim-compute elohim-render elohim-token epr; do
  echo "=== $member ==="
  awk '/^\[dependencies\]/,/^\[/' "$member/Cargo.toml" 2>/dev/null | grep -E "^[a-z]" | sed 's/=.*//; s/ //g' | sort -u
done > /tmp/elohim-member-deps.txt
cat /tmp/elohim-member-deps.txt
```

- [ ] **Step 2: Find deps used by 2+ members**

```bash
cd /projects/elohim/elohim
awk '/===/{name=$2; next} {print $0}' /tmp/elohim-member-deps.txt | sort | uniq -c | sort -rn | awk '$1 >= 2 {print}' | head -30
```

Top candidates: likely `serde`, `serde_json`, `tokio`, `anyhow`, `thiserror`, `tracing`, `chrono`, plus internal `path = "..."` deps.

Note the top 10-15 most-shared deps.

### Task 12: Populate [workspace.dependencies]

**Files:**
- Modify: `/projects/elohim/elohim/Cargo.toml`
- Modify: each affected workspace member `Cargo.toml`

- [ ] **Step 1: Determine target versions**

For each candidate from Task 11: target = highest version any member currently uses (unless that creates compat issues).

- [ ] **Step 2: Edit elohim/Cargo.toml's [workspace.dependencies]**

Open `/projects/elohim/elohim/Cargo.toml`, find `[workspace.dependencies]` (currently empty). Populate:

```toml
[workspace.dependencies]
# --- Serialization & data ---
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_bytes = "0.11"

# --- Async runtime ---
tokio = { version = "1.0", features = ["full"] }

# --- Error handling ---
anyhow = "1.0"
thiserror = "1.0"

# --- Logging ---
tracing = "0.1"

# --- Date/time ---
chrono = "0.4"

# --- Internal crates (path-deps within workspace) ---
elohim-epr = { path = "epr" }
constitution = { path = "constitution" }
elohim-compute = { path = "elohim-compute" }
# ... add any internal pulled into 2+ members

# --- Cryptography & CID ---
cid = "0.11"
multihash = "0.19"
ed25519-dalek = "2.1"
```

Use the actual version numbers from Step 1.

- [ ] **Step 3: Switch each member to workspace deps**

For each member that uses one of these deps, change its `Cargo.toml`:

Before:
```toml
[dependencies]
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.140"
tokio = { version = "1.0", features = ["full"] }
```

After:
```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
```

Features added on top of workspace defaults go in the member:
```toml
tokio = { workspace = true, features = ["macros"] }
```

- [ ] **Step 4: Verify each member builds**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build --workspace 2>&1 | tail -15
```

Expected: all members build clean. If a member fails, the workspace dep is missing features that member needs — extend the workspace `features` list.

- [ ] **Step 5: Verify dep tree got smaller**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER=sccache RUSTFLAGS="" cargo tree -d --workspace 2>&1 | head -30
```

Compare against the pre-change output (Task 11 baseline). Expected: significantly fewer dupe pairs.

- [ ] **Step 6: Run tests**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo test --workspace --lib 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /projects/elohim && git add elohim/Cargo.toml elohim/*/Cargo.toml && git commit -m "$(cat <<'EOF'
chore(workspace): unify common deps into [workspace.dependencies]

Populates the previously-empty [workspace.dependencies] table in
elohim/Cargo.toml with deps used by 2+ workspace members: serde,
serde_json, tokio, anyhow, thiserror, tracing, chrono, plus internal
crate path-deps.

Each member switched from per-crate version pins to { workspace = true }.
Features added on top of workspace defaults stay at the member level.

Effect: forces single-version compilation per dep within elohim/.
cargo tree -d --workspace shows significantly fewer dupe pairs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6 — Transitive Dedupe Pass

### Task 13: Baseline duplicates

**Files:** none (research)

- [ ] **Step 1: Capture duplicates report**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER=sccache RUSTFLAGS="" cargo tree -d --workspace 2>&1 > /tmp/cargo-dupes-baseline.txt
wc -l /tmp/cargo-dupes-baseline.txt
head -40 /tmp/cargo-dupes-baseline.txt
```

- [ ] **Step 2: Identify actionable cases**

For each dupe pair, run `cargo tree -i` to find consumers:

```bash
cd /projects/elohim/elohim
for crate in bit-set bit-vec block-buffer cpufeatures crypto-common digest; do
  echo "=== $crate (all versions) ==="
  RUSTC_WRAPPER=sccache RUSTFLAGS="" cargo tree -i $crate 2>&1 | head -20
  echo ""
done
```

Classify each:
- **Pinned by direct dep we control** → actionable; bump our dep
- **Pinned by transitive of Holochain/libp2p/iroh** → less actionable; might bump parent crate
- **Pinned by old internal crate** → bump the internal crate

### Task 14: Resolve actionable duplicates

**Files:**
- Modify: `/projects/elohim/elohim/Cargo.toml` and affected members

For each actionable dupe from Task 13:

- [ ] **Step 1: Bump the version**

Workspace dep:
```toml
some-crate = "0.NEW"  # was "0.OLD"
```

Member-only dep: edit that member's `Cargo.toml` directly.

- [ ] **Step 2: Update Cargo.lock**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER=sccache RUSTFLAGS="" cargo update 2>&1 | tail -10
```

- [ ] **Step 3: Verify workspace builds**

```bash
cd /projects/elohim/elohim && RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build --workspace 2>&1 | tail -10
```

Expected: build succeeds. If an API break, revert that specific bump and accept the dupe.

- [ ] **Step 4: Verify dupes count is lower**

```bash
cd /projects/elohim/elohim && RUSTC_WRAPPER=sccache RUSTFLAGS="" cargo tree -d --workspace 2>&1 | wc -l
```

Compare against baseline. The number should drop.

- [ ] **Step 5: Commit each round**

```bash
cd /projects/elohim && git add elohim/Cargo.toml elohim/Cargo.lock && git commit -m "$(cat <<'EOF'
chore(deps): dedupe transitive dep versions — round 1

Bumps the following deps to eliminate duplicates from
`cargo tree -d --workspace`:

- <list of bumps>

Each separately-compiled dupe is a multiplier on target/ size and
compile time. This round reduces dupe count from N to M.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Repeat for additional rounds. Stop when remaining dupes are all forced by transitives you can't control.

---

## Phase 7 — Measurement & Verification

### Task 15: Capture the new baseline

**Files:**
- Create: `genesis/docs/measurements/2026-05-17-compilation-load-baseline.md`

- [ ] **Step 1: Cold build (no cache)**

```bash
cd /projects/elohim/elohim && \
  cargo clean && \
  time RUSTFLAGS="" RUSTC_WRAPPER="" cargo build --workspace --release 2>&1 | tail -3
```

Capture wall time.

- [ ] **Step 2: Warm-cache build**

```bash
cd /projects/elohim/elohim && \
  cargo clean && \
  time RUSTFLAGS="" RUSTC_WRAPPER=sccache cargo build --workspace --release 2>&1 | tail -3
sccache --show-stats 2>&1 | grep -E "Cache hits|Cache misses"
```

Capture wall time and hit rate.

- [ ] **Step 3: target/ size**

```bash
du -sh /projects/.cargo-target-pool/family/*/elohim__elohim-*/release 2>&1
df -h /projects | tail -1
```

- [ ] **Step 4: Write the measurements doc**

```bash
mkdir -p genesis/docs/measurements
cat > genesis/docs/measurements/2026-05-17-compilation-load-baseline.md <<EOF
# Compilation Load — 2026-05-17 baseline after cargo-registry rollout

Captured after Phases 1-6 of plan
\`2026-05-17-cargo-registry-and-compilation-load-reduction.md\`.

## Configuration

- Cargo proxy at \`https://nexus.ethosengine.com/repository/cargo/\`
- Workspace deps unified across \`elohim/\` workspace
- sccache S3 backend repaved
- Workspace member count: 11
- Internal crates published: 1 (elohim-epr 0.1.0)

## Measurements

| Metric | Value |
|---|---|
| Cold build wall time (elohim/ workspace, release, no sccache) | _FILL IN_ |
| Warm sccache build wall time (release) | _FILL IN_ |
| sccache hit rate (warm build) | _FILL IN_ |
| target/ size (release, elohim-storage slot, both families) | _FILL IN_ |
| cargo tree -d --workspace duplicate pairs (post-unification) | _FILL IN_ |
| Total PVC usage at end of plan | _FILL IN_ |

## Methodology

- Cold: \`cargo clean\` then build with \`RUSTC_WRAPPER=""\`
- Warm: \`cargo clean\` then build with \`RUSTC_WRAPPER=sccache\` on warm bucket
- Dupes: \`cargo tree -d --workspace | wc -l\`
- PVC: \`df -h /projects | tail -1\`

## Next steps

Refactor sprint per
\`genesis/docs/plans/2026-05-17-structural-refactor-sprint.md\` —
sibling-module decomposition, SDK boundary, utility-crate extractions,
feature gating, optional multi-workspace unification.
EOF
```

Fill in the `_FILL IN_` values from Steps 1-3.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add genesis/docs/measurements/2026-05-17-compilation-load-baseline.md && git commit -m "$(cat <<'EOF'
docs(measurements): compilation-load baseline after cargo-registry rollout

Captures cold/warm build times, sccache hit rate, target/ size, and
dupe count after Phases 1-6 of the cargo-registry plan. Becomes the
baseline that the structural refactor sprint will measure against.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Notes

**Spec coverage:**
- ✓ Repave the poisoned sccache bucket (Phase 1 — moved to front so all later builds benefit)
- ✓ Cargo client config wired to Nexus (Phase 2)
- ✓ First publish — `elohim-epr` 0.1.0 (Phase 3)
- ✓ Switch one downstream consumer (Phase 4)
- ✓ Workspace dep unification (Phase 5 — biggest single lever)
- ✓ Transitive dedupe pass (Phase 6)
- ✓ Measurement & baseline (Phase 7)

**Out of scope** (deferred to `2026-05-17-structural-refactor-sprint.md`):
- Sibling-module decomposition (http.rs / content_store/lib.rs / views.rs)
- SDK boundary extraction (elohim-storage → elohim-sdk separation)
- Additional utility crate extractions (elohim-provenance, elohim-schema-tools, elohim-test-fixtures)
- Feature-gating audit (libp2p / iroh / holochain-conductor behind features)
- Multi-workspace unification (bring elohim-storage / holochain / cache-core into one workspace)

**Risk areas:**
- Task 2 (repave) takes substantial wall time; budget 30-60 min depending on machine
- Task 9 (publish) is the first irreversible action on the registry — always dry-run
- Task 12 (workspace deps consolidation) may surface unexpected feature mismatches — be prepared to extend feature lists
- Task 14 (dedupe bumps) may break a member if a transitive API change is incompatible — be prepared to revert individual bumps

**Notes for the executing agent:**
- The "internal crate path-deps within workspace" entries in Task 12 stay as `path = "..."` (not registry), because workspace siblings reach each other directly. After Phase 3 publishes elohim-epr, the workspace's own members can still use path; ONLY external consumers (steward, doorway-service, elohim-storage which is OUT of the workspace) switch to the registry version.
- If a member has `package.publish = false`, don't add `publish = ["elohim"]` — that's for crates intended to be published.
- Expected output snippets are shape-guides, not literal matchers.

---

**Plan complete and saved to `genesis/docs/plans/2026-05-17-cargo-registry-and-compilation-load-reduction.md`.**

Execution options:

1. **Subagent-Driven (recommended)** — `superpowers:subagent-driven-development`. Fresh subagent per task, review between tasks.
2. **Inline Execution** — `superpowers:executing-plans`. Batch with checkpoints.

Choose after companion plan (`2026-05-17-structural-refactor-sprint.md`) is reviewed.
