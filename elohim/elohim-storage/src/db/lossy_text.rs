//! Resilient SQLite `TEXT` decoding — one corrupt row must not fail the batch.
//!
//! # Why this exists
//!
//! diesel 2.3.5 decoded `String` from a SQLite `TEXT` column via
//! `SqliteValue::read_text()` + `str::from_utf8_unchecked` — unsound, but
//! *tolerant*: a column holding non-UTF-8 bytes still produced a value.
//! diesel 2.3.11 fixed the soundness hole by routing
//! `String: FromSql<Text, Sqlite>` through
//! `<&str as FromSqlRef>::from_sql` → `SqliteValue::as_utf8_str()`, which is a
//! strict `str::from_utf8`. That is the correct fix for the unsoundness, but it
//! converts a per-value oddity into a **whole-batch failure**: a single
//! non-UTF-8 byte in one row makes the entire `.load()` return `Err`, so one bad
//! row takes down a whole list query.
//!
//! This repo has already paid for that failure class once — a single poisoned
//! scope row emptied the entire `EprRouter` (Welcome at `/`, 404 on `/lamad`),
//! cured by degrading per-row instead of failing closed. The rule that came out
//! of it: **a projection that feeds a read path degrades per-row; it never
//! fail-closes the batch.**
//!
//! # What this provides
//!
//! [`LossyText`] / [`LossyTextOpt`] are `deserialize_as` shims that keep the
//! *tolerance* of 2.3.5 without its unsoundness. They decode through the
//! still-public, still-lossy [`SqliteValue::read_text`], which routes to
//! `String::from_utf8_lossy` — invalid bytes become U+FFFD REPLACEMENT
//! CHARACTER, the row survives, and the batch keeps its other rows.
//!
//! Use them on a model field without changing the field's type:
//!
//! ```ignore
//! #[derive(Queryable, Selectable)]
//! #[diesel(table_name = economic_events)]
//! pub struct EconomicEvent {
//!     // …
//!     #[diesel(deserialize_as = LossyTextOpt)]
//!     pub note: Option<String>,
//! }
//! ```
//!
//! # Where to apply it (and where not to)
//!
//! Every `TEXT` column in this crate is written from a Rust `String`/`&str`,
//! which is UTF-8 by construction — so no in-crate write path can produce a
//! non-UTF-8 `TEXT` value, and blanket-wrapping ~800 string fields across ~97
//! model structs would be churn without a matching threat. Reach for these
//! types on columns whose *contract* is "opaque text the substrate does not
//! constrain" — free-form human notes, opaque JSON payloads projected from
//! peers — where a byte-level surprise is semantically possible rather than
//! merely a symptom of storage corruption. Columns holding CIDs, hashes,
//! ISO-8601 timestamps, agent ids, or enum-vocabulary tokens are constrained by
//! their writers and do not need wrapping.
//!
//! Note the residual gap these types do **not** close: they make *`TEXT`* decode
//! fail-soft, not every column class. A corrupt `INTEGER`/`REAL`/`Binary` value,
//! or a `NULL` in a non-`Option` field, still fails the whole `.load()`. Closing
//! that requires per-row loading (`load_iter()` + `filter_map` + `warn!`) at the
//! batch-load call sites, which is a separate, larger change.

use diesel::deserialize::{self, FromSql, Queryable};
use diesel::sql_types::{Nullable, Text};
use diesel::sqlite::{Sqlite, SqliteValue};

/// Lossy-decoding shim for a non-nullable `TEXT` column.
///
/// Invalid UTF-8 bytes decode to U+FFFD rather than failing the batch load.
/// Use via `#[diesel(deserialize_as = LossyText)]` on a `String` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossyText(pub String);

impl From<LossyText> for String {
    fn from(value: LossyText) -> Self {
        value.0
    }
}

impl FromSql<Text, Sqlite> for LossyText {
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        // `read_text()` is diesel's lossy reader (`String::from_utf8_lossy`
        // under the hood) — unlike `String`'s strict `as_utf8_str()` path.
        Ok(Self(value.read_text().to_owned()))
    }
}

