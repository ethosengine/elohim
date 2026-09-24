//! The lifestream render — a person's own observations, arranged by the
//! observation-lifestream recipe (ruling R-A4 of the post-station-4 plan).
//!
//! [`render_stream`] is pure: rows in (already loaded, titles already looked
//! up), view out. It holds the whole of the arrangement so the route stays a
//! loader and the recipe stays the only place a threshold lives:
//!
//! - **audience `self`** — only rows whose `observer_cid` is the requester are
//!   rendered. The route already loads only those; the render refuses the rest
//!   again so the invariant does not depend on the caller.
//! - **lens** — a key of the recipe's lens table (`all` when absent); an
//!   unknown lens is an error, never a silent `all`. The lens keeps a kind and
//!   an optional `min_dwell_ms`; the request's `kind` narrows further.
//! - **window** — `Nd` or `Nh` ending at `asOf` (default: the recipe's
//!   `window_default`, ending now). Nothing else parses.
//! - **ranking** — the recipe's `recency` then `dwell_ms`: newest first, then
//!   longest dwell; the log offset breaks the last tie so the order is total.
//! - **omissions** — named lines for what the view cannot show or vouch for:
//!   `signature:` (rows are unsigned until the signing graduation), `window:`
//!   (how many selected rows fell outside the window) and `payload:` (rows
//!   whose payload did not parse, rendered with zero dwell and depth — never a
//!   500).
//!
//! `total_count` is how many rows the lens selected before the window.

use std::cmp::Reverse;
use std::fmt;

use serde::Deserialize;

use super::recipe::{self, LifestreamRecipe};
use crate::db::models::ObservationRow;
use crate::views::{ObservationStreamEntryView, ObservationStreamView, RecipeRefView};

/// The lens applied when a request names none: the recipe's keep-everything lens.
pub const DEFAULT_LENS: &str = "all";

/// The omission line for unsigned rows (the name the A4 view contract chose).
pub const SIGNATURE_ABSENT_OMISSION: &str =
    "signature: absent — observations are unsigned until the signing graduation";

/// The recipe a stream renders through, with its content address.
#[derive(Debug, Clone, Copy)]
pub struct RecipeInUse<'a> {
    pub spec: &'a LifestreamRecipe,
    pub cid: &'a str,
}

impl RecipeInUse<'static> {
    /// The recipe compiled into this binary.
    pub fn compiled() -> Self {
        Self {
            spec: recipe::recipe(),
            cid: recipe::recipe_cid(),
        }
    }
}

/// One loaded row and the subject's title, when the node knows it.
#[derive(Debug, Clone)]
pub struct StreamRow {
    pub observation: ObservationRow,
    pub title: Option<String>,
}

/// What the request asked for. `None` means "the recipe's default".
#[derive(Debug, Clone, Default)]
pub struct StreamParams<'a> {
    /// The requester — the only observer whose rows render.
    pub requester: &'a str,
    /// Unix epoch seconds the window ends at (default: now).
    pub as_of: Option<i64>,
    /// `Nd` | `Nh` (default: the recipe's `window_default`).
    pub window: Option<&'a str>,
    /// A key of the recipe's lens table (default: [`DEFAULT_LENS`]).
    pub lens: Option<&'a str>,
    /// Keep only this observation kind.
    pub kind: Option<&'a str>,
}

/// A request the recipe cannot render. Every variant is the caller's to fix (400).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    InvalidWindow(String),
    UnknownLens { lens: String, declared: Vec<String> },
    InvalidAsOf(i64),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::InvalidWindow(w) => write!(
                f,
                "window {w:?} is not Nd or Nh (N a positive whole number of days or hours)"
            ),
            StreamError::UnknownLens { lens, declared } => write!(
                f,
                "unknown lens {lens:?}: the recipe declares {}",
                declared.join(", ")
            ),
            StreamError::InvalidAsOf(t) => {
                write!(f, "asOf {t} is not unix epoch seconds at or after 0")
            }
        }
    }
}

impl std::error::Error for StreamError {}

/// Parse the window grammar `Nd | Nh` into seconds. `N` is a positive whole
/// number without a leading zero; anything else (weeks, fractions, signs,
/// overflow) is `None`.
pub fn parse_window(window: &str) -> Option<i64> {
    let (digits, unit) = window.split_at(window.len().checked_sub(1)?);
    let unit_secs: i64 = match unit {
        "d" => 86_400,
        "h" => 3_600,
        _ => return None,
    };
    if digits.is_empty() || digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok()?.checked_mul(unit_secs)
}

