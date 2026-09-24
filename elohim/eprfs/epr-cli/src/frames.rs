//! Frame atoms and the native frame classifier (Lane C, ruling R-C3).
//!
//! A values guard (sovereignty, ownership) used to carry its vocabulary as a hand-rolled phrase
//! list in each host. The vocabulary now lives in content-addressed frame atoms under
//! [`FRAMES_DIR`], read at runtime by both hosts, and this module turns a write into a
//! [`FrameClassification`] against one of them:
//!
//! - **identity** — an atom's CID is the policy-registry row recipe
//!   (`compute_cid(canonical_body(atom))`), so the bytes are exactly the ones Python's
//!   `policy_content_hash` hashes and `frameRef` names the same atom in both hosts;
//! - **shadow text** — lowercase, then NFKC, then the ontology's confusable table, character by
//!   character, each folded character remembering the byte offset of the ORIGINAL character it
//!   came from, so evidence spans index the bytes the author wrote;
//! - **net-new only** — the post and prior contents are split on `\n` and diffed as multisets of
//!   lines; only added and removed lines are scanned, and the write fires only when the added
//!   lines carry at least `min_net_new` more phrase hits than the removed ones (cleaning or
//!   maintaining existing framing never fires);
//! - **verdict** — an in-content marker (`<frame>-frame: <value>`) whose value is `apex` is
//!   `Drift` at confidence 1; any other declared marker is `Legitimate`; hits with no marker are
//!   `Abstain` at the coarse proxy `min(1, hits / 3)`;
//! - **scan cap** — beyond `scan_cap_bytes` of a document the fold is skipped (lowercase-only
//!   substring scan) and `unscanned-tail` is recorded in the evidence.
//!
//! The Python mirror (`.claude/scripts/_lib/frame_atoms.py`) reproduces the verdict, spans and
//! reason line; the classification CID is minted only here (ruling R-C4, no second DAG-CBOR
//! encoder).

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use cid::Cid;
use elohim_epr::{cid::compute_cid, FrameClassification, FrameEvidence, FrameVerdict, Span};
use eprfs_core::BlobCid;
use eprfs_meta::{canonical_body, GovernanceWrite};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use unicode_normalization::UnicodeNormalization;

/// Where the frame atoms live, relative to the repository root.
pub const FRAMES_DIR: &str = "elohim/sdk/schemas/v1/frames";
/// The ontology atom (normalization + confusables + testimony exemption) — not a frame.
pub const ONTOLOGY_FILE: &str = "frame-ontology.json";
/// The atom schema — not a frame.
pub const SCHEMA_FILE: &str = "frame.schema.json";
/// Recorded in `matchedRecallSignal` when part of the scanned text lay beyond the scan cap.
pub const UNSCANNED_TAIL: &str = "unscanned-tail";
/// The marker value that adjudicates a frame as the apex (drift) rather than a legitimate frame.
const APEX_ANSWER: &str = "apex";

