//! `epr covenant` — the authority half of the session covenant.
//!
//! Once per agent session the steward of record answers one bundle of asks; each answer is a
//! clause that brit signs and indexes (`brit-build-ref covenant put|list|show`). This module
//! decides what the bundle contains and whether each clause is HONOURED. brit decides neither.
//!
//! A non-zero exit means the covenant could not be read (brit missing, a clause unparseable, the
//! repository unreadable): callers then apply the full rule, so an unreadable covenant never
//! lowers an obligation. Spec: `genesis/docs/superpowers/specs/2026-10-19-session-covenant-epr-design.md`.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

use chrono::{DateTime, FixedOffset, Utc};
use serde_json::{json, Map, Value};

use crate::{actor, device_key, flow::memory, git};

/// Environment override naming the `brit-build-ref` binary.
pub const BRIT_BIN_ENV: &str = "BRIT_BUILD_REF_BIN";

const AFFIRMS: &str = "affirmsStewardOfRecord";
const PURPOSE: &str = "declaresPurpose";
const VARIES: &str = "variesRule";
const REVOKES: &str = "revokesClause";
const KINDS: [&str; 4] = [AFFIRMS, PURPOSE, VARIES, REVOKES];

type CovResult<T> = Result<T, String>;

pub fn usage() -> String {
    "usage: epr covenant <offer|show|check --rule ID --path P|record --kind K --party human:<h> \
     --quote Q [--purpose T --write-set GLOB…] [--rule ID --paths GLOB…] [--revokes CID] \
     [--valid-until RFC3339]> --repo R --session S [--json]"
        .into()
}

// ---------------------------------------------------------------------------
// arguments
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Args {
    repo: Option<PathBuf>,
    session: Option<String>,
    kind: Option<String>,
    party: Option<String>,
    quote: Option<String>,
    purpose: Option<String>,
    write_set: Vec<String>,
    rule: Option<String>,
    paths: Vec<String>,
    revokes: Option<String>,
    valid_until: Option<String>,
    path: Option<String>,
}

fn parse_args(args: &[String]) -> CovResult<Args> {
    let mut out = Args::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        index += 1;
        let multi = matches!(flag, "--write-set" | "--paths");
        if flag == "--json" {
            continue;
        }
        if !flag.starts_with("--") {
            return Err(format!("unexpected argument `{flag}`\n{}", usage()));
        }
        let mut values = Vec::new();
        while index < args.len() && (values.is_empty() || multi) && !args[index].starts_with("--") {
            values.push(args[index].clone());
            index += 1;
        }
        if values.is_empty() {
            return Err(format!("{flag} needs a value"));
        }
        let single = || values[0].clone();
        match flag {
            "--repo" => out.repo = Some(PathBuf::from(single())),
            "--session" => out.session = Some(single()),
            "--kind" => out.kind = Some(single()),
            "--party" => out.party = Some(single()),
            "--quote" => out.quote = Some(single()),
            "--purpose" => out.purpose = Some(single()),
            "--write-set" => out.write_set.extend(values),
            "--rule" => out.rule = Some(single()),
            "--paths" => out.paths.extend(values),
            "--revokes" => out.revokes = Some(single()),
            "--valid-until" => out.valid_until = Some(single()),
            "--path" => out.path = Some(single()),
            other => return Err(format!("unknown argument `{other}`\n{}", usage())),
        }
    }
    Ok(out)
}

/// Entry point invoked by `main` for `epr covenant …`.
pub fn run(args: &[String]) -> CovResult<ExitCode> {
    let verb = args.first().map(String::as_str).ok_or_else(usage)?;
    if !matches!(verb, "offer" | "record" | "check" | "show") {
        return Err(format!("unknown covenant command `{verb}`\n{}", usage()));
    }
    let args = parse_args(&args[1..])?;
    let start = match &args.repo {
        Some(repo) => repo.clone(),
        None => std::env::current_dir()
            .map_err(|e| format!("cannot read the working directory: {e}"))?,
    };
    let session = args
        .session
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| crate::session_envelope::current_id().ok())
        .ok_or("no P1 session is available")?;
    let _envelope = if matches!(verb, "record" | "check") {
        Some(crate::session_envelope::live_guard(Some(&session), &start)?)
    } else {
        None
    };
    let root = match &_envelope {
        Some(guard) => guard.root.clone(),
        None => git::discover_root(&start).map_err(|e| e.to_string())?,
    };
    let (facet_session, _scope) = crate::session_envelope::covenant_scope(&session)?;
    let value = match verb {
        "offer" => offer(&root, &session, &facet_session)?,
        "show" => show(&root, &session, &facet_session)?,
        "check" => check(&root, &facet_session, &args)?,
        _ => record(&root, &session, &facet_session, &args)?,
    };
    println!("{value}");
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// brit
// ---------------------------------------------------------------------------