/// The fields of a content-viewed payload the stream reads.
#[derive(Deserialize)]
struct DwellPayload {
    #[serde(default)]
    dwell_ms: Option<u64>,
    #[serde(default)]
    scroll_depth_pct: Option<u64>,
}

/// `(dwell_ms, scroll_depth_pct, readable)`. A payload that is not a JSON
/// object with integer dwell/depth fields is unreadable: zeros. A missing
/// field reads as 0 (a kind that declares no dwell has none). Depth is
/// clamped to 100 — it is a percentage of the page.
fn read_payload(payload_json: &str) -> (u64, u8, bool) {
    let parsed = serde_json::from_str::<serde_json::Value>(payload_json)
        .ok()
        .filter(serde_json::Value::is_object)
        .and_then(|v| serde_json::from_value::<DwellPayload>(v).ok());
    match parsed {
        Some(p) => {
            let depth = p.scroll_depth_pct.unwrap_or(0).min(100) as u8;
            (p.dwell_ms.unwrap_or(0), depth, true)
        }
        None => (0, 0, false),
    }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Render the requester's lifestream through `recipe`. Pure: no I/O, no clock
/// (`now` is the default `asOf`).
pub fn render_stream(
    rows: &[StreamRow],
    recipe: &RecipeInUse<'_>,
    params: &StreamParams<'_>,
    now: i64,
) -> Result<ObservationStreamView, StreamError> {
    let window = params.window.unwrap_or(recipe.spec.window_default.as_str());
    let window_secs =
        parse_window(window).ok_or_else(|| StreamError::InvalidWindow(window.to_string()))?;

    let as_of = params.as_of.unwrap_or(now);
    if as_of < 0 {
        return Err(StreamError::InvalidAsOf(as_of));
    }
    let window_start = as_of.saturating_sub(window_secs);

    let lens_name = params.lens.unwrap_or(DEFAULT_LENS);
    let lens = recipe
        .spec
        .lenses
        .get(lens_name)
        .ok_or_else(|| StreamError::UnknownLens {
            lens: lens_name.to_string(),
            declared: recipe.spec.lenses.keys().cloned().collect(),
        })?;

    let mut total: u64 = 0;
    let mut older: u64 = 0;
    let mut later: u64 = 0;
    let mut unreadable: u64 = 0;
    let mut unsigned = false;
    let mut ranked: Vec<(ObservationStreamEntryView, i64)> = Vec::new();

    for row in rows {
        let o = &row.observation;
        // Audience `self`: another observer's row never renders.
        if o.observer_cid != params.requester {
            continue;
        }
        if lens
            .kind
            .as_deref()
            .is_some_and(|k| k != o.observation_kind)
        {
            continue;
        }
        if params.kind.is_some_and(|k| k != o.observation_kind) {
            continue;
        }
        let (dwell_ms, scroll_depth_pct, readable) = read_payload(&o.payload_json);
        if lens.min_dwell_ms.is_some_and(|min| dwell_ms < min) {
            continue;
        }
        total += 1;

        if o.observed_at < window_start {
            older += 1;
            continue;
        }
        if o.observed_at > as_of {
            later += 1;
            continue;
        }
        if !readable {
            unreadable += 1;
        }
        if o.signature_b64.is_empty() {
            unsigned = true;
        }
        ranked.push((
            ObservationStreamEntryView {
                observed_at: o.observed_at,
                kind: o.observation_kind.clone(),
                subject_cid: o.subject_cid.clone(),
                title: row.title.clone().filter(|t| !t.is_empty()),
                dwell_ms,
                scroll_depth_pct,
            },
            o.log_offset,
        ));
    }

    // Recency, then dwell, then the log's own order.
    ranked.sort_by_key(|(e, offset)| {
        (
            Reverse(e.observed_at),
            Reverse(e.dwell_ms),
            Reverse(*offset),
        )
    });

    let mut omissions = Vec::new();
    if unsigned {
        omissions.push(SIGNATURE_ABSENT_OMISSION.to_string());
    }
    if older > 0 {
        let verb = if older == 1 { "is" } else { "are" };
        omissions.push(format!(
            "window: {} {verb} outside {window}",
            plural(older, "older observation", "older observations")
        ));
    }
    if later > 0 {
        omissions.push(format!(
            "window: {} after asOf {} not shown",
            plural(later, "observation", "observations"),
            if later == 1 { "is" } else { "are" }
        ));
    }
    if unreadable > 0 {
        omissions.push(format!(
            "payload: {} did not parse; shown with zero dwell and depth",
            plural(unreadable, "observation", "observations")
        ));
    }

    Ok(ObservationStreamView {
        as_of,
        window: window.to_string(),
        recipe: RecipeRefView {
            name: recipe.spec.id.clone(),
            cid: recipe.cid.to_string(),
        },
        lens: lens_name.to_string(),
        entries: ranked.into_iter().map(|(e, _)| e).collect(),
        omissions,
        total_count: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(observer: &str, offset: i64, at: i64, kind: &str, payload: &str) -> StreamRow {
        StreamRow {
            observation: ObservationRow {
                observer_cid: observer.into(),
                log_cid: "blake3:00".into(),
                log_offset: offset,
                observed_at: at,
                seq: offset + 1,
                observation_kind: kind.into(),
                subject_cid: Some(format!("bafy-{observer}-{offset}")),
                subject_kind: Some("content".into()),
                payload_json: payload.into(),
                observer_household_cid: None,
                observer_collective_cid: None,
                observer_region: None,
                observer_archetype: None,
                observer_compute_class: None,
                signature_b64: String::new(),
            },
            title: None,
        }
    }

    fn params(requester: &str) -> StreamParams<'_> {
        StreamParams {
            requester,
            ..Default::default()
        }
    }

    #[test]
    fn window_grammar_is_days_or_hours_only() {
        assert_eq!(parse_window("7d"), Some(7 * 86_400));
        assert_eq!(parse_window("48h"), Some(48 * 3_600));
        assert_eq!(parse_window("1h"), Some(3_600));
        for bad in [
            "",
            "d",
            "h",
            "0d",
            "07d",
            "7w",
            "-3d",
            "+3d",
            "3.5d",
            "7 d",
            "7D",
            "7dd",
            "99999999999999999999d",
        ] {
            assert_eq!(parse_window(bad), None, "{bad:?}");
        }
        // Parses as i64 but overflows once scaled to seconds.
        assert_eq!(parse_window(&format!("{}d", i64::MAX / 10)), None);
    }

    #[test]
    fn render_refuses_rows_of_another_observer_even_when_handed_them() {
        let rows = vec![
            row(
                "jessica",
                0,
                100,
                "lamad:content-viewed",
                r#"{"dwell_ms":1}"#,
            ),
            row("james", 0, 100, "lamad:content-viewed", r#"{"dwell_ms":1}"#),
        ];
        let view = render_stream(&rows, &RecipeInUse::compiled(), &params("jessica"), 200).unwrap();
        assert_eq!(view.total_count, 1);
        assert_eq!(
            view.entries[0].subject_cid.as_deref(),
            Some("bafy-jessica-0")
        );
    }

    #[test]
    fn a_json_array_payload_is_unreadable_not_positional() {
        assert_eq!(read_payload("[5, 10]"), (0, 0, false));
        assert_eq!(read_payload("{}"), (0, 0, true));
        assert_eq!(
            read_payload(r#"{"dwell_ms":5,"scroll_depth_pct":250}"#),
            (5, 100, true)
        );
        assert_eq!(read_payload(r#"{"dwell_ms":-5}"#), (0, 0, false));
    }

    #[test]
    fn unknown_lens_names_the_declared_ones() {
        let mut p = params("jessica");
        p.lens = Some("nope");
        let err = render_stream(&[], &RecipeInUse::compiled(), &p, 0).unwrap_err();
        assert_eq!(
            err.to_string(),
            "unknown lens \"nope\": the recipe declares all, content, long-dwell"
        );
    }

    #[test]
    fn negative_as_of_is_refused() {
        let mut p = params("jessica");
        p.as_of = Some(-1);
        assert_eq!(
            render_stream(&[], &RecipeInUse::compiled(), &p, 0).unwrap_err(),
            StreamError::InvalidAsOf(-1)
        );
    }

    #[test]
    fn empty_stream_prints_provenance_and_no_omissions() {
        let view = render_stream(&[], &RecipeInUse::compiled(), &params("jessica"), 1_000).unwrap();
        assert_eq!(view.recipe.cid, recipe::recipe_cid());
        assert_eq!(view.window, "7d");
        assert_eq!(view.lens, "all");
        assert!(view.omissions.is_empty(), "{:?}", view.omissions);
    }
}
