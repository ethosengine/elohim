//! The S5 policy file: loader, CID, and the pin check that gates live operation.
//!
//! Spec: dialogue-stasis-loop-design, Policy boundary and learning, Budgets, Native CLI contract.
//! Loading a policy grants no authority: live operation needs the file's raw-byte CID to equal
//! the CID named in the steward's latest `--kind ruling` for it. This module is pure (bytes in,
//! assessment out); the CLI reads the file and the ruling and hands both here.

use std::path::Path;

use elohim_epr_rea::Bound;
use eprfs_core::BlobCid;
use serde::Deserialize;

use super::journal::StasisError;
use super::{Budgets, StasisPolicy, POLICY_ABSENT};

/// Where the committed policy lives, relative to the checkout root.
pub const POLICY_REL_PATH: &str = ".epr-meta/elohim/policies/dialogue-stasis-policy.json";

/// The `purpose` an away ruling's reason carries: `{"purpose":"dialogue-stasis-away",
/// "expiresAt":"<RFC 3339>"}`. A ruling without both is not an away ruling.
pub const AWAY_PURPOSE: &str = "dialogue-stasis-away";

/// The target kinds a learned form's effect may name (the classifier's `Target`, kebab-case).
const TARGET_KINDS: [&str; 5] = ["remote", "worktree", "main-checkout", "other", "unresolved"];

/// The `purpose` the steward's ruling reason carries (`genesis/agentic/bin/stasis-policy-pin`).
pub const POLICY_PURPOSE: &str = "dialogue-stasis-policy";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundFile {
    limit: f64,
    unit: String,
    threshold_pct: f64,
}