struct Brit {
    bin: PathBuf,
    repo: PathBuf,
    session: String,
}

impl Brit {
    /// `BRIT_BUILD_REF_BIN` (which must then exist), else `brit-build-ref` on PATH.
    fn resolve(repo: &Path, session: &str) -> CovResult<Self> {
        let bin = match std::env::var_os(BRIT_BIN_ENV).filter(|v| !v.is_empty()) {
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.is_file() {
                    return Err(format!(
                        "brit-build-ref not found: {BRIT_BIN_ENV}={} is not a file",
                        path.display()
                    ));
                }
                path
            }
            None => std::env::var_os("PATH")
                .and_then(|paths| {
                    std::env::split_paths(&paths)
                        .map(|dir| dir.join("brit-build-ref"))
                        .find(|candidate| candidate.is_file())
                })
                .ok_or_else(|| {
                    format!("brit-build-ref not found: set {BRIT_BIN_ENV} or put it on PATH")
                })?,
        };
        Ok(Self {
            bin,
            repo: repo.to_path_buf(),
            session: session.to_owned(),
        })
    }

    fn call(&self, args: &[&str]) -> CovResult<Value> {
        let output = Command::new(&self.bin)
            .arg("--repo")
            .arg(&self.repo)
            .arg("covenant")
            .args(args)
            .output()
            .map_err(|e| format!("cannot run {}: {e}", self.bin.display()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let first = stderr.lines().next().unwrap_or("").trim();
            return Err(format!(
                "brit-build-ref covenant {} failed ({}){}",
                args.first().copied().unwrap_or(""),
                output.status,
                if first.is_empty() {
                    String::new()
                } else {
                    format!(": {first}")
                }
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("brit-build-ref covenant output is not JSON: {e}"))
    }

    fn list(&self) -> CovResult<Vec<Clause>> {
        let value = self.call(&["list", "--session", &self.session, "--output-json"])?;
        let rows = value
            .as_array()
            .ok_or("brit-build-ref covenant list did not return an array")?;
        let mut clauses = rows
            .iter()
            .map(Clause::parse)
            .collect::<CovResult<Vec<_>>>()?;
        clauses.sort_by_key(|c| c.valid_from);
        Ok(clauses)
    }

    fn put(&self, put: &Put) -> CovResult<Value> {
        let payload = put.payload.to_string();
        let mut args = vec![
            "put",
            "--session",
            &self.session,
            "--kind",
            put.kind,
            "--party",
            &put.party,
            "--quote",
            &put.quote,
            "--payload-json",
            &payload,
        ];
        if let Some(until) = &put.valid_until {
            args.extend(["--valid-until", until]);
        }
        args.push("--output-json");
        self.call(&args)
    }
}

struct Put {
    kind: &'static str,
    party: String,
    quote: String,
    payload: Value,
    valid_until: Option<String>,
}

// ---------------------------------------------------------------------------
// clauses
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Clause {
    cid: String,
    kind: String,
    party: String,
    quote: String,
    payload: Value,
    valid_from: DateTime<FixedOffset>,
    valid_until: Option<DateTime<FixedOffset>>,
}

impl Clause {
    fn parse(row: &Value) -> CovResult<Self> {
        let text = |value: &Value, key: &str| -> CovResult<String> {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("a covenant clause carries no `{key}`"))
        };
        let clause = row
            .get("clause")
            .ok_or("a covenant row carries no `clause`")?;
        let time = |key: &str| -> CovResult<DateTime<FixedOffset>> {
            DateTime::parse_from_rfc3339(&text(clause, key)?)
                .map_err(|e| format!("clause `{key}` is not RFC3339: {e}"))
        };
        let valid_until = match clause.get("validUntil") {
            None | Some(Value::Null) => None,
            Some(_) => Some(time("validUntil")?),
        };
        Ok(Self {
            cid: text(row, "cid")?,
            kind: text(clause, "kind")?,
            party: text(clause, "party")?,
            quote: text(clause, "quote")?,
            payload: clause.get("payload").cloned().unwrap_or(Value::Null),
            valid_from: time("validFrom")?,
            valid_until,
        })
    }

    fn field(&self, key: &str) -> Option<&str> {
        self.payload.get(key).and_then(Value::as_str)
    }

    fn strings(&self, key: &str) -> Vec<String> {
        self.payload
            .get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// manifests
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Variance {
    floor: String,
    requires: String,
    offered_when: String,
}

#[derive(Clone)]
struct RuleDecl {
    id: String,
    class: Option<String>,
    when: Value,
    variance: Option<Variance>,
}

struct Manifest {
    /// The manifest file, repository-relative (`genesis/a2o/.epr-meta`).
    rel: String,
    /// The directory it governs, repository-relative; empty for the root.
    dir: String,
    bytes: Vec<u8>,
    rules: Vec<RuleDecl>,
}

fn frontmatter_or_whole(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            return &rest[..end];
        }
    }
    text
}

fn manifest_dir(rel: &str) -> String {
    let subject = match rel.strip_suffix("/.epr-meta/manifest.md") {
        Some(dir) => dir,
        None if rel == ".epr-meta/manifest.md" => "",
        None => rel.strip_suffix(".epr-meta").unwrap_or(rel),
    };
    subject.trim_end_matches('/').to_string()
}

fn yaml_str(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    map.get(serde_yaml::Value::String(key.into()))
        .and_then(serde_yaml::Value::as_str)
        .map(|s| s.trim().to_string())
}

fn parse_rules(text: &str) -> Vec<RuleDecl> {
    let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(frontmatter_or_whole(text)) else {
        return Vec::new();
    };
    let Some(rules) = doc.get("rules").and_then(serde_yaml::Value::as_sequence) else {
        return Vec::new();
    };
    rules
        .iter()
        .filter_map(|rule| {
            let map = rule.as_mapping()?;
            let id = yaml_str(map, "id")?;
            let variance = map
                .get(serde_yaml::Value::String("parameters".into()))
                .and_then(|p| p.get("variance"))
                .and_then(serde_yaml::Value::as_mapping)
                .and_then(|v| {
                    Some(Variance {
                        floor: yaml_str(v, "floor").filter(|f| !f.is_empty())?,
                        requires: yaml_str(v, "requires")
                            .unwrap_or_else(|| "steward-of-record".into()),
                        offered_when: yaml_str(v, "offered-when").unwrap_or_default(),
                    })
                });
            let when = map
                .get(serde_yaml::Value::String("when".into()))
                .and_then(|w| serde_json::to_value(w).ok())
                .unwrap_or(Value::Null);
            Some(RuleDecl {
                id,
                class: yaml_str(map, "class"),
                when,
                variance,
            })
        })
        .collect()
}

fn load_manifest(root: &Path, rel: &str) -> Option<Manifest> {
    let bytes = std::fs::read(root.join(rel)).ok()?;
    let rules = parse_rules(&String::from_utf8_lossy(&bytes));
    Some(Manifest {
        rel: rel.to_string(),
        dir: manifest_dir(rel),
        bytes,
        rules,
    })
}

/// Every `.epr-meta` manifest of the repository whose bytes contain `needle`, tracked or not.
fn manifests_containing(root: &Path, needle: &str) -> CovResult<Vec<Manifest>> {
    let output = crate::git::command(root)
        .current_dir(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            ":(glob)**/.epr-meta",
            ":(glob)**/.epr-meta/manifest.md",
        ])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !output.status.success() {
        return Err("git ls-files could not list the repository's .epr-meta manifests".into());
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let mut found = BTreeSet::new();
    for rel in listing.split('\0').filter(|r| !r.is_empty()) {
        let is_manifest = rel == ".epr-meta"
            || rel.ends_with("/.epr-meta")
            || rel == ".epr-meta/manifest.md"
            || rel.ends_with("/.epr-meta/manifest.md");
        if is_manifest && root.join(rel).is_file() {
            found.insert(rel.to_string());
        }
    }
    Ok(found
        .iter()
        .filter_map(|rel| {
            let bytes = std::fs::read(root.join(rel)).ok()?;
            String::from_utf8_lossy(&bytes)
                .contains(needle)
                .then(|| load_manifest(root, rel))?
        })
        .collect())
}

/// `dir` is `path` itself or one of its ancestors (both repository-relative).
fn governs(dir: &str, path: &str) -> bool {
    dir.is_empty() || path == dir || path.starts_with(&format!("{dir}/"))
}

fn manifest_cid(bytes: &[u8]) -> String {
    elohim_epr::cid::compute_raw_cid(bytes).to_string()
}

// ---------------------------------------------------------------------------
// standing
// ---------------------------------------------------------------------------

/// The human this session speaks for here: the session's human claim, else the device's
/// roster-bound standing (the same read `epr actor current` makes).
fn standing_subject(root: &Path, session: &str) -> CovResult<Option<String>> {
    let key = device_key::resolve_path().ok();
    let current = actor::current_on_device_scoped(root, session, key.as_deref())
        .map_err(|e| format!("cannot read standing: {e}"))?;
    if let Some(claim) = &current.claim {
        if claim.claimed.starts_with("human:") {
            return Ok(Some(claim.claimed.clone()));
        }
    }
    if let Some(standing) = current.standing {
        return Ok(Some(standing.subject));
    }
    // A session an agent claimed still sits on a device that may stand for a human.
    Ok(key
        .and_then(|k| actor::standing_on_device(root, &k))
        .map(|s| s.subject))
}

// ---------------------------------------------------------------------------
// honouring
// ---------------------------------------------------------------------------

struct Verdict {
    honoured: bool,
    reason: String,
}

impl Verdict {
    fn yes(reason: impl Into<String>) -> Self {
        Self {
            honoured: true,
            reason: reason.into(),
        }
    }
    fn no(reason: impl Into<String>) -> Self {
        Self {
            honoured: false,
            reason: reason.into(),
        }
    }
}

struct Covenant<'a> {
    root: &'a Path,
    now: DateTime<FixedOffset>,
    standing: Option<String>,
    clauses: Vec<Clause>,
}

