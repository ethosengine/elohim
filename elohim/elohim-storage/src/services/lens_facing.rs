//! Lens-market facing adapter (lens-market S5) — the service layer above the DAO.
//!
//! Mirrors `mishpat_commitment_facing.rs`: impure loaders (`&mut conn`) + a pure
//! per-row projection + a `build_*` orchestrator. The actual aggregation is the
//! DB-free `elohim_facings::folds::lens_affinity` fold — this adapter only loads
//! the DAO rows (the A-class `lenses` forward index + the C-class `lens_selections`
//! input) and maps them into the wire `LensBindingView`.
//!
//! ## What this produces
//! `build_lens_bindings(scope)` → the plural set of lenses governing an EPR, each
//! side-by-side with its peers (NO collapse — that is the whole point), affinity-
//! ranked (distinct attested selectors, gaming-resistant). A row whose `telos_json`
//! is unparseable degrades to `valid: false` — surfaced, never dropped (fail-closed
//! per row, the EprRouter lesson), and sorted to the bottom.
//!
//! ## Deferred (plan A6)
//! - `affinity_in_context` reads the `lens_selections` C-class table, which is
//!   DORMANT until the ballot/selection write-path lands — so affinity is 0 for all
//!   lenses today (a still-valid plural market: lenses surfaced, no ranking signal).
//! - `current_verdict` is the lens's OWN deterministic reading (its `rule` firing).
//!   The rule-evaluation engine is future work, so it is `None` in this slice.
//!
//! Plan: 2026-06-27-plural-mishpat-lenses-service-layer-plan.md (S5).

use diesel::sqlite::SqliteConnection;
use elohim_facings::folds::lens_affinity::{affinity_in_scope, LensSelectionRow};
use elohim_facings::folds::lens_contention::{contention_index, LensVerdictRow};
use elohim_facings::folds::lens_selector::{classify_regime, RegimeStatus};
use elohim_views::{LensBindingView, LensMarketView};

use crate::db::models::Lens;

/// Project one `lenses` DB row into its typed wire view, given its earned affinity.
///
/// `telos_summary` is parsed from `telos_json`'s `summary` field. An unparseable
/// `telos_json` degrades the row to `valid: false` (surfaced-but-excluded), never a
/// panic and never a dropped row.
fn to_binding_view(lens: Lens, affinity: u32) -> LensBindingView {
    let (telos_summary, valid) = match serde_json::from_str::<serde_json::Value>(&lens.telos_json) {
        Ok(v) => (
            v.get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            true,
        ),
        Err(_) => (String::new(), false),
    };
    LensBindingView {
        lens_cid: lens.cid,
        school: lens.school,
        role: lens.role,
        telos_summary,
        affinity_in_context: affinity,
        // The lens's own deterministic reading (rule firing) — rule-evaluation engine
        // is future work (plan A6); None until it fires here.
        current_verdict: None,
        valid,
    }
}

/// Load the affinity fold input (C-class selections) for a scope, mapped to the
/// DB-free fold row. A query error degrades to empty (warn-and-continue).
fn load_selection_rows(conn: &mut SqliteConnection, epr_scope: &str) -> Vec<LensSelectionRow> {
    match crate::db::lens_market::selections_in_scope(conn, epr_scope) {
        Ok(rows) => rows
            .into_iter()
            .map(|s| LensSelectionRow {
                lens_cid: s.lens_cid,
                selector_agent: s.selector_agent,
                epr_scope: s.epr_scope,
                selected_at: s.selected_at,
            })
            .collect(),
        Err(e) => {
            tracing::warn!("load_selection_rows: query failed: {e}");
            Vec::new()
        }
    }
}

/// Build the plural set of [`LensBindingView`]s governing an EPR, affinity-ranked.
///
/// Reads the A-class forward index (`find_lenses_governing_epr`, fail-closed to
/// notarized + non-revoked) and the C-class selection input, runs the affinity
/// fold, and projects each lens. Ordering is deterministic: valid rows first, then
/// affinity descending, then `lens_cid` ascending (tie-break). An empty / unknown
/// scope yields an empty list (never an error).
pub fn build_lens_bindings(conn: &mut SqliteConnection, epr_scope: &str) -> Vec<LensBindingView> {
    let lenses = match crate::db::lenses::find_lenses_governing_epr(conn, epr_scope) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("build_lens_bindings: lens query failed: {e}");
            Vec::new()
        }
    };
    let selection_rows = load_selection_rows(conn, epr_scope);
    let affinity_map = affinity_in_scope(&selection_rows, epr_scope);

    let mut views: Vec<LensBindingView> = lenses
        .into_iter()
        .map(|lens| {
            let affinity = affinity_map.get(&lens.cid).copied().unwrap_or(0) as u32;
            to_binding_view(lens, affinity)
        })
        .collect();

    // Deterministic plural ordering: valid first (degraded rows sink, still surfaced),
    // then affinity desc, then lens_cid asc.
    views.sort_by(|a, b| {
        b.valid
            .cmp(&a.valid)
            .then(b.affinity_in_context.cmp(&a.affinity_in_context))
            .then(a.lens_cid.cmp(&b.lens_cid))
    });
    views
}

