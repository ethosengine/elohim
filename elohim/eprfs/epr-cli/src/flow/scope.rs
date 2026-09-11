//! `epr flow report scope` and `epr flow hold --scope` — the plate reconciled to the substrate.
//!
//! The native port of `scope-reconcile.py` (the mover + its SessionStart report) and
//! `focus-baseline.py` (the reader twin). One substrate, two readings: the report says what the
//! plate WOULD reconcile to, the hold verb performs it.
//!
//! **The tree IS the plate.** A doc declaring `requires_env:` a capability `cluster-state.yaml` does
//! not currently mark available is `git mv`d into a parallel `held/` tree OUTSIDE the planner and
//! runner scan paths — structurally sequestered, so it cannot false-fail or consume planning focus.
//! When the capability returns it moves back. A `git mv` broadcasts the scope change atomically to
//! every consumer at once, which is why the move is a filesystem act rather than a flag.
//!
//! **Held is not deleted, and not regressed.** Partial availability is the steady state. The three
//! asymmetries below all follow from that, and each has cost real work when it was got wrong:
//!
//! - **Gap-granular, not doc-granular.** A doc is held whole only when EVERY one of its stations is
//!   blocked. A mixed plan — most stations testable on household nodes, a few needing an absent
//!   canvas — stays on the plate. This is what "iroh ≠ shem" means operationally.
//! - **held → live requires AFFIRMATIVE evidence.** A held doc with no parseable scope info STAYS
//!   held and is reported as an anomaly for operator triage. Auto-publishing on the absence of a
//!   reason is how a doc escapes into a runner that cannot run it.
//! - **An unknown capability blocks.** A `requires_env` naming no cluster-state resource is vocab
//!   drift (`harbor` vs `harbor-registry`), and a capability nobody tracks might be the real blocker
//!   hiding under a wrong name. On `.feature` files the same unknown is IGNORED, because a2o's
//!   `@requires:` namespace mixes hardware capabilities with fixture preconditions.
//!
//! The gap source is the DOCUMENT, not a decompose cache: [`super::gaps`] extracts the same stations
//! `decompose.py` wrote into `gap-items/`, so the hold decision reads the plan rather than a
//! rendering of the plan that could be stale.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use super::cluster_state::{self, ClusterState};
use super::env_scope;
use super::gaps;
use super::placement::requires_env_of;
use super::{parse_frontmatter, FlowError, FlowResult};

/// `(live_dir, held_dir)`. The held dir is OUTSIDE the live scan/glob path, so a held doc is
/// structurally invisible to the planner and the runner rather than merely flagged.
const ZONES: [(&str, &str); 4] = [
    (
        "genesis/docs/superpowers/specs",
        "genesis/docs/superpowers/held/specs",
    ),
    (
        "genesis/docs/superpowers/plans",
        "genesis/docs/superpowers/held/plans",
    ),
    ("genesis/docs/plans", "genesis/docs/held/plans"),
    ("genesis/a2o/features", "genesis/a2o/held/features"),
];

/// The provenance marker on a derived `suspended` flag. The reconciler only ever touches its OWN
/// flags — an operator-manual suspension without this marker survives every cascade.
const PROVENANCE: &str = "scope-reconcile:";

/// The cluster resource whose availability drives `ELOHIM_REMOTE_COMPUTE_STATUS`.
const REMOTE_COMPUTE_RESOURCE: &str = "shem";

const STOP: &str = r#"# ⛔ STOP — held artifacts (require unavailable hardware)

The docs under this `held/` tree declare a `requires_env:` capability that is **not currently available**
(see `genesis/manifests/cluster-state.yaml`). They are sequestered here, OUTSIDE the planner/runner scan
path, so they don't false-fail or consume planning focus. This is **held, NOT deleted or regressed** —
partial availability is the steady state.

- Do NOT edit or "fix" these as if broken. They are correct; the hardware is absent.
- Inbound `cites:` to these resolve as **HELD-CITE** (content-addressed; not dead) — do not delete the link.
- They move back automatically when their capability returns: `epr flow hold --scope --apply`.

Managed by `epr flow hold --scope`. The mover, not a human, files things here.
"#;

/// One proposed live↔held move.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeMove {
    pub src: String,
    pub dst: String,
    /// The blocking capabilities. Empty on a return-to-plate whose capability simply came back.
    pub caps: Vec<String>,
    /// The destination is already occupied, so this move is REFUSED.
    ///
    /// Derived WITH the move rather than discovered at write time. A collision the operator learns
    /// about only from `--apply` is one they learn about mid-move, and the dry run's whole job is to
    /// be the cheap half of the loop. One of the two files is stale and clobbering would destroy
    /// content, so neither side is touched until a human says which.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub conflict: bool,
}