impl<'a> Covenant<'a> {
    fn read(root: &'a Path, brit: &Brit) -> CovResult<Self> {
        Ok(Self {
            root,
            now: Utc::now().fixed_offset(),
            standing: standing_subject(root, &brit.session)?,
            clauses: brit.list()?,
        })
    }

    fn position(&self, cid: &str) -> Option<usize> {
        self.clauses.iter().position(|c| c.cid == cid)
    }

    /// The cid of a `revokesClause` by the same party naming `index`'s clause. Every clause is
    /// scanned, not only later ones: a revocation names its target's CID, so it cannot precede it,
    /// and timestamp order (ties, clock skew) must never let a revocation be missed.
    fn revoked_by(&self, index: usize) -> Option<&str> {
        let target = &self.clauses[index];
        self.clauses
            .iter()
            .find(|c| {
                c.kind == REVOKES
                    && c.party == target.party
                    && c.field("revokes") == Some(&target.cid)
            })
            .map(|c| c.cid.as_str())
    }

    fn window(&self, clause: &Clause) -> Option<String> {
        if self.now < clause.valid_from {
            return Some(format!(
                "not valid until {}",
                clause.valid_from.to_rfc3339()
            ));
        }
        match clause.valid_until {
            Some(until) if self.now > until => Some(format!("expired at {}", until.to_rfc3339())),
            _ => None,
        }
    }