/// One frame atom — mirrors `frame.schema.json` exactly; unknown keys refuse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameAtom {
    pub id: String,
    pub version: u64,
    pub validator: String,
    pub apex_concept: String,
    pub family: String,
    pub polarity: String,
    pub cost_class: String,
    pub binding: String,
    pub linguistic_definition: String,
    pub reason_clause: String,
    pub rubric: FrameRubric,
    pub recall_signal: RecallSignal,
    pub cites: Vec<String>,
    pub established_by: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRubric {
    pub question: String,
    pub legitimate_frames: Vec<LegitimateFrame>,
    pub apex_answer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegitimateFrame {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecallSignal {
    pub phrases: Vec<String>,
    pub markers: Vec<String>,
    pub min_net_new: usize,
    pub scan_cap_bytes: usize,
    pub cosine_floor_permille: u16,
    pub probe_min_net_new_bytes: usize,
}

/// The ontology atom: one normalization table both hosts read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameOntology {
    pub id: String,
    pub version: u64,
    pub cites: Vec<String>,
    pub normalization: Normalization,
    pub testimony_exempt: TestimonyExempt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    pub nfkc: bool,
    pub lowercase: bool,
    pub confusables: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestimonyExempt {
    pub path_prefixes: Vec<String>,
    pub frontmatter_key: String,
}

/// Why the atoms could not be read. A guard that meets one never passes the write.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path} is not a valid frame document: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("two frame atoms bind validator `{0}`")]
    DuplicateValidator(String),
}

/// A frame atom paired with its content address.
pub type LoadedFrame = (FrameAtom, Cid);

/// The atom's CID by the registry-row recipe: `compute_cid(canonical_body(row))` over the parsed
/// JSON, the same canonical bytes Python's `policy_content_hash` hashes.
pub fn atom_cid(raw: &Value) -> Result<Cid, String> {
    let yaml = serde_yaml::to_value(raw).map_err(|error| error.to_string())?;
    let serde_yaml::Value::Mapping(row) = yaml else {
        return Err("a frame atom must be a JSON object".into());
    };
    let body = canonical_body(&row).map_err(|error| error.to_string())?;
    Ok(compute_cid(&body))
}

fn read_json(path: &Path) -> Result<Value, FrameError> {
    let raw = fs::read_to_string(path).map_err(|source| FrameError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|error| FrameError::Invalid {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// `^<head>[<tail>]*$` over ASCII classes, the shape every id pattern in `frame.schema.json` has.
fn ascii_pattern(value: &str, prefix: &str, first: fn(u8) -> bool, rest: fn(u8) -> bool) -> bool {
    let Some(body) = value.strip_prefix(prefix) else {
        return false;
    };
    let bytes = body.as_bytes();
    !bytes.is_empty() && first(bytes[0]) && bytes[1..].iter().all(|b| rest(*b))
}

fn lower_alnum_dash(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
}

/// The constraints `frame.schema.json` states beyond the shape serde already enforces
/// (ruling R-C12, review M1). An atom that breaks its own schema is refused as
/// [`FrameError::Invalid`], so the guard degrades to `Unavailable` — the declared class — rather
/// than looping, panicking, or silently never matching. The Python loader
/// (`frame_atoms._validate`) refuses exactly the same atoms.
pub fn validate_atom(atom: &FrameAtom) -> Result<(), String> {
    let non_empty = [
        ("apex_concept", &atom.apex_concept),
        ("linguistic_definition", &atom.linguistic_definition),
        ("reason_clause", &atom.reason_clause),
        ("rubric.question", &atom.rubric.question),
        ("established_by", &atom.established_by),
    ];
    for (key, value) in non_empty {
        if value.is_empty() {
            return Err(format!("`{key}` must not be empty"));
        }
    }
    if !ascii_pattern(
        &atom.id,
        "frame-",
        |b| b.is_ascii_lowercase(),
        lower_alnum_dash,
    ) {
        return Err(format!(
            "`id` `{}` must match ^frame-[a-z][a-z0-9-]*$",
            atom.id
        ));
    }
    if !ascii_pattern(
        &atom.validator,
        "epr:validator-",
        |b| b.is_ascii_lowercase(),
        lower_alnum_dash,
    ) {
        return Err(format!(
            "`validator` `{}` must match ^epr:validator-[a-z][a-z0-9-]*$",
            atom.validator
        ));
    }
    if atom.version < 1 {
        return Err("`version` must be at least 1".into());
    }
    for (key, value, want) in [
        ("family", &atom.family, "defeater"),
        ("polarity", &atom.polarity, "incriminating"),
        ("cost_class", &atom.cost_class, "high-fp-cost"),
        ("binding", &atom.binding, "binding-local"),
        ("rubric.apex_answer", &atom.rubric.apex_answer, APEX_ANSWER),
    ] {
        if value != want {
            return Err(format!("`{key}` must be `{want}`, not `{value}`"));
        }
    }
    if atom.rubric.legitimate_frames.is_empty() {
        return Err("`rubric.legitimate_frames` needs at least one frame".into());
    }
    for frame in &atom.rubric.legitimate_frames {
        if !ascii_pattern(&frame.id, "", |b| b.is_ascii_lowercase(), lower_alnum_dash) {
            return Err(format!(
                "legitimate frame id `{}` must match ^[a-z][a-z0-9-]*$",
                frame.id
            ));
        }
        if frame.description.is_empty() {
            return Err(format!(
                "legitimate frame `{}` needs a description",
                frame.id
            ));
        }
    }
    if atom.cites.is_empty() || atom.cites.iter().any(String::is_empty) {
        return Err("`cites` needs at least one non-empty cite".into());
    }
    let signal = &atom.recall_signal;
    if signal.phrases.is_empty() {
        return Err("`recall_signal.phrases` needs at least one phrase".into());
    }
    for (i, phrase) in signal.phrases.iter().enumerate() {
        // `minLength: 1` — an empty phrase matches at every offset with length 0.
        if phrase.is_empty() {
            return Err("an empty recall_signal phrase would match everywhere".into());
        }
        // `^[^A-Z]+$` — phrases match the LOWERCASED shadow; an uppercase one never matches.
        if phrase.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(format!(
                "recall_signal phrase `{phrase}` has uppercase letters; the shadow is lowercased, \
                 so it could never match"
            ));
        }
        if signal.phrases[..i].contains(phrase) {
            return Err(format!("recall_signal phrase `{phrase}` is listed twice"));
        }
    }
    if signal.markers.is_empty() {
        return Err("`recall_signal.markers` needs at least one marker".into());
    }
    for (i, marker) in signal.markers.iter().enumerate() {
        let well_formed = marker.strip_suffix(':').is_some_and(|body| {
            ascii_pattern(
                body,
                "",
                |b| b.is_ascii_lowercase(),
                |b| b.is_ascii_lowercase() || b == b'-',
            )
        });
        if !well_formed {
            return Err(format!(
                "recall_signal marker `{marker}` must match ^[a-z][a-z-]*:$"
            ));
        }
        if signal.markers[..i].contains(marker) {
            return Err(format!("recall_signal marker `{marker}` is listed twice"));
        }
    }
    if signal.min_net_new < 1 {
        return Err("`recall_signal.min_net_new` must be at least 1".into());
    }
    if signal.scan_cap_bytes < 1 {
        return Err("`recall_signal.scan_cap_bytes` must be at least 1".into());
    }
    if signal.cosine_floor_permille > 1000 {
        return Err("`recall_signal.cosine_floor_permille` must be at most 1000".into());
    }
    Ok(())
}

/// Parse one atom strictly, check it against its schema's constraints, and mint its CID.
pub fn load_atom(path: &Path) -> Result<LoadedFrame, FrameError> {
    let raw = read_json(path)?;
    let invalid = |message: String| FrameError::Invalid {
        path: path.to_path_buf(),
        message,
    };
    let atom: FrameAtom =
        serde_json::from_value(raw.clone()).map_err(|error| invalid(error.to_string()))?;
    validate_atom(&atom).map_err(invalid)?;
    let cid = atom_cid(&raw).map_err(invalid)?;
    Ok((atom, cid))
}

/// Every frame atom under [`FRAMES_DIR`], keyed by the validator ref it binds.
pub fn load_frames(repo_root: &Path) -> Result<BTreeMap<String, LoadedFrame>, FrameError> {
    let dir = repo_root.join(FRAMES_DIR);
    let entries = fs::read_dir(&dir).map_err(|source| FrameError::Io {
        path: dir.clone(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| FrameError::Io {
                path: dir.clone(),
                source,
            })?
            .path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".json") && name != ONTOLOGY_FILE && name != SCHEMA_FILE {
            paths.push(path);
        }
    }
    paths.sort();
    let mut frames = BTreeMap::new();
    for path in paths {
        let (atom, cid) = load_atom(&path)?;
        let validator = atom.validator.clone();
        if frames.insert(validator.clone(), (atom, cid)).is_some() {
            return Err(FrameError::DuplicateValidator(validator));
        }
    }
    Ok(frames)
}

/// The ontology atom paired with its content address (`ontologyRef`).
pub type LoadedOntology = (FrameOntology, Cid);

/// The ontology atom, read strictly, with its CID by the same registry-row recipe as a frame
/// atom's (review W2): the fold table is part of what a classification means, so the evidence
/// names it and a classification reproduces from `(targetCid, frameRef, ontologyRef)`.
pub fn load_ontology(repo_root: &Path) -> Result<LoadedOntology, FrameError> {
    let path = repo_root.join(FRAMES_DIR).join(ONTOLOGY_FILE);
    let raw = read_json(&path)?;
    let invalid = |message: String| FrameError::Invalid {
        path: path.clone(),
        message,
    };
    let ontology: FrameOntology =
        serde_json::from_value(raw.clone()).map_err(|error| invalid(error.to_string()))?;
    let cid = atom_cid(&raw).map_err(invalid)?;
    Ok((ontology, cid))
}

/// The frame bound to `validator` plus the ontology, or why they could not be read.
pub fn frame_for(
    repo_root: &Path,
    validator: &str,
) -> Result<(LoadedFrame, LoadedOntology), FrameError> {
    let mut frames = load_frames(repo_root)?;
    let frame = frames
        .remove(validator)
        .ok_or_else(|| FrameError::Invalid {
            path: repo_root.join(FRAMES_DIR),
            message: format!("no frame atom binds validator `{validator}`"),
        })?;
    Ok((frame, load_ontology(repo_root)?))
}

/// `bafyreig…rn6e` — the short CID spelling `epr` and the surfacing hook print.
pub fn short_cid(cid: &str) -> String {
    if cid.chars().count() > 14 {
        let head: String = cid.chars().take(8).collect();
        let tail: String = cid
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}…{tail}")
    } else {
        cid.to_string()
    }
}

/// The lowercase verdict word the reason line ends with.
pub fn verdict_word(verdict: FrameVerdict) -> &'static str {
    match verdict {
        FrameVerdict::Legitimate => "legitimate",
        FrameVerdict::Drift => "drift",
        FrameVerdict::Abstain => "abstain",
    }
}

/// The one reason-line format both hosts render (byte-identical in `frame_atoms.py`).
pub fn reason_line(
    atom: &FrameAtom,
    frame_ref: &str,
    classification: &str,
    verdict: &str,
) -> String {
    format!(
        "{} · frame {} · classification {} · {verdict}",
        atom.reason_clause,
        short_cid(frame_ref),
        short_cid(classification),
    )
}

/// The raw-codec (0x55) sha2-256 CID of the write's content — the judged target.
fn target_cid(content: &str) -> Cid {
    *BlobCid::compute_raw(content.as_bytes()).as_cid()
}

/// Folded text with, per folded byte, the original byte span of the character it came from.
struct Shadow {
    folded: String,
    origin: Vec<(usize, usize)>,
}

struct Folder {
    lowercase: bool,
    nfkc: bool,
    confusables: HashMap<char, String>,
}

impl Folder {
    fn new(ontology: &FrameOntology) -> Self {
        let confusables = ontology
            .normalization
            .confusables
            .iter()
            .filter_map(|(from, to)| {
                let mut chars = from.chars();
                match (chars.next(), chars.next()) {
                    (Some(single), None) => Some((single, to.clone())),
                    _ => None,
                }
            })
            .collect();
        Self {
            lowercase: ontology.normalization.lowercase,
            nfkc: ontology.normalization.nfkc,
            confusables,
        }
    }

    /// Fold `text` (which starts at byte `base` of its document). Characters at document offsets
    /// at or beyond `cap` are lowercased only — the substring-only degradation.
    fn shadow(&self, text: &str, base: usize, cap: usize) -> (Shadow, bool) {
        let mut shadow = Shadow {
            folded: String::with_capacity(text.len()),
            origin: Vec::with_capacity(text.len()),
        };
        let mut beyond_cap = false;
        let push = |shadow: &mut Shadow, c: char, span: (usize, usize)| {
            shadow.folded.push(c);
            for _ in 0..c.len_utf8() {
                shadow.origin.push(span);
            }
        };
        for (offset, original) in text.char_indices() {
            let start = base + offset;
            let span = (start, start + original.len_utf8());
            let lowered: Vec<char> = if self.lowercase {
                original.to_lowercase().collect()
            } else {
                vec![original]
            };
            if start >= cap {
                beyond_cap = true;
                for c in lowered {
                    push(&mut shadow, c, span);
                }
                continue;
            }
            for l in lowered {
                let normalized: Vec<char> = if self.nfkc {
                    std::iter::once(l).nfkc().collect()
                } else {
                    vec![l]
                };
                for n in normalized {
                    match self.confusables.get(&n) {
                        Some(to) => to.chars().for_each(|c| push(&mut shadow, c, span)),
                        None => push(&mut shadow, n, span),
                    }
                }
            }
        }
        (shadow, beyond_cap)
    }
}

/// `(byte offset, line)` for every `\n`-separated line — `\n` only, so both hosts split alike.
fn lines_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    for line in text.split('\n') {
        out.push((start, line));
        start += line.len() + 1;
    }
    out
}