impl ScopeMove {
    /// The CONFLICT line, rendered immediately under its own move — the kit's interleaving, so a
    /// reader sees WHICH move was refused rather than a list of paths after the summary. Empty when
    /// there is no collision.
    fn render_conflict(&self) -> String {
        if self.conflict {
            format!(
                "  ⚠ CONFLICT  {} -> {} (dst occupied — SKIPPED, operator triage)\n",
                self.src, self.dst
            )
        } else {
            String::new()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VocabWarning {
    pub path: String,
    pub caps: Vec<String>,
}

/// One deployments.json human whose `suspended` flag disagrees with the derivation.
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentDrift {
    pub name: String,
    /// `suspend` | `unsuspend` | `adopt` | `manual-hold` | `vocab`.
    pub action: String,
    pub caps: Vec<String>,
}

impl DeploymentDrift {
    /// Whether this drift is something the reconciler may act on. `manual-hold` and `vocab` are
    /// reports, never writes.
    pub fn actionable(&self) -> bool {
        matches!(self.action.as_str(), "suspend" | "unsuspend" | "adopt")
    }
}

/// The scope reading.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeReport {
    pub command: String,
    pub cluster_state: String,
    pub updated: String,
    pub available: Vec<String>,
    /// `name -> raw scalar` for every declared-but-not-available resource.
    pub unavailable: BTreeMap<String, String>,
    pub to_held: Vec<ScopeMove>,
    pub to_live: Vec<ScopeMove>,
    pub vocab: Vec<VocabWarning>,
    /// Held docs with no parseable scope info — they STAY held; operator triage.
    pub held_anomalies: Vec<String>,
    pub deployments: Vec<DeploymentDrift>,
    /// The `scope-pending-moves@1` fold value: held + live + actionable deployment flags. The kit's
    /// `_observation.py` summed exactly these three from the printed line; folding the number here
    /// lets that bridge stop parsing prose.
    pub pending_moves: usize,
}

impl ScopeReport {
    /// The parts segment of the headline — everything before the `→` pointer.
    ///
    /// Split out because the pointer is the one piece that legitimately differs between the kit's
    /// line and the native one (the kit names its own script), while the counts and capability names
    /// must be identical. A parity test compares THIS.
    pub fn headline_parts(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.to_live.is_empty() {
            // "to return to plate" covers both a returned capability and a mixed plan with
            // household-testable stations. Do NOT imply the cap returned: a mixed doc's caps are
            // still blocked, and saying "ready to expand" there would be a false all-clear.
            parts.push(format!("{} to return to plate", self.to_live.len()));
        }
        if !self.to_held.is_empty() {
            let caps = self.held_caps();
            let note = if caps.is_empty() {
                String::new()
            } else {
                format!(" ({})", caps.join(","))
            };
            parts.push(format!("{} to hold{note}", self.to_held.len()));
        }
        let actionable: Vec<&DeploymentDrift> =
            self.deployments.iter().filter(|d| d.actionable()).collect();
        if !actionable.is_empty() {
            let caps: BTreeSet<&String> = actionable.iter().flat_map(|d| d.caps.iter()).collect();
            let note = if caps.is_empty() {
                String::new()
            } else {
                format!(
                    " ({})",
                    caps.iter()
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            parts.push(format!("{} deployment flag(s){note}", actionable.len()));
        }
        if parts.is_empty() {
            "scope: aligned ✅  (plate matches substrate)".to_string()
        } else {
            format!("scope: ⚠ {}", parts.join(" · "))
        }
    }

    fn held_caps(&self) -> Vec<String> {
        let set: BTreeSet<&String> = self
            .to_held
            .iter()
            .flat_map(|m| m.caps.iter())
            .filter(|c| !c.is_empty())
            .collect();
        set.into_iter().cloned().collect()
    }

    /// The vocab-drift suffix — `⚠ unknown-cap: X` plus the nodeType arm.
    pub fn vocab_note(&self) -> String {
        let mut note = String::new();
        if !self.vocab.is_empty() {
            let caps: BTreeSet<&String> = self.vocab.iter().flat_map(|v| v.caps.iter()).collect();
            note.push_str(&format!(
                "  ⚠ unknown-cap: {} (vocab drift vs cluster-state)",
                caps.iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        let dep_vocab: BTreeSet<&String> = self
            .deployments
            .iter()
            .filter(|d| d.action == "vocab")
            .map(|d| &d.name)
            .collect();
        if !dep_vocab.is_empty() {
            note.push_str(&format!(
                "  ⚠ nodeType-vocab: {}",
                dep_vocab
                    .iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        note
    }

    /// The full SessionStart line: parts, the next-action pointer when there is something to do, and
    /// the vocab note.
    ///
    /// The pointer is the piece the native headline had LOST — a reader told "3 to hold" with no
    /// verb learns a fact and no move. `aligned` prints no pointer, because there is nothing to
    /// point at.
    pub fn headline(&self) -> String {
        let parts = self.headline_parts();
        if self.pending_moves == 0 {
            format!("{parts}{}", self.vocab_note())
        } else {
            format!(
                "{parts}  →  epr flow hold --scope --apply{}",
                self.vocab_note()
            )
        }
    }

    /// The dry-run/apply rendering — one line per proposed move, then the summary.
    ///
    /// Returns the text rather than printing it, following [`FocusBaseline::render`] two functions
    /// away. The caller prints; a test asserts on the SHIPPED string. That distinction is not
    /// stylistic: while this printed directly, the only way to test its output was to re-implement
    /// it in the test — and a test that asserts against its own copy of a renderer passes happily
    /// after the real one is deleted, which is exactly how the CONFLICT line survived a round of
    /// review as a closed finding while being absent from the binary.
    pub fn render(&self, applied: bool) -> String {
        let mut out = String::new();
        for m in &self.to_held {
            out.push_str(&format!(
                "  → HELD   {}  (every gap blocked; needs {})\n",
                m.src,
                if m.caps.is_empty() {
                    "an unavailable cap".to_string()
                } else {
                    format!("{:?}", m.caps)
                }
            ));
            out.push_str(&m.render_conflict());
        }
        for m in &self.to_live {
            if m.caps.is_empty() {
                out.push_str(&format!(
                    "  ← LIVE   {}  (capability available again)\n",
                    m.dst
                ));
            } else {
                out.push_str(&format!(
                    "  ← LIVE   {}  (mixed-scope: has household-testable gaps; some still need {:?})\n",
                    m.dst, m.caps
                ));
            }
            out.push_str(&m.render_conflict());
        }
        for path in &self.held_anomalies {
            out.push_str(&format!(
                "  ⚠ HELD-STAYS  {path}  (no parseable requires_env — cannot confirm safe to publish; operator triage)\n"
            ));
        }
        for v in &self.vocab {
            out.push_str(&format!(
                "  ⚠ VOCAB  {}  requires_env {:?} unknown to cluster-state (reconcile the name)\n",
                v.path, v.caps
            ));
        }
        for d in &self.deployments {
            out.push_str(&match d.action.as_str() {
                "suspend" => format!(
                    "  ⏸ SUSPEND   {}  (no nodeType placeable; needs {:?})\n",
                    d.name, d.caps
                ),
                "unsuspend" => format!("  ▶ UNSUSPEND {}  (capability returned)\n", d.name),
                "adopt" => format!(
                    "  ✦ ADOPT     {}  (suspended matches derivation; adding provenance marker)\n",
                    d.name
                ),
                "manual-hold" => format!(
                    "  ⚠ MANUAL-HOLD {}  (operator-owned suspension; derivation says placeable — left alone)\n",
                    d.name
                ),
                _ => format!(
                    "  ⚠ VOCAB     {}  nodeTypes {:?} match no provides_node_types in cluster-state\n",
                    d.name, d.caps
                ),
            });
        }
        let verb = if applied { "APPLIED" } else { "DRY-RUN" };
        let avail = if self.available.is_empty() {
            "(none)".to_string()
        } else {
            format!("{:?}", self.available)
        };
        out.push_str(&format!(
            "epr flow hold --scope [{verb}]: available={avail}\n"
        ));
        let dep = self.deployments.iter().filter(|d| d.actionable()).count();
        out.push_str(&format!(
            "  → held: {}   ← live: {}   ⏸ deployments: {}\n",
            self.to_held.len(),
            self.to_live.len(),
            dep
        ));
        if !applied && self.pending_moves > 0 {
            out.push_str("  (re-run with --apply to git mv + reconcile deployment flags)\n");
        }
        out
    }
}

/// The scope substrate, read once and shared by the report, the mover and the focus baseline.
struct Substrate {
    path: PathBuf,
    /// Where to look for the act lane-contract siblings, in order. The manifest's OWN directory
    /// first, then the repository's `genesis/manifests` — so a `--cluster-state` override pointed at
    /// an isolated copy still finds the lane contracts, instead of silently losing every `@act:`
    /// rescue and proposing lane-only features for `held/`.
    manifests_dirs: Vec<PathBuf>,
    state: ClusterState,
    available: BTreeSet<String>,
    /// ALL resource names — the vocabulary. Wider than the declared set on purpose (see the module
    /// header's "an unknown capability blocks").
    known: BTreeSet<String>,
}

fn substrate(root: &Path, override_path: Option<&Path>) -> Substrate {
    let path = override_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("genesis/manifests/cluster-state.yaml"));
    let repo_manifests = root.join("genesis/manifests");
    let mut manifests_dirs = Vec::with_capacity(2);
    if let Some(own) = path.parent() {
        manifests_dirs.push(own.to_path_buf());
    }
    if !manifests_dirs.contains(&repo_manifests) {
        manifests_dirs.push(repo_manifests);
    }
    let state = cluster_state::load(&path);
    let available = state.available_names();
    let known = state.all_names();
    Substrate {
        path,
        manifests_dirs,
        state,
        available,
        known,
    }
}

/// The scope verdict for one artifact.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every station blocked — uniformly out of scope.
    Held,
    /// At least one satisfiable station — mixed or testable, belongs on the plate.
    Live,
    /// No scope information at all. A held doc in this state STAYS held.
    Ambiguous,
}

/// The capabilities `<doc>` declares. `.feature` files read `@requires:` tags above the first
/// `Feature:` keyword; everything else reads frontmatter `requires_env`.
fn declared_caps(path: &Path, text: &str) -> Vec<String> {
    if path.extension().is_some_and(|e| e == "feature") {
        return feature_caps(text);
    }
    requires_env_of(&parse_frontmatter(text))
}

/// Feature-LEVEL `@requires:` tags — the tag line(s) above `Feature:`, gherkin comments skipped.
fn feature_caps(text: &str) -> Vec<String> {
    let mut caps = Vec::new();
    for line in text.lines() {
        let stripped = line.trim_start();
        if stripped.starts_with("Feature:") {
            break;
        }
        if stripped.starts_with('#') {
            continue;
        }
        caps.extend(env_scope::requires_tags(stripped));
    }
    caps
}

/// Scenario-LEVEL `@requires:` tags — everything BELOW the first `Feature:`. These are runtime-
/// skipped, never held: the file stays on the plate and the scenario sits out.
fn scenario_caps(text: &str) -> Vec<String> {
    let mut caps = Vec::new();
    let mut seen_feature = false;
    for line in text.lines() {
        if line.trim_start().starts_with("Feature:") {
            seen_feature = true;
            continue;
        }
        if seen_feature {
            caps.extend(env_scope::requires_tags(line));
        }
    }
    caps
}

/// The `available` set to gate one artifact against.
///
/// `.md` docs are unaffected: frontmatter `requires_env` has no act concept, so their semantics stay
/// exactly the live manifest's set. A `.feature` declaring `@act:<id>` gets that act's OWN baseline
/// UNIONed in — additive only, never removing a live-available cap. A feature tagged
/// `@act:i @requires:owned-substrate` is exercised ONLY via the Act I lane, where owned-substrate is
/// available; gating it on the default lane's deliberate withholding would be a false HELD.
fn effective_available(path: &Path, text: &str, sub: &Substrate) -> BTreeSet<String> {
    if !path.extension().is_some_and(|e| e == "feature") {
        return sub.available.clone();
    }
    let Some(act) = env_scope::feature_act(text) else {
        return sub.available.clone();
    };
    let mut caps = sub.available.clone();
    caps.extend(env_scope::act_baseline_caps_in(&act, &sub.manifests_dirs));
    caps
}

/// `(verdict, blocking caps)`.
///
/// The doc's LIVE frontmatter `requires_env` is the inheritance default, read fresh from the file,
/// so a doc-level scope change takes effect immediately. Its own stations supply the per-gap
/// `@requires:` OVERRIDES; a doc with no extractable stations decides purely on the default.
fn scope_verdict(
    path: &Path,
    text: &str,
    available: &BTreeSet<String>,
    known: &BTreeSet<String>,
) -> (Verdict, Vec<String>) {
    let doc_req: Vec<String> = declared_caps(path, text)
        .into_iter()
        .filter(|c| known.contains(c))
        .collect();

    // Stations exist only for markdown docs; a `.feature` is one artifact, not a station list.
    if !path.extension().is_some_and(|e| e == "feature") {
        let slug = gaps::slug_for(path);
        let decomposition = gaps::decompose(&slug, text, doc_req.clone());
        // The station path is taken only when there is SCOPE INFORMATION to resolve — a doc-level
        // `requires_env`, or at least one station carrying an `@requires:` tag.
        //
        // Stations alone are not scope information, and reading them as such quietly repeals the
        // affirmative-evidence rule this module's header states: every station of a doc that says
        // nothing about scope is trivially "not blocked", so the doc reads `Live`, and a held doc
        // with no scope info at all would be proposed back onto the plate for the sole reason that
        // it has checkboxes. The kit reaches its station path only when a decompose RECORD exists,
        // which is why it never had this shape; deriving stations from every document is what
        // exposes it. Ambiguity and station-derivation are separable, and this is the separation.
        let has_scope_info = !doc_req.is_empty()
            || decomposition
                .items
                .iter()
                .any(|item| !item.requires_env.is_empty());
        if !decomposition.items.is_empty() && has_scope_info {
            let mut blocked_caps: BTreeSet<String> = BTreeSet::new();
            let mut satisfiable = false;
            for item in &decomposition.items {
                let resolved = env_scope::resolved_requires_env(&item.requires_env, &doc_req);
                if env_scope::gap_blocked(&resolved, available, known) {
                    blocked_caps.extend(
                        resolved
                            .into_iter()
                            .filter(|c| known.contains(c) && !available.contains(c)),
                    );
                } else {
                    satisfiable = true;
                }
            }
            let verdict = if satisfiable {
                Verdict::Live
            } else {
                Verdict::Held
            };
            return (verdict, blocked_caps.into_iter().collect());
        }
    }

    if doc_req.is_empty() {
        return (Verdict::Ambiguous, Vec::new());
    }
    let missing: Vec<String> = doc_req
        .iter()
        .filter(|c| !available.contains(*c))
        .cloned()
        .collect();
    let verdict = if missing.is_empty() {
        Verdict::Live
    } else {
        Verdict::Held
    };
    (verdict, missing)
}

/// Every `.md` and `.feature` under `dir`, recursively, sorted — `.md` first, then `.feature`,
/// matching the mover's two-pass walk.
fn zone_files(dir: &Path) -> Vec<PathBuf> {
    let mut md = Vec::new();
    let mut feature = Vec::new();
    collect(dir, &mut md, &mut feature);
    md.sort();
    feature.sort();
    md.extend(feature);
    md
}

fn collect(dir: &Path, md: &mut Vec<PathBuf>, feature: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(&path, md, feature);
        } else if path.extension().is_some_and(|e| e == "md") {
            md.push(path);
        } else if path.extension().is_some_and(|e| e == "feature") {
            feature.push(path);
        }
    }
}

/// Read an artifact the way every kit reader reads one: `errors="replace"`.
///
/// A non-UTF-8 byte in one document must not remove it from the scan. A doc silently absent from
/// the scope walk is a doc that can never be held and never returned — the mover would simply not
/// see it, which is a worse failure than any mojibake in a printed path.
fn read_lossy(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Derive the whole scope reading. Pure — no side effects.
pub fn scope_report(root: &Path, cluster_state_override: Option<&Path>) -> FlowResult<ScopeReport> {
    let sub = substrate(root, cluster_state_override);
    let mut to_held = Vec::new();
    let mut to_live = Vec::new();
    let mut vocab = Vec::new();
    let mut held_anomalies = Vec::new();

    for (live_rel, held_rel) in ZONES {
        let live = root.join(live_rel);
        let held = root.join(held_rel);
        for path in zone_files(&live) {
            let Some(text) = read_lossy(&path) else {
                continue;
            };
            record_vocab(&path, &text, &sub, &mut vocab, root);
            let available = effective_available(&path, &text, &sub);
            let (verdict, caps) = scope_verdict(&path, &text, &available, &sub.known);
            if verdict == Verdict::Held {
                let suffix = path.strip_prefix(&live).unwrap_or(&path);
                let destination = held.join(suffix);
                to_held.push(ScopeMove {
                    src: rel(root, &path),
                    dst: rel(root, &destination),
                    caps,
                    conflict: destination.exists(),
                });
            }
        }
        for path in zone_files(&held) {
            if path.file_name().is_some_and(|n| n == "CLAUDE.md") {
                continue;
            }
            let Some(text) = read_lossy(&path) else {
                continue;
            };
            record_vocab(&path, &text, &sub, &mut vocab, root);
            let available = effective_available(&path, &text, &sub);
            let (verdict, caps) = scope_verdict(&path, &text, &available, &sub.known);
            match verdict {
                Verdict::Live => {
                    let suffix = path.strip_prefix(&held).unwrap_or(&path);
                    let destination = live.join(suffix);
                    to_live.push(ScopeMove {
                        src: rel(root, &path),
                        dst: rel(root, &destination),
                        caps,
                        conflict: destination.exists(),
                    });
                }
                // held → live needs AFFIRMATIVE evidence. No parseable scope info is not evidence.
                Verdict::Ambiguous => held_anomalies.push(rel(root, &path)),
                Verdict::Held => {}
            }
        }
    }

    let deployments = deployment_drift(root, &sub);
    let pending =
        to_held.len() + to_live.len() + deployments.iter().filter(|d| d.actionable()).count();

    Ok(ScopeReport {
        command: "flow report scope".into(),
        cluster_state: rel(root, &sub.path),
        updated: sub.state.updated.clone(),
        available: sub.available.iter().cloned().collect(),
        unavailable: sub
            .state
            .available_map()
            .into_iter()
            .filter(|(_, v)| v != "true")
            .collect(),
        to_held,
        to_live,
        vocab,
        held_anomalies,
        deployments,
        pending_moves: pending,
    })
}

/// An unknown capability on a `.md` is vocab drift and is recorded. On a `.feature` it is IGNORED:
/// a2o's `@requires:` namespace mixes hardware capabilities with fixture preconditions
/// (`doorway`, `seeded-content`), and warning on those would bury the real drift.
fn record_vocab(
    path: &Path,
    text: &str,
    sub: &Substrate,
    out: &mut Vec<VocabWarning>,
    root: &Path,
) {
    if path.extension().is_some_and(|e| e == "feature") {
        return;
    }
    let unknown: Vec<String> = declared_caps(path, text)
        .into_iter()
        .filter(|c| !sub.known.contains(c))
        .collect();
    if !unknown.is_empty() {
        out.push(VocabWarning {
            path: rel(root, path),
            caps: unknown,
        });
    }
}

// ── deployments.json arm ────────────────────────────────────────────────────────────────────────
// `suspended` gates deploy-render, seeding and the a2o test lane. It is DERIVED from cluster-state
// so the two homes cannot drift apart — which they did on 2026-06-03, when eleven shem-only humans
// stayed declared-deployed for days after shem went down and every genesis run hammered a
// non-resolving conductor.

fn deployments_path(root: &Path) -> PathBuf {
    root.join("genesis/orchestrator/data/deployments.json")
}

fn humans(root: &Path) -> Vec<serde_json::Value> {
    let path = deployments_path(root);
    // An ABSENT file is honest absence — a repository with no deployments manifest has no humans to
    // reconcile. An UNREADABLE or UNPARSEABLE one is not: it would render as "0 deployment drift",
    // which reads as "nothing to do" about a question that could not be asked. Say so on stderr;
    // the reading still degrades to empty rather than taking the whole scope report down.
    if !path.exists() {
        return Vec::new();
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!(
                "scope: {} is unreadable ({err}) — deployment drift reported as 0, which is UNKNOWN, not none",
                path.display()
            );
            return Vec::new();
        }
    };
    let value = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "scope: {} is unparseable ({err}) — deployment drift reported as 0, which is UNKNOWN, not none",
                path.display()
            );
            return Vec::new();
        }
    };
    match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(map) => map
            .get("humans")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Pure derivation, no side effects.
pub fn deployment_drift_at(
    root: &Path,
    cluster_state_override: Option<&Path>,
) -> Vec<DeploymentDrift> {
    deployment_drift(root, &substrate(root, cluster_state_override))
}

fn deployment_drift(root: &Path, sub: &Substrate) -> Vec<DeploymentDrift> {
    let provides = sub.state.provides_map();
    let mut out = Vec::new();
    for human in humans(root) {
        let Some(name) = human.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let node_types: Vec<String> = human
            .get("nodeTypes")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let caps_per_nt: Vec<BTreeSet<String>> = node_types
            .iter()
            .map(|nt| provides.get(nt).cloned().unwrap_or_default())
            .collect();
        if !node_types.is_empty() && caps_per_nt.iter().all(BTreeSet::is_empty) {
            out.push(DeploymentDrift {
                name: name.to_string(),
                action: "vocab".into(),
                caps: node_types,
            });
            continue;
        }
        let placeable = caps_per_nt
            .iter()
            .any(|caps| caps.iter().any(|c| sub.available.contains(c)));
        let missing: Vec<String> = caps_per_nt
            .iter()
            .flatten()
            .filter(|c| !sub.available.contains(*c))
            .cloned()
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let suspended = human
            .get("suspended")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let ours = human
            .get("suspendedBy")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|v| v.starts_with(PROVENANCE));
        let action = match (placeable, suspended, ours) {
            (false, false, _) => "suspend",
            (false, true, false) => "adopt",
            (true, true, true) => "unsuspend",
            (true, true, false) => "manual-hold",
            _ => continue,
        };
        out.push(DeploymentDrift {
            name: name.to_string(),
            action: action.into(),
            caps: if action == "unsuspend" {
                Vec::new()
            } else {
                missing
            },
        });
    }
    out
}

/// Line-edit `deployments.json` to match the derivation.
///
/// Line-based so the hand-maintained formatting and `$comment` fields survive, and re-validated with
/// a JSON parse before the write — an unparseable result is DISCARDED. Fail closed: this file gates
/// every deploy.
fn reconcile_deployments(root: &Path, drift: &[DeploymentDrift]) -> FlowResult<()> {
    let actionable: Vec<&DeploymentDrift> = drift.iter().filter(|d| d.actionable()).collect();
    if actionable.is_empty() {
        return Ok(());
    }
    let path = deployments_path(root);
    // A write the caller asked for that silently does nothing is the failure this arm must not have:
    // `--apply` would print APPLIED having reconciled nothing. Refuse by name instead.
    let text = std::fs::read_to_string(&path).map_err(|err| {
        FlowError::InvalidArguments(format!(
            "{} names {} actionable deployment flag(s) but {} is unreadable ({err})",
            "the derivation",
            actionable.len(),
            path.display()
        ))
    })?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for d in actionable {
        let needle = format!("\"name\": \"{}\",", d.name);
        let Some(idx) = lines.iter().position(|l| l.trim() == needle) else {
            continue;
        };
        let indent: String = lines[idx]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let end = lines
            .iter()
            .enumerate()
            .skip(idx + 1)
            .find(|(_, l)| l.trim_start().starts_with("\"name\":"))
            .map(|(k, _)| k)
            .unwrap_or(lines.len());
        let sus = (idx..end).find(|k| lines[*k].trim_start().starts_with("\"suspended\":"));
        let prov = (idx..end).find(|k| lines[*k].trim_start().starts_with("\"suspendedBy\":"));
        let provline = format!(
            "{indent}\"suspendedBy\": \"{PROVENANCE}{}\",",
            d.caps.join(",")
        );
        match d.action.as_str() {
            "suspend" => match sus {
                Some(s) => {
                    lines[s] = format!("{indent}\"suspended\": true,");
                    match prov {
                        Some(p) => lines[p] = provline,
                        None => lines.insert(s + 1, provline),
                    }
                }
                None => {
                    lines.insert(idx + 1, provline);
                    lines.insert(idx + 1, format!("{indent}\"suspended\": true,"));
                }
            },
            "adopt" => match prov {
                Some(p) => lines[p] = provline,
                None => {
                    if let Some(s) = sus {
                        lines.insert(s + 1, provline);
                    }
                }
            },
            "unsuspend" => {
                let mut targets: Vec<usize> = [sus, prov].into_iter().flatten().collect();
                targets.sort_unstable_by(|a, b| b.cmp(a));
                for k in targets {
                    lines.remove(k);
                }
            }
            _ => {}
        }
    }
    let new = format!("{}\n", lines.join("\n"));
    serde_json::from_str::<serde_json::Value>(&new).map_err(|err| {
        FlowError::InvalidArguments(format!(
            "refusing to write {}: the line edit produced unparseable JSON ({err})",
            path.display()
        ))
    })?;
    std::fs::write(&path, new)?;
    Ok(())
}

// ── the mover ───────────────────────────────────────────────────────────────────────────────────

/// Move `src` → `dst`.
///
/// The last-line defence only. An occupied destination is refused here too, but it was already
/// derived as [`ScopeMove::conflict`] and printed by the renderer — for the dry run as much as for
/// the apply — so reaching this arm means the tree changed between the derivation and the write.
fn git_mv(root: &Path, src: &Path, dst: &Path) -> FlowResult<bool> {
    if dst.exists() {
        return Ok(false);
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("git")
        .arg("mv")
        .arg(src)
        .arg(dst)
        .current_dir(root)
        .output();
    let moved = status.map(|o| o.status.success()).unwrap_or(false);
    if !moved {
        // Not git-tracked → a plain rename, then `git add` so git-as-ledger still holds.
        std::fs::rename(src, dst)?;
        let _ = Command::new("git")
            .arg("add")
            .arg(dst)
            .current_dir(root)
            .output();
    }
    Ok(true)
}

/// Perform the reconciliation the report proposes. `apply == false` is a dry run that prints exactly
/// what `--apply` would do.
pub fn hold_scope(
    root: &Path,
    apply: bool,
    cluster_state_override: Option<&Path>,
) -> FlowResult<ScopeReport> {
    let report = scope_report(root, cluster_state_override)?;
    print!("{}", report.render(apply));
    if !apply {
        return Ok(report);
    }
    for m in report.to_held.iter().chain(report.to_live.iter()) {
        // Refused at the DERIVATION, not at the write: the CONFLICT line is already on screen,
        // printed by the same render the dry run prints.
        if m.conflict {
            continue;
        }
        git_mv(root, &root.join(&m.src), &root.join(&m.dst))?;
    }
    // STOP markers + orphan cleanup: a zone with real held docs gets the STOP; an emptied zone has
    // its orphan STOP removed and its now-empty subdirs pruned.
    for (_, held_rel) in ZONES {
        let held = root.join(held_rel);
        if !held.is_dir() {
            continue;
        }
        let real: Vec<PathBuf> = zone_files(&held)
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|n| n != "CLAUDE.md"))
            .collect();
        let zone_root = match held.file_name().and_then(|n| n.to_str()) {
            Some("specs") | Some("plans") | Some("features") => held
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| held.clone()),
            _ => held.clone(),
        };
        if !real.is_empty() {
            let stop = zone_root.join("CLAUDE.md");
            if !stop.exists() {
                std::fs::create_dir_all(&zone_root)?;
                std::fs::write(&stop, STOP)?;
            }
        } else {
            let stop = zone_root.join("CLAUDE.md");
            if stop.exists() {
                let _ = Command::new("git")
                    .args(["rm", "-q", "--ignore-unmatch"])
                    .arg(&stop)
                    .current_dir(root)
                    .output();
                if stop.exists() {
                    std::fs::remove_file(&stop)?;
                }
            }
            prune_empty_descendants(&held);
        }
    }
    reconcile_deployments(root, &report.deployments)?;
    // Keep the co-located focus baseline in lockstep with the plate, so a fresh agent reads an
    // always-current "what's in focus vs held" without running anything.
    let baseline = focus_baseline(root, cluster_state_override)?;
    // Create the parent first. A repository with no `.claude/` (a fresh clone, a scoped worktree)
    // otherwise took every move, printed APPLIED, and THEN exited non-zero on the baseline write —
    // an error after the work, which reads as "the reconcile failed" when it succeeded.
    let baseline_path = root.join(".claude/subject-focus.md");
    if let Some(parent) = baseline_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&baseline_path, baseline.render(None))?;
    // Re-derive so the returned report describes the tree AFTER the move, not before it.
    scope_report(root, cluster_state_override)
}