    /// Does the session hold an honoured steward affirmation by `party`?
    fn affirmed(&self, party: &str) -> CovResult<Verdict> {
        let mut last = Verdict::no(format!(
            "the session holds no steward-of-record affirmation by {party}"
        ));
        for (index, clause) in self.clauses.iter().enumerate() {
            if clause.kind == AFFIRMS && clause.party == party {
                let verdict = self.verdict_at(index, false)?;
                if verdict.honoured {
                    return Ok(verdict);
                }
                last = verdict;
            }
        }
        Ok(last)
    }

    fn verdict_at(&self, index: usize, deep: bool) -> CovResult<Verdict> {
        let clause = &self.clauses[index];
        if !KINDS.contains(&clause.kind.as_str()) {
            return Ok(Verdict::no(format!(
                "unknown clause kind `{}`",
                clause.kind
            )));
        }
        if clause.kind == REVOKES {
            let target = clause.field("revokes").and_then(|cid| self.position(cid));
            return Ok(match target {
                Some(t) if self.clauses[t].party == clause.party => {
                    Verdict::yes(format!("revokes {}", self.clauses[t].cid))
                }
                Some(_) => Verdict::no("revokes another party's clause"),
                None => Verdict::no("revokes a clause that is not in this session"),
            });
        }
        if let Some(reason) = self.window(clause) {
            return Ok(Verdict::no(reason));
        }
        if let Some(by) = self.revoked_by(index) {
            return Ok(Verdict::no(format!("revoked by clause {by}")));
        }
        if clause.kind == AFFIRMS {
            if self.standing.as_deref() != Some(clause.party.as_str()) {
                return Ok(Verdict::no(format!(
                    "{} no longer has standing on this device",
                    clause.party
                )));
            }
            if let Some(reason) = memory::steward_of_record_refusal(self.root, &clause.party)
                .map_err(|e| format!("cannot read the collective: {e}"))?
            {
                return Ok(Verdict::no(reason));
            }
            return Ok(Verdict::yes(format!(
                "{} stands as steward of record",
                clause.party
            )));
        }
        // purpose / variesRule: only under an honoured affirmation by the same party, who
        // still has standing (an affirmation is honoured only while it does).
        if deep {
            let affirmed = self.affirmed(&clause.party)?;
            if !affirmed.honoured {
                return Ok(Verdict::no(format!(
                    "no honoured steward-of-record affirmation by {}: {}",
                    clause.party, affirmed.reason
                )));
            }
        }
        if clause.kind == PURPOSE {
            return Ok(Verdict::yes(
                "purpose declared under an honoured affirmation",
            ));
        }
        self.manifest_verdict(clause)
    }