// diesel's `FromSqlRow` blanket impl routes through `Queryable` (not `FromSql`
// directly), so a `deserialize_as` target must implement it. `Row = Self`
// delegates the actual decode to the `FromSql` impl above via
// `FromStaticSqlRow` — the same shape diesel itself uses for `*const str`.
impl Queryable<Text, Sqlite> for LossyText {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

/// Lossy-decoding shim for a `Nullable<Text>` column.
///
/// `NULL` decodes to `None`; invalid UTF-8 bytes decode to U+FFFD rather than
/// failing the batch load. Use via `#[diesel(deserialize_as = LossyTextOpt)]`
/// on an `Option<String>` field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LossyTextOpt(pub Option<String>);

impl From<LossyTextOpt> for Option<String> {
    fn from(value: LossyTextOpt) -> Self {
        value.0
    }
}

impl FromSql<Nullable<Text>, Sqlite> for LossyTextOpt {
    fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(Self(Some(value.read_text().to_owned())))
    }

    /// Overridden because the default `from_nullable_sql` errors on `NULL`
    /// (`UnexpectedNullError`); this type owns the nullability itself.
    fn from_nullable_sql(value: Option<SqliteValue<'_, '_, '_>>) -> deserialize::Result<Self> {
        match value {
            Some(value) => <Self as FromSql<Nullable<Text>, Sqlite>>::from_sql(value),
            None => Ok(Self(None)),
        }
    }
}

impl Queryable<Nullable<Text>, Sqlite> for LossyTextOpt {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel::sql_types::Nullable as SqlNullable;
    use diesel::sql_types::Text as SqlText;

    /// A valid-UTF-8 `TEXT` value round-trips unchanged through the shim.
    #[test]
    fn lossy_text_passes_valid_utf8_through_unchanged() {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory sqlite");
        let value = diesel::select(diesel::dsl::sql::<SqlText>("'héllo ✓'"))
            .get_result::<LossyText>(&mut conn)
            .expect("valid utf-8 must decode");
        assert_eq!(value.0, "héllo ✓");
    }

    /// The regression this module exists for: a non-UTF-8 BLOB in a
    /// TEXT-affinity position decodes lossily instead of erroring.
    #[test]
    fn lossy_text_replaces_invalid_utf8_instead_of_erroring() {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory sqlite");

        // Strict `String` decode is expected to FAIL on these bytes — this is
        // the diesel 2.3.11 behaviour that broke the batch load.
        let strict =
            diesel::select(diesel::dsl::sql::<SqlText>("x'ff'")).get_result::<String>(&mut conn);
        assert!(
            strict.is_err(),
            "guard: strict String decode must still reject invalid utf-8, \
             otherwise this shim is no longer needed"
        );

        // The shim degrades to U+FFFD and keeps the value.
        let lossy = diesel::select(diesel::dsl::sql::<SqlText>("x'ff'"))
            .get_result::<LossyText>(&mut conn)
            .expect("lossy decode must succeed on invalid utf-8");
        assert_eq!(lossy.0, "\u{fffd}");
    }

    /// `NULL` must decode to `None`, not `UnexpectedNullError`.
    #[test]
    fn lossy_text_opt_decodes_null_as_none() {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory sqlite");
        let value = diesel::select(diesel::dsl::sql::<SqlNullable<SqlText>>("NULL"))
            .get_result::<LossyTextOpt>(&mut conn)
            .expect("NULL must decode to None");
        assert_eq!(value.0, None);
    }

    /// A present nullable value decodes lossily, same as the non-null shim.
    #[test]
    fn lossy_text_opt_replaces_invalid_utf8_instead_of_erroring() {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory sqlite");
        let value = diesel::select(diesel::dsl::sql::<SqlNullable<SqlText>>("x'ff'"))
            .get_result::<LossyTextOpt>(&mut conn)
            .expect("lossy decode must succeed on invalid utf-8");
        assert_eq!(value.0, Some("\u{fffd}".to_string()));
    }

    /// The `From` conversions the `deserialize_as` shim relies on.
    #[test]
    fn conversions_into_field_types_are_transparent() {
        assert_eq!(String::from(LossyText("x".to_string())), "x");
        assert_eq!(
            Option::<String>::from(LossyTextOpt(Some("y".to_string()))),
            Some("y".to_string())
        );
        assert_eq!(Option::<String>::from(LossyTextOpt(None)), None);
    }
}
