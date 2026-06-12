use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_fetcher::DataFetcher;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RenderSpec {
    Echo,
    AngularSsr,
}

impl RenderSpec {
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "echo" => Ok(Self::Echo),
            "angular-ssr" => Ok(Self::AngularSsr),
            other => Err(crate::RenderError::UnsupportedSpec(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RenderLimits {
    pub wall_time_ms: u64,
    pub memory_mb: u64,
    pub max_fetches: u32,
    pub max_output_bytes: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            wall_time_ms: 2_000,
            memory_mb: 128,
            max_fetches: 32,
            max_output_bytes: 2 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    pub id: String,
    pub content_hash: String,
}

pub struct RenderContext {
    pub spec: RenderSpec,
    pub url: String,
    pub data_fetcher: Arc<dyn DataFetcher>,
    pub limits: RenderLimits,
}

#[derive(Debug, Clone)]
pub struct RenderOutput {
    pub html: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub fetched_inputs: Vec<ContentRef>,
    /// Below-the-framework observability: the render lifecycle timeline and the
    /// terminal classification that distinguishes a healthy-empty render from a
    /// degenerate stall. Captured at the V8/DataFetcher boundary so every
    /// `Renderer` impl inherits it — framework-agnostic, server-side.
    pub trace: RenderTrace,
}

impl Default for RenderOutput {
    fn default() -> Self {
        Self {
            html: String::new(),
            status: 200,
            headers: Vec::new(),
            fetched_inputs: Vec::new(),
            trace: RenderTrace::default(),
        }
    }
}

// =============================================================================
// Render trace — the "dataplane eyes"
// =============================================================================

/// How a render ended. The cut a flat `fetched_inputs` list cannot express:
/// `RenderedEmpty` (a fetch truthfully returned nothing) vs `Stalled` (a fetch
/// never settled within budget — the "arrivedPayloads: []" degenerate).
///
/// Serialized kebab-case to match the `x-ssr-terminal` response-header value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RenderTerminal {
    /// At least one data fetch arrived with bytes; HTML carries content.
    #[default]
    Rendered,
    /// A fetch happened and every arrival was empty (0 rows / 404). Truthful empty.
    RenderedEmpty,
    /// A fetch never settled within the soft budget; the render gave up gracefully.
    Stalled,
    /// The hard wall-time limit fired and the render was killed.
    TimedOut,
    /// A fetch or the render itself threw (and nothing stalled).
    Errored,
}

/// The outcome of a single data fetch issued during a render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum FetchOutcome {
    /// The fetch settled. `empty` is true for a 0-row body or a 404 — a truthful
    /// "nothing here" that must not be confused with a stall.
    Arrived {
        status: u16,
        bytes: usize,
        duration_ms: u64,
        empty: bool,
    },
    /// The fetch never settled within the soft budget — the degenerate case.
    Stalled { waited_ms: u64 },
    /// The fetch returned an error.
    Errored { message: String, duration_ms: u64 },
}

/// One data fetch's breadcrumb in the render timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetchEvent {
    pub url: String,
    pub method: String,
    /// Milliseconds from render start when this fetch was issued.
    pub offset_ms: u64,
    pub outcome: FetchOutcome,
}

/// The render lifecycle timeline — the Comet `readableStateHistory` analog,
/// captured server-side below the framework.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderTrace {
    /// Correlation token threaded across browser ↔ doorway ↔ storage-peer
    /// (reuses the existing `X-Observation-Id` seam).
    pub trace_id: String,
    /// Per-fetch breadcrumbs, in issue order.
    pub fetches: Vec<FetchEvent>,
    /// How the render ended.
    pub terminal: RenderTerminal,
    /// Total wall time of the render in milliseconds.
    pub wall_ms: u64,
}

impl RenderTerminal {
    /// The kebab-case wire string — the `x-ssr-terminal` header value. Kept in
    /// lockstep with the serde representation (asserted in tests).
    pub fn as_str(&self) -> &'static str {
        match self {
            RenderTerminal::Rendered => "rendered",
            RenderTerminal::RenderedEmpty => "rendered-empty",
            RenderTerminal::Stalled => "stalled",
            RenderTerminal::TimedOut => "timed-out",
            RenderTerminal::Errored => "errored",
        }
    }
}