    /// Rule 6: the manifest bytes and the rule's floor are still what the clause was granted over.
    fn manifest_verdict(&self, clause: &Clause) -> CovResult<Verdict> {
        let (Some(rule), Some(manifest_path), Some(cid), Some(floor)) = (
            clause.field("rule"),
            clause.field("manifestPath"),
            clause.field("manifestCid"),
            clause.field("floor"),
        ) else {
            return Ok(Verdict::no(
                "the variance clause carries an incomplete payload",
            ));
        };
        let Some(manifest) = load_manifest(self.root, manifest_path) else {
            return Ok(Verdict::no(format!(
                "rule edited since the clause: manifest {manifest_path} is gone"
            )));
        };
        if manifest_cid(&manifest.bytes) != cid {
            return Ok(Verdict::no(format!(
                "rule edited since the clause: {manifest_path} no longer has the bytes it was granted over"
            )));
        }
        let declared = manifest
            .rules
            .iter()
            .find(|r| r.id == rule)
            .and_then(|r| r.variance.as_ref());
        match declared {
            Some(v) if v.floor == floor => Ok(Verdict::yes(format!(
                "rule `{rule}` unchanged since the clause, floor as declared"
            ))),
            Some(_) => Ok(Verdict::no(format!(
                "rule edited since the clause: `{rule}` declares a different floor"
            ))),
            None => Ok(Verdict::no(format!(
                "rule edited since the clause: `{rule}` declares no variance now"
            ))),
        }
    }

    fn verdicts(&self) -> CovResult<Vec<Verdict>> {
        (0..self.clauses.len())
            .map(|i| self.verdict_at(i, true))
            .collect()
    }
}

fn glob_options() -> glob::MatchOptions {
    glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    }
}

fn covers(paths: &[String], path: &str) -> bool {
    paths.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches_with(path, glob_options()))
            .unwrap_or(false)
    })
}

fn clause_json(clause: &Clause, verdict: &Verdict) -> Value {
    json!({
        "cid": clause.cid,
        "kind": clause.kind,
        "party": clause.party,
        "quote": clause.quote,
        "validFrom": clause.valid_from.to_rfc3339(),
        "honoured": verdict.honoured,
        "reason": verdict.reason,
    })
}

// ---------------------------------------------------------------------------
// offer
// ---------------------------------------------------------------------------