/// Remove empty subdirectories UNDER `dir`, never `dir` itself.
///
/// The zone root stays. The kit's `rglob("*")` walk can only ever reach descendants, so deleting the
/// root is a removal Python would not make — and the root is a meaningful location whose
/// disappearance reads as "this zone no longer exists" rather than "this zone is currently empty".
fn prune_empty_descendants(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    for sub in subdirs {
        prune_empty_tree(&sub);
    }
}

/// Depth-first: prune this directory's children, then this directory if it is now empty.
fn prune_empty_tree(dir: &Path) {
    prune_empty_descendants(dir);
    let empty = std::fs::read_dir(dir)
        .map(|mut e| e.next().is_none())
        .unwrap_or(false);
    if empty {
        let _ = std::fs::remove_dir(dir);
    }
}

// ── the developer flip ──────────────────────────────────────────────────────────────────────────
// `shem` is a RUNTIME capability, but a developer flips it deliberately rather than waiting for a
// probe. The two signal homes must stay coherent: `cluster-state.yaml` is durable and planning-
// facing; `ELOHIM_REMOTE_COMPUTE_STATUS` is runtime. The runtime signal is DERIVED here, so they
// cannot disagree.

/// `on|available|true` → `true`; `off|unavailable|false` → `false`; `degraded` → `degraded`.
fn state_word(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "on" | "available" | "true" => Some("true"),
        "off" | "unavailable" | "false" => Some("false"),
        "degraded" => Some("degraded"),
        _ => None,
    }
}