/// Load the contention fold input (C-class verdicts) for a scope, mapped to the
/// DB-free fold row. A query error degrades to empty (warn-and-continue).
fn load_verdict_rows(conn: &mut SqliteConnection, epr_scope: &str) -> Vec<LensVerdictRow> {
    match crate::db::lens_market::verdicts_in_scope(conn, epr_scope) {
        Ok(rows) => rows
            .into_iter()
            .map(|v| LensVerdictRow {
                epr_scope: v.epr_scope,
                lens_cid: v.lens_cid,
                verdict: v.verdict,
                agent: v.agent,
            })
            .collect(),
        Err(e) => {
            tracing::warn!("load_verdict_rows: query failed: {e}");
            Vec::new()
        }
    }
}

/// Map the regime-drift verdict to its wire string (schema: `stable|drifting|breached`).
fn regime_label(status: RegimeStatus) -> String {
    match status {
        RegimeStatus::Stable => "stable",
        RegimeStatus::Drifting => "drifting",
        RegimeStatus::Breached => "breached",
    }
    .to_string()
}

/// Assemble the full [`LensMarketView`] over one EPR — the composite the route serves.
///
/// Composes the S5 plural bindings with the market-level contention (controversy
/// spread over verdicts) and the regime-drift status. `computed_at` is INJECTED by
/// the caller (the handler) — never `Utc::now()` inside the fold layer, so the
/// assembly is deterministic and testable.
///
/// ## This-slice honesty (plan A6)
/// - `regime_status` is `classify_regime` over (now == prev) ⇒ `stable`: drift
///   detection needs a PRIOR temporal snapshot to compare against, which is the
///   deferred snapshot-history leg. The fusion primitive is wired and ready; it
///   simply has no history to fire on yet.
/// - `open_bounty_cid` is always `None` until the bounty write-path lands.
/// - An unknown / empty scope yields an empty-but-VALID market (lenses `[]`,
///   contention `0.0`, regime `stable`), never an error.
pub fn build_lens_market_view(
    conn: &mut SqliteConnection,
    epr_scope: &str,
    computed_at: String,
) -> LensMarketView {
    let lenses = build_lens_bindings(conn, epr_scope);
    let verdict_rows = load_verdict_rows(conn, epr_scope);
    let contention = contention_index(&verdict_rows, epr_scope);

    // Regime-drift is the joint affinity-decay ∧ contention-rise fusion (spec §8).
    // No prior temporal snapshot exists in this slice (deferred), so now == prev ⇒
    // Stable. The market-level affinity signal is the total earned affinity.
    let total_affinity: usize = lenses.iter().map(|l| l.affinity_in_context as usize).sum();
    let regime = classify_regime(total_affinity, total_affinity, contention, contention);

    LensMarketView {
        epr_scope: epr_scope.to_string(),
        lenses,
        contention_index: contention,
        regime_status: regime_label(regime),
        open_bounty_cid: None,
        computed_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{NewLens, NewLensSelection};
    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn seed_lens(conn: &mut SqliteConnection, cid: &str, school: &str, telos_json: &str) {
        crate::db::lenses::upsert_with_anchor(
            conn,
            NewLens {
                cid: cid.to_string(),
                governs_epr: "epr:lamad-spa".to_string(),
                school: school.to_string(),
                role: "lens".to_string(),
                rule_json: r#"{"predicate":"x"}"#.to_string(),
                telos_json: telos_json.to_string(),
                version_parent: None,
                revoked_at: None,
                dht_anchor_hash: Some(format!("anchor-{cid}")),
            },
        )
        .expect("seed lens");
    }

    fn seed_selection(conn: &mut SqliteConnection, lens_cid: &str, agent: &str) {
        crate::db::lens_market::insert_selection(
            conn,
            NewLensSelection {
                id: format!("{lens_cid}:epr:lamad-spa:{agent}"),
                lens_cid: lens_cid.to_string(),
                selector_agent: agent.to_string(),
                epr_scope: "epr:lamad-spa".to_string(),
                selected_at: "2026-06-27T00:00:00Z".to_string(),
            },
        )
        .expect("seed selection");
    }

    /// S5a — two lenses both surface, side by side (NO collapse), affinity-ranked.
    #[test]
    fn two_lenses_surface_plural_affinity_ranked() {
        let mut conn = test_conn();
        seed_lens(
            &mut conn,
            "lens:georgist",
            "georgist",
            r#"{"summary":"tax land value"}"#,
        );
        seed_lens(
            &mut conn,
            "lens:beerian",
            "beerian",
            r#"{"summary":"keep the system viable"}"#,
        );
        // georgist earns 2 distinct selectors; beerian 1.
        seed_selection(&mut conn, "lens:georgist", "agentA");
        seed_selection(&mut conn, "lens:georgist", "agentB");
        seed_selection(&mut conn, "lens:beerian", "agentA");

        let views = build_lens_bindings(&mut conn, "epr:lamad-spa");
        assert_eq!(views.len(), 2, "both lenses surface — no collapse");
        // georgist ranks first (affinity 2 > 1).
        assert_eq!(views[0].school, "georgist");
        assert_eq!(views[0].affinity_in_context, 2);
        assert_eq!(views[0].telos_summary, "tax land value");
        assert!(views[0].valid);
        assert!(views[0].current_verdict.is_none());
        assert_eq!(views[1].school, "beerian");
        assert_eq!(views[1].affinity_in_context, 1);
    }

    /// S5b — distinct-selector affinity (gaming-resistant): the same agent selecting
    /// twice counts once.
    #[test]
    fn affinity_counts_distinct_selectors() {
        let mut conn = test_conn();
        seed_lens(&mut conn, "lens:x", "georgist", r#"{"summary":"s"}"#);
        seed_selection(&mut conn, "lens:x", "agentA");
        seed_selection(&mut conn, "lens:x", "agentA"); // same agent → idempotent id → counts once

        let views = build_lens_bindings(&mut conn, "epr:lamad-spa");
        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].affinity_in_context, 1,
            "distinct selectors only — gaming-resistant"
        );
    }

    /// S5c — a poisoned lens (unparseable telos_json) degrades to valid:false,
    /// surfaced (not dropped) and sorted to the bottom.
    #[test]
    fn poisoned_lens_degrades_not_dropped() {
        let mut conn = test_conn();
        seed_lens(&mut conn, "lens:good", "georgist", r#"{"summary":"ok"}"#);
        seed_lens(&mut conn, "lens:bad", "beerian", "this is not json");

        let views = build_lens_bindings(&mut conn, "epr:lamad-spa");
        assert_eq!(views.len(), 2, "degraded row surfaced, not dropped");
        // good row first (valid), bad row last (degraded).
        assert!(views[0].valid);
        assert_eq!(views[0].lens_cid, "lens:good");
        assert!(!views[1].valid, "poisoned telos_json → valid:false");
        assert_eq!(views[1].lens_cid, "lens:bad");
        assert_eq!(views[1].telos_summary, "");
    }

    /// S5d — unknown scope → empty (never an error).
    #[test]
    fn unknown_scope_is_empty() {
        let mut conn = test_conn();
        assert!(build_lens_bindings(&mut conn, "epr:nope").is_empty());
    }

    fn seed_verdict(conn: &mut SqliteConnection, lens_cid: &str, agent: &str, verdict: &str) {
        crate::db::lens_market::insert_verdict(
            conn,
            crate::db::models::NewLensVerdict {
                id: format!("epr:lamad-spa:{lens_cid}:{agent}"),
                epr_scope: "epr:lamad-spa".to_string(),
                lens_cid: lens_cid.to_string(),
                verdict: verdict.to_string(),
                agent: agent.to_string(),
                created_at: "2026-06-27T00:00:00Z".to_string(),
            },
        )
        .expect("seed verdict");
    }

    /// S6a — the composite assembler builds a full LensMarketView: plural bindings +
    /// market contention + regime + (deferred) bounty None + injected computed_at.
    #[test]
    fn market_view_assembles_full() {
        let mut conn = test_conn();
        seed_lens(
            &mut conn,
            "lens:georgist",
            "georgist",
            r#"{"summary":"tax land"}"#,
        );
        seed_lens(
            &mut conn,
            "lens:beerian",
            "beerian",
            r#"{"summary":"viability"}"#,
        );
        // A perfectly-split verdict set → maximal controversy (contention_index 1.0).
        seed_verdict(&mut conn, "lens:georgist", "agentA", "agree");
        seed_verdict(&mut conn, "lens:georgist", "agentB", "disagree");

        let view =
            build_lens_market_view(&mut conn, "epr:lamad-spa", "2026-06-27T12:00:00Z".into());

        assert_eq!(view.epr_scope, "epr:lamad-spa");
        assert_eq!(view.lenses.len(), 2, "both lenses surface in the market");
        assert!(
            (view.contention_index - 1.0).abs() < f64::EPSILON,
            "1 agree + 1 disagree = perfectly split = 1.0, got {}",
            view.contention_index
        );
        // No prior snapshot → no detectable drift → stable.
        assert_eq!(view.regime_status, "stable");
        assert!(view.open_bounty_cid.is_none(), "bounty write-path deferred");
        assert_eq!(
            view.computed_at, "2026-06-27T12:00:00Z",
            "caller-injected time"
        );
    }

    /// S6b — unknown scope yields an empty-but-VALID market, never an error.
    #[test]
    fn market_view_unknown_scope_is_empty_valid() {
        let mut conn = test_conn();
        let view = build_lens_market_view(&mut conn, "epr:nope", "2026-06-27T12:00:00Z".into());
        assert!(view.lenses.is_empty());
        assert_eq!(view.contention_index, 0.0);
        assert_eq!(view.regime_status, "stable");
        assert!(view.open_bounty_cid.is_none());
        assert_eq!(view.epr_scope, "epr:nope");
    }
}