fn offer(root: &Path, session: &str, facet_session: &str) -> CovResult<Value> {
    let brit = Brit::resolve(root, facet_session)?;
    let covenant = Covenant::read(root, &brit)?;
    let verdicts = covenant.verdicts()?;
    let honoured = |kind: &str| {
        covenant
            .clauses
            .iter()
            .zip(&verdicts)
            .any(|(c, v)| c.kind == kind && v.honoured)
    };
    let mut pending = Vec::new();
    if !honoured(AFFIRMS) {
        pending.push(json!({
            "kind": AFFIRMS,
            "question": "Do you stand as the steward of record for this session's work?",
        }));
    }
    if !honoured(PURPOSE) {
        pending.push(json!({
            "kind": PURPOSE,
            "question": "What is this session's purpose, and which paths may it write?",
        }));
    }
    let held_rules: BTreeSet<(String, String)> = covenant
        .clauses
        .iter()
        .zip(&verdicts)
        .filter(|(c, v)| c.kind == VARIES && v.honoured)
        .filter_map(|(c, _)| {
            Some((
                c.field("rule")?.to_string(),
                c.field("manifestPath")?.to_string(),
            ))
        })
        .collect();
    let mut variable = Vec::new();
    for manifest in manifests_containing(root, "variance")? {
        for rule in &manifest.rules {
            let Some(variance) = &rule.variance else {
                continue;
            };
            if !matches!(rule.class.as_deref(), Some("inject" | "dispatch"))
                || held_rules.contains(&(rule.id.clone(), manifest.rel.clone()))
            {
                continue;
            }
            variable.push(json!({
                "rule": rule.id,
                "manifest": manifest.rel,
                "scope": rule.when,
                "floor": variance.floor,
                "offeredWhen": variance.offered_when,
                "requires": variance.requires,
            }));
        }
    }
    let held: Vec<Value> = covenant
        .clauses
        .iter()
        .zip(&verdicts)
        .filter(|(_, v)| v.honoured)
        .map(|(c, v)| clause_json(c, v))
        .collect();
    Ok(json!({
        "session": session,
        "standing": covenant.standing,
        "pending": pending,
        "variable": variable,
        "held": held,
    }))
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

pub(crate) fn show(root: &Path, session: &str, facet_session: &str) -> CovResult<Value> {
    let brit = Brit::resolve(root, facet_session)?;
    let covenant = Covenant::read(root, &brit)?;
    let verdicts = covenant.verdicts()?;
    let mut steward = Value::Null;
    let mut purpose = Value::Null;
    let mut variances = Vec::new();
    let mut clauses = Vec::new();
    for (clause, verdict) in covenant.clauses.iter().zip(&verdicts) {
        clauses.push(clause_json(clause, verdict));
        if !verdict.honoured {
            continue;
        }
        match clause.kind.as_str() {
            AFFIRMS => steward = json!(clause.party),
            PURPOSE => {
                purpose = json!({
                    "purpose": clause.field("purpose").unwrap_or_default(),
                    "writeSet": clause.strings("writeSet"),
                });
            }
            VARIES => variances.push(json!({
                "clause": clause.cid,
                "rule": clause.field("rule"),
                "paths": clause.strings("paths"),
                "floor": clause.field("floor"),
                "party": clause.party,
                "validUntil": clause.valid_until.map(|t| t.to_rfc3339()),
            })),
            _ => {}
        }
    }
    Ok(json!({
        "session": session,
        "steward": steward,
        "purpose": purpose,
        "variances": variances,
        "clauses": clauses,
    }))
}

// ---------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------

/// `path` as a normalised repository-relative string, or `None` when it does not name a place
/// inside the repository. The deepest existing ancestor is canonicalised (symlinks resolved) and
/// the canonical root stripped; any `..`, `.` or root component left over is refused, so no
/// spelling of a path outside a clause's directory can read as inside it.
fn repo_relative(root: &Path, path: &str) -> Option<String> {
    let candidate = Path::new(path);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let canonical_root = root.canonicalize().ok()?;
    let mut existing = absolute.as_path();
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    let resolved = loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut resolved = canonical;
            for part in tail.iter().rev() {
                resolved.push(part);
            }
            break resolved;
        }
        tail.push(existing.file_name()?);
        existing = existing.parent()?;
    };
    let relative = resolved.strip_prefix(&canonical_root).ok()?;
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    Some(relative.to_string_lossy().into_owned())
}

fn not_held(variable: bool, reason: impl Into<String>) -> Value {
    json!({
        "status": "not-held",
        "variable": variable,
        "clause": Value::Null,
        "floor": Value::Null,
        "party": Value::Null,
        "reason": reason.into(),
    })
}