/// The runtime export, derived from cluster-state.
pub fn remote_compute_status(root: &Path, cluster_state_override: Option<&Path>) -> &'static str {
    if substrate(root, cluster_state_override)
        .available
        .contains(REMOTE_COMPUTE_RESOURCE)
    {
        "available"
    } else {
        "unavailable"
    }
}

/// `export ELOHIM_REMOTE_COMPUTE_STATUS=…` — usage: `eval "$(epr flow hold --scope --env)"`.
pub fn env_line(root: &Path, cluster_state_override: Option<&Path>) -> String {
    format!(
        "export ELOHIM_REMOTE_COMPUTE_STATUS={}",
        remote_compute_status(root, cluster_state_override)
    )
}

/// Set `<resource>.available = <value>` in cluster-state.yaml. Line-based, matching the lenient
/// parser. Returns whether a line actually changed.
fn flip_resource(path: &Path, resource: &str, value: &str) -> FlowResult<bool> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(false);
    };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut in_block = false;
    let mut changed = false;
    for line in lines.iter_mut() {
        if line.trim_end() == format!("  {resource}:") {
            in_block = true;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(rest) = line.strip_prefix("    available:") {
            let leading: String = rest.chars().take_while(|c| c.is_whitespace()).collect();
            let after = rest.trim_start();
            let current = after.split_whitespace().next().unwrap_or("");
            // `^(    available:\s*)(\S+)(.*)$` — the kit's pattern needs a value to REPLACE. A bare
            // `available:` matches nothing there and is left alone; writing `    available:false`
            // into it would be a line only this implementation can produce, and the parser above
            // deliberately does not read a bare field as a declaration either. Keep scanning: a
            // later real `available:` line in the same block is still the one to edit.
            if current.is_empty() {
                continue;
            }
            let tail = &after[current.len()..];
            if current != value {
                *line = format!("    available:{leading}{value}{tail}");
                changed = true;
            }
            break;
        }
        if line.starts_with("  ") && line.chars().nth(2).is_some_and(|c| c.is_ascii_alphabetic()) {
            break; // the next resource — this block declares no `available:` line
        }
    }
    if changed {
        std::fs::write(path, format!("{}\n", lines.join("\n")))?;
    }
    Ok(changed)
}

