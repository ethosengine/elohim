---
title: EPR-app deliverability verdict — slice 1 (peer verdict, wire, stage gate) — Implementation Plan
id: epr-app-deliverability-verdict-slice1-plan
status: landed
class: process-meta
domain: peer-hoster dataplane (T2) app delivery × deploy definition-of-done
sprint: delivery-verdict
requires_env: [household-nodes]
cites:
  - "epr-app-delivery-verdict-layered-fallback-design | EPR-app delivery verdict and layered fallback: the landing tells the truth on arrival | sha256:cef986af6515e913 | path: genesis/docs/superpowers/specs/2026-09-05-epr-app-delivery-verdict-layered-fallback-design.md"
  - genesis/a2o/features/dataplane/served-shell-boots.feature
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - scripts/ci/stage-spa-blob.sh
  - elohim/elohim-storage/seam-registry.yaml
---

# EPR-app deliverability verdict — slice 1 implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The storage peer judges, from the bundle bytes alone, whether an EPR app head can boot; it says so on every `/apps` surface; and the app pipeline refuses to author a head the peer calls broken.

**Architecture:** A pure function over the extracted ZIP entries (`judge_deliverability`) runs inside the extraction walk storage already does, memoised per canonical blob hash in process memory. Two headers (`X-Deliverability`, `X-Deliverability-Reason`) ride the app-file response and the `_capability` probe, and the doorway already forwards every `x-` header from those routes, so nothing changes in doorway-service. `stage-spa-blob.sh` reads the verdict for the CID it just uploaded and refuses to proceed on `broken`; the Jenkinsfile turns that refusal into a hard FAILURE before `authorHeadOnce`. Spec §2, §2.2, §2.3, §7 (stage leg), §8 (`judge_deliverability`).

**Tech Stack:** Rust (elohim-storage: hyper, tokio, `zip`, `lazy_static` + prometheus), bash + curl (CI scripts), Jenkins declarative helpers (heredoc-free), a2o (cucumber-js, TypeScript) for the closing check.

**Spec:** `genesis/docs/superpowers/specs/2026-09-05-epr-app-delivery-verdict-layered-fallback-design.md`

## Global Constraints

- Native cargo in `elohim/elohim-storage` keeps the custom getrandom flag (`RUSTFLAGS` untouched) and sets `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`. Under the RAM guard add `CARGO_BUILD_JOBS=1 --config "profile.dev.package.elohim-storage.debug=0"`; run only the focused test filters named in each task; end every cargo command with `; echo EXIT=$?` on its own line and read that line. `cargo nextest` is not installed.
- No new SQLite table in this slice: the verdict is a re-derivable memo (spec §8 names the SQLite projection; persistence is deferred to slice 2 because re-derivation costs one ZIP walk per app per boot — recorded here as the deviation).
- No doorway-service change in this slice. If a step seems to need one, stop: the doorway forwards `x-` headers already (`routes/apps.rs` "Forward storage's capability headers").
- Header vocabulary is exact and lower-case: `X-Deliverability: boots | broken | not-judged`; `X-Deliverability-Reason: missing-asset:<name> | invalid-zip | no-index | not-held`.
- Jenkinsfile helpers stay heredoc-free (CPS 64 KB limit): bash bodies live under `scripts/ci/`.
- Commits are local only (the integrator pushes). One commit per task, message body says what the task proves.
- A new decision predicate is registered in `elohim/elohim-storage/seam-registry.yaml` in the same task that creates it (the census `placement-audit.py --epr-meta` fails loud otherwise).

---

### Task 1: `judge_deliverability` — the pure verdict over extracted entries

**Files:**
- Create: `elohim/elohim-storage/src/app_deliverability.rs`
- Modify: `elohim/elohim-storage/src/main.rs:55` (add `pub mod app_deliverability;` beside `pub mod blob_reach;`)
- Test: inline `#[cfg(test)] mod tests` in the new file

**Interfaces:**
- Produces:
  ```rust
  pub enum DeliverabilityVerdict { Boots, Broken(BrokenReason), NotJudged(NotJudgedWhy) }
  pub enum BrokenReason { MissingAsset(String), InvalidZip, NoIndex }
  pub enum NotJudgedWhy { NotHeld }
  impl DeliverabilityVerdict {
      pub fn header_value(&self) -> &'static str;      // "boots" | "broken" | "not-judged"
      pub fn reason_value(&self) -> Option<String>;    // "missing-asset:main-X.js" | "invalid-zip" | "no-index" | "not-held"
  }
  pub fn judge_deliverability(entries: &[(String, Vec<u8>)]) -> DeliverabilityVerdict;
  pub fn shell_asset_refs(index_html: &str) -> Vec<String>;   // same-origin relative refs, in document order
  ```
- Consumes: nothing from other tasks.

- [ ] **Step 1: Write the failing tests**