fn check(root: &Path, facet_session: &str, args: &Args) -> CovResult<Value> {
    let rule_id = args.rule.as_deref().ok_or("check needs --rule ID")?;
    let raw_path = args.path.as_deref().ok_or("check needs --path P")?;
    let brit = Brit::resolve(root, facet_session)?;
    let Some(path) = repo_relative(root, raw_path) else {
        return Ok(not_held(
            true,
            format!("{raw_path} does not name a normalised path inside the repository"),
        ));
    };

    // Does the rule that applies to this path declare a variance at all?
    let manifests = manifests_containing(root, rule_id)?;
    let mut declared: Vec<(&Manifest, &RuleDecl)> = manifests
        .iter()
        .filter_map(|m| m.rules.iter().find(|r| r.id == rule_id).map(|r| (m, r)))
        .collect();
    // Nearest governing manifest first; one that governs elsewhere only as a fallback.
    declared.sort_by_key(|(m, _)| (!governs(&m.dir, &path), std::cmp::Reverse(m.dir.len())));
    let Some((governing, rule)) = declared.first() else {
        return Ok(not_held(
            false,
            format!("rule `{rule_id}` is declared in no .epr-meta manifest"),
        ));
    };
    // A clause attenuates the bytes of ONE manifest. It holds only where that manifest is the
    // nearest one declaring the rule for this path; a same-id rule elsewhere is not what was granted.
    let governing_rel = governs(&governing.dir, &path).then(|| governing.rel.clone());
    if rule.variance.is_none() {
        return Ok(not_held(
            false,
            format!("rule `{rule_id}` declares no parameters.variance; the full rule applies"),
        ));
    }

    let covenant = Covenant::read(root, &brit)?;
    let mut best: Option<(bool, String)> = None; // (path matched, reason) of the latest non-held
    for index in (0..covenant.clauses.len()).rev() {
        let clause = &covenant.clauses[index];
        if clause.kind != VARIES || clause.field("rule") != Some(rule_id) {
            continue;
        }
        let on_path = covers(&clause.strings("paths"), &path);
        let mut verdict = covenant.verdict_at(index, true)?;
        if verdict.honoured && clause.field("manifestPath") != governing_rel.as_deref() {
            verdict = Verdict::no(format!(
                "clause {} was granted over {}, but the manifest governing {path} for `{rule_id}` \
                 is {}",
                clause.cid,
                clause.field("manifestPath").unwrap_or("?"),
                governing_rel.as_deref().unwrap_or("none")
            ));
        }
        if on_path && verdict.honoured {
            return Ok(json!({
                "status": "held",
                "variable": true,
                "clause": clause.cid,
                "floor": clause.field("floor"),
                "party": clause.party,
                "reason": format!(
                    "clause {} by {} covers {path}: {}",
                    clause.cid, clause.party, verdict.reason
                ),
            }));
        }
        let reason = if on_path {
            verdict.reason
        } else {
            format!("{path} is outside the paths of clause {}", clause.cid)
        };
        match &best {
            Some((true, _)) => {}
            Some((false, _)) if !on_path => {}
            _ => best = Some((on_path, reason)),
        }
    }
    Ok(not_held(
        true,
        best.map(|(_, reason)| reason).unwrap_or_else(|| {
            format!("this session holds no variance clause for rule `{rule_id}`")
        }),
    ))
}

// ---------------------------------------------------------------------------
// record
// ---------------------------------------------------------------------------

fn record(root: &Path, session: &str, facet_session: &str, args: &Args) -> CovResult<Value> {
    let brit = Brit::resolve(root, facet_session)?;
    let kind = args.kind.as_deref().ok_or("record needs --kind")?;
    let kind: &'static str = KINDS
        .iter()
        .copied()
        .find(|k| *k == kind)
        .ok_or_else(|| format!("unknown --kind `{kind}`; one of {}", KINDS.join(", ")))?;
    let party = args
        .party
        .clone()
        .ok_or("record needs --party human:<handle>")?;
    let quote = args
        .quote
        .clone()
        .filter(|q| !q.trim().is_empty())
        .ok_or("record needs --quote: the party's own words")?;
    if party
        .strip_prefix("human:")
        .is_none_or(|handle| handle.is_empty())
    {
        return Err(format!(
            "--party `{party}` is not a human:<handle>; only a human answers a covenant"
        ));
    }
    if let Some(until) = &args.valid_until {
        DateTime::parse_from_rfc3339(until)
            .map_err(|e| format!("--valid-until `{until}` is not RFC3339: {e}"))?;
    }
    let covenant = Covenant::read(root, &brit)?;
    if covenant.standing.as_deref() != Some(party.as_str()) {
        return Err(format!(
            "{party} has no standing on this device for session `{session}` (standing: {}); \
             the session's human claim or the device's roster-bound standing must be {party}",
            covenant.standing.as_deref().unwrap_or("none")
        ));
    }

    let payload = if kind == AFFIRMS {
        if let Some(reason) = memory::steward_of_record_refusal(root, &party)
            .map_err(|e| format!("cannot read the collective: {e}"))?
        {
            return Err(reason);
        }
        json!({})
    } else {
        let affirmed = covenant.affirmed(&party)?;
        if !affirmed.honoured {
            return Err(format!(
                "{kind} needs an honoured affirmsStewardOfRecord by {party} in this session first: {}",
                affirmed.reason
            ));
        }
        match kind {
            PURPOSE => {
                let purpose = args
                    .purpose
                    .clone()
                    .filter(|p| !p.trim().is_empty())
                    .ok_or("declaresPurpose needs --purpose")?;
                json!({ "purpose": purpose, "writeSet": args.write_set })
            }
            VARIES => varies_payload(root, args)?,
            _ => revokes_payload(&covenant, &party, args)?,
        }
    };

    let put = Put {
        kind,
        party,
        quote,
        payload,
        valid_until: args.valid_until.clone(),
    };
    let stored = brit.put(&put)?;
    let cid = stored
        .get("cid")
        .cloned()
        .ok_or("brit-build-ref covenant put returned no cid")?;
    Ok(json!({
        "status": "recorded",
        "cid": cid,
        "clause": stored.get("clause").cloned().unwrap_or(Value::Null),
    }))
}