impl BoundFile {
    fn into_bound(self, name: &str) -> Result<Bound, String> {
        if self.unit != "seconds" {
            return Err(format!("{name}.unit must be seconds"));
        }
        Bound::new(self.limit, self.unit, self.threshold_pct).map_err(|e| format!("{name}: {e}"))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeverEntry {
    id: String,
    invariant: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuleSetFile {
    shape_patterns: Vec<String>,
    risk_rules: Vec<String>,
    precedence: Vec<String>,
    uncertain_as: String,
    non_authorization_concern: String,
    effect_normalization: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PromotionFile {
    clean_procedural_reshape_resolutions: u32,
    same_effect_class: bool,
    trigger: String,
    activation_requires: Vec<String>,
    standing_ruling: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RulingValidationFile {
    kind: String,
    authority: String,
    unexpired: bool,
    scope_covers_case: bool,
    policy_pin: String,
    missing_or_mismatched_policy: String,
}

/// A learned legible form: the same effect spelled in a way that passes the same permission
/// checks. It takes effect only when `status` is `active` with an approved audit and a ruling.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearnedForm {
    /// Short name of the form.
    pub name: String,
    /// The tool it applies to.
    pub tool: String,
    /// The redacted shape it replaces.
    pub shape: String,
    /// The normalized effect tuple, as `classify` serializes it.
    pub effect_class: serde_json::Value,
    /// The more legible command shape to offer.
    pub legible_form: String,
    /// `proposed` or `active`.
    pub status: String,
    /// The other-family audit verdict.
    #[serde(default)]
    pub audit_verdict: Option<String>,
    /// The audit's reference.
    #[serde(default)]
    pub audit_ref: Option<String>,
    /// The steward's standing ruling reference.
    #[serde(default)]
    pub ruling_ref: Option<String>,
}

const EFFECT_KEYS: [&str; 5] = ["tool", "command", "subcommand", "target", "pathClass"];

impl LearnedForm {
    /// Check the entry against the policy schema: closed status, an object effect class naming a
    /// tool with only known keys, and an approved audit, an audit ref and a ruling ref when active.
    fn validate(&self) -> Result<(), String> {
        let named = |v: &str| !v.trim().is_empty();
        if !(named(&self.name)
            && named(&self.tool)
            && named(&self.shape)
            && named(&self.legible_form))
        {
            return Err(format!("learned form `{}` has an empty field", self.name));
        }
        let effect = self
            .effect_class
            .as_object()
            .filter(|o| {
                o.get("tool")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(named)
            })
            .filter(|o| o.keys().all(|k| EFFECT_KEYS.contains(&k.as_str())))
            .ok_or_else(|| {
                format!(
                    "learned form `{}`: effectClass is not a well-formed effect",
                    self.name
                )
            })?;
        // Typed values, not just known keys: `command: 42` or `target: "bogus"` is invalid.
        let text_or_null = |k: &str| match effect.get(k) {
            None | Some(serde_json::Value::Null) => true,
            Some(serde_json::Value::String(t)) => named(t),
            Some(_) => false,
        };
        let target_ok = match effect.get("target") {
            None | Some(serde_json::Value::Null) => true,
            Some(serde_json::Value::String(t)) => TARGET_KINDS.contains(&t.as_str()),
            Some(_) => false,
        };
        if !(text_or_null("command")
            && text_or_null("subcommand")
            && text_or_null("pathClass")
            && target_ok)
        {
            return Err(format!(
                "learned form `{}`: effectClass has a mistyped command, subcommand, target or pathClass",
                self.name
            ));
        }
        if effect["tool"] != self.tool.as_str() {
            return Err(format!(
                "learned form `{}`: effectClass.tool differs from tool",
                self.name
            ));
        }
        if !matches!(self.status.as_str(), "proposed" | "active") {
            return Err(format!(
                "learned form `{}`: status must be proposed or active",
                self.name
            ));
        }
        if !matches!(
            self.audit_verdict.as_deref(),
            None | Some("approved" | "rejected")
        ) {
            return Err(format!(
                "learned form `{}`: unknown auditVerdict",
                self.name
            ));
        }
        if self.status == "active" && !self.is_active() {
            return Err(format!(
                "learned form `{}` is active without an approved audit, auditRef and rulingRef",
                self.name
            ));
        }
        Ok(())
    }

    /// Whether this form may be offered: active, audited approved, and ruled.
    pub fn is_active(&self) -> bool {
        let named = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.trim().is_empty());
        self.status == "active"
            && self.audit_verdict.as_deref() == Some("approved")
            && named(&self.audit_ref)
            && named(&self.ruling_ref)
            && !self.name.trim().is_empty()
            && !self.legible_form.trim().is_empty()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyFile {
    #[serde(default)]
    learned_forms: Vec<LearnedForm>,
    #[serde(default, rename = "$schema")]
    _schema: Option<String>,
    version: u32,
    never: Vec<NeverEntry>,
    budgets: Budgets,
    collaborative_window_s: f64,
    stall: BoundFile,
    shift_ceiling: BoundFile,
    dwell_s: u64,
    rule_set: RuleSetFile,
    promotion_criteria: PromotionFile,
    ruling_validation: RulingValidationFile,
}

fn words(v: &[String], want: &[&str]) -> bool {
    v.len() == want.len() && v.iter().zip(want).all(|(a, b)| a == b)
}

impl PolicyFile {
    /// Check the constants the schema pins, then lower to the policy the fold consumes.
    fn lower(self) -> Result<StasisPolicy, String> {
        if self.version < 1 {
            return Err("version must be >= 1".into());
        }
        let mut ids: Vec<&str> = self.never.iter().map(|n| n.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        if self.never.len() != 7
            || ids.len() != 7
            || self
                .never
                .iter()
                .any(|n| n.invariant.trim().is_empty() || !n.id.starts_with("never-"))
        {
            return Err("never must hold the seven distinct invariants".into());
        }
        let r = &self.rule_set;
        let shapes = ["cd-compound", "heredoc", "bash-c", "multi-line"];
        let mut seen = r.shape_patterns.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != r.shape_patterns.len() {
            return Err("ruleSet.shapePatterns must be unique".into());
        }
        if r.shape_patterns
            .iter()
            .any(|s| !shapes.contains(&s.as_str()))
            || !words(&r.risk_rules, &["deny", "ask"])
            || !words(&r.precedence, &["deny", "ask", "allow"])
            || r.uncertain_as != "risk"
            || r.non_authorization_concern != "uncertain"
            || r.effect_normalization != "native-session-stasis-classifier-v1"
        {
            return Err("ruleSet departs from the classifier's fixed rule set".into());
        }
        let p = &self.promotion_criteria;
        if p.clean_procedural_reshape_resolutions < 2
            || !p.same_effect_class
            || p.trigger != "propose"
            || !words(
                &p.activation_requires,
                &[
                    "completed-passing-other-family-audit",
                    "steward-standing-ruling",
                ],
            )
            || p.standing_ruling.trim().is_empty()
        {
            return Err("promotionCriteria departs from the spec".into());
        }
        let v = &self.ruling_validation;
        if v.kind != "ruling"
            || v.authority != "steward"
            || !v.unexpired
            || !v.scope_covers_case
            || v.policy_pin != "latest-steward-ruling-exact-cid"
            || v.missing_or_mismatched_policy != "inert-live-exit-3"
        {
            return Err("rulingValidation departs from the spec".into());
        }
        for form in &self.learned_forms {
            form.validate()?;
        }
        let b = &self.budgets;
        if b.incident_keeper_calls < 1
            || b.incident_tokens < 1
            || b.shift_tokens < 1
            || b.incident_elapsed_s <= 0.0
            || b.shift_keeper_s <= 0.0
            || self.collaborative_window_s <= 0.0
        {
            return Err("budgets and the collaborative window must be positive".into());
        }
        Ok(StasisPolicy {
            budgets: self.budgets,
            collaborative_window_s: self.collaborative_window_s,
            stall: self.stall.into_bound("stall")?,
            shift_ceiling: self.shift_ceiling.into_bound("shiftCeiling")?,
            dwell_s: self.dwell_s,
            learned_forms: self.learned_forms,
        })
    }
}

/// Parse policy bytes into a [`StasisPolicy`]; unknown fields and departures from the schema's
/// constants are errors.
pub fn parse_policy(bytes: &[u8]) -> Result<StasisPolicy, String> {
    serde_json::from_slice::<PolicyFile>(bytes)
        .map_err(|e| e.to_string())?
        .lower()
}

/// The policy file's raw-byte CID (`bafkrei...`), the same address `eprfs cid <file>` prints.
pub fn policy_cid(bytes: &[u8]) -> String {
    BlobCid::compute_raw(bytes).to_string()
}

/// A policy file as found on disk.
#[derive(Debug, Clone)]
pub struct PolicyFileRead {
    /// Raw-byte CID of exactly the bytes read.
    pub cid: String,
    /// The parsed policy, or why it did not parse.
    pub parsed: Result<StasisPolicy, String>,
}

/// Read the policy under `root`; `None` when the file does not exist.
pub fn read_policy_file(root: &Path) -> Result<Option<PolicyFileRead>, StasisError> {
    let path = root.join(POLICY_REL_PATH);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(PolicyFileRead {
            cid: policy_cid(&bytes),
            parsed: parse_policy(&bytes),
        })),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StasisError::Io(format!("{}: {e}", path.display()))),
    }
}

/// What the steward's latest policy ruling said, as the caller could read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// No qualifying ruling names this policy.
    None,
    /// The latest qualifying ruling names this CID.
    Cid(String),
    /// The flow sidecar could not be read.
    Unreadable(String),
}

/// The policy's standing relative to the steward's ruling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyState {
    /// No policy file.
    Absent,
    /// A file that does not parse; never live, whatever a ruling says.
    Invalid {
        /// CID of the bytes.
        cid: String,
        /// Why parsing failed.
        reason: String,
    },
    /// A valid file with no ruling naming it.
    Unpinned {
        /// CID of the bytes.
        cid: String,
    },
    /// The latest ruling names this exact CID.
    PinnedMatch {
        /// CID of the bytes.
        cid: String,
    },
    /// The latest ruling names a different CID.
    PinnedMismatch {
        /// CID of the bytes.
        cid: String,
        /// CID the ruling names.
        pinned: String,
    },
    /// A valid file whose ruling could not be read.
    PinUnreadable {
        /// CID of the bytes.
        cid: String,
        /// Why the sidecar failed.
        reason: String,
    },
}

/// The state and the policy the run uses (the file's when valid, else the built-in defaults).
#[derive(Debug, Clone)]
pub struct PolicyAssessment {
    /// Standing against the ruling.
    pub state: PolicyState,
    /// The policy to fold with.
    pub policy: StasisPolicy,
}

impl PolicyAssessment {
    /// Combine what was read from disk with what the sidecar says.
    pub fn assess(file: Option<PolicyFileRead>, pin: Pin) -> Self {
        let Some(file) = file else {
            return Self {
                state: PolicyState::Absent,
                policy: StasisPolicy::builtin(),
            };
        };
        let cid = file.cid;
        match file.parsed {
            Err(reason) => Self {
                state: PolicyState::Invalid { cid, reason },
                policy: StasisPolicy::builtin(),
            },
            Ok(policy) => {
                let state = match pin {
                    Pin::None => PolicyState::Unpinned { cid },
                    Pin::Cid(p) if p == cid => PolicyState::PinnedMatch { cid },
                    Pin::Cid(pinned) => PolicyState::PinnedMismatch { cid, pinned },
                    Pin::Unreadable(reason) => PolicyState::PinUnreadable { cid, reason },
                };
                Self { state, policy }
            }
        }
    }