/// A line and the byte offset it starts at.
type Line<'a> = (usize, &'a str);

/// Multiset line diff: the post lines not matched by a prior line (in post order), and the
/// prior lines left unmatched.
fn line_delta<'a>(prior: &'a str, post: &'a str) -> (Vec<Line<'a>>, Vec<Line<'a>>) {
    let mut remaining: HashMap<&str, Vec<(usize, &str)>> = HashMap::new();
    for (offset, line) in lines_with_offsets(prior) {
        remaining.entry(line).or_default().push((offset, line));
    }
    for bucket in remaining.values_mut() {
        bucket.reverse();
    }
    let mut added = Vec::new();
    for (offset, line) in lines_with_offsets(post) {
        match remaining.get_mut(line).and_then(Vec::pop) {
            Some(_) => {}
            None => added.push((offset, line)),
        }
    }
    let mut removed: Vec<(usize, &str)> = remaining.into_values().flatten().collect();
    removed.sort_unstable();
    (added, removed)
}

struct Scan {
    hits: usize,
    spans: Vec<Span>,
    phrases: Vec<String>,
    beyond_cap: bool,
}

fn scan(lines: &[(usize, &str)], phrases: &[String], folder: &Folder, cap: usize) -> Scan {
    let mut out = Scan {
        hits: 0,
        spans: Vec::new(),
        phrases: Vec::new(),
        beyond_cap: false,
    };
    for (base, line) in lines {
        if line.is_empty() {
            continue;
        }
        let (shadow, beyond) = folder.shadow(line, *base, cap);
        out.beyond_cap |= beyond;
        // An empty phrase is refused by the loader (R-C12); skipping it here keeps a hand-built
        // atom from underflowing `at + len - 1`.
        for phrase in phrases.iter().filter(|phrase| !phrase.is_empty()) {
            for (at, matched) in shadow.folded.match_indices(phrase.as_str()) {
                out.hits += 1;
                out.spans.push(Span {
                    start: shadow.origin[at].0 as u64,
                    end: shadow.origin[at + matched.len() - 1].1 as u64,
                });
                if !out.phrases.contains(phrase) {
                    out.phrases.push(phrase.clone());
                }
            }
        }
    }
    out.spans
        .sort_unstable_by_key(|span| (span.start, span.end));
    out.phrases.sort();
    out
}

