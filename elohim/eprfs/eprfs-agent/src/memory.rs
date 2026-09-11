//! Local collective memory contracts. Authored references are local B-class inputs;
//! projections are reconstructible C-class context, never membership or acceptance proofs.
//! These specialize existing content/observation grammar, not core EPR kinds.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileRef {
    pub path: String,
    /// Exact raw file BlobCid, including any metadata; unlike a document body CID.
    pub cid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reach {
    Private,
    Workspace,
    Repository,
}

/// Local collective declaration reuses charter/steward concepts, without importing
/// Qahal's notarized Membership struct or inventing network CIDs/block heights.
/// Existing actor claims own mutable session attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collective {
    pub version: u32,
    pub id: String,
    pub display_name: String,
    pub charter: String,
    pub steward: String,
    pub participation: String,
    pub source_rules: Vec<SourceRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRule {
    pub path: String,
    pub max_reach: Reach,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub resource: FileRef,
    pub reach: Reach,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contribution {
    pub version: u32,
    pub collective: FileRef,
    pub author: String,
    pub steward: String,
    pub scope: String,
    pub reach: Reach,
    pub concern: String,
    pub claim: String,
    pub uncertainty: Vec<String>,
    pub sources: Vec<Source>,
    pub supersedes: Vec<FileRef>,
    pub contradicts: Vec<FileRef>,
    /// Present only when the claim was authored OUTSIDE the flow plane and imported verbatim.
    ///
    /// Additive and skipped when absent, so every contribution file written before this field
    /// existed still parses and still serializes to the same bytes — the projection receipts that
    /// embed a contribution are unchanged by its arrival.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported: Option<Imported>,
}

/// The provenance of an imported entry: who wrote the bytes, where they live, and how the entry
/// declared itself.
///
/// This is deliberately NOT a second identity. `Contribution::author` stays the acting
/// participant's registered agent claim (the collective refuses anything else), and
/// `Contribution::steward` stays the collective's steward. `git_author` records the human whose
/// commit the pinned bytes came from — provenance of a source, never a claim of standing, and
/// never a minted persona for a human who never asked for one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Imported {
    /// The entry's own display name — its `title:`, else its `name:`, else its file stem.
    pub display: String,
    /// The entry's file name, which is the link target a projected index renders.
    pub file: String,
    /// The entry's repository-relative path: the scope the claim was authored for.
    pub scope_path: String,
    /// `Name <email>` of the commit the pinned source bytes came from.
    pub git_author: String,
    /// The entry's declared `metadata.type`.
    pub entry_type: String,
    /// False when the entry declared `index: false` — it stays a contribution and stays out of
    /// the projected index, exactly as its author asked.
    pub indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectionRequest {
    pub version: u32,
    pub collective: FileRef,
    pub purpose: String,
    pub audience: Reach,
    pub inputs: Vec<FileRef>,
    pub omissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackKind {
    StaleSource,
    MisleadingProjection,
    OmittedContradiction,
    PoorSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feedback {
    pub version: u32,
    pub collective: FileRef,
    pub target: FileRef,
    pub kind: FeedbackKind,
    pub passage: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Graduation {
    pub version: u32,
    pub collective: FileRef,
    pub contribution: FileRef,
    /// Exact existing native approved verdict event, never a claimed boolean.
    pub review: String,
    pub audience: Reach,
}