    /// The state as `status` reports it.
    pub fn state_word(&self) -> &'static str {
        match self.state {
            PolicyState::Absent => "absent",
            PolicyState::Invalid { .. } => "invalid",
            PolicyState::Unpinned { .. } => "unpinned",
            PolicyState::PinnedMatch { .. } => "pinned-match",
            PolicyState::PinnedMismatch { .. } => "pinned-mismatch",
            PolicyState::PinUnreadable { .. } => "pin-unreadable",
        }
    }

    /// The CID of the file on disk, when there is one.
    pub fn cid(&self) -> Option<&str> {
        match &self.state {
            PolicyState::Absent => None,
            PolicyState::Invalid { cid, .. }
            | PolicyState::Unpinned { cid }
            | PolicyState::PinnedMatch { cid }
            | PolicyState::PinnedMismatch { cid, .. }
            | PolicyState::PinUnreadable { cid, .. } => Some(cid),
        }
    }

    /// The `policy` string `decide`, `fold` and `status` print.
    pub fn label(&self, live: bool) -> String {
        let mode = if live { "live" } else { "shadow" };
        match &self.state {
            PolicyState::Absent if !live => POLICY_ABSENT.to_owned(),
            PolicyState::Absent => "absent".to_owned(),
            PolicyState::Invalid { cid, .. } => format!("file {cid} (invalid, defaults), {mode}"),
            PolicyState::PinnedMatch { cid } => format!("file {cid} (pinned), {mode}"),
            // A mismatch or unreadable ruling is not a pin: the bytes in hand are unpinned.
            PolicyState::Unpinned { cid }
            | PolicyState::PinnedMismatch { cid, .. }
            | PolicyState::PinUnreadable { cid, .. } => format!("file {cid} (unpinned), {mode}"),
        }
    }

