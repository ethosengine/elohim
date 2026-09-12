//! The question bank — a fixed set of `elohim_epr_rea::Intent`s IN SCOPE OF the recall recipe
//! itself (station 3, task 3.1).
//!
//! Every question the bank declares is a `consume` intent whose `in_scope_of` is the CID of the
//! contract's own bytes (`Contract::method_cid()`): the bank does not describe some other
//! process, it describes a reader's use of THIS one. The bank lives in a sibling file
//! (`.epr-meta/elohim/algorithms/recall-questions.json`), named by the contract's own
//! `question_bank` pointer rather than embedded inline, so the contract stays the recipe and the
//! bank stays its own versioned artifact — the same split `lens_table` chose not to take (that
//! table IS embedded) because a lens level is small, stable data the contract already owns, while
//! a question bank is content that grows and is authored/reviewed on its own terms.
//!
//! **Wire shape vs. real shape.** `elohim_epr_rea::Intent` and its `in_scope_of: Cid` field derive
//! `Serialize`/`Deserialize` from the `cid` crate's own impl, which — deliberately, per that
//! crate's own doc comment — refuses to read a CID back from a bare JSON string; it round-trips
//! only through its own byte-array newtype envelope. A hand-authored question bank naming its
//! recipe as `"bafkrei…"` (the only sane way to author it) would therefore fail to deserialize
//! directly into `Intent`. This module deserializes the human-authored wire shape (plain strings
//! for `action`, `in_scope_of`, `raised_by`) into a `RawQuestion` first, parses each CID with
//! [`cid::Cid`]'s `FromStr` (the same route `render.rs` already uses), and only then constructs
//! the real `Intent`. `ReaVerb`, `ResourceSpec` and `AgentRef` all deserialize from their natural
//! JSON forms directly (see each type's own `#[serde]` attributes), so only the CID needs this
//! detour.

use super::*;
use cid::Cid;
use elohim_epr::witness::ReaVerb;
use elohim_epr_rea::{AgentRef, Intent, ResourceSpec};
use serde::Deserialize;

/// One question the bank declares: a VF `Intent` to consume some evidence, plus WHERE in the
/// repository the answer is authoritative and WHAT sentence there counts as having reached it.
#[derive(Debug, Clone)]
pub struct Question {
    pub id: String,
    pub intent: Intent,
    pub reached_when: ReachedWhen,
    /// The directory this question's evidence lives under — a `contained()`-style scope hint for
    /// a future consumer, not itself validated against `source_roots` here (a whole-tree question
    /// legitimately declares `"."`, which is not, and should not be, a `source_roots` member —
    /// see `focus_area`'s own `scope != "."` special-case in `discovery.rs`).
    pub scope: String,
}

/// The authoritative file and sentence that answer a question.
#[derive(Debug, Clone)]
pub struct ReachedWhen {
    pub path: PathBuf,
    pub assertion: String,
    pub terms: Vec<String>,
}

/// The human-authorable wire shape one question is read from.
#[derive(Debug, Deserialize)]
struct RawQuestion {
    id: String,
    intent: RawIntent,
    reached_when: RawReachedWhen,
    scope: String,
}

/// `action`, `resource_spec` and `raised_by` deserialize directly into their real types —
/// `ReaVerb` is `#[serde(rename_all = "lowercase")]`, `ResourceSpec` is a plain `classified_as` +
/// `quantity` struct, and `AgentRef`'s derived newtype-struct impl forwards transparently to its
/// inner `String`. Only `in_scope_of` needs the string detour this module's doc explains.
#[derive(Debug, Deserialize)]
struct RawIntent {
    action: ReaVerb,
    resource_spec: ResourceSpec,
    in_scope_of: String,
    raised_by: AgentRef,
}

#[derive(Debug, Deserialize)]
struct RawReachedWhen {
    path: PathBuf,
    assertion: String,
    #[serde(default)]
    terms: Vec<String>,
}

/// The bank file's own top-level shape: a declared `recipe` CID (checked against the contract
/// that named this file) and the question array.
#[derive(Debug, Deserialize)]
struct RawBank {
    #[allow(dead_code)]
    version: u32,
    recipe: String,
    questions: Vec<RawQuestion>,
}

/// Parse one question bank file's raw bytes into real `Question`s, refusing — never silently
/// dropping a row or falling back to an empty bank — on any shape the contract's own pointer did
/// not actually produce: bad JSON, a CID that doesn't parse, or a `recipe`/`in_scope_of` that does
/// not match the pinned `recipe_cid` (the contract this bank claims to be in scope of).
pub(super) fn parse_bank(raw: &[u8], path: &Path, recipe_cid: &str) -> FlowResult<Vec<Question>> {
    let bank: RawBank = serde_json::from_slice(raw).map_err(|e| {
        refused(format!(
            "recall contract `question_bank` names {}, which is declared but malformed: {e}",
            path.display()
        ))
    })?;
    if bank.recipe != recipe_cid {
        return Err(refused(format!(
            "recall contract `question_bank` names {}, which is declared but malformed: its recipe {} does not match the contract's own method cid {recipe_cid}",
            path.display(),
            bank.recipe
        )));
    }
    let mut questions = Vec::with_capacity(bank.questions.len());
    for raw_question in bank.questions {
        let in_scope_of: Cid = raw_question.intent.in_scope_of.parse().map_err(|e| {
            refused(format!(
                "recall contract `question_bank` names {}, which is declared but malformed: question `{}` has an invalid in_scope_of cid: {e}",
                path.display(),
                raw_question.id
            ))
        })?;
        if in_scope_of.to_string() != recipe_cid {
            return Err(refused(format!(
                "recall contract `question_bank` names {}, which is declared but malformed: question `{}`'s in_scope_of does not match the contract's own method cid",
                path.display(),
                raw_question.id
            )));
        }
        questions.push(Question {
            id: raw_question.id,
            intent: Intent {
                action: raw_question.intent.action,
                resource_spec: raw_question.intent.resource_spec,
                in_scope_of,
                raised_by: raw_question.intent.raised_by,
            },
            reached_when: ReachedWhen {
                path: raw_question.reached_when.path,
                assertion: raw_question.reached_when.assertion,
                terms: raw_question.reached_when.terms,
            },
            scope: raw_question.scope,
        });
    }
    Ok(questions)
}