```rust
// elohim/elohim-storage/src/app_deliverability.rs  (tests module at the bottom)
#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, body: &str) -> (String, Vec<u8>) {
        (name.to_string(), body.as_bytes().to_vec())
    }

    const INDEX: &str = r#"<!doctype html><html><head>
      <link rel="stylesheet" href="styles-7XLYMW2X.css">
      <link rel="icon" href="favicon.ico">
      <script src="https://cdn.example/x.js"></script>
      <script src="/chrome/omni-element.abc.js"></script>
    </head><body>
      <script src="polyfills-X2TQPNDQ.js" type="module"></script>
      <script src="main-7QFGHX5X.js" type="module"></script>
    </body></html>"#;

    #[test]
    fn shell_asset_refs_keeps_only_same_origin_relative_scripts_and_stylesheets() {
        let refs = shell_asset_refs(INDEX);
        assert_eq!(
            refs,
            vec![
                "styles-7XLYMW2X.css".to_string(),
                "polyfills-X2TQPNDQ.js".to_string(),
                "main-7QFGHX5X.js".to_string(),
            ],
            "icons, CDN scripts and the doorway-injected /chrome/ script are not bundle assets"
        );
    }

    #[test]
    fn a_bundle_whose_index_names_only_held_assets_boots() {
        let entries = vec![
            entry("index.html", INDEX),
            entry("styles-7XLYMW2X.css", "body{}"),
            entry("polyfills-X2TQPNDQ.js", "//p"),
            entry("main-7QFGHX5X.js", "//m"),
        ];
        assert!(matches!(judge_deliverability(&entries), DeliverabilityVerdict::Boots));
    }

    #[test]
    fn a_bundle_whose_index_names_a_missing_entry_script_is_broken_and_names_it() {
        // The 2026-09-04 shape: the shell names an entry script the bundle does not hold.
        let entries = vec![
            entry("index.html", INDEX),
            entry("styles-7XLYMW2X.css", "body{}"),
            entry("polyfills-X2TQPNDQ.js", "//p"),
        ];
        match judge_deliverability(&entries) {
            DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(name)) => {
                assert_eq!(name, "main-7QFGHX5X.js");
            }
            other => panic!("expected Broken(MissingAsset), got {other:?}"),
        }
    }

    #[test]
    fn a_bundle_with_no_index_is_broken_no_index() {
        let entries = vec![entry("main-7QFGHX5X.js", "//m")];
        assert!(matches!(
            judge_deliverability(&entries),
            DeliverabilityVerdict::Broken(BrokenReason::NoIndex)
        ));
    }

    #[test]
    fn a_nested_index_resolves_assets_relative_to_its_own_directory() {
        // Angular dists are sometimes zipped with a top-level folder.
        let entries = vec![
            entry("browser/index.html", r#"<script src="main-A.js"></script>"#),
            entry("browser/main-A.js", "//m"),
        ];
        assert!(matches!(judge_deliverability(&entries), DeliverabilityVerdict::Boots));
    }

    #[test]
    fn header_and_reason_values_are_the_wire_vocabulary() {
        assert_eq!(DeliverabilityVerdict::Boots.header_value(), "boots");
        assert_eq!(DeliverabilityVerdict::Boots.reason_value(), None);
        let b = DeliverabilityVerdict::Broken(BrokenReason::MissingAsset("main-X.js".into()));
        assert_eq!(b.header_value(), "broken");
        assert_eq!(b.reason_value().as_deref(), Some("missing-asset:main-X.js"));
        assert_eq!(
            DeliverabilityVerdict::Broken(BrokenReason::InvalidZip).reason_value().as_deref(),
            Some("invalid-zip")
        );
        assert_eq!(
            DeliverabilityVerdict::NotJudged(NotJudgedWhy::NotHeld).header_value(),
            "not-judged"
        );
        assert_eq!(
            DeliverabilityVerdict::NotJudged(NotJudgedWhy::NotHeld).reason_value().as_deref(),
            Some("not-held")
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run (from `elohim/elohim-storage`):
```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev CARGO_BUILD_JOBS=1 \
cargo test --lib app_deliverability --config "profile.dev.package.elohim-storage.debug=0"; echo EXIT=$?
```
Expected: compile error — `judge_deliverability`, `shell_asset_refs`, `DeliverabilityVerdict` not found; `EXIT=101`.

- [ ] **Step 3: Write the minimal implementation**

```rust
//! Deliverability verdict — a pure derivation of a bundle's bytes.
//!
//! Whether an EPR app head can boot is a property of the ZIP, not of any
//! doorway: `index.html` either names assets the bundle holds or it does not.
//! The peer that holds the bytes judges them once (inside the extraction walk
//! it already performs) and every other peer re-derives the same answer from
//! the same CID. Spec: 2026-09-05 EPR-app delivery verdict §2.
//!
//! The 2026-09-04 incident this exists for: a shell naming `main-EAKNZDUP.js`
//! was served while the bundle held `main-7QFGHX5X.js` — a blank page for every
//! visitor, and nothing on the serving side had judged the head at all.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokenReason {
    /// `index.html` names a same-origin asset the bundle does not hold.
    MissingAsset(String),
    /// The blob is not a readable ZIP archive.
    InvalidZip,
    /// No `index.html` (top-level or nested) in the bundle.
    NoIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotJudgedWhy {
    /// The bytes are not held locally yet (syncing / absent) — honest absence.
    NotHeld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliverabilityVerdict {
    Boots,
    Broken(BrokenReason),
    NotJudged(NotJudgedWhy),
}

impl DeliverabilityVerdict {
    pub fn header_value(&self) -> &'static str {
        match self {
            Self::Boots => "boots",
            Self::Broken(_) => "broken",
            Self::NotJudged(_) => "not-judged",
        }
    }

    pub fn reason_value(&self) -> Option<String> {
        match self {
            Self::Boots => None,
            Self::Broken(BrokenReason::MissingAsset(name)) => Some(format!("missing-asset:{name}")),
            Self::Broken(BrokenReason::InvalidZip) => Some("invalid-zip".to_string()),
            Self::Broken(BrokenReason::NoIndex) => Some("no-index".to_string()),
            Self::NotJudged(NotJudgedWhy::NotHeld) => Some("not-held".to_string()),
        }
    }
}