impl RenderTrace {
    /// Classify how a render ended from its fetch outcomes and whether the hard
    /// wall-time limit fired.
    ///
    /// Precedence reflects *how the render ended*, not merely what was observed:
    /// a hard timeout kill outranks a graceful soft-budget give-up, which
    /// outranks an error, which outranks the settled-data verdict.
    pub fn classify(fetches: &[FetchEvent], hard_timeout: bool) -> RenderTerminal {
        if hard_timeout {
            return RenderTerminal::TimedOut;
        }
        if fetches
            .iter()
            .any(|f| matches!(f.outcome, FetchOutcome::Stalled { .. }))
        {
            return RenderTerminal::Stalled;
        }
        if fetches
            .iter()
            .any(|f| matches!(f.outcome, FetchOutcome::Errored { .. }))
        {
            return RenderTerminal::Errored;
        }
        let mut any_nonempty = false;
        let mut any_empty = false;
        for f in fetches {
            if let FetchOutcome::Arrived { empty, .. } = f.outcome {
                if empty {
                    any_empty = true;
                } else {
                    any_nonempty = true;
                }
            }
        }
        if any_nonempty {
            RenderTerminal::Rendered
        } else if any_empty {
            RenderTerminal::RenderedEmpty
        } else {
            // No data fetches at all — a static render is just rendered.
            RenderTerminal::Rendered
        }
    }
}

#[cfg(test)]
mod trace_tests {
    use super::*;

    fn arrived(empty: bool) -> FetchEvent {
        FetchEvent {
            url: "/api/v1/x".into(),
            method: "GET".into(),
            offset_ms: 0,
            outcome: FetchOutcome::Arrived {
                status: if empty { 404 } else { 200 },
                bytes: if empty { 0 } else { 128 },
                duration_ms: 5,
                empty,
            },
        }
    }

    fn stalled() -> FetchEvent {
        FetchEvent {
            url: "/api/v1/slow".into(),
            method: "GET".into(),
            offset_ms: 0,
            outcome: FetchOutcome::Stalled { waited_ms: 1200 },
        }
    }

    fn errored() -> FetchEvent {
        FetchEvent {
            url: "/api/v1/boom".into(),
            method: "GET".into(),
            offset_ms: 0,
            outcome: FetchOutcome::Errored {
                message: "connection reset".into(),
                duration_ms: 3,
            },
        }
    }

    #[test]
    fn rendered_when_a_fetch_arrives_with_bytes() {
        assert_eq!(
            RenderTrace::classify(&[arrived(false)], false),
            RenderTerminal::Rendered
        );
    }

    #[test]
    fn rendered_empty_when_every_fetch_arrives_empty() {
        assert_eq!(
            RenderTrace::classify(&[arrived(true), arrived(true)], false),
            RenderTerminal::RenderedEmpty
        );
    }

    #[test]
    fn rendered_when_no_fetches_at_all() {
        // A static render with no data fetches is just rendered, not empty.
        assert_eq!(RenderTrace::classify(&[], false), RenderTerminal::Rendered);
    }

    #[test]
    fn stalled_outranks_a_healthy_arrival() {
        // The whole point: one outstanding fetch makes the render degenerate,
        // even when other data arrived fine.
        assert_eq!(
            RenderTrace::classify(&[arrived(false), stalled()], false),
            RenderTerminal::Stalled
        );
    }

    #[test]
    fn errored_when_a_fetch_throws_and_none_stall() {
        assert_eq!(
            RenderTrace::classify(&[arrived(false), errored()], false),
            RenderTerminal::Errored
        );
    }

    #[test]
    fn stalled_outranks_errored() {
        assert_eq!(
            RenderTrace::classify(&[errored(), stalled()], false),
            RenderTerminal::Stalled
        );
    }

    #[test]
    fn hard_timeout_outranks_everything() {
        assert_eq!(
            RenderTrace::classify(&[stalled(), errored()], true),
            RenderTerminal::TimedOut
        );
    }

    #[test]
    fn terminal_serializes_kebab_case_for_the_header() {
        // x-ssr-terminal header value must be the kebab-case form.
        let json = serde_json::to_string(&RenderTerminal::RenderedEmpty).unwrap();
        assert_eq!(json, "\"rendered-empty\"");
    }

    #[test]
    fn as_str_stays_in_lockstep_with_serde() {
        // Drift guard: the header accessor must match the serde wire form for
        // every variant, so a new terminal can't silently diverge.
        for t in [
            RenderTerminal::Rendered,
            RenderTerminal::RenderedEmpty,
            RenderTerminal::Stalled,
            RenderTerminal::TimedOut,
            RenderTerminal::Errored,
        ] {
            let serde_form = serde_json::to_string(&t).unwrap();
            assert_eq!(format!("\"{}\"", t.as_str()), serde_form);
        }
    }
}