/// Declared markers in the post content, in atom order, each with its (lowercased, trimmed)
/// value — the text after the marker on the same line.
fn declared_markers(post: &str, markers: &[String]) -> Vec<(String, String)> {
    let lowered = post.to_lowercase();
    let mut found = Vec::new();
    for marker in markers.iter().filter(|marker| !marker.is_empty()) {
        for (at, _) in lowered.match_indices(marker.as_str()) {
            let rest = &lowered[at + marker.len()..];
            let value = rest.split('\n').next().unwrap_or("");
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            found.push((marker.clone(), value.to_string()));
        }
    }
    found
}

/// Classify a write against one frame. `None` when the write adds no net-new apex phrases.
pub fn classify(
    write: &GovernanceWrite,
    frame: &LoadedFrame,
    ontology: &FrameOntology,
) -> Option<(FrameClassification, Cid)> {
    let (atom, frame_ref) = frame;
    let signal = &atom.recall_signal;
    let post = write.content.as_deref().unwrap_or("");
    let prior = if write.is_new {
        ""
    } else {
        write.prior_content.as_deref().unwrap_or("")
    };
    let folder = Folder::new(ontology);
    let (added, removed) = line_delta(prior, post);
    let gained = scan(&added, &signal.phrases, &folder, signal.scan_cap_bytes);
    let lost = scan(&removed, &signal.phrases, &folder, signal.scan_cap_bytes);
    if gained.hits < lost.hits + signal.min_net_new {
        return None;
    }
    let net_new = gained.hits - lost.hits;

    let markers = declared_markers(post, &signal.markers);
    let apex = markers
        .iter()
        .find(|(_, value)| value == APEX_ANSWER || value == &atom.rubric.apex_answer);
    let (verdict, confidence, rubric_answer) = match (apex, markers.first()) {
        (Some((_, value)), _) => (FrameVerdict::Drift, 1.0, Some(value.clone())),
        (None, Some((_, value))) => (
            FrameVerdict::Legitimate,
            1.0,
            (!value.is_empty()).then(|| value.clone()),
        ),
        (None, None) => (FrameVerdict::Abstain, (net_new as f64 / 3.0).min(1.0), None),
    };

    let mut matched = gained.phrases;
    for marker in &signal.markers {
        if markers.iter().any(|(declared, _)| declared == marker) {
            matched.push(marker.clone());
        }
    }
    if gained.beyond_cap {
        matched.push(UNSCANNED_TAIL.to_string());
    }

    let classification = FrameClassification {
        target_cid: target_cid(post),
        frame_ref: *frame_ref,
        verdict,
        confidence,
        evidence: FrameEvidence {
            spans: gained.spans,
            matched_recall_signal: matched,
            rubric_answer,
            reason_ref: None,
        },
    };
    let cid = classification.content_cid().ok()?;
    Some((classification, cid))
}

