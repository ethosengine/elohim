//! Content search view types — the wire shape of `GET /db/content/search`.
//!
//! Schema (source of truth): `elohim/sdk/schemas/v1/views/content-search-view.schema.json`
//! (Category C: an assembled read projection over the storage content projection's derived
//! lexical fold). Plan: post-station-4 sprint, Lane S, ruling R-S4.
//!
//! The fold and its lag are answer envelopes (view rule 11): `present` with a value, `absent`
//! when there is no fold (an observed absence), `unreachable` when it could not be read. The
//! envelope is monomorphic on the wire — each enum below narrows `value` to this view's payload
//! and inherits `state`/`reason` from `../objects/answer.schema.json`. The reader an answer was
//! shaped for is never on the wire; the lens it resolved is.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One search answer: the recipe, lens and selection it ranked under, the fold it read, the
/// admitted candidates of this page, facets over the admitted set, and every omission named.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchView {
    pub query: String,
    /// True only when the candidates were ordered by the declared recipe over a present fold.
    pub ranking_known: bool,
    pub recipe: ContentSearchRecipeView,
    pub lens: ContentSearchLensView,
    /// The selection rule, one line.
    pub selection: String,
    pub fold: ContentSearchFoldAnswer,
    pub fold_lag: ContentSearchFoldLagAnswer,
    pub candidates: Vec<ContentSearchCandidateView>,
    pub facets: ContentSearchFacetsView,
    pub omissions: Vec<String>,
    pub unresolved: Vec<String>,
    #[ts(type = "number")]
    pub total_count: u64,
}

/// The declared fusion recipe the ranking ran under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchRecipeView {
    pub name: String,
    pub cid: String,
    #[ts(type = "number")]
    pub k: u64,
    pub order_only: bool,
    pub producers: Vec<ContentSearchProducerView>,
}

/// One producer of a ranking and the method (IndexMeasure CID) it ranks under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchProducerView {
    pub id: String,
    pub method: String,
}

/// The lens level the view resolved, and whether the reader asked for it (`requested`) or it
/// was the recipe's default (`defaulted`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchLensView {
    pub level: String,
    pub choice_count: u32,
    pub cid: String,
    pub provenance: String,
}

/// The fold the ranking read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchFoldView {
    /// The IndexMeasure CID the fold executed.
    pub measure: String,
    /// `complete`, `degraded` or `failed` — the latest attestation's state.
    pub state: String,
    pub attestation_cid: String,
    /// Unix epoch seconds.
    #[ts(type = "number")]
    pub at: i64,
}

/// How far the fold is behind the content rows, against the measure's declared bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchFoldLagView {
    #[ts(type = "number")]
    pub behind: u64,
    #[ts(type = "number")]
    pub limit: u64,
    pub unit: String,
    pub within: bool,
}

/// The one legal reason for an `absent` answer: absence was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub enum ContentSearchAbsentReason {
    ObservedAbsent,
}

/// Why no answer arrived (`../enums/answer-reason.schema.json`, the unreachable subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub enum ContentSearchUnreachableReason {
    Timeout,
    TransportError,
    Refused,
    Unverifiable,
    NotYetDelivered,
}

/// The fold, as an answer envelope narrowed to [`ContentSearchFoldView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub enum ContentSearchFoldAnswer {
    Present {
        value: ContentSearchFoldView,
    },
    Absent {
        reason: ContentSearchAbsentReason,
    },
    Unreachable {
        reason: ContentSearchUnreachableReason,
    },
}

/// The fold's lag, as an answer envelope narrowed to [`ContentSearchFoldLagView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub enum ContentSearchFoldLagAnswer {
    Present {
        value: ContentSearchFoldLagView,
    },
    Absent {
        reason: ContentSearchAbsentReason,
    },
    Unreachable {
        reason: ContentSearchUnreachableReason,
    },
}

/// One admitted candidate, with which producer ranked it under which method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchCandidateView {
    pub content_id: String,
    pub title: String,
    pub content_type: String,
    pub reach: String,
    /// The trust legibility label (`notarized` · `published` · `unconfirmed`); never a number.
    pub trust: String,
    pub tags: Vec<String>,
    /// The fused reciprocal-rank score, for ORDER only.
    pub score: f64,
    pub producer: String,
    pub method: String,
    /// `None` when the ranking producer located no section (one provenance).
    pub best_section: Option<ContentSearchSectionView>,
}

/// The section of a candidate a match landed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchSectionView {
    pub title: String,
    pub snippet: String,
}

/// Facet counts over the admitted candidates only.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct ContentSearchFacetsView {
    pub content_type: Vec<FacetCountView>,
    pub reach: Vec<FacetCountView>,
    pub tags: Vec<FacetCountView>,
}

/// One facet value and how many admitted candidates carry it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct FacetCountView {
    pub value: String,
    #[ts(type = "number")]
    pub count: u64,
}
