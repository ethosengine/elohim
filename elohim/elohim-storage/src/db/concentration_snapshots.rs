//! concentration_snapshots — insert + latest-effective lookup.
//! Writer-side k>=5 firewall lives in the SERVICE (concentration_service);
//! this layer is mechanical.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::concentration_snapshots as cs;
use super::models::{ConcentrationSnapshot, NewConcentrationSnapshot};

pub fn insert_snapshot(
    conn: &mut SqliteConnection,
    new: NewConcentrationSnapshot,
) -> QueryResult<ConcentrationSnapshot> {
    diesel::insert_into(cs::table)
        .values(&new)
        .execute(conn)?;
    cs::table
        .filter(cs::id.eq(&new.id))
        .first(conn)
}

/// Most recent snapshot for (h_app_id, substrate_signal, governance_layer) —
/// the effective C the decay path reads. None = never computed (callers fall
/// back to GradientConfig defaults with C treated as c_target → base-rate-only).
pub fn latest_snapshot(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    substrate_signal: &str,
    governance_layer: &str,
) -> QueryResult<Option<ConcentrationSnapshot>> {
    cs::table
        .filter(cs::h_app_id.eq(h_app_id))
        .filter(cs::substrate_signal.eq(substrate_signal))
        .filter(cs::governance_layer.eq(governance_layer))
        .order(cs::computed_at.desc())
        .first(conn)
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn insert_then_latest_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let row = NewConcentrationSnapshot {
            id: "attention:community:2026-06-10T03:00:00Z".into(),
            h_app_id: "shefa".into(),
            substrate_signal: "attention".into(),
            governance_layer: "community".into(),
            n: 6,
            mu: 100.0,
            ge: 0.2,
            ge_squashed: 0.1667,
            top_share: 0.3,
            gini: 0.25,
            c_composite: 0.22,
            alpha: 1.0,
            top_q: 0.01,
        };
        insert_snapshot(&mut conn, row).expect("insert");
        let got = latest_snapshot(&mut conn, "shefa", "attention", "community")
            .expect("query")
            .expect("row");
        assert_eq!(got.n, 6);
        assert!((got.c_composite - 0.22).abs() < 1e-6);
    }
}
