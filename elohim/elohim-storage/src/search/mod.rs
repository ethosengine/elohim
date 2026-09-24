//! Content search at the peer (post-station-4 sprint, Lane S; serves `recall-reaches-authority`).
//!
//! The peer folds its own content projection into a lexical index and answers ranked searches
//! from it. Everything here is Category C — a derived projection, rebuilt on corruption, never a
//! migration, never synced — over the content rows (themselves projections of notarized content):
//!
//! - [`measure`] — the governed declarations the search runs under: the `content-lexical-index`
//!   `IndexMeasure` and the `rrf-v2` content search recipe, compiled in and checked at boot;
//! - [`fold`] — [`fold::SearchIndex`], the incremental bm25 fold of the content rows into
//!   `<storage_dir>/index/<measure-cid>/fold.sqlite`, with its attestation beside it, run at
//!   boot, on [`fold::SearchIndex::notify`] from the content service, and on a timer;
//! - [`reader`] — who an answer is shaped for (the reach gate's input), never on the wire;
//! - [`query`] — `GET /db/content/search`: the query, and [`query::answer`], one ranked,
//!   reach-gated `ContentSearchView` over the fold.
pub mod fold;
pub mod measure;
pub mod query;
pub mod reader;

pub use fold::{FoldReport, FoldSnapshot, SearchIndex};
pub use measure::Declared;