/// Same-origin, relative `<script src>` and `<link rel="stylesheet" href>`
/// references in document order. Skips anything with a scheme or a leading
/// `//` (CDN), and the doorway-injected `/chrome/` island, which is never part
/// of a bundle. A leading `/` is treated as bundle-root-relative.
pub fn shell_asset_refs(index_html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tag in html_tags(index_html) {
        let lower = tag.to_ascii_lowercase();
        let attr = if lower.starts_with("<script") {
            "src"
        } else if lower.starts_with("<link") && attr_value(&lower, "rel").as_deref() == Some("stylesheet") {
            "href"
        } else {
            continue;
        };
        let Some(raw) = attr_value(&tag, attr) else { continue };
        if raw.contains("://") || raw.starts_with("//") || raw.starts_with("data:") {
            continue;
        }
        let trimmed = raw.trim_start_matches('/');
        if trimmed.starts_with("chrome/") || trimmed.is_empty() {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// Judge the extracted entries of one bundle. `entries` is the `(name, bytes)`
/// list the extraction walk already produces; directories are not included.
pub fn judge_deliverability(entries: &[(String, Vec<u8>)]) -> DeliverabilityVerdict {
    let Some((index_name, index_bytes)) = entries
        .iter()
        .find(|(n, _)| n == "index.html" || n.ends_with("/index.html"))
    else {
        return DeliverabilityVerdict::Broken(BrokenReason::NoIndex);
    };
    let index_dir = index_name
        .rfind('/')
        .map(|i| &index_name[..=i])
        .unwrap_or("");
    let held: HashSet<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    let index_html = String::from_utf8_lossy(index_bytes);
    for asset in shell_asset_refs(&index_html) {
        let in_index_dir = format!("{index_dir}{asset}");
        if held.contains(in_index_dir.as_str()) || held.contains(asset.as_str()) {
            continue;
        }
        return DeliverabilityVerdict::Broken(BrokenReason::MissingAsset(asset));
    }
    DeliverabilityVerdict::Boots
}

/// Every `<...>` tag as a slice, without a parser dependency: the shell is a
/// build artifact, not user input, so a tag scanner is enough.
fn html_tags(html: &str) -> impl Iterator<Item = &str> {
    html.match_indices('<').filter_map(move |(start, _)| {
        let rest = &html[start..];
        let end = rest.find('>')?;
        Some(&rest[..=end])
    })
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{attr}=");
    let mut search = 0;
    while let Some(pos) = lower[search..].find(&needle) {
        let at = search + pos;
        let before = if at == 0 { ' ' } else { lower.as_bytes()[at - 1] as char };
        if before.is_whitespace() {
            let value_start = at + needle.len();
            let rest = &tag[value_start..];
            let quote = rest.chars().next()?;
            return if quote == '"' || quote == '\'' {
                rest[1..].find(quote).map(|e| rest[1..1 + e].to_string())
            } else {
                Some(rest.split(|c: char| c.is_whitespace() || c == '>').next()?.to_string())
            };
        }
        search = at + needle.len();
    }
    None
}
```

Then add `pub mod app_deliverability;` to `elohim/elohim-storage/src/main.rs` directly under `pub mod blob_reach;`.

- [ ] **Step 4: Run the tests to verify they pass**

Same command as Step 2. Expected: `test result: ok. 6 passed` and `EXIT=0`.

- [ ] **Step 5: Register the predicate in the seam registry**

Append to `elohim/elohim-storage/seam-registry.yaml` under `decisionPoints:` (line numbers from the file you just wrote; keep the four-state vocabulary):

```yaml
  - name: judge_deliverability
    kind: verdict-fn
    sourceLocation:
      file: src/app_deliverability.rs
      modulePath: app_deliverability::judge_deliverability
      line: 74
    summary: >
      Pure derivation of a bundle's deliverability from its extracted entries: Boots iff an
      index.html is present and every same-origin script/stylesheet it names is held by the
      bundle; Broken(MissingAsset|InvalidZip|NoIndex) otherwise; NotJudged(NotHeld) is the
      honest-absence arm the callers use when the bytes are not local. Runs inside the
      extraction walk (no extra I/O), memoised per canonical blob hash. Born from the
      2026-09-04 blank landing (spec 2026-09-05 EPR-app delivery verdict §2).
    concernIds:
      - id: C0
        status: answered
        justification: >
          Plane stated: peer-hoster dataplane (T2). The doorway never computes this; it
          relays the peer's answer as headers.
      - id: C4
        status: answered
        justification: >
          NotJudged(NotHeld) is distinct from Broken on every surface; absence of bytes
          is never read as a broken bundle.
      - id: C5
        status: answered
        justification: >
          Any receiver re-derives the verdict from the bytes; no forwarded claim is trusted.
      - id: C6a
        status: answered
        justification: >
          One judgement per canonical hash, inside a walk that already reads every entry;
          memoised for the process lifetime.
      - id: C8
        status: answered
        justification: >
          Typed verdict + reason on the wire and counted
          (elohim_app_deliverability_verdict_total{verdict,reason}).
      - id: C10
        status: answered
        justification: >
          Additive headers only; a consumer that does not know X-Deliverability sees no
          change (verify-projected-head.sh's SKIP idiom for absent fields).
      - id: C1
        status: n-a
        justification: no election or self-nomination is involved.
      - id: C2
        status: n-a
        justification: no authority is granted or revoked by the verdict.
      - id: C3
        status: answered
        justification: a verdict never blocks a serve; Broken is reported, the bytes still flow.
      - id: C6b
        status: answered
        justification: idempotent — the same entries always yield the same verdict.
      - id: C7
        status: answered
        justification: the header advertises exactly the verdict the peer serves by.
      - id: C9
        status: n-a
        justification: no identity lineage is touched.
      - id: C11
        status: n-a
        justification: not a backpressure source; it adds no I/O.
      - id: C12
        status: n-a
        justification: the verdict is public per head; the reach-gated diagnostic is slice 4.
      - id: C13
        status: n-a
        justification: no authority tier is introduced by this predicate.
      - id: C14
        status: n-a
        justification: the witnessed-transition graduation is held (spec §8).
    contractTests:
      - file: src/app_deliverability.rs
        name: a_bundle_whose_index_names_only_held_assets_boots
      - file: src/app_deliverability.rs
        name: a_bundle_whose_index_names_a_missing_entry_script_is_broken_and_names_it
      - file: src/app_deliverability.rs
        name: a_bundle_with_no_index_is_broken_no_index
      - file: src/app_deliverability.rs
        name: shell_asset_refs_keeps_only_same_origin_relative_scripts_and_stylesheets
```

Run the census and read its storage line:
```bash
python3 .claude/scripts/memory-kit/placement-audit.py --epr-meta 2>&1 | grep -E "elohim-storage|uncited|unregistered"; echo EXIT=$?
```
Expected: the `elohim-storage` row's point count is one higher than before and `uncited` did not grow.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/app_deliverability.rs elohim/elohim-storage/src/main.rs elohim/elohim-storage/seam-registry.yaml
git commit -m "feat(storage): judge_deliverability — a bundle boots iff its index names only assets it holds

Pure verdict over the extracted entries (Boots | Broken(MissingAsset|InvalidZip|NoIndex) |
NotJudged(NotHeld)) with the wire vocabulary for X-Deliverability(-Reason). Registered
in seam-registry with four contract tests. Spec §2 / §8 (slice 1, task 1)."
```

---

### Task 2: Judge inside the extraction walk, memoise per hash, say it on the wire

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:319` (struct field), `:889` (constructor default), `:9354-9410` (extraction walk), the app-file response builder just after `:9446`, and `handle_app_capability` around `:8940-8960`
- Test: `elohim/elohim-storage/src/http.rs` test module (beside `is_content_address_recognizes_cidv1_and_rejects_slugs` at `:17664`)

**Interfaces:**
- Consumes: `app_deliverability::{judge_deliverability, DeliverabilityVerdict, BrokenReason, NotJudgedWhy}` (Task 1).
- Produces:
  ```rust
  // on the HTTP server struct
  deliverability_memo: Arc<tokio::sync::RwLock<HashMap<String, DeliverabilityVerdict>>>,  // key: canonical blob hash
  async fn deliverability_for(&self, blob_hash: &str) -> DeliverabilityVerdict;          // memo hit or NotJudged(NotHeld)
  async fn record_deliverability(&self, blob_hash: &str, verdict: DeliverabilityVerdict);
  fn with_deliverability_headers(builder: http::response::Builder, verdict: &DeliverabilityVerdict) -> http::response::Builder;
  ```

- [ ] **Step 1: Write the failing test**

```rust
// in the existing tests module of elohim/elohim-storage/src/http.rs
#[test]
fn deliverability_headers_carry_the_wire_vocabulary() {
    use crate::app_deliverability::{BrokenReason, DeliverabilityVerdict};
    let b = Response::builder();
    let resp = with_deliverability_headers(
        b,
        &DeliverabilityVerdict::Broken(BrokenReason::MissingAsset("main-EAKNZDUP.js".into())),
    )
    .body(Full::new(Bytes::new()))
    .unwrap();
    assert_eq!(resp.headers().get("X-Deliverability").unwrap(), "broken");
    assert_eq!(
        resp.headers().get("X-Deliverability-Reason").unwrap(),
        "missing-asset:main-EAKNZDUP.js"
    );
    let ok = with_deliverability_headers(Response::builder(), &DeliverabilityVerdict::Boots)
        .body(Full::new(Bytes::new()))
        .unwrap();
    assert_eq!(ok.headers().get("X-Deliverability").unwrap(), "boots");
    assert!(ok.headers().get("X-Deliverability-Reason").is_none());
}

#[tokio::test]
async fn an_unjudged_hash_reads_not_judged_not_held() {
    use crate::app_deliverability::{DeliverabilityVerdict, NotJudgedWhy};
    let memo: Arc<tokio::sync::RwLock<HashMap<String, DeliverabilityVerdict>>> = Default::default();
    let v = deliverability_lookup(&memo, "sha256-deadbeef").await;
    assert_eq!(v, DeliverabilityVerdict::NotJudged(NotJudgedWhy::NotHeld));
    memo.write().await.insert("sha256-deadbeef".into(), DeliverabilityVerdict::Boots);
    assert_eq!(deliverability_lookup(&memo, "sha256-deadbeef").await, DeliverabilityVerdict::Boots);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev CARGO_BUILD_JOBS=1 \
cargo test --lib deliverability --config "profile.dev.package.elohim-storage.debug=0"; echo EXIT=$?
```
Expected: compile error on `with_deliverability_headers` / `deliverability_lookup`; `EXIT=101`.

- [ ] **Step 3: Implement**

Free functions (near `content_address_header` at `:562`):
```rust
/// Stamp the peer's deliverability verdict on an app surface. Additive: a
/// consumer that does not know the header sees no change.
fn with_deliverability_headers(
    mut builder: http::response::Builder,
    verdict: &crate::app_deliverability::DeliverabilityVerdict,
) -> http::response::Builder {
    builder = builder.header("X-Deliverability", verdict.header_value());
    if let Some(reason) = verdict.reason_value() {
        builder = builder.header("X-Deliverability-Reason", reason);
    }
    builder
}

async fn deliverability_lookup(
    memo: &Arc<tokio::sync::RwLock<HashMap<String, crate::app_deliverability::DeliverabilityVerdict>>>,
    blob_hash: &str,
) -> crate::app_deliverability::DeliverabilityVerdict {
    memo.read()
        .await
        .get(blob_hash)
        .cloned()
        .unwrap_or(crate::app_deliverability::DeliverabilityVerdict::NotJudged(
            crate::app_deliverability::NotJudgedWhy::NotHeld,
        ))
}
```

Struct field (beside `extraction_cache` at `:319`) and its default in the constructor at `:889`:
```rust
    /// Deliverability verdict per canonical blob hash — a memo of a pure
    /// derivation (app_deliverability), re-derived on the next extraction if
    /// absent. Process-lifetime; persistence is slice 2.
    deliverability_memo: Arc<tokio::sync::RwLock<HashMap<String, crate::app_deliverability::DeliverabilityVerdict>>>,
```
```rust
            deliverability_memo: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
```

In `handle_app_request`: at the `Invalid ZIP archive` arm (`:9354-9366`) record `Broken(InvalidZip)` before returning:
```rust
                self.deliverability_memo.write().await.insert(
                    blob_hash.clone(),
                    crate::app_deliverability::DeliverabilityVerdict::Broken(
                        crate::app_deliverability::BrokenReason::InvalidZip,
                    ),
                );
```
After the walk fills `all_files` and before `cache.put_app` (`:9405`), judge and memoise:
```rust
        // The peer judges the head it holds — once per hash, inside the walk
        // that already read every entry (spec 2026-09-05 §2.2).
        let verdict = crate::app_deliverability::judge_deliverability(&all_files);
        tracing::info!(
            target: "storage::deliverability",
            blob_hash = %blob_hash,
            slug = ?resolved_slug,
            verdict = verdict.header_value(),
            reason = ?verdict.reason_value(),
            "app head judged"
        );
        self.deliverability_memo
            .write()
            .await
            .insert(blob_hash.clone(), verdict.clone());
```
On the served-file response builder (the `let mut builder = Response::builder().status(StatusCode::OK)` after `:9446`) and on the SPA-fallback builder, wrap: `let mut builder = with_deliverability_headers(builder, &verdict);` (the extracted-cache hit path at `:9026-9060` reads the memo with `deliverability_lookup(&self.deliverability_memo, hash).await` and stamps the same way).

In `handle_app_capability` (after the `X-Blob-Hash` header at ~`:8958`):
```rust
        if let Some(hash) = &blob_hash {
            let verdict = deliverability_lookup(&self.deliverability_memo, hash).await;
            builder = with_deliverability_headers(builder, &verdict);
        }
```

- [ ] **Step 4: Run the tests to verify they pass**

Same command as Step 2. Expected: `2 passed` for the new names, `EXIT=0`. Then the whole storage lib under the guard flags:
```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev CARGO_BUILD_JOBS=1 \
cargo test --lib --features "p2p p2p-iroh" --config "profile.dev.package.elohim-storage.debug=0"; echo EXIT=$?
```
Expected: `0 failed`, `EXIT=0`. If the guard sheds it (`signal: 15`), retry once; if shed twice, record that in the task report — do not judge from a partial run.

- [ ] **Step 5: Prove it on the household mesh binary (eyes on the wire)**

Rebuild into the mesh's debug slot and restart storage (`just mesh storage-restart <peer>` per `project_local_mesh_binary_slot_and_restart`), then:
```bash
DOORWAY=http://localhost:8888   # the mesh doorway
curl -sI "$DOORWAY/apps/elohim-host-landing/index.html" | grep -i "^x-deliverability"
curl -sI "$DOORWAY/apps/elohim-host-landing/_capability" | grep -i "^x-deliverability"
```
Expected: `X-Deliverability: boots` on both (the Prologue-staged bundle is whole). Paste both lines into the commit body.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): judge the app head inside the extraction walk; X-Deliverability on /apps surfaces

Memoised per canonical blob hash; Invalid ZIP records Broken(invalid-zip);
_capability reports not-judged/not-held until the bytes have been walked.
Doorway forwards the headers unchanged. Spec §2.2–§2.3 (slice 1, task 2)."
```

---

### Task 3: Count it — `elohim_app_deliverability_verdict_total`

**Files:**
- Modify: `elohim/elohim-storage/src/metrics.rs:216` (declare beside `IDENTITY_NAMESPACE_VIOLATIONS`), `:2244` (register), and the judge site in `http.rs` (Task 2's `let verdict = …` block)
- Test: `elohim/elohim-storage/src/metrics.rs` tests module

**Interfaces:**
- Produces: `pub static ref APP_DELIVERABILITY_VERDICTS: IntCounterVec` with labels `["verdict", "reason"]`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn app_deliverability_verdict_counter_is_registered_and_labelled() {
    register_all();
    APP_DELIVERABILITY_VERDICTS
        .with_label_values(&["broken", "missing-asset"])
        .inc();
    let families = REGISTRY.gather();
    let f = families
        .iter()
        .find(|f| f.get_name() == "elohim_app_deliverability_verdict_total")
        .expect("counter registered");
    let m = f.get_metric().iter().find(|m| {
        m.get_label().iter().any(|l| l.get_name() == "verdict" && l.get_value() == "broken")
    });
    assert!(m.is_some(), "verdict label present");
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev CARGO_BUILD_JOBS=1 \
cargo test --lib app_deliverability_verdict_counter --config "profile.dev.package.elohim-storage.debug=0"; echo EXIT=$?
```
Expected: compile error, `APP_DELIVERABILITY_VERDICTS` not found.

- [ ] **Step 3: Implement**

Declaration (in the `lazy_static!` block beside `IDENTITY_NAMESPACE_VIOLATIONS`):
```rust
    /// The peer's deliverability verdict per judged app head (spec 2026-09-05
    /// §5.3). label reason is the reason CLASS ("missing-asset", "invalid-zip",
    /// "no-index", "none") — never the asset name, which is unbounded.
    pub static ref APP_DELIVERABILITY_VERDICTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_app_deliverability_verdict_total",
            "App heads judged by this peer, by verdict and reason class.",
        ),
        &["verdict", "reason"],
    )
    .unwrap();
```
Registration (in `register_all`, beside the identity line): `let _ = REGISTRY.register(Box::new(APP_DELIVERABILITY_VERDICTS.clone()));`

At the judge site in `http.rs` (Task 2), after the `tracing::info!`:
```rust
        let reason_class = verdict
            .reason_value()
            .map(|r| r.split(':').next().unwrap_or("none").to_string())
            .unwrap_or_else(|| "none".to_string());
        crate::metrics::APP_DELIVERABILITY_VERDICTS
            .with_label_values(&[verdict.header_value(), &reason_class])
            .inc();
```

- [ ] **Step 4: Run to verify it passes**

Same command. Expected `1 passed`, `EXIT=0`.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/metrics.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): count deliverability verdicts by verdict and reason class (C8)"
```

---

### Task 4: The stage gate — refuse to author a head the peer calls broken

**Files:**
- Create: `scripts/ci/deliverability-gate.sh`
- Modify: `scripts/ci/stage-spa-blob.sh` (inside `stage_once`, after the PUT success branch and before `if [ "${DO_PATCH}" = "1" ]`)
- Test: a shell test against a fake doorway: `scripts/ci/tests/deliverability-gate.test.sh` (create the `tests/` dir if absent; bash + python `http.server`)

**Interfaces:**
- Produces: `deliverability-gate.sh <doorway-epr-url> <blob-hash> [attempts]` → exit 0 on `boots`, exit 2 on `broken` (prints the reason), exit 0 with a `⚠ NOT-JUDGED` line on `not-judged` after the attempts are exhausted (honest absence is not a red). Env `DELIVERABILITY_GATE=strict` turns not-judged into exit 3.
- Consumes: the `X-Deliverability` / `X-Deliverability-Reason` headers (Task 2) on `GET /apps/{hash}/index.html` and `HEAD /apps/{hash}/_capability`.

- [ ] **Step 1: Write the failing test**

```bash
#!/usr/bin/env bash
# scripts/ci/tests/deliverability-gate.test.sh — run: bash scripts/ci/tests/deliverability-gate.test.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
gate="$here/../deliverability-gate.sh"
port=18099
python3 - "$port" <<'PY' &
import sys, http.server
port = int(sys.argv[1])
class H(http.server.BaseHTTPRequestHandler):
    def _v(self):
        if "broken" in self.path: return ("broken", "missing-asset:main-EAKNZDUP.js")
        if "unjudged" in self.path: return ("not-judged", "not-held")
        return ("boots", None)
    def do_GET(self):
        v, r = self._v()
        self.send_response(200); self.send_header("X-Deliverability", v)
        if r: self.send_header("X-Deliverability-Reason", r)
        self.end_headers(); self.wfile.write(b"<html></html>")
    def do_HEAD(self):
        v, r = self._v()
        self.send_response(200); self.send_header("X-Deliverability", v)
        if r: self.send_header("X-Deliverability-Reason", r)
        self.end_headers()
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", port), H).serve_forever()
PY
srv=$!
trap 'kill $srv' EXIT
sleep 1
base="http://127.0.0.1:$port"

bash "$gate" "$base" "sha256-boots" 1 && echo "PASS boots"
if bash "$gate" "$base" "sha256-broken" 1; then echo "FAIL broken should exit 2"; exit 1; else rc=$?; [ "$rc" -eq 2 ] && echo "PASS broken rc=2"; fi
bash "$gate" "$base" "sha256-unjudged" 1 && echo "PASS not-judged is advisory"
if DELIVERABILITY_GATE=strict bash "$gate" "$base" "sha256-unjudged" 1; then echo "FAIL strict not-judged should exit 3"; exit 1; else rc=$?; [ "$rc" -eq 3 ] && echo "PASS strict rc=3"; fi
echo "ALL PASS"
```

- [ ] **Step 2: Run to verify it fails**

Run: `bash scripts/ci/tests/deliverability-gate.test.sh; echo EXIT=$?`
Expected: `deliverability-gate.sh: No such file` and a non-zero EXIT.

- [ ] **Step 3: Write the gate**

```bash
#!/usr/bin/env bash
# deliverability-gate.sh — ask the peer whether the head we just uploaded can boot.
#
# Usage: deliverability-gate.sh <doorway-epr-url> <blob-hash> [attempts]
#
# The storage peer judges a bundle from its bytes when it first extracts it
# (elohim-storage app_deliverability, spec 2026-09-05 §2). This gate forces that
# first extraction with one GET of index.html by CONTENT ADDRESS (never the
# slug — the slug still points at the previous head), then reads the verdict off
# HEAD /apps/{hash}/_capability. Exit codes:
#   0  boots (or not-judged after the attempts, advisory — see DELIVERABILITY_GATE)
#   2  broken — the reason is printed; the caller must NOT author this head
#   3  not-judged under DELIVERABILITY_GATE=strict
# Born from 2026-09-04: app #1691's bundle was authored, served and blank for a
# day; the peer could have said "missing-asset:main-EAKNZDUP.js" before the
# head existed.
set -uo pipefail
DOORWAY_EPR_URL="${1:?doorway epr url}"
BLOB_HASH="${2:?blob hash}"
ATTEMPTS="${3:-6}"
GATE_MODE="${DELIVERABILITY_GATE:-advisory}"

verdict=""; reason=""
for attempt in $(seq 1 "$ATTEMPTS"); do
    # Force extraction (and the judgement) by content address; ignore the body.
    curl -sS -o /dev/null --max-time 120 "${DOORWAY_EPR_URL}/apps/${BLOB_HASH}/index.html" || true
    headers="$(curl -sS -I --max-time 30 "${DOORWAY_EPR_URL}/apps/${BLOB_HASH}/_capability" || true)"
    verdict="$(printf '%s' "$headers" | tr -d '\r' | awk -F': ' 'tolower($1)=="x-deliverability"{print $2}')"
    reason="$(printf '%s' "$headers" | tr -d '\r' | awk -F': ' 'tolower($1)=="x-deliverability-reason"{print $2}')"
    case "$verdict" in
        boots)  echo "  ✓ deliverability: ${BLOB_HASH} boots (peer-judged)"; exit 0 ;;
        broken) echo "  ✗ deliverability: ${BLOB_HASH} is BROKEN — ${reason:-no reason} — refusing to author this head" >&2; exit 2 ;;
        *)      echo "  … deliverability: not judged yet (attempt ${attempt}/${ATTEMPTS}, header='${verdict:-absent}')"; sleep 5 ;;
    esac
done
if [ "$GATE_MODE" = "strict" ]; then
    echo "  ✗ deliverability: ${BLOB_HASH} NOT-JUDGED after ${ATTEMPTS} attempts (strict) — refusing to author" >&2; exit 3
fi
echo "  ⚠ deliverability: ${BLOB_HASH} NOT-JUDGED after ${ATTEMPTS} attempts — the peer holds no verdict (pre-cure peer or bytes not walked); proceeding, advisory" >&2
exit 0
```
`chmod +x scripts/ci/deliverability-gate.sh`.

In `stage-spa-blob.sh` `stage_once`, after the PUT branch resolves (`✓ blob uploaded` / `already stored`) and before `if [ "${DO_PATCH}" = "1" ]`, add:
```bash
    # The peer judges the bytes we just staged; a broken bundle never gets a
    # head. Exit 2 from the gate is a VERDICT, not a transport blip — surface
    # it as the stage's own failure code so the retry ladder does not re-try
    # a deterministic answer.
    local gate_rc=0
    bash "$(dirname "$0")/deliverability-gate.sh" "${DOORWAY_EPR_URL}" "${SPA_HASH}" || gate_rc=$?
    if [ "${gate_rc}" -eq 2 ] || [ "${gate_rc}" -eq 3 ]; then
        echo "BROKEN_HEAD ${SLUG} ${KIND} ${SPA_HASH}" > "${DELIVERABILITY_VERDICT_FILE:-/dev/null}" 2>/dev/null || true
        return 2
    fi
```
and in the retry ladder at the bottom (the `if stage_once; then … else` block, ~`:460-470`), treat `2` as terminal:
```bash
    stage_once; rc=$?
    if [ "${rc}" -eq 0 ]; then … existing success … ; fi
    if [ "${rc}" -eq 2 ]; then
        echo "ERROR: [${SLUG}] the peer judged ${SPA_HASH} BROKEN — not retrying a deterministic verdict" >&2
        exit 2
    fi
```
(read the ladder's exact shape before editing; keep its linear backoff for every other rc).

- [ ] **Step 4: Run the test to verify it passes**

Run: `bash scripts/ci/tests/deliverability-gate.test.sh; echo EXIT=$?` → `ALL PASS`, `EXIT=0`. Also `bash -n scripts/ci/stage-spa-blob.sh; echo EXIT=$?` → `EXIT=0`.

- [ ] **Step 5: Commit**

```bash
git add scripts/ci/deliverability-gate.sh scripts/ci/tests/deliverability-gate.test.sh scripts/ci/stage-spa-blob.sh
git commit -m "ci(stage): the peer's deliverability verdict gates the head — broken bundles are never authored

deliverability-gate.sh forces the first extraction by content address and reads
X-Deliverability; exit 2 (broken) is terminal in stage-spa-blob.sh, not retried.
not-judged stays advisory unless DELIVERABILITY_GATE=strict. Spec §7 (slice 1)."
```

---

### Task 5: Jenkins — a broken head is a FAILURE before `authorHeadOnce`

**Files:**
- Modify: `Jenkinsfile:223-295` (`stageSpaBlobs`) and `:296-330` (`authorHeadOnce`)
- Test: `groovy -e` is not available; the check is `scripts/ci/jenkinsfile-size-check` style — run the existing pre-push Jenkinsfile guard if present (`grep -n "Jenkinsfile" .husky/pre-push.bash`) and `wc -c Jenkinsfile` stays under 60000

**Interfaces:**
- Consumes: `stage-spa-blob.sh` exit `2` and the `DELIVERABILITY_VERDICT_FILE` marker (Task 4).
- Produces: `outcomes["deliverability|${slug}|${kind}"] = 'broken'` and a hard `error(...)` before any head is authored.

- [ ] **Step 1: Wire the marker in `stageSpaBlobs`**

Inside the `for (bundle in bundles)` loop, replace the `withEnv([...])` list and add the marker read (heredoc-free):
```groovy
        def verdictFile = "${env.WORKSPACE}/.ci-deliverability-${bundle.slug}-${kind}.txt"
        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE',
                   message: "seed ${host} ${bundle.slug} (${kind}): blob byte upload failed after retries; see junit testcase") {
            withEnv(["STORAGE_API_KEY_ADMIN=${adminKey ?: ''}", "DO_PATCH=${doPatch}", "DELIVERABILITY_VERDICT_FILE=${verdictFile}"]) {
                sh "bash '${env.WORKSPACE}/scripts/ci/stage-spa-blob.sh' '${bundle.distDir}' '${bundle.slug}' '${doorwayEprUrl}' '${kind}'"
            }
            outcomes[outcomeKey] = true
        }
        if (fileExists(verdictFile)) {
            outcomes["deliverability|${bundle.slug}|${kind}".toString()] = readFile(verdictFile).trim()
        }
```

- [ ] **Step 2: Refuse in `authorHeadOnce`**

At the top of `authorHeadOnce`, before the doorway loop:
```groovy
    def verdictKey = "deliverability|${bundle.slug}|${kind}".toString()
    if (outcomes[verdictKey]?.startsWith('BROKEN_HEAD')) {
        // A peer judged this bundle from its bytes: it cannot boot. Authoring
        // the head would mint a witnessed pointer to a blank page (2026-09-04).
        // Hard FAILURE on purpose — the UNSTABLE dependency-chain rule is for
        // transient substrate churn, and a broken bundle is not churn.
        error("authorHeadOnce: ${bundle.slug} (${kind}) refused — ${outcomes[verdictKey]}. The bundle's index names an asset it does not hold; fix the build, do not re-run.")
    }
```

- [ ] **Step 3: Verify the size guard and syntax shape**

```bash
wc -c Jenkinsfile; grep -c '"""' Jenkinsfile; echo EXIT=$?
```
Expected: byte count under 60000 (note the number in the commit body) and the triple-quote count unchanged from `git show HEAD:Jenkinsfile | grep -c '"""'` (no new heredocs).

- [ ] **Step 4: Commit**

```bash
git add Jenkinsfile
git commit -m "ci(app): a peer-judged broken bundle is a hard FAILURE before authorHeadOnce

stageSpaBlobs records the deliverability marker per (slug, kind); authorHeadOnce
error()s on BROKEN_HEAD so no witnessed head is ever minted for a bundle that
cannot boot. Spec §7 (slice 1, task 5)."
```

---

### Task 6: Close the slice — story, habit delta, register

**Files:**
- Modify: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (one DELTA line), `genesis/manifests/habits.yaml` (re-projected, never hand-edited)
- Test: `genesis/a2o/features/dataplane/served-shell-boots.feature` against the household mesh

**Interfaces:**
- Consumes: Tasks 1–5 landed locally; the mesh storage binary rebuilt with Task 2.

- [ ] **Step 1: Run the boot story on the household mesh**

From `genesis/a2o` (mesh up, storage restarted onto the new binary in Task 2 Step 5):
```bash
E2E_DOORWAY_ALPHA=http://localhost:8888 pnpm exec cucumber-js --name "handed a page that can boot" --format summary; echo EXIT=$?
```
Expected: `1 scenario (1 passed)`, `EXIT=0` (the mesh's staged bundle is whole). If HELD by the act gate, add `ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act2-neighbourhood.yaml` and re-run; record which lane admitted it.

- [ ] **Step 2: Prove the gate refuses a broken bundle, on the mesh**

Build a deliberately broken ZIP from the staged landing dist (delete its `main-*.js`), upload it by content address, and run the gate:
```bash
work=$(mktemp -d); cd "$work"
unzip -q /path/to/staged/elohim-host-landing.zip -d bundle   # the Prologue's staged zip (see hc-mesh.sh mesh_seed_env)
rm bundle/main-*.js
(cd bundle && zip -qr ../broken.zip .)
HASH="sha256-$(sha256sum broken.zip | awk '{print $1}')"
curl -sS -X PUT -H 'Content-Type: application/zip' -H "X-Blob-Hash: ${HASH}" -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" --data-binary @broken.zip "http://localhost:8888/admin/seed/blob"
bash /projects/elohim/scripts/ci/deliverability-gate.sh http://localhost:8888 "${HASH}"; echo EXIT=$?
```
Expected: `✗ deliverability: … is BROKEN — missing-asset:main-….js` and `EXIT=2`. Paste both lines into the DELTA.

- [ ] **Step 3: Write the DELTA and re-project**

Prepend to the body of `doorway/doorway-service/.epr-meta/doorway-failover.habit.md` (below the frontmatter), one line:
```
DELTA 2026-09-05 (RED preserved; slice 1 of the delivery verdict landed LOCALLY): the storage peer now judges every app head from its bytes inside the extraction walk (app_deliverability::judge_deliverability, memoised per hash) and says so on /apps surfaces (X-Deliverability / -Reason); stage-spa-blob.sh reads the verdict by content address and exit 2 (broken) is terminal; authorHeadOnce error()s on BROKEN_HEAD so a bundle that cannot boot never gets a witnessed head. Mesh proof: served-shell-boots 1/1 on the household lane; a deliberately broken bundle refused with `missing-asset:<name>` EXIT=2. Storage lib <N>/<N>, seam census: judge_deliverability registered, 4 contract tests. Flip to green still needs the fleet build carrying 44b5d94b5 + this slice with the scenario green there.
```
Then `python3 .claude/scripts/habits-project.py` and `python3 .claude/scripts/habits-project.py --check` (expect `is current`).

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/.epr-meta/doorway-failover.habit.md genesis/manifests/habits.yaml
git commit -m "habit(doorway-failover): slice 1 delta — the peer judges the head, the stage refuses a broken one (mesh-proven)"
```

---

## Self-review

- **Spec coverage (slice 1 = spec §9 item 1):** §2.1 check → Task 1; §2.2 extraction-time judgement + memo → Task 2; §2.3 `X-Deliverability` on `_capability` and the shell → Task 2 (the `/db/content/{slug}` row field is spec §2.3 too but belongs to slice 2 with the view-schema change — stated in Global Constraints as deferred); §5.3 metric + §5.5 log → Tasks 2–3; §7 stage leg → Tasks 4–5; §8 `judge_deliverability` registration → Task 1 Step 5; "each slice ends with the story + one delta" → Task 6.
- **Placeholders:** none; every step has its code or exact command. Task 5 asks the implementer to read the ladder's shape before editing one block — that is a read instruction with the target block quoted, not a placeholder.
- **Type consistency:** `DeliverabilityVerdict::{Boots, Broken(BrokenReason), NotJudged(NotJudgedWhy)}`, `header_value()`, `reason_value()` are used identically in Tasks 1–3; `with_deliverability_headers` and `deliverability_lookup` are defined in Task 2 and used only there; the gate's exit codes (0/2/3) match between Task 4's script, its test, and Task 5's Groovy.

## Landed (2026-09-05)

Eleven commits on dev, each reviewed (spec + quality) and re-reviewed after its fix round; final whole-branch review BLOCK → SHIP after one fix wave:
`b75362ac3` `e92d3b8e6` (Task 1) · `a75ab6855` (Task 2) · `67da5542d` `13a459a61` (Task 3) · `78466b2bc` (Task 4) · `245dc874f` `df8c51bb0` (Task 5) · `f81763594` (Task 6) · `5335626e6` `d7bd2c5ad` `81812d1e7` (final fix wave).

Stated deviations from the text above, all ruled in the run ledger: the mod list is in `src/lib.rs`; seam-registry test keys follow the live schema; asset resolution follows browser semantics (root-relative at the root only, document-relative at the index's directory only — the two-way fallback in Task 1 was a plan defect); the gate applies to `KIND=browser` bundles only (server bundles have no `index.html`; their renderer criterion is slice 2); a gate rc outside {0,2,3} is a loud advisory; `authorHeadOnce` skips a `BROKEN_HEAD` bundle and one `error()` after Phase 2 fails the build (the brief's in-loop `error()` was swallowed by `catchError`); the verdict marker is cleared before each host's run; the Jenkinsfile byte ceiling was a wrong proxy; the p2p-featured storage test run defers to the pre-push gate (RAM guard); the household-mesh story run defers to the fleet build (mesh lease held) and the refusal proof ran against a standalone storage instance.

Carried to slice 2: `deliverability` on the content row; SQLite persistence of the memo; judgement at declared-head adoption; `ServerRenderFails`; counter per distinct CID rather than per extraction; `<base href>` and `modulepreload` refs; `_capability` header for an un-indexed slug.