/// The opaque evidence value a guard hands to `ValidatorOutcome::Classified`. The top-level keys
/// are the ones the Python mirror carries too (`frameRef`, `ontologyRef`, `verdict`, `spans`,
/// `confidence`, `reason`, `classificationCid`); `classification` is the full record, with its
/// CIDs spelled as strings (one encoding per meaning in governed JSON). `ontologyRef` sits here,
/// beside `classificationCid`, and not inside `FrameClassification`: the primitive in
/// `elohim-epr` is unchanged, and the fold table's address rides in the carried evidence
/// (ruling R-C14, review W2).
pub fn evidence_value(
    classification: &FrameClassification,
    classification_cid: &Cid,
    ontology_ref: &Cid,
    reason: &str,
) -> Value {
    let mut record = serde_json::to_value(classification).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut record {
        map.insert(
            "targetCid".into(),
            Value::String(classification.target_cid.to_string()),
        );
        map.insert(
            "frameRef".into(),
            Value::String(classification.frame_ref.to_string()),
        );
    }
    json!({
        "frameRef": classification.frame_ref.to_string(),
        "ontologyRef": ontology_ref.to_string(),
        "verdict": verdict_word(classification.verdict),
        "spans": classification.evidence.spans,
        "confidence": classification.confidence,
        "reason": reason,
        "classification": record,
        "classificationCid": classification_cid.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_validators::ElohimRepositoryValidators;
    use eprfs_core::{GovernanceRule, GovernanceRuleClass, GovernanceRulePredicate};
    use eprfs_meta::{ValidatorOutcome, ValidatorProvider, ValidatorRequest};
    use tempfile::TempDir;

    const SOVEREIGNTY: &str = "epr:validator-sovereignty-ontology-guard";
    const OWNERSHIP: &str = "epr:validator-ownership-ontology-guard";

    /// The atom CIDs by the registry-row recipe. The Python mirror pins the same strings
    /// (`frame_atoms_test.py::atom_cid_equals_rust_pin`); a change to an atom's bytes reds both.
    const SOVEREIGNTY_ATOM_CID: &str =
        "bafyreif2vuz6tnzwtvkgogulw25u2h65edipkbyxw5guf2yyezy5i5zo4e";
    const OWNERSHIP_ATOM_CID: &str = "bafyreif2f3eve4kg5skanqi4exvigznllxuqsy6r6w6t4pgibp2cybx2xe";
    /// Habit `guards-sense-frames` check (c): a fixed write under the sovereignty atom.
    const GOLDEN_CLASSIFICATION_CID: &str =
        "bafyreifb4rgfeqi74y2ulv2hnijbu6n3fjrgap2y6fhmrqnpnazg6lmenm";

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap()
    }

    fn frame(validator: &str) -> (LoadedFrame, FrameOntology) {
        let (loaded, (ontology, _)) =
            frame_for(&repo_root(), validator).expect("the live atoms load");
        (loaded, ontology)
    }

    fn write(prior: Option<&str>, content: &str) -> GovernanceWrite {
        let mut write = GovernanceWrite::new("genesis/docs/content/elohim-protocol/note.md");
        write.is_new = prior.is_none();
        write.prior_content = prior.map(str::to_string);
        write.content = Some(content.to_string());
        write
    }

    fn classify_as(
        validator: &str,
        prior: Option<&str>,
        content: &str,
    ) -> Option<FrameClassification> {
        let (loaded, ontology) = frame(validator);
        classify(&write(prior, content), &loaded, &ontology).map(|(c, _)| c)
    }

    fn rule() -> GovernanceRule {
        GovernanceRule {
            id: "sovereignty-ontology-guard".into(),
            class: GovernanceRuleClass::Dispatch,
            when: Value::Null,
            predicate: GovernanceRulePredicate::Validator,
            parameters: Value::String(SOVEREIGNTY.into()),
            policy_ref: None,
            why: None,
        }
    }

    fn outcome(root: &Path, validator: &str, content: &str) -> ValidatorOutcome {
        let rule = rule();
        let write = write(None, content);
        ElohimRepositoryValidators.evaluate(&ValidatorRequest {
            repo_root: root,
            reference: validator,
            rule: &rule,
            write: &write,
            cid: None,
            fuel: None,
        })
    }

    #[test]
    fn every_atom_in_dir_parses_strict() {
        let frames = load_frames(&repo_root()).expect("every atom parses strictly");
        assert_eq!(
            frames.keys().cloned().collect::<Vec<_>>(),
            vec![OWNERSHIP.to_string(), SOVEREIGNTY.to_string()]
        );
        for (validator, (atom, _)) in &frames {
            assert_eq!(&atom.validator, validator);
            assert_eq!(atom.rubric.apex_answer, APEX_ANSWER);
            assert!(!atom.reason_clause.is_empty());
        }
        load_ontology(&repo_root()).expect("the ontology parses strictly");

        // Strict: an unknown key (a lifecycle key smuggled into the atom) refuses.
        let path = repo_root()
            .join(FRAMES_DIR)
            .join("frame-sovereignty-apex.json");
        let mut raw = read_json(&path).unwrap();
        raw["status"] = json!("active");
        assert!(serde_json::from_value::<FrameAtom>(raw).is_err());
    }

    #[test]
    fn atom_cid_matches_registry_row_recipe() {
        let root = repo_root();
        let frames = load_frames(&root).unwrap();
        for (validator, file, pin) in [
            (
                SOVEREIGNTY,
                "frame-sovereignty-apex.json",
                SOVEREIGNTY_ATOM_CID,
            ),
            (
                OWNERSHIP,
                "frame-ownership-inalienable.json",
                OWNERSHIP_ATOM_CID,
            ),
        ] {
            let raw = read_json(&root.join(FRAMES_DIR).join(file)).unwrap();
            // The canon-lift recipe, spelled out: canonical_body over the row, then compute_cid.
            let serde_yaml::Value::Mapping(row) = serde_yaml::to_value(&raw).unwrap() else {
                panic!("atom is an object");
            };
            let body = canonical_body(&row).unwrap();
            let recipe = compute_cid(&body);
            let (_, loaded) = &frames[validator];
            assert_eq!(*loaded, recipe, "{file}");
            // The digest is the sha256 of Python's `json.dumps(sort_keys, compact, ensure_ascii)`.
            let text = String::from_utf8(body).unwrap();
            assert!(text.starts_with("{\"apex_concept\":"), "{text}");
            println!("{file} frameRef = {recipe}");
            assert_eq!(
                recipe.to_string(),
                pin,
                "{file} atom CID drift — actual = {recipe}"
            );
        }
    }

    #[test]
    fn nfkc_and_confusable_fold_reports_spans_in_original() {
        // U+2011 NON-BREAKING HYPHEN and a Cyrillic о (U+043E) in "self‑sоvereign".
        let content = "We are self\u{2011}s\u{43e}vereign now.";
        let classification = classify_as(SOVEREIGNTY, None, content).expect("homoglyphs fold");
        assert_eq!(classification.verdict, FrameVerdict::Abstain);
        let start = content.find("self").unwrap();
        let end = content.find(" now").unwrap();
        assert_eq!(
            classification.evidence.spans,
            vec![Span {
                start: start as u64,
                end: end as u64
            }]
        );
        assert_eq!(&content[start..end], "self\u{2011}s\u{43e}vereign");
        assert_eq!(
            classification.evidence.matched_recall_signal,
            vec!["self-sovereign".to_string()]
        );

        // NFKC: fullwidth letters fold to ASCII; the span still indexes the original bytes.
        let wide = "Ｓｅｌｆ-sovereign.";
        let classification = classify_as(SOVEREIGNTY, None, wide).expect("NFKC folds");
        assert_eq!(
            classification.evidence.spans,
            vec![Span {
                start: 0,
                end: wide.find('.').unwrap() as u64
            }]
        );
    }

    #[test]
    fn net_new_only_never_traps_cleaning() {
        let prior = "Platforms promise true data ownership and ownership rights.\n";
        // Maintenance beside existing framing: nothing net-new.
        let post = format!("{prior}Custody is stewarded, not held.\n");
        assert!(classify_as(OWNERSHIP, Some(prior), &post).is_none());
        // Cleaning the framing out.
        assert!(classify_as(OWNERSHIP, Some(prior), "Platforms promise stewardship.\n").is_none());
        // Rewording a line that already carried the framing keeps the count — not net-new.
        let reworded = "Platforms loudly promise true data ownership and ownership rights.\n";
        assert!(classify_as(OWNERSHIP, Some(prior), reworded).is_none());
        // Adding one more fires, and the span points into the added line only.
        let post = format!("{prior}Contributors receive outright ownership.\n");
        let classification = classify_as(OWNERSHIP, Some(prior), &post).expect("net-new fires");
        let start = post.find("outright ownership").unwrap() as u64;
        assert_eq!(
            classification.evidence.spans,
            vec![Span {
                start,
                end: start + "outright ownership".len() as u64
            }]
        );
    }

    #[test]
    fn marker_legitimate_is_silent() {
        let content = "sovereignty-frame: adversary\nNation-states promise digital sovereignty.";
        let classification = classify_as(SOVEREIGNTY, None, content).unwrap();
        assert_eq!(classification.verdict, FrameVerdict::Legitimate);
        assert_eq!(
            classification.evidence.rubric_answer.as_deref(),
            Some("adversary")
        );
        assert!(matches!(
            outcome(&repo_root(), SOVEREIGNTY, content),
            ValidatorOutcome::Pass
        ));
        // Ownership honours both markers.
        for marker in ["stewardship-frame: bounded", "sovereignty-frame: adversary"] {
            let content = format!("{marker}\nPlatforms promise true data ownership.");
            assert!(matches!(
                outcome(&repo_root(), OWNERSHIP, &content),
                ValidatorOutcome::Pass
            ));
        }
    }

    #[test]
    fn marker_apex_is_drift_confidence_1() {
        let content = "sovereignty-frame: apex\nWe are self-sovereign.";
        let classification = classify_as(SOVEREIGNTY, None, content).unwrap();
        assert_eq!(classification.verdict, FrameVerdict::Drift);
        assert_eq!(classification.confidence, 1.0);
        assert_eq!(
            classification.evidence.rubric_answer.as_deref(),
            Some("apex")
        );
        assert_eq!(
            classification.evidence.matched_recall_signal,
            vec![
                "self-sovereign".to_string(),
                "sovereignty-frame:".to_string()
            ]
        );
        assert!(matches!(
            outcome(&repo_root(), SOVEREIGNTY, content),
            ValidatorOutcome::Classified { .. }
        ));
    }

    #[test]
    fn hit_without_marker_is_abstain_confidence_proxy() {
        let one = classify_as(SOVEREIGNTY, None, "Fully sovereign.").unwrap();
        assert_eq!(one.verdict, FrameVerdict::Abstain);
        assert_eq!(one.confidence, 1.0 / 3.0);
        assert_eq!(one.evidence.rubric_answer, None);
        let many = classify_as(
            SOVEREIGNTY,
            None,
            "Fully sovereign.\nDigital sovereignty.\nSovereign identity.\nSelf sovereignty.",
        )
        .unwrap();
        assert_eq!(many.confidence, 1.0);
        assert!(classify_as(SOVEREIGNTY, None, "Stewardship, not sovereignty.").is_none());
    }

    #[test]
    fn scan_cap_degrades_to_substring_and_records_unscanned_tail() {
        let ((mut atom, cid), ontology) = frame(SOVEREIGNTY);
        atom.recall_signal.scan_cap_bytes = 16;
        let homoglyph = "self\u{2011}s\u{43e}vereign";
        // Line 1 lies under the cap (folded); lines 2-3 lie beyond it (lowercase substring only).
        let content = format!("{homoglyph} one\nfully sovereign\n{homoglyph} two\n");
        let (classification, _) =
            classify(&write(None, &content), &(atom, cid), &ontology).expect("fires");
        let beyond = content.find("fully").unwrap() as u64;
        assert_eq!(
            classification.evidence.spans,
            vec![
                Span {
                    start: 0,
                    end: homoglyph.len() as u64
                },
                Span {
                    start: beyond,
                    end: beyond + "fully sovereign".len() as u64
                },
            ],
            "the tail homoglyph is not folded, the tail ASCII phrase still matches"
        );
        assert_eq!(
            classification
                .evidence
                .matched_recall_signal
                .last()
                .map(String::as_str),
            Some(UNSCANNED_TAIL)
        );
    }

    #[test]
    fn golden_fixture_classification_cid() {
        let (loaded, ontology) = frame(SOVEREIGNTY);
        let mut fixture =
            GovernanceWrite::new("genesis/docs/content/elohim-protocol/golden-frame.md");
        fixture.is_new = true;
        fixture.content = Some(
            "# Golden frame fixture\n\nThe protocol names self-sovereign identity its apex.\n"
                .into(),
        );
        let (classification, cid) = classify(&fixture, &loaded, &ontology).unwrap();
        assert_eq!(classification.verdict, FrameVerdict::Abstain);
        assert_eq!(cid, classification.content_cid().unwrap());
        println!("golden classification CID = {cid}");
        assert_eq!(
            cid.to_string(),
            GOLDEN_CLASSIFICATION_CID,
            "golden classification CID drift — actual = {cid}"
        );
    }

    /// A fixture repo root carrying the live atoms with the sovereignty atom rewritten by `edit`.
    fn fixture_with_sovereignty_atom(edit: impl FnOnce(&mut Value)) -> TempDir {
        let dir = TempDir::new().unwrap();
        let frames = dir.path().join(FRAMES_DIR);
        fs::create_dir_all(&frames).unwrap();
        for name in [
            ONTOLOGY_FILE,
            "frame-sovereignty-apex.json",
            "frame-ownership-inalienable.json",
        ] {
            fs::copy(repo_root().join(FRAMES_DIR).join(name), frames.join(name)).unwrap();
        }
        let path = frames.join("frame-sovereignty-apex.json");
        let mut raw = read_json(&path).unwrap();
        edit(&mut raw);
        fs::write(&path, serde_json::to_string_pretty(&raw).unwrap()).unwrap();
        dir
    }

    /// Review M1 (ruling R-C12): an empty phrase used to reach `scan`, where
    /// `match_indices("")` matches at offset 0 with length 0 and `at + 0 - 1` underflowed —
    /// a panic in the decision authority. The loader now refuses an atom that breaks its own
    /// schema (`minLength: 1`), so the guard is `Unavailable` (clamped to the declared class),
    /// never a panic and never a pass.
    #[test]
    fn m1_empty_phrase_is_refused_as_invalid_atom() {
        let dir = fixture_with_sovereignty_atom(|raw| {
            raw["recall_signal"]["phrases"]
                .as_array_mut()
                .unwrap()
                .push(json!(""));
        });
        match load_frames(dir.path()) {
            Err(FrameError::Invalid { message, .. }) => {
                assert!(message.contains("phrase"), "{message}")
            }
            other => panic!("an empty phrase must refuse the atom, got {other:?}"),
        }
        for content in ["We are self-sovereign.", "Plain stewardship prose."] {
            assert!(matches!(
                outcome(dir.path(), SOVEREIGNTY, content),
                ValidatorOutcome::Unavailable
            ));
        }
        // An empty marker is refused the same way (it would match at every offset).
        let dir = fixture_with_sovereignty_atom(|raw| {
            raw["recall_signal"]["markers"] = json!([""]);
        });
        assert!(matches!(
            load_frames(dir.path()),
            Err(FrameError::Invalid { .. })
        ));
        // Defense in depth: a hand-built atom that never passed the loader still cannot panic.
        let ((mut atom, cid), ontology) = frame(SOVEREIGNTY);
        atom.recall_signal.phrases.push(String::new());
        atom.recall_signal.markers.push(String::new());
        let classified = classify(
            &write(None, "We are self-sovereign."),
            &(atom, cid),
            &ontology,
        )
        .expect("the real phrase still fires");
        assert_eq!(classified.0.evidence.spans.len(), 1);
    }

    /// Review M1 (ruling R-C12): phrases are matched against the LOWERCASED shadow, so an
    /// uppercase phrase can never match — a silently dead guard. The schema says `^[^A-Z]+$`;
    /// the loader refuses what the schema refuses. Markers follow `^[a-z][a-z-]*:$`, and the
    /// other `minLength` / `minimum` constraints refuse the same way.
    #[test]
    fn m1_uppercase_phrase_is_refused() {
        let dir = fixture_with_sovereignty_atom(|raw| {
            raw["recall_signal"]["phrases"][0] = json!("Self-Sovereign");
        });
        assert!(matches!(
            load_frames(dir.path()),
            Err(FrameError::Invalid { .. })
        ));
        assert!(matches!(
            outcome(dir.path(), SOVEREIGNTY, "We are self-sovereign."),
            ValidatorOutcome::Unavailable
        ));
        for edit in [
            (|raw: &mut Value| raw["recall_signal"]["markers"] = json!(["Sovereignty-frame:"]))
                as fn(&mut Value),
            |raw| raw["recall_signal"]["markers"] = json!(["sovereignty-frame"]),
            |raw| raw["recall_signal"]["phrases"] = json!([]),
            |raw| raw["recall_signal"]["phrases"] = json!(["fully sovereign", "fully sovereign"]),
            |raw| raw["recall_signal"]["min_net_new"] = json!(0),
            |raw| raw["recall_signal"]["scan_cap_bytes"] = json!(0),
            |raw| raw["recall_signal"]["cosine_floor_permille"] = json!(1001),
            |raw| raw["reason_clause"] = json!(""),
            |raw| raw["family"] = json!("supporter"),
            |raw| raw["rubric"]["apex_answer"] = json!("summit"),
            |raw| raw["rubric"]["legitimate_frames"] = json!([]),
            |raw| raw["id"] = json!("Frame-X"),
        ] {
            let dir = fixture_with_sovereignty_atom(edit);
            assert!(
                matches!(load_frames(dir.path()), Err(FrameError::Invalid { .. })),
                "a schema-violating atom must refuse"
            );
        }
        // The live atoms satisfy their schema.
        load_frames(&repo_root()).expect("the live atoms are schema-valid");
    }

    /// Review W2 (ruling R-C14): the fold table (NFKC + confusables) decides what a phrase hit
    /// is, so the carried evidence names it by content address, beside `classificationCid`.
    #[test]
    fn w2_evidence_carries_ontology_ref() {
        let root = repo_root();
        let raw = read_json(&root.join(FRAMES_DIR).join(ONTOLOGY_FILE)).unwrap();
        let recipe = atom_cid(&raw).unwrap();
        let (_, loaded) = load_ontology(&root).unwrap();
        assert_eq!(loaded, recipe);
        println!("frame-ontology.json ontologyRef = {recipe}");
        let ValidatorOutcome::Classified { evidence, .. } =
            outcome(&root, SOVEREIGNTY, "We are self-sovereign.")
        else {
            panic!("the guard classifies");
        };
        assert_eq!(evidence["ontologyRef"], json!(recipe.to_string()));
        assert!(evidence["classificationCid"].is_string());
    }

    #[test]
    fn unreadable_atom_flags_never_passes() {
        let empty = TempDir::new().unwrap();
        for validator in [SOVEREIGNTY, OWNERSHIP] {
            // Even a write with no apex phrase: without the atom the guard cannot know that.
            for content in ["We are self-sovereign.", "Plain stewardship prose."] {
                assert!(
                    matches!(
                        outcome(empty.path(), validator, content),
                        ValidatorOutcome::Unavailable
                    ),
                    "{validator}: an unreadable atom must never pass"
                );
            }
        }
        // A malformed atom is unreadable too.
        let dir = TempDir::new().unwrap();
        let frames = dir.path().join(FRAMES_DIR);
        fs::create_dir_all(&frames).unwrap();
        fs::copy(
            repo_root().join(FRAMES_DIR).join(ONTOLOGY_FILE),
            frames.join(ONTOLOGY_FILE),
        )
        .unwrap();
        fs::write(
            frames.join("frame-sovereignty-apex.json"),
            "{\"id\": \"frame-x\"}",
        )
        .unwrap();
        assert!(matches!(
            outcome(dir.path(), SOVEREIGNTY, "We are self-sovereign."),
            ValidatorOutcome::Unavailable
        ));
    }
}