/// `--set <resource>=<on|off|degraded>` — the developer flip. Edits the durable home, shows the
/// coherent runtime export, then reconciles the plate (dry-run unless `--apply`).
pub fn set_state(
    root: &Path,
    arg: &str,
    apply: bool,
    cluster_state_override: Option<&Path>,
) -> FlowResult<ScopeReport> {
    let Some((resource, want)) = arg.split_once('=') else {
        return Err(FlowError::InvalidArguments(
            "usage: epr flow hold --scope --set <resource>=<on|off|degraded>".into(),
        ));
    };
    let (resource, want) = (resource.trim(), want.trim());
    let Some(value) = state_word(want) else {
        return Err(FlowError::InvalidArguments(format!(
            "--set: unknown state `{want}` (use on|off|degraded)"
        )));
    };
    let sub = substrate(root, cluster_state_override);
    if !sub.known.contains(resource) {
        return Err(FlowError::InvalidArguments(format!(
            "--set: unknown resource `{resource}` (known: {:?})",
            sub.known
        )));
    }
    let changed = flip_resource(&sub.path, resource, value)?;
    println!(
        "cluster-state: {resource}.available = {value}{}",
        if changed { "" } else { "  (already set)" }
    );
    if resource == REMOTE_COMPUTE_RESOURCE {
        println!(
            "runtime ⇒  eval \"$(epr flow hold --scope --env)\"   # {}",
            env_line(root, cluster_state_override)
        );
    }
    println!();
    hold_scope(root, apply, cluster_state_override)
}