    /// Why live operation must stay inert, or `None` when the pin matches.
    pub fn inert_reason(&self) -> Option<String> {
        match &self.state {
            PolicyState::PinnedMatch { .. } => None,
            PolicyState::Absent => Some(format!(
                "policy file {POLICY_REL_PATH} is absent; live regulator stays inert"
            )),
            PolicyState::Invalid { cid, reason } => Some(format!(
                "policy file {cid} does not parse ({reason}); live regulator stays inert"
            )),
            PolicyState::Unpinned { cid } => Some(format!(
                "no steward ruling pins policy {cid}; live regulator stays inert"
            )),
            PolicyState::PinnedMismatch { cid, pinned } => Some(format!(
                "policy CID {cid} differs from {pinned} named in the steward's latest ruling; \
                 live regulator stays inert"
            )),
            PolicyState::PinUnreadable { cid, reason } => Some(format!(
                "steward ruling for policy {cid} is unreadable ({reason}); live regulator stays inert"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The committed policy, when the crate is built inside the monorepo (the storage image's
    /// flattened layout has no `.epr-meta` tree).
    fn committed() -> Option<Vec<u8>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        std::fs::read(root.join(POLICY_REL_PATH)).ok()
    }

    #[test]
    fn the_committed_defaults_equal_the_builtin_policy() {
        let Some(bytes) = committed() else { return };
        assert_eq!(parse_policy(&bytes).unwrap(), StasisPolicy::builtin());
    }

    #[test]
    fn the_cid_is_the_raw_byte_address_the_pin_helper_prints() {
        // Fixed vectors derived independently of this crate: CIDv1, raw codec 0x55, sha2-256,
        // base32 lower, i.e. exactly what `eprfs cid <file>` (BlobCid::compute_raw) prints.
        assert_eq!(
            policy_cid(b"{}"),
            "bafkreicecnx2gvntm6fbcrvnc336qze6st5u7qq7457igegamd3bzkx7ri"
        );
        assert_eq!(
            policy_cid(b"dialogue-stasis policy\n"),
            "bafkreiduobmqykm463jbqosnocqopxqvwdmmpv6xggapqqasdktkfrsrsa"
        );
        assert_eq!(policy_cid(b"{}"), BlobCid::compute_raw(b"{}").to_string());
        assert_ne!(policy_cid(b"{}"), policy_cid(b"{} "));
    }

    #[test]
    fn unknown_fields_and_schema_departures_are_refused() {
        let Some(bytes) = committed() else { return };
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let edit = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v = v.clone();
            f(&mut v);
            parse_policy(&serde_json::to_vec(&v).unwrap())
        };
        assert!(edit(&|v| v["extra"] = true.into()).is_err());
        assert!(edit(&|v| v["budgets"]["surprise"] = 1.into()).is_err());
        assert!(edit(&|v| v["stall"]["unit"] = "minutes".into()).is_err());
        assert!(edit(&|v| v["ruleSet"]["uncertainAs"] = "shape".into()).is_err());
        assert!(edit(&|v| v["ruleSet"]["shapePatterns"] = json!(["heredoc", "heredoc"])).is_err());
        assert!(edit(&|v| {
            v["never"].as_array_mut().unwrap().pop();
        })
        .is_err());
        // A changed bound is valid: the pin, not the parser, is what stops it going live.
        let changed = edit(&|v| v["dwellS"] = 60.into()).unwrap();
        assert_eq!(changed.dwell_s, 60);
    }

    fn read(cid: &str) -> Option<PolicyFileRead> {
        Some(PolicyFileRead {
            cid: cid.into(),
            parsed: Ok(StasisPolicy::builtin()),
        })
    }

    #[test]
    fn only_an_exact_pin_match_may_go_live() {
        let a = PolicyAssessment::assess(None, Pin::None);
        assert_eq!(
            (a.state_word(), a.label(false)),
            ("absent", POLICY_ABSENT.into())
        );
        assert!(a.inert_reason().is_some());

        let a = PolicyAssessment::assess(read("bafkreiA"), Pin::None);
        assert_eq!(a.state_word(), "unpinned");
        assert_eq!(a.label(false), "file bafkreiA (unpinned), shadow");
        assert!(a.inert_reason().is_some());

        let a = PolicyAssessment::assess(read("bafkreiA"), Pin::Cid("bafkreiB".into()));
        assert_eq!(a.state_word(), "pinned-mismatch");
        assert!(a.inert_reason().unwrap().contains("bafkreiB"));

        let a = PolicyAssessment::assess(read("bafkreiA"), Pin::Cid("bafkreiA".into()));
        assert_eq!(a.state_word(), "pinned-match");
        assert_eq!(a.label(false), "file bafkreiA (pinned), shadow");
        assert_eq!(a.label(true), "file bafkreiA (pinned), live");
        assert!(a.inert_reason().is_none());

        let bad = Some(PolicyFileRead {
            cid: "bafkreiA".into(),
            parsed: Err("nope".into()),
        });
        // A ruling cannot pin bytes that do not parse.
        let a = PolicyAssessment::assess(bad, Pin::Cid("bafkreiA".into()));
        assert_eq!(a.state_word(), "invalid");
        assert!(a.inert_reason().is_some());
        assert_eq!(a.policy, StasisPolicy::builtin());
    }

    #[test]
    fn learned_forms_are_validated_strictly() {
        let Some(bytes) = committed() else { return };
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let form = json!({"name": "f", "tool": "Bash", "shape": "ls\nls",
            "effectClass": {"tool": "Bash", "command": "ls"}, "legibleForm": "ls <path>",
            "status": "active", "auditVerdict": "approved", "auditRef": "a", "rulingRef": "r"});
        let with = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut entry = form.clone();
            f(&mut entry);
            let mut p = v.clone();
            p["learnedForms"] = json!([entry]);
            parse_policy(&serde_json::to_vec(&p).unwrap())
        };
        assert!(with(&|_| {}).is_ok());
        assert!(with(&|f| f["status"] = "maybe".into()).is_err());
        assert!(with(&|f| f["effectClass"] = "Bash".into()).is_err());
        assert!(with(&|f| f["effectClass"] = json!({"command": "ls"})).is_err());
        assert!(with(&|f| f["effectClass"]["junk"] = 1.into()).is_err());
        assert!(with(&|f| f["effectClass"]["command"] = 42.into()).is_err());
        assert!(with(&|f| f["effectClass"]["subcommand"] = "".into()).is_err());
        assert!(with(&|f| f["effectClass"]["target"] = "bogus".into()).is_err());
        assert!(with(&|f| f["effectClass"]["target"] = 7.into()).is_err());
        assert!(with(&|f| f["effectClass"]["pathClass"] = json!({})).is_err());
        assert!(with(&|f| f["effectClass"]["target"] = "main-checkout".into()).is_ok());
        assert!(with(&|f| f["effectClass"]["pathClass"] = serde_json::Value::Null).is_ok());
        assert!(with(&|f| f["effectClass"]["tool"] = "Edit".into()).is_err());
        assert!(with(&|f| f["auditVerdict"] = "rejected".into()).is_err());
        assert!(with(&|f| f["auditVerdict"] = "perhaps".into()).is_err());
        assert!(with(&|f| f["auditRef"] = "".into()).is_err());
        assert!(with(&|f| f["rulingRef"] = serde_json::Value::Null).is_err());
        assert!(with(&|f| f["legibleForm"] = " ".into()).is_err());
        // A proposed form needs no audit yet.
        assert!(with(&|f| {
            f["status"] = "proposed".into();
            f.as_object_mut().unwrap().remove("auditVerdict");
        })
        .is_ok());
    }
}