fn revokes_payload(covenant: &Covenant, party: &str, args: &Args) -> CovResult<Value> {
    let target = args
        .revokes
        .as_deref()
        .ok_or("revokesClause needs --revokes CID")?;
    let index = covenant
        .position(target)
        .ok_or_else(|| format!("clause {target} is not in this session"))?;
    if covenant.clauses[index].party != party {
        return Err(format!(
            "clause {target} belongs to {}, not {party}; only its own party revokes a clause",
            covenant.clauses[index].party
        ));
    }
    Ok(json!({ "revokes": target }))
}

fn varies_payload(root: &Path, args: &Args) -> CovResult<Value> {
    let rule_id = args.rule.as_deref().ok_or("variesRule needs --rule ID")?;
    if args.paths.is_empty() {
        return Err("variesRule needs --paths GLOB…".into());
    }
    let manifests = manifests_containing(root, rule_id)?;
    let declared: Vec<(&Manifest, &RuleDecl)> = manifests
        .iter()
        .filter_map(|m| m.rules.iter().find(|r| r.id == rule_id).map(|r| (m, r)))
        .collect();
    let (manifest, rule) = match declared.as_slice() {
        [] => {
            return Err(format!(
                "rule `{rule_id}` is declared in no .epr-meta manifest"
            ))
        }
        [one] => *one,
        many => {
            return Err(format!(
                "rule `{rule_id}` is declared in {} manifests ({}); a variance names one rule",
                many.len(),
                many.iter()
                    .map(|(m, _)| m.rel.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    };
    if !matches!(rule.class.as_deref(), Some("inject" | "dispatch")) {
        return Err(format!(
            "rule `{rule_id}` is class `{}`; only inject and dispatch rules can vary",
            rule.class.as_deref().unwrap_or("unset")
        ));
    }
    let Some(variance) = &rule.variance else {
        return Err(format!(
            "rule `{rule_id}` declares no parameters.variance; a rule is born without a floor and \
             nothing grants one"
        ));
    };
    let dir_components: Vec<&str> = manifest.dir.split('/').filter(|c| !c.is_empty()).collect();
    for pattern in &args.paths {
        glob::Pattern::new(pattern)
            .map_err(|e| format!("--paths `{pattern}` is not a glob: {e}"))?;
        let path = Path::new(pattern);
        let clean =
            path.is_relative() && path.components().all(|c| matches!(c, Component::Normal(_)));
        let parts: Vec<&str> = pattern.split('/').collect();
        if !clean
            || parts.len() <= dir_components.len()
            || parts[..dir_components.len()] != dir_components[..]
        {
            return Err(format!(
                "--paths `{pattern}` falls outside {} (the directory {} governs): a variance only \
                 narrows, it never widens",
                if manifest.dir.is_empty() {
                    "the repository root"
                } else {
                    &manifest.dir
                },
                manifest.rel
            ));
        }
    }
    let mut payload = Map::new();
    payload.insert("rule".into(), json!(rule_id));
    payload.insert("manifestPath".into(), json!(manifest.rel));
    payload.insert("manifestCid".into(), json!(manifest_cid(&manifest.bytes)));
    payload.insert("paths".into(), json!(args.paths));
    payload.insert("floor".into(), json!(variance.floor));
    Ok(Value::Object(payload))
}