// ── the fold ────────────────────────────────────────────────────────────────────────────────────

/// Append the `scope-pending-moves@1` fold for this reading.
///
/// This is the producer the station-one observation bridge was standing in for. The bridge ran
/// `placement-audit.py --headline` and regex-scraped `N to hold` / `N to return to plate` /
/// `N deployment flag(s)` back out of the printed prose; the number is now computed here, so the
/// bridge has nothing left to parse.
///
/// Keyed by git HEAD, for the same reason the bridge keyed by it: the fold's identity is its
/// content, so two runs at one commit mint ONE atom (idempotent), while the same count returning at
/// a later commit is a new measurement rather than a silent re-read of the old one. Without the key
/// a value that went 3 → 0 → 3 would fold its third reading onto its first, and the report — which
/// takes the LATEST admissible fold — would answer 0.
///
/// Fail-open in every arm. A report that refused because the measure registry was mid-edit would be
/// a report nobody could rely on, and the fold is a by-product of the reading, never its point.
pub fn fold_pending_moves(root: &Path, pending_moves: usize) -> Option<String> {
    let measures = super::measures::default_measures(root);
    if !measures.is_file() {
        return None;
    }
    let mut env: BTreeMap<String, String> = BTreeMap::new();
    if let Some(head) = git_head(root) {
        env.insert("head".to_string(), head);
    }
    let actor = super::note::NoteActor {
        as_ref: None,
        session: None,
    };
    super::note::observe(
        root,
        "observation",
        "scope-pending-moves@1",
        ".",
        pending_moves as f64,
        Some("count"),
        &env,
        Some("epr flow report scope: held<->live moves plus actionable deployment flags pending against cluster-state"),
        &actor,
        &measures,
    )
    .ok()
    .map(|outcome| outcome.record_cid)
}

fn git_head(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

// ── the doc-scope reading (`placement-audit.py --focus`) ────────────────────────────────────────

/// The surfaces `placement-audit.py --focus` walks for documents that DECLARE `requires_env`.
///
/// Deliberately not the placement ledger's surface list: this walk skips `INDEX.md` and `CLAUDE.md`
/// only, keeps `README.md`, and includes `.claude/memory`. Transcribed rather than unified, because
/// the two readings answer different questions and quietly widening one to match the other would
/// move counts a consumer reads.
const DOC_SCOPE_SURFACES: [&str; 8] = [
    "genesis/docs/superpowers/specs",
    "genesis/docs/superpowers/plans",
    "genesis/docs/plans",
    "genesis/docs/content/elohim-protocol/architecture",
    "genesis/docs/content/elohim-protocol/history",
    "genesis/docs/superpowers/notes",
    "genesis/docs/research",
    ".claude/memory",
];

/// One document that declares an environment requirement.
#[derive(Debug, Clone, Serialize)]
pub struct DocScopeRow {
    pub path: String,
    pub requires_env: Vec<String>,
    /// The declared capabilities that are not currently available. Empty means in scope.
    pub missing: Vec<String>,
}

/// The currently-testable document surface — the native `placement-audit.py --focus`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocScopeReport {
    pub cluster_state: String,
    pub updated: String,
    pub available: Vec<String>,
    pub unavailable: BTreeMap<String, String>,
    pub in_scope: Vec<DocScopeRow>,
    pub blocked: Vec<DocScopeRow>,
}

/// Walk the doc surfaces for `requires_env` declarations and split them by availability.
pub fn doc_scope(root: &Path, cluster_state_override: Option<&Path>) -> FlowResult<DocScopeReport> {
    let sub = substrate(root, cluster_state_override);
    let mut in_scope = Vec::new();
    let mut blocked = Vec::new();
    for surface in DOC_SCOPE_SURFACES {
        let dir = root.join(surface);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
            .collect();
        files.sort();
        for file in files {
            let name = file.file_name().map(|n| n.to_string_lossy().to_string());
            if name.as_deref() == Some("INDEX.md") || name.as_deref() == Some("CLAUDE.md") {
                continue;
            }
            let Some(text) = read_lossy(&file) else {
                continue;
            };
            let caps = requires_env_of(&parse_frontmatter(&text));
            if caps.is_empty() {
                continue;
            }
            let missing: Vec<String> = caps
                .iter()
                .filter(|c| !sub.available.contains(*c))
                .cloned()
                .collect();
            let row = DocScopeRow {
                path: rel(root, &file),
                requires_env: caps,
                missing,
            };
            if row.missing.is_empty() {
                in_scope.push(row);
            } else {
                blocked.push(row);
            }
        }
    }
    Ok(DocScopeReport {
        cluster_state: sub
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        updated: sub.state.updated.clone(),
        available: sub.available.iter().cloned().collect(),
        unavailable: sub
            .state
            .available_map()
            .into_iter()
            .filter(|(_, v)| v != "true")
            .collect(),
        in_scope,
        blocked,
    })
}

/// Render a capability list the way the kit prints one, so a reader diffing the two sees no
/// difference where there is none.
fn py_list(items: &[String]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl DocScopeReport {
    pub fn render(&self) {
        println!(
            "FOCUS — currently-testable surface   (cluster-state: {} @ {})",
            self.cluster_state, self.updated
        );
        println!("{}", "=".repeat(72));
        let avail = if self.available.is_empty() {
            "(none)".to_string()
        } else {
            self.available.join(", ")
        };
        let unavail = if self.unavailable.is_empty() {
            "(none)".to_string()
        } else {
            self.unavailable
                .iter()
                .map(|(k, v)| format!("{k}({v})"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("  AVAILABLE   : {avail}");
        println!("  UNAVAILABLE : {unavail}");
        println!(
            "\n  IN SCOPE for refinement-stability ({}):",
            self.in_scope.len()
        );
        for row in &self.in_scope {
            println!(
                "    ✅ {}   requires_env: {}",
                row.path,
                py_list(&row.requires_env)
            );
        }
        if self.in_scope.is_empty() {
            println!("    (none declared)");
        }
        println!(
            "\n  BLOCKED-BY-ENV ({}) — HELD, out of scope now, NOT regressed:",
            self.blocked.len()
        );
        for row in &self.blocked {
            println!(
                "    ⛔ {}   needs {} → missing {}",
                row.path,
                py_list(&row.requires_env),
                py_list(&row.missing)
            );
        }
        if self.blocked.is_empty() {
            println!("    (none)");
        }
        println!("\n  NOTE: this lists only .md docs that DECLARE requires_env. The a2o TEST SURFACE (per-subject");
        println!("  in-focus vs held features/scenarios) is the subject baseline:");
        println!("    drill in: epr flow report placement --focus [<subject>] [--brief]");
        println!("\n  → Edit cluster-state.yaml and re-run: scope cascades immediately.");
    }
}

// ── the focus baseline ──────────────────────────────────────────────────────────────────────────

/// One subject's narrowing.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSubject {
    pub in_focus: Vec<String>,
    /// `(feature, held caps, scenario count)` — live features with scenario-level holds.
    pub mixed: Vec<MixedFeature>,
    /// `(feature, caps)` — features sequestered in the held tree.
    pub held: Vec<HeldFeature>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MixedFeature {
    pub path: String,
    pub caps: Vec<String>,
    pub scenarios: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeldFeature {
    /// Relative to the held features root, the way the baseline prints it.
    pub path: String,
    pub caps: Vec<String>,
}

/// The per-subject FOCUS baseline — the READER twin of the mover.
///
/// The model mirrors cluster-state itself: NO narrowing in a subject means it is FAIR-GAME, worked
/// freely; a capability going down NARROWS a subject, and the narrowing is rendered so an agent
/// picks the in-focus slice rather than bailing on in-scope work or burning effort on out-of-scope
/// artifacts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusBaseline {
    pub available: Vec<String>,
    pub unavailable: BTreeMap<String, String>,
    pub roles: BTreeMap<String, String>,
    pub subjects: BTreeMap<String, FocusSubject>,
}

/// The a2o feature-directory subject: the first path segment under `features/`.
///
/// A DELIVERABLE-TARGET key (path-based), never a vocabulary key — keying by `shem`/`iroh` would
/// re-introduce the name collision the subject-routed decomposition design named.
fn subject_of(path: &Path, tree: &Path) -> String {
    let Ok(relative) = path.strip_prefix(tree) else {
        return path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
    };
    let parts: Vec<_> = relative.components().collect();
    if parts.len() > 1 {
        parts[0].as_os_str().to_string_lossy().to_string()
    } else {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

pub fn focus_baseline(
    root: &Path,
    cluster_state_override: Option<&Path>,
) -> FlowResult<FocusBaseline> {
    let sub = substrate(root, cluster_state_override);
    // `known` here is the DECLARED set: only a resource that claims an availability can narrow a
    // subject. This is focus-baseline's reading, and it is narrower than the mover's vocabulary.
    let declared = sub.state.declared_names();
    let live = root.join("genesis/a2o/features");
    let held = root.join("genesis/a2o/held/features");
    let mut subjects: BTreeMap<String, FocusSubject> = BTreeMap::new();

    for path in zone_files(&live)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "feature"))
    {
        let Some(text) = read_lossy(&path) else {
            continue;
        };
        let entry = subjects.entry(subject_of(&path, &live)).or_default();
        let held_scn: Vec<String> = scenario_caps(&text)
            .into_iter()
            .filter(|c| declared.contains(c) && !sub.available.contains(c))
            .collect();
        if held_scn.is_empty() {
            entry.in_focus.push(rel(root, &path));
        } else {
            let caps: BTreeSet<String> = held_scn.iter().cloned().collect();
            entry.mixed.push(MixedFeature {
                path: rel(root, &path),
                caps: caps.into_iter().collect(),
                scenarios: held_scn.len(),
            });
        }
    }
    for path in zone_files(&held)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "feature"))
    {
        let Some(text) = read_lossy(&path) else {
            continue;
        };
        let entry = subjects.entry(subject_of(&path, &held)).or_default();
        let caps: BTreeSet<String> = feature_caps(&text)
            .into_iter()
            .filter(|c| declared.contains(c) && !sub.available.contains(c))
            .collect();
        let caps: Vec<String> = if caps.is_empty() {
            vec!["(an unavailable cap)".to_string()]
        } else {
            caps.into_iter().collect()
        };
        entry.held.push(HeldFeature {
            path: path
                .strip_prefix(&held)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/"),
            caps,
        });
    }

    Ok(FocusBaseline {
        available: sub.available.iter().cloned().collect(),
        unavailable: sub
            .state
            .available_map()
            .into_iter()
            .filter(|(_, v)| v != "true")
            .collect(),
        roles: sub.state.roles(),
        subjects,
    })
}

impl FocusBaseline {
    fn why(&self, caps: &BTreeSet<String>) -> String {
        caps.iter()
            .map(|c| match self.roles.get(c) {
                Some(role) if !role.is_empty() => format!("{c} unavailable — {role}"),
                _ => format!("{c} unavailable"),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The rendered baseline. `only_subject` narrows to one subject; `None` renders the whole thing.
    pub fn render(&self, only_subject: Option<&str>) -> String {
        self.render_with(only_subject, false)
    }

    /// `--brief`: the narrowed subject NAMES and a drill-in pointer, without the per-subject
    /// sections. The form `prep-brainstorm.py` embeds, where the full baseline would bury the
    /// brainstorm it is preparing.
    pub fn render_brief(&self) -> String {
        self.render_with(None, true)
    }

    fn render_with(&self, only_subject: Option<&str>, brief: bool) -> String {
        let mut out: Vec<String> = Vec::new();
        let (mut fair, mut narrowed): (Vec<&String>, Vec<&String>) = (Vec::new(), Vec::new());
        for (name, s) in &self.subjects {
            if s.held.is_empty() && s.mixed.is_empty() {
                fair.push(name);
            } else {
                narrowed.push(name);
            }
        }

        out.push("# SUBJECT FOCUS BASELINE   (generated — sibling of subject-routing.yaml; refreshed on every scope flip)".into());
        out.push("#".into());
        out.push("# No narrowing in a subject ⇒ it is FAIR-GAME: work it freely, like ranging over plans/specs out of".into());
        out.push("# the box. A capability going down narrows a subject — its focus, why, and options are shown below.".into());
        out.push("# Source of truth: genesis/manifests/cluster-state.yaml + the a2o held/ tree (via epr flow report scope).".into());
        out.push(String::new());
        let avail = if self.available.is_empty() {
            "(none)".to_string()
        } else {
            self.available.join(", ")
        };
        let unavail = if self.unavailable.is_empty() {
            "(none — full focus)".to_string()
        } else {
            self.unavailable
                .iter()
                .map(|(k, v)| format!("{k}({v})"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push(format!(
            "substrate: available [{avail}]   unavailable [{unavail}]"
        ));
        out.push(String::new());

        if narrowed.is_empty() {
            out.push("✅ FULL FOCUS — every subject is fair-game (no capability narrowing active). Work anything;".into());
            out.push("   pick by vision×readiness as usual. (`epr flow hold --scope --set <cap>=off --apply` narrows the".into());
            out.push("   plate when you want a focused slice.)".into());
            return format!("{}\n", out.join("\n"));
        }

        if only_subject.is_none() {
            out.push(format!(
                "FAIR-GAME subjects ({}) — no narrowing, everything in focus:",
                fair.len()
            ));
            let joined = fair
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" · ");
            out.push(format!(
                "  {}",
                if joined.is_empty() { "(none)" } else { &joined }
            ));
            out.push(String::new());
            out.push(format!(
                "NARROWED subjects ({}) — a capability is down; focus + options per subject:",
                narrowed.len()
            ));
            if brief {
                narrowed.sort();
                let names = narrowed
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ");
                out.push(format!(
                    "  {}",
                    if names.is_empty() { "(none)" } else { &names }
                ));
                out.push("  → drill in: epr flow report placement --focus <name>".to_string());
                return format!("{}\n", out.join("\n"));
            }
            out.push(String::new());
        }

        let owned: Vec<String>;
        let targets: Vec<&String> = match only_subject {
            Some(subject) => {
                owned = vec![subject.to_string()];
                owned.iter().collect()
            }
            None => {
                narrowed.sort();
                narrowed.clone()
            }
        };
        for name in targets {
            let Some(s) = self.subjects.get(name) else {
                out.push(format!(
                    "### {name} — unknown subject (fair-game if not listed as narrowed)"
                ));
                continue;
            };
            if s.held.is_empty() && s.mixed.is_empty() {
                out.push(format!(
                    "### {name}   ✅ fair-game ({} in focus, nothing held)",
                    s.in_focus.len()
                ));
                out.push(String::new());
                continue;
            }
            let mut held_caps: BTreeSet<String> = BTreeSet::new();
            for h in &s.held {
                held_caps.extend(h.caps.iter().filter(|c| self.declared(c)).cloned());
            }
            for m in &s.mixed {
                held_caps.extend(m.caps.iter().cloned());
            }
            let cap_list = held_caps
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!(
                "### {name}   ⚠ narrowed — {} down",
                if cap_list.is_empty() {
                    "env"
                } else {
                    &cap_list
                }
            ));
            out.push(format!(
                "  IN FOCUS  : {} live feature(s), fully testable on available compute",
                s.in_focus.len()
            ));
            for f in s.in_focus.iter().take(6) {
                out.push(format!("              · {f}"));
            }
            if s.in_focus.len() > 6 {
                out.push(format!("              · … +{} more", s.in_focus.len() - 6));
            }
            for m in &s.mixed {
                let base = Path::new(&m.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| m.path.clone());
                out.push(format!(
                    "  MIXED     : {base} — {} scenario(s) need {} (runtime-skipped, NOT failed)",
                    m.scenarios,
                    m.caps.join(",")
                ));
            }
            for h in &s.held {
                let real: Vec<&str> = h
                    .caps
                    .iter()
                    .filter(|c| self.declared(c))
                    .map(String::as_str)
                    .collect();
                out.push(format!(
                    "  HELD      : {} — needs {} · returns when {} available",
                    h.path,
                    h.caps.join(","),
                    if real.is_empty() {
                        "the cap".to_string()
                    } else {
                        real.join(",")
                    }
                ));
            }
            out.push(format!("  WHY       : {}", self.why(&held_caps)));
            out.push(format!(
                "  OPTIONS   : (a) work the in-focus + any household scenarios now  \
                 (b) expand the plate: epr flow hold --scope --set {}=on --apply  \
                 (c) pivot to a fair-game subject",
                held_caps
                    .iter()
                    .next()
                    .map(String::as_str)
                    .unwrap_or("<cap>")
            ));
            out.push(format!(
                "  BASELINE/PIVOT: /shift or /brainstorm scoped to {name}'s in-focus slice; to pivot, pick a FAIR-GAME subject above"
            ));
            out.push(String::new());
        }
        format!("{}\n", out.join("\n"))
    }

    /// Whether a capability is one cluster-state actually declares (as opposed to the
    /// `(an unavailable cap)` placeholder a held feature with no tracked tag carries).
    fn declared(&self, cap: &str) -> bool {
        self.roles.contains_key(cap)
            || self.unavailable.contains_key(cap)
            || self.available.iter().any(|a| a == cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn feature_caps_read_only_above_the_feature_keyword() {
        let text =
            "@requires:local-conductor\n# @requires:ignored\nFeature: x\n  @requires:below\n";
        assert_eq!(feature_caps(text), vec!["local-conductor"]);
        assert_eq!(scenario_caps(text), vec!["below"]);
    }

    #[test]
    fn a_doc_whose_every_station_is_blocked_is_held_whole() {
        let text = "---\nrequires_env: [shem]\n---\n\n- [ ] one\n- [ ] two\n";
        let (verdict, caps) = scope_verdict(
            Path::new("plans/x.md"),
            text,
            &set(&["household-nodes"]),
            &set(&["shem", "household-nodes"]),
        );
        assert_eq!(verdict, Verdict::Held);
        assert_eq!(caps, vec!["shem".to_string()]);
    }

    #[test]
    fn a_mixed_plan_stays_on_the_plate() {
        // No doc-level default; only one station names the absent capability.
        let text = "---\ntitle: x\n---\n\n- [ ] household work\n- [ ] cross-node @requires:shem\n";
        let (verdict, caps) = scope_verdict(
            Path::new("plans/x.md"),
            text,
            &set(&["household-nodes"]),
            &set(&["shem", "household-nodes"]),
        );
        assert_eq!(verdict, Verdict::Live);
        assert_eq!(caps, vec!["shem".to_string()]);
    }

    #[test]
    fn a_doc_with_no_scope_information_is_ambiguous_never_published() {
        let (verdict, caps) = scope_verdict(
            Path::new("plans/x.md"),
            "# just prose\n",
            &set(&["household-nodes"]),
            &set(&["household-nodes"]),
        );
        assert_eq!(verdict, Verdict::Ambiguous);
        assert!(caps.is_empty());
    }

    #[test]
    fn an_untracked_capability_does_not_gate() {
        // `doorway` is an a2o fixture precondition, not a cluster resource.
        let text = "@requires:doorway\nFeature: x\n";
        let (verdict, _) = scope_verdict(
            Path::new("features/x.feature"),
            text,
            &set(&["household-nodes"]),
            &set(&["household-nodes"]),
        );
        assert_eq!(verdict, Verdict::Ambiguous);
    }

    #[test]
    fn the_headline_forms_are_the_kits() {
        let mut report = ScopeReport {
            command: "flow report scope".into(),
            cluster_state: "genesis/manifests/cluster-state.yaml".into(),
            updated: "2026-06-04".into(),
            available: vec![],
            unavailable: BTreeMap::new(),
            to_held: vec![],
            to_live: vec![],
            vocab: vec![],
            held_anomalies: vec![],
            deployments: vec![],
            pending_moves: 0,
        };
        assert_eq!(
            report.headline(),
            "scope: aligned ✅  (plate matches substrate)"
        );
        report.to_held = vec![ScopeMove {
            src: "a".into(),
            dst: "b".into(),
            caps: vec!["local-conductor".into(), "owned-substrate".into()],
            conflict: false,
        }];
        report.pending_moves = 1;
        assert_eq!(
            report.headline_parts(),
            "scope: ⚠ 1 to hold (local-conductor,owned-substrate)"
        );
        assert!(report
            .headline()
            .contains("→  epr flow hold --scope --apply"));
        report.to_live = vec![ScopeMove {
            src: "c".into(),
            dst: "d".into(),
            caps: vec![],
            conflict: false,
        }];
        report.pending_moves = 2;
        // to_live is named FIRST, matching the kit's part order.
        assert!(report
            .headline_parts()
            .starts_with("scope: ⚠ 1 to return to plate · 1 to hold"));
    }

    #[test]
    fn unknown_caps_render_the_vocab_drift_note() {
        let report = ScopeReport {
            command: "flow report scope".into(),
            cluster_state: String::new(),
            updated: String::new(),
            available: vec![],
            unavailable: BTreeMap::new(),
            to_held: vec![],
            to_live: vec![],
            vocab: vec![VocabWarning {
                path: "specs/x.md".into(),
                caps: vec!["harbor".into()],
            }],
            held_anomalies: vec![],
            deployments: vec![],
            pending_moves: 0,
        };
        assert_eq!(
            report.headline(),
            "scope: aligned ✅  (plate matches substrate)  ⚠ unknown-cap: harbor (vocab drift vs cluster-state)"
        );
    }

    #[test]
    fn state_words_cover_the_kits_vocabulary() {
        assert_eq!(state_word("on"), Some("true"));
        assert_eq!(state_word("OFF"), Some("false"));
        assert_eq!(state_word("degraded"), Some("degraded"));
        assert_eq!(state_word("maybe"), None);
    }

    #[test]
    fn the_subject_is_the_first_segment_under_features() {
        let tree = Path::new("/r/genesis/a2o/features");
        assert_eq!(
            subject_of(Path::new("/r/genesis/a2o/features/lms/a.feature"), tree),
            "lms"
        );
        assert_eq!(
            subject_of(Path::new("/r/genesis/a2o/features/top.feature"), tree),
            "top"
        );
    }
}
