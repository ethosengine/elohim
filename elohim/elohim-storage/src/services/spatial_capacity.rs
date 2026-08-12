//! Spatial Carrying Capacity Enforcement Service
//!
//! Reads Place carrying capacity and enforces limits on resource allocation.
//! The chain: Resource → SpatialContext → Place → CarryingCapacity → enforcement.
//!
//! Planet-scale limits aggregate up the H3 hierarchy:
//! parcel → community → bioregion → global.
//!
//! Source of truth: Place.carrying_capacity_json (DHT-projected, Category A)
//!
//! ## The dimensional contract (why usage is windowed)
//!
//! `max_sustainable_yield` is a **rate** by its own name — a sustainable yield is
//! per period. Utilization is therefore `harvest ÷ regeneration`, and that is only
//! a ratio when BOTH sides are flows of the same tempo
//! ([`MeasureKind::divide`](elohim_epr::measure::MeasureKind::divide)).
//!
//! This module used to sum consumption over ALL TIME and divide that cumulative
//! **level** by the yield **rate**. Under the measure algebra that quotient is a
//! *time* (Meadows' coverage time), not a utilization — so the `<= 1.0` allow gate
//! and the `> 0.8` governance trigger were both reading a number with no such
//! interpretation. Worse, it was monotonically non-decreasing: any Place with
//! history eventually read >100% and refused every allocation forever, with no
//! reachable recovery state. When the yield IS a rate, usage is now summed over
//! exactly one period of the yield's own denominator, which makes the comparison
//! Rate ÷ Rate = Ratio.
//!
//! ## Not every capacity is a rate
//!
//! `unit` is the ONLY signal of what `max_sustainable_yield` is denominated in, and
//! plenty of capacities are levels, not flows: `"dwellings"`, `"hectares"`,
//! `"people"` name a ceiling on a stock. Windowing one of those would silently drop
//! every occupancy older than the window and permit over-allocation — the same class
//! of fail-open this module was fixed for, pointed the other way. So a unit with **no
//! time denominator is read as a LEVEL and is not windowed**; the comparison is
//! Level ÷ Level = Ratio, which the algebra also admits.
//!
//! A `/` alone does not make a declaration a flow. `"people/km2"`, `"kg/ha"` and
//! `"m3/km2"` are **densities**: a level per an EXTENT, not per a tempo. Nothing about
//! a density expires, so it is read as a level and is not windowed either. What
//! decides is what the denominator NAMES — a tempo makes a rate, an extent keeps a
//! level, and a denominator that is neither (`"liters/fortnight"`, a bare `"m"`) is
//! REFUSED rather than guessed at in either direction. See [`yield_denominator`].
//!
//! ## Timestamps are compared as instants, never as strings — and SQL does not decide
//!
//! `has_point_in_time` is projected verbatim from `payload.hasPointInTime`
//! (`projector/mapping.rs`) and its schema says only "ISO 8601 timestamp", so the
//! column is a free-text field in practice. Two distinct hazards live there, and only
//! one of them is about zone offsets:
//!
//! - **Offset.** `2026-08-11T05:00:00-07:00` is 12:00Z, but it sorts *lexically* below
//!   a cutoff of `2026-08-11T12:00:00`.
//! - **Format.** A stamp need not be an ISO rendering at all. Epoch seconds
//!   (`"1754899200"`), an empty string, or any other producer's idea of a timestamp
//!   has no lexical relationship to an ISO cutoff whatsoever.
//!
//! A string compare mishandles both, and in the same direction: it drops a row,
//! under-reads utilization, and allows an allocation that should be refused. **There
//! is therefore NO SQL bound on the timestamp.** Every candidate row is loaded and the
//! window decision is made in Rust after parsing each stamp to an instant
//! ([`UsageWindow::admits`]). Anything unparseable is COUNTED, not dropped: on a limit
//! gate, over-counting refuses and under-counting allows.
//!
//! An earlier revision kept a deliberately-slack SQL prefilter for index locality. It
//! was removed on correctness grounds: it handled the offset hazard (via a day of
//! slack) but not the format hazard, so `admits()`'s fail-closed rule never ran for a
//! non-ISO stamp — a proven fail-open. Widening the predicate to `>= floor OR NOT
//! <well-formed-prefix>` would have restored correctness only approximately, *and* a
//! disjunction of that shape is unindexable in SQLite anyway — so the optimization it
//! was defending no longer existed. The scan is narrowed by the equality filters
//! (`at_location`, `resource_conforms_to`, `h_app_id`), which is where the selectivity
//! actually is; see [`compute_usage`] for the cost this leaves behind.
//!
//! ## Which utilization is authoritative
//!
//! `CarryingCapacity.current_utilization` is a **declared snapshot** carried on the
//! DHT-projected value object; it drifts the moment an event lands and is never
//! consulted here. The authority for an allocation decision is the value this
//! module derives from the event projection. See the utilization-home note in
//! `genesis/data/timeline/backlog/2026-08-11-carrying-capacity-cumulative-vs-rate-unit-error.md`.

use diesel::prelude::*;
use elohim_epr::measure::{MeasureKind, Period};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::context::AppContext;
use crate::db::diesel_schema::{economic_events, places};
use crate::error::StorageError;
use crate::services::spatial::CarryingCapacity;

/// Result of checking carrying capacity for a resource allocation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapacityCheckResult {
    /// Whether the allocation is allowed within carrying capacity
    pub is_allowed: bool,
    /// Utilization ratio (0.0 to 1.0+), measured over one period of the yield's
    /// denominator when the unit names one. Derived from economic events — this, not
    /// the Place's stored currentUtilization, is the authority for an allocation.
    pub current_utilization: f64,
    /// Utilization ratio after the proposed allocation, on the same basis
    pub utilization_after: f64,
    /// Maximum sustainable yield for this resource category. A RATE per the period its
    /// unit denominates ("liters/day"), or a LEVEL when the unit names no denominator
    /// ("dwellings") or names an extent rather than a tempo ("people/km2")
    pub max_sustainable_yield: f64,
    /// Unit of measurement. A TIME denominator (the "day" in "liters/day") selects the
    /// period usage is measured over; no denominator ("dwellings") or an EXTENT
    /// denominator ("people/km2", a density) is a level and is measured cumulatively
    pub unit: String,
    /// Usage at this place — within the yield's period for a rate unit, cumulative for
    /// a level or density unit
    pub current_usage: f64,
    /// Whether governance should be triggered (>80% threshold)
    pub trigger_governance: bool,
    /// Place ID checked
    pub place_id: String,
    /// Resource category checked
    pub resource_category: String,
}

/// Get the carrying capacity for a specific resource category at a Place.
///
/// Parses the Place's `carrying_capacity_json` and finds the entry
/// matching the requested resource category.
pub fn get_place_capacity(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
) -> Result<Option<CarryingCapacity>, StorageError> {
    // Load place
    let capacity_json: String = places::table
        .filter(places::id.eq(place_id))
        .filter(places::h_app_id.eq(ctx.h_app_id()))
        .select(places::carrying_capacity_json)
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Place not found: {}", place_id))
            }
            _ => StorageError::Internal(format!("Failed to fetch place capacity: {}", e)),
        })?;

    // Parse carrying capacity array
    let capacities: Vec<CarryingCapacity> =
        serde_json::from_str(&capacity_json).unwrap_or_default();

    // Find matching resource category
    Ok(capacities
        .into_iter()
        .find(|c| c.resource_category == resource_category))
}

/// What a capacity's `unit` says about the denominator of `max_sustainable_yield`.
///
/// `unit` is the only signal available without changing the DHT-projected shape, and
/// it is LOAD-BEARING: this classification decides whether usage is windowed at all,
/// which is a 365x swing between `"liters"` and `"liters/day"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum YieldDenominator {
    /// No denominator at all — `"dwellings"`, `"hectares"`, `"people"`, `"liters"`.
    /// The declared yield is a **level**: a ceiling on a stock, not a flow. Usage is
    /// NOT windowed, because dropping older occupancy from a level would understate
    /// it and permit over-allocation.
    Absent,
    /// A time denominator this module recognizes — the yield is a **rate** of that
    /// tempo, and usage is windowed to exactly one period of it.
    Period(Period),
    /// A recognized denominator that is an **extent, not a tempo** — `"people/km2"`,
    /// `"kg/ha"`, `"m3/km2"`, `"kg/capita"`. A density is a level per area (or per
    /// head, or per volume), which is still a LEVEL: nothing about it is per-period, so
    /// windowing it would drop history that has not expired and permit
    /// over-allocation. Treated exactly like [`Absent`](YieldDenominator::Absent) —
    /// the only reason it is a separate variant is that a `/` is present and must be
    /// shown to have been *read*, not merely tolerated.
    Density,
    /// A denominator is present and is neither a recognized tempo nor a recognized
    /// extent. Ambiguous abbreviations land here on purpose: a bare `"m"` is minute or
    /// month or metre depending on who wrote it, and guessing is how the next unit
    /// error gets in. `"liters/fortnight"` lands here too — plainly a tempo, but not
    /// one in the vocabulary, so it is refused rather than silently read as a density.
    Unrecognized,
}

/// Denominators that name an EXTENT rather than a tempo, making the unit a density.
///
/// A whitelist, not a fallback: "any denominator that is not a time unit is a density"
/// would swallow `"liters/fortnight"` — a real tempo outside the vocabulary — and read
/// it as a level, which is the fail-open this classifier exists to prevent. Anything
/// not listed here and not a tempo is [`YieldDenominator::Unrecognized`].
///
/// Deliberately EXCLUDED: bare `"m"` (minute / month / metre) and bare `"a"` (annum /
/// are). Ambiguity between a tempo and an extent is exactly the case that must refuse.
const DENSITY_DENOMINATORS: &[&str] = &[
    // Area
    "ha",
    "hectare",
    "hectares",
    "km2",
    "km²",
    "sqkm",
    "m2",
    "m²",
    "sqm",
    "cm2",
    "cm²",
    "mi2",
    "sqmi",
    "acre",
    "acres",
    "ft2",
    "ft²",
    "sqft",
    // Volume
    "m3",
    "m³",
    "km3",
    "km³",
    "l",
    "liter",
    "liters",
    "litre",
    "litres",
    "ml",
    "gal",
    "gallon",
    "gallons",
    // Length
    "km",
    "cm",
    "mm",
    "mi",
    "mile",
    "miles",
    "ft",
    "foot",
    "feet",
    // Mass
    "kg",
    "g",
    "t",
    "tonne",
    "tonnes",
    "ton",
    "tons",
    // Population / holding — a per-head or per-household density is still a level
    "capita",
    "person",
    "people",
    "head",
    "household",
    "households",
    "dwelling",
    "dwellings",
    "resident",
    "residents",
];

/// Classify the capacity's `unit` into [`YieldDenominator`].
///
/// A unit that spells a TEMPO (`"liters/day"`, `"m3 per year"`) is a rate. A unit with
/// no `/` and no ` per ` is a level. A unit that spells an EXTENT (`"people/km2"`,
/// `"kg/ha"`) is a density, which is also a level — the presence of a `/` does not by
/// itself make a declaration a flow. A denominator that is neither is refused by the
/// caller rather than assumed either way.
pub(crate) fn yield_denominator(unit: &str) -> YieldDenominator {
    let lower = unit.trim().to_ascii_lowercase();
    let Some((_, denominator)) = lower
        .rsplit_once(" per ")
        .or_else(|| lower.rsplit_once('/'))
    else {
        return YieldDenominator::Absent;
    };

    let denominator = denominator.trim();
    match denominator {
        "s" | "sec" | "secs" | "second" | "seconds" => YieldDenominator::Period(Period::Second),
        "min" | "mins" | "minute" | "minutes" => YieldDenominator::Period(Period::Minute),
        "h" | "hr" | "hrs" | "hour" | "hours" => YieldDenominator::Period(Period::Hour),
        "d" | "day" | "days" => YieldDenominator::Period(Period::Day),
        "w" | "wk" | "wks" | "week" | "weeks" => YieldDenominator::Period(Period::Week),
        "mo" | "mon" | "month" | "months" => YieldDenominator::Period(Period::Month),
        "y" | "yr" | "yrs" | "year" | "years" | "a" | "annum" => {
            YieldDenominator::Period(Period::Year)
        }
        _ if DENSITY_DENOMINATORS.contains(&denominator) => YieldDenominator::Density,
        _ => YieldDenominator::Unrecognized,
    }
}

/// The measure kinds of (usage, yield) implied by the unit — the input to the
/// dimensional guard in [`check_carrying_capacity`].
///
/// These are derived from the unit, not asserted, which is what makes the guard a
/// real check rather than a restatement of its own assumption:
///
/// - `Period(p)`  → `Rate{p} ÷ Rate{p}` = `Ratio`. Allowed; usage is windowed.
/// - `Absent`     → `Level ÷ Level` = `Ratio`. Allowed; usage is NOT windowed.
/// - `Density`    → `Level ÷ Level` = `Ratio`. Allowed; usage is NOT windowed. A
///   density's denominator is an extent, not a tempo, so nothing about it expires and
///   both sides stay levels. (The extent itself cancels: `people/km²` over
///   `people/km²` is a pure ratio, and this module compares like with like.)
/// - `Unrecognized` → the declaration MIGHT be a rate of unknown tempo, so usage cannot
///   be windowed and the only thing computable is a cumulative `Level`. `Level ÷ Rate`
///   is a *coverage time*, not a utilization, so the guard refuses. The `per` chosen
///   here is immaterial: `Level ÷ Rate{_}` is `Level` for every period.
pub(crate) fn measure_kinds(denominator: YieldDenominator) -> (MeasureKind, MeasureKind) {
    match denominator {
        YieldDenominator::Period(per) => (MeasureKind::Rate { per }, MeasureKind::Rate { per }),
        YieldDenominator::Absent | YieldDenominator::Density => {
            (MeasureKind::Level, MeasureKind::Level)
        }
        YieldDenominator::Unrecognized => {
            (MeasureKind::Level, MeasureKind::Rate { per: Period::Year })
        }
    }
}

/// Wall-clock span of one period.
///
/// Month and year are the conventional fixed approximations (30 / 365 days) rather
/// than calendar arithmetic: the window is a measurement span, not a billing cycle,
/// and a drifting window boundary would make consecutive readings incomparable.
fn period_span(period: Period) -> chrono::Duration {
    match period {
        Period::Second => chrono::Duration::seconds(1),
        Period::Minute => chrono::Duration::minutes(1),
        Period::Hour => chrono::Duration::hours(1),
        Period::Day => chrono::Duration::days(1),
        Period::Week => chrono::Duration::weeks(1),
        Period::Month => chrono::Duration::days(30),
        Period::Year => chrono::Duration::days(365),
    }
}

/// One period of usage, ending now.
///
/// The window has exactly ONE bound and it is an instant. There is deliberately no
/// string form of it: a lexical bound handed to SQL would decide the fate of rows
/// whose stamps do not sort like an ISO timestamp, and the module contract is that SQL
/// never decides (see the module docs).
#[derive(Debug, Clone)]
struct UsageWindow {
    /// The lower bound. Every candidate row's `has_point_in_time` is parsed to an
    /// instant and compared against THIS — never against a string.
    start: chrono::DateTime<chrono::Utc>,
}

impl UsageWindow {
    fn ending_at(now: chrono::DateTime<chrono::Utc>, period: Period) -> Self {
        Self {
            start: now - period_span(period),
        }
    }

    /// Whether a raw `has_point_in_time` falls at or after the window's start.
    ///
    /// Offset-bearing (`…-07:00`), `Z`-terminated, fractional-second and zone-less
    /// renderings all resolve to the same instant here. A timestamp this cannot parse
    /// is COUNTED: on a limit-enforcement gate, over-counting refuses an allocation
    /// that might have been fine, while under-counting permits one that is not.
    fn admits(&self, raw: &str) -> bool {
        match parse_event_instant(raw) {
            Some(at) => at >= self.start,
            None => true,
        }
    }
}

/// Parse an event timestamp into a UTC instant.
///
/// RFC 3339 first (this is what the input schema asks for, offsets included); then a
/// zone-less `YYYY-MM-DDTHH:MM:SS[.fff]` rendering, which several in-tree producers
/// emit and which everything in this codebase means as UTC.
fn parse_event_instant(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|naive| naive.and_utc())
}

/// Compute resource usage at a Place, optionally restricted to one period.
///
/// Sums `resource_quantity_value` for consume/use events at the given location. With
/// a `window`, the result is a **rate** (usage per period) and is what makes the
/// comparison against a per-period yield dimensionally valid. Without one — a level
/// capacity — it is the cumulative level, which is the correct left-hand side for a
/// `Level ÷ Level` ratio.
///
/// The sum is folded in Rust rather than by SQL `SUM()` because the window predicate
/// is an instant comparison, not a string comparison; see [`UsageWindow::admits`]. For
/// the same reason there is no SQL bound on `has_point_in_time` at all: a lexical
/// bound cannot order a non-ISO stamp, so applying one would decide the row's fate
/// before the fail-closed rule could run.
///
/// COST: both paths therefore materialize every historical `consume`/`use` row for
/// this Place and category, however narrow the window. That is a performance residual,
/// recorded in
/// `genesis/data/timeline/backlog/2026-08-11-carrying-capacity-cumulative-vs-rate-unit-error.md`;
/// it is not a correctness one, and it is not paid for by a bound that can silently
/// drop a row.
fn compute_usage(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
    window: Option<&UsageWindow>,
) -> Result<f64, StorageError> {
    let query = economic_events::table
        .filter(economic_events::h_app_id.eq(ctx.h_app_id()))
        .filter(economic_events::at_location.eq(place_id))
        .filter(
            economic_events::action
                .eq("consume")
                .or(economic_events::action.eq("use")),
        )
        .filter(economic_events::resource_conforms_to.eq(resource_category));

    let rows: Vec<(String, Option<f32>)> = query
        .select((
            economic_events::has_point_in_time,
            economic_events::resource_quantity_value,
        ))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to compute usage: {}", e)))?;

    Ok(rows
        .into_iter()
        .filter(|(at, _)| window.is_none_or(|w| w.admits(at)))
        .map(|(_, value)| value.unwrap_or(0.0) as f64)
        .sum())
}

/// Check whether a proposed resource allocation is within carrying capacity.
///
/// Returns a detailed result including current utilization and whether
/// governance should be triggered (>80% threshold).
///
/// When the capacity's unit denominates a period, usage is measured over exactly one
/// such period (see [`yield_denominator`]), so the utilization is a genuine
/// `harvest ÷ regeneration` ratio and a Place that returns to within its yield
/// recovers. When the unit names a level (`"dwellings"`) or a density
/// (`"people/km2"`), usage is compared level-to-level and is not windowed. When the
/// unit's denominator is neither a tempo nor an extent this service reads, this returns
/// `Err` rather than a number with no interpretation.
pub fn check_carrying_capacity(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
    additional_amount: f64,
) -> Result<CapacityCheckResult, StorageError> {
    let capacity = get_place_capacity(conn, ctx, place_id, resource_category)?;

    match capacity {
        None => {
            // No carrying capacity defined — allow but warn
            Ok(CapacityCheckResult {
                is_allowed: true,
                current_utilization: 0.0,
                utilization_after: 0.0,
                max_sustainable_yield: 0.0,
                unit: "unknown".to_string(),
                current_usage: 0.0,
                trigger_governance: false,
                place_id: place_id.to_string(),
                resource_category: resource_category.to_string(),
            })
        }
        Some(cap) => {
            let denominator = yield_denominator(&cap.unit);

            // The dimensional law, executed rather than commented. Both kinds are
            // DERIVED from `cap.unit` — a tempo denominator gives Rate ÷ Rate, a level
            // or density unit gives Level ÷ Level, and a denominator we cannot read
            // (neither tempo nor extent) gives Level ÷ Rate,
            // whose quotient is a coverage time. Only the first two are utilizations,
            // so the third is refused rather than reported as a number with no such
            // interpretation — that silence is what shipped the original defect.
            let (usage_kind, yield_kind) = measure_kinds(denominator);
            if usage_kind.divide(yield_kind) != Some(MeasureKind::Ratio) {
                return Err(StorageError::Internal(format!(
                    "carrying capacity for place {place_id} / {resource_category} declares unit \
                     {:?}, whose denominator is neither a time unit (making it a rate) nor an \
                     extent this service recognises (making it a density, and so a level). The \
                     only usage computable for it is a cumulative level, and level ÷ rate is a \
                     coverage time, not a utilization. Refusing to report a number with no such \
                     interpretation.",
                    cap.unit
                )));
            }

            // A rate capacity is measured over one period of its own denominator; a
            // level capacity is compared level-to-level and MUST NOT be windowed.
            let window = match denominator {
                YieldDenominator::Period(per) => {
                    Some(UsageWindow::ending_at(chrono::Utc::now(), per))
                }
                YieldDenominator::Absent
                | YieldDenominator::Density
                | YieldDenominator::Unrecognized => None,
            };
            let current_usage =
                compute_usage(conn, ctx, place_id, resource_category, window.as_ref())?;
            let usage_after = current_usage + additional_amount;
            let current_util = if cap.max_sustainable_yield > 0.0 {
                current_usage / cap.max_sustainable_yield
            } else {
                0.0
            };
            let util_after = if cap.max_sustainable_yield > 0.0 {
                usage_after / cap.max_sustainable_yield
            } else {
                0.0
            };

            Ok(CapacityCheckResult {
                is_allowed: util_after <= 1.0,
                current_utilization: current_util,
                utilization_after: util_after,
                max_sustainable_yield: cap.max_sustainable_yield,
                unit: cap.unit,
                current_usage,
                trigger_governance: util_after > 0.8,
                place_id: place_id.to_string(),
                resource_category: resource_category.to_string(),
            })
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::economic_events;
    use crate::db::models::NewPlace;
    use crate::db::places::upsert_place;
    use chrono::{Duration, Utc};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn capacity_json(category: &str, yield_per_period: f64, unit: &str) -> String {
        serde_json::to_string(&vec![CarryingCapacity {
            resource_category: category.to_string(),
            max_sustainable_yield: yield_per_period,
            unit: unit.to_string(),
            // Deliberately stale: the stored field is a declared snapshot, never the
            // authority for an allocation decision (see module docs).
            current_utilization: 0.0,
            data_quality: "estimated".to_string(),
            source: None,
            measured_at: None,
        }])
        .expect("serialize carrying capacity")
    }

    fn seed_place(conn: &mut SqliteConnection, ctx: &AppContext, id: &str, capacity: String) {
        upsert_place(
            conn,
            ctx,
            NewPlace {
                id: id.to_string(),
                h_app_id: ctx.h_app_id().to_string(),
                dht_anchor_hash: format!("anchor-{id}"),
                name: id.to_string(),
                place_type: "watershed".to_string(),
                constitutional_layer: "community".to_string(),
                h3_index: "8928308280fffff".to_string(),
                h3_resolution: 9,
                geometry_json: "{}".to_string(),
                centroid_lat: 0.0,
                centroid_lng: 0.0,
                parent_place_id: None,
                osm_reference_json: None,
                carrying_capacity_json: capacity,
                governing_collective_id: None,
                status: "active".to_string(),
                created_by: "test".to_string(),
                created_at: "2020-01-01T00:00:00Z".to_string(),
                updated_at: "2020-01-01T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
        )
        .expect("seed place");
    }

    /// Seed `days` consecutive daily consumption events of `amount` each at `place_id`.
    ///
    /// Each event is offset by an extra hour so that no event ever lands exactly on a
    /// day-window boundary — the test is about history LENGTH, not boundary arithmetic.
    fn seed_daily_usage(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        place_id: &str,
        category: &str,
        amount: f32,
        days: i64,
    ) {
        let now = Utc::now();
        for d in 0..days {
            let at = now - Duration::days(d) - Duration::hours(1);
            let at_iso = at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let id = format!("{place_id}-{category}-{d}");
            diesel::insert_into(economic_events::table)
                .values((
                    economic_events::id.eq(&id),
                    economic_events::h_app_id.eq(ctx.h_app_id()),
                    economic_events::action.eq("use"),
                    economic_events::provider.eq("agent-provider"),
                    economic_events::receiver.eq("agent-receiver"),
                    economic_events::resource_conforms_to.eq(category),
                    economic_events::resource_quantity_value.eq(amount),
                    economic_events::has_point_in_time.eq(&at_iso),
                    economic_events::state.eq("settled"),
                    economic_events::created_at.eq(&at_iso),
                    economic_events::at_location.eq(place_id),
                ))
                .execute(conn)
                .expect("insert economic event");
        }
    }

    /// THE regression: a Place consuming steadily and comfortably WITHIN its
    /// sustainable yield must not drift toward refusal as history accumulates.
    ///
    /// Two Places consume at exactly the same rate (100 units/day against a
    /// 1000 units/day yield). One has 30 days of history, the other 400. The
    /// reading must be identical and must allow the allocation.
    ///
    /// Before the windowing fix this failed on both counts: `compute_current_usage`
    /// summed ALL-TIME (3,000 vs 40,000), so utilization read 3.1 vs 40.1 and every
    /// allocation was refused forever, with no reachable recovery state.
    #[test]
    fn steady_consumption_within_yield_does_not_drift_toward_refusal() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-short-history",
            capacity_json("water", 1000.0, "liters/day"),
        );
        seed_place(
            &mut conn,
            &ctx,
            "place-long-history",
            capacity_json("water", 1000.0, "liters/day"),
        );

        seed_daily_usage(&mut conn, &ctx, "place-short-history", "water", 100.0, 30);
        seed_daily_usage(&mut conn, &ctx, "place-long-history", "water", 100.0, 400);

        let short = check_carrying_capacity(&mut conn, &ctx, "place-short-history", "water", 100.0)
            .expect("check short-history place");
        let long = check_carrying_capacity(&mut conn, &ctx, "place-long-history", "water", 100.0)
            .expect("check long-history place");

        assert!(
            short.is_allowed,
            "30 days of within-yield consumption must not refuse: util_after={}",
            short.utilization_after
        );
        assert!(
            long.is_allowed,
            "400 days of the SAME within-yield consumption must not refuse: util_after={}",
            long.utilization_after
        );
        assert!(
            (short.utilization_after - long.utilization_after).abs() < 1e-9,
            "history length must not change the reading: short={} long={}",
            short.utilization_after,
            long.utilization_after
        );
        assert!(
            !long.trigger_governance,
            "a Place at 20% of its yield must not trip the 0.8 governance threshold (got {})",
            long.utilization_after
        );
        assert!(
            (long.utilization_after - 0.2).abs() < 1e-9,
            "100 used + 100 proposed against a 1000/day yield is 0.2, got {}",
            long.utilization_after
        );
    }

    /// The fix must not become "always allow": genuine overshoot within the
    /// window still refuses.
    #[test]
    fn overshoot_within_the_window_is_still_refused() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-overshoot",
            capacity_json("water", 1000.0, "liters/day"),
        );
        // 1,200 units consumed today — over the 1,000/day yield on its own.
        seed_daily_usage(&mut conn, &ctx, "place-overshoot", "water", 1200.0, 1);

        let result = check_carrying_capacity(&mut conn, &ctx, "place-overshoot", "water", 100.0)
            .expect("check overshooting place");

        assert!(
            !result.is_allowed,
            "consumption above the sustainable yield must be refused, util_after={}",
            result.utilization_after
        );
        assert!(
            result.trigger_governance,
            "overshoot must trip the governance threshold"
        );
    }

    /// The unit's denominator is load-bearing, not decorative: the same history read
    /// against a per-year yield covers a wider window than against a per-day one.
    #[test]
    fn the_units_denominator_selects_the_window() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-daily",
            capacity_json("water", 100_000.0, "liters/day"),
        );
        seed_place(
            &mut conn,
            &ctx,
            "place-annual",
            capacity_json("water", 100_000.0, "liters per year"),
        );
        seed_daily_usage(&mut conn, &ctx, "place-daily", "water", 100.0, 200);
        seed_daily_usage(&mut conn, &ctx, "place-annual", "water", 100.0, 200);

        let daily = check_carrying_capacity(&mut conn, &ctx, "place-daily", "water", 0.0)
            .expect("check per-day capacity");
        let annual = check_carrying_capacity(&mut conn, &ctx, "place-annual", "water", 0.0)
            .expect("check per-year capacity");

        assert!(
            (daily.current_usage - 100.0).abs() < 1e-6,
            "a per-day yield sees one day of history, got {}",
            daily.current_usage
        );
        assert!(
            (annual.current_usage - 20_000.0).abs() < 1e-6,
            "a per-year yield sees all 200 days of history, got {}",
            annual.current_usage
        );
    }

    /// The unit classifier: level, rate-of-a-known-tempo, or refuse.
    #[test]
    fn the_unit_classifies_into_level_rate_or_refusal() {
        // No denominator at all: a LEVEL. Never windowed.
        assert_eq!(yield_denominator("liters"), YieldDenominator::Absent);
        assert_eq!(yield_denominator("dwellings"), YieldDenominator::Absent);
        assert_eq!(yield_denominator("hectares"), YieldDenominator::Absent);
        assert_eq!(yield_denominator(""), YieldDenominator::Absent);

        // A readable denominator, in both spellings.
        assert_eq!(
            yield_denominator("liters/day"),
            YieldDenominator::Period(Period::Day)
        );
        assert_eq!(
            yield_denominator("kWh per year"),
            YieldDenominator::Period(Period::Year)
        );
        assert_eq!(
            yield_denominator("m3/hr"),
            YieldDenominator::Period(Period::Hour)
        );
        assert_eq!(
            yield_denominator("tonnes per month"),
            YieldDenominator::Period(Period::Month)
        );

        // Ambiguous or unknown denominators are not guessed at — "m" could be minute
        // or month, and a "fortnight" is simply not in the vocabulary.
        assert_eq!(
            yield_denominator("liters/m"),
            YieldDenominator::Unrecognized
        );
        assert_eq!(
            yield_denominator("liters per fortnight"),
            YieldDenominator::Unrecognized
        );
    }

    /// The dimensional guard is LIVE, not a restatement of its own assumption: the
    /// three classifications produce three different algebra outcomes, and exactly one
    /// of them is refused.
    #[test]
    fn the_dimensional_guard_admits_two_shapes_and_refuses_the_third() {
        let (u, y) = measure_kinds(YieldDenominator::Period(Period::Day));
        assert_eq!(
            u.divide(y),
            Some(MeasureKind::Ratio),
            "rate ÷ rate is a ratio"
        );

        let (u, y) = measure_kinds(YieldDenominator::Absent);
        assert_eq!(
            u.divide(y),
            Some(MeasureKind::Ratio),
            "level ÷ level is a ratio"
        );

        let (u, y) = measure_kinds(YieldDenominator::Unrecognized);
        assert_eq!(
            u.divide(y),
            Some(MeasureKind::Level),
            "level ÷ rate is a coverage TIME — not a utilization, so it must be refused"
        );
    }

    /// The refusal branch is reachable through the public API, not just in the algebra:
    /// a capacity whose unit names an unreadable denominator errors instead of silently
    /// reporting a coverage time as a utilization.
    #[test]
    fn an_unreadable_denominator_refuses_rather_than_reporting_a_non_ratio() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-unreadable-unit",
            capacity_json("water", 1000.0, "liters per fortnight"),
        );
        seed_daily_usage(&mut conn, &ctx, "place-unreadable-unit", "water", 100.0, 3);

        let err = check_carrying_capacity(&mut conn, &ctx, "place-unreadable-unit", "water", 100.0)
            .expect_err("an unreadable denominator must refuse, not return a number");

        let message = err.to_string();
        assert!(
            message.contains("liters per fortnight"),
            "the refusal must name the offending unit, got: {message}"
        );
    }

    /// THE second fail-open the windowing fix could have introduced: a LEVEL capacity
    /// (`"dwellings"` — a ceiling on a stock, not a flow) must not be windowed. If it
    /// were, occupancy older than the window would vanish, utilization would fall, and
    /// an over-allocation would be permitted.
    ///
    /// 480 dwellings occupied over 400 days against a 500-dwelling ceiling: proposing
    /// 30 more must be refused. Under a 365-day rate window the older 35 rows drop out,
    /// the reading falls to 0.87 with room to spare, and the allocation is allowed.
    #[test]
    fn a_level_capacity_is_not_windowed_and_still_refuses_over_allocation() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-shelter",
            capacity_json("shelter", 500.0, "dwellings"),
        );
        // 400 daily occupancy events of 1.2 dwellings each = 480 occupied.
        seed_daily_usage(&mut conn, &ctx, "place-shelter", "shelter", 1.2, 400);

        let result = check_carrying_capacity(&mut conn, &ctx, "place-shelter", "shelter", 30.0)
            .expect("check level-denominated capacity");

        assert!(
            (result.current_usage - 480.0).abs() < 1e-3,
            "a level capacity must count ALL occupancy, not one window of it, got {}",
            result.current_usage
        );
        assert!(
            !result.is_allowed,
            "480 + 30 against a 500-dwelling ceiling must be refused, util_after={}",
            result.utilization_after
        );
        assert!(
            result.trigger_governance,
            "a level capacity over its ceiling must trip governance"
        );
    }

    /// An offset-bearing timestamp INSIDE the window must be counted.
    ///
    /// `has_point_in_time` is projected verbatim from the payload and its schema
    /// permits a UTC offset, so an event stamped `…T05:00:00-07:00` (= 12:00Z) sorts
    /// LEXICALLY below a `…T12:00:00` cutoff. A string compare drops it, utilization
    /// under-reads, and an allocation that should be refused is allowed. This test
    /// pins the instant comparison that prevents that.
    ///
    /// The capacity is per-HOUR on purpose: the offset (7h) must exceed the window
    /// span for the lexical bug to be reachable at all, and a per-day window would
    /// hide it behind the window's own width.
    #[test]
    fn an_offset_bearing_timestamp_inside_the_window_is_counted() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-offset",
            capacity_json("water", 1000.0, "liters/hour"),
        );

        // 30 minutes ago — comfortably inside the 1-hour window — but WRITTEN in a
        // zone whose wall-clock rendering sorts 7 hours below the UTC cutoff.
        let instant = Utc::now() - Duration::minutes(30);
        let offset = chrono::FixedOffset::west_opt(7 * 3600).expect("valid -07:00 offset");
        let stamped = instant
            .with_timezone(&offset)
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        assert!(
            stamped.ends_with("-07:00"),
            "the fixture must actually carry an offset, got {stamped}"
        );

        diesel::insert_into(economic_events::table)
            .values((
                economic_events::id.eq("offset-stamped-usage"),
                economic_events::h_app_id.eq(ctx.h_app_id()),
                economic_events::action.eq("consume"),
                economic_events::provider.eq("agent-provider"),
                economic_events::receiver.eq("agent-receiver"),
                economic_events::resource_conforms_to.eq("water"),
                economic_events::resource_quantity_value.eq(900.0f32),
                economic_events::has_point_in_time.eq(&stamped),
                economic_events::state.eq("settled"),
                economic_events::created_at.eq(&stamped),
                economic_events::at_location.eq("place-offset"),
            ))
            .execute(&mut conn)
            .expect("insert offset-stamped event");

        let result = check_carrying_capacity(&mut conn, &ctx, "place-offset", "water", 200.0)
            .expect("check offset-stamped place");

        assert!(
            (result.current_usage - 900.0).abs() < 1e-3,
            "an offset-bearing timestamp inside the window must be counted, got {}",
            result.current_usage
        );
        assert!(
            !result.is_allowed,
            "900 + 200 against a 1000/day yield must be refused, util_after={}",
            result.utilization_after
        );
    }

    /// The instant comparison must not become "count everything": an offset-bearing
    /// timestamp genuinely OUTSIDE the window is still excluded. Every row for the
    /// Place reaches the Rust fold (there is no SQL time bound), so the instant
    /// comparison is the only thing that can exclude it.
    #[test]
    fn an_offset_bearing_timestamp_outside_the_window_is_still_excluded() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-offset-stale",
            capacity_json("water", 1000.0, "liters/day"),
        );

        // 30 hours ago: outside a 1-day window. Nothing in SQL filters on time, so the
        // row is loaded and only the instant comparison can exclude it.
        let instant = Utc::now() - Duration::hours(30);
        let offset = chrono::FixedOffset::east_opt(5 * 3600).expect("valid +05:00 offset");
        let stamped = instant
            .with_timezone(&offset)
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();

        diesel::insert_into(economic_events::table)
            .values((
                economic_events::id.eq("stale-offset-usage"),
                economic_events::h_app_id.eq(ctx.h_app_id()),
                economic_events::action.eq("consume"),
                economic_events::provider.eq("agent-provider"),
                economic_events::receiver.eq("agent-receiver"),
                economic_events::resource_conforms_to.eq("water"),
                economic_events::resource_quantity_value.eq(5_000.0f32),
                economic_events::has_point_in_time.eq(&stamped),
                economic_events::state.eq("settled"),
                economic_events::created_at.eq(&stamped),
                economic_events::at_location.eq("place-offset-stale"),
            ))
            .execute(&mut conn)
            .expect("insert stale offset-stamped event");

        let result = check_carrying_capacity(&mut conn, &ctx, "place-offset-stale", "water", 100.0)
            .expect("check stale offset place");

        assert!(
            result.current_usage.abs() < 1e-6,
            "an event 30h old must leave a 1-day window, got {}",
            result.current_usage
        );
        assert!(result.is_allowed);
    }

    /// A timestamp this service cannot parse is COUNTED, never dropped: on a limit
    /// gate, over-counting refuses and under-counting allows.
    ///
    /// This fixture is ISO-shaped garbage. The harder case — a stamp that cannot be
    /// lexically ordered against a floor at all — is
    /// [`stamps_that_cannot_be_lexically_ordered_are_still_counted`], and it is the one
    /// that pins the absence of a SQL time bound.
    #[test]
    fn an_unparseable_timestamp_fails_closed_and_is_counted() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-garbage-stamp",
            capacity_json("water", 1000.0, "liters/day"),
        );

        // ISO-shaped enough to reach the parser, not a timestamp.
        diesel::insert_into(economic_events::table)
            .values((
                economic_events::id.eq("garbage-stamped-usage"),
                economic_events::h_app_id.eq(ctx.h_app_id()),
                economic_events::action.eq("consume"),
                economic_events::provider.eq("agent-provider"),
                economic_events::receiver.eq("agent-receiver"),
                economic_events::resource_conforms_to.eq("water"),
                economic_events::resource_quantity_value.eq(1_500.0f32),
                economic_events::has_point_in_time.eq("not-a-timestamp"),
                economic_events::state.eq("settled"),
                economic_events::created_at.eq("not-a-timestamp"),
                economic_events::at_location.eq("place-garbage-stamp"),
            ))
            .execute(&mut conn)
            .expect("insert garbage-stamped event");

        let result = check_carrying_capacity(&mut conn, &ctx, "place-garbage-stamp", "water", 0.0)
            .expect("check garbage-stamped place");

        assert!(
            (result.current_usage - 1_500.0).abs() < 1e-3,
            "an unreadable timestamp must be counted, not silently dropped, got {}",
            result.current_usage
        );
        assert!(
            !result.is_allowed,
            "the fail-closed reading must refuse, util_after={}",
            result.utilization_after
        );
    }

    /// Seed one `water` consumption event with a caller-chosen raw timestamp.
    ///
    /// The stamp is written verbatim — the point of the callers below is that the
    /// column accepts renderings this service cannot order lexically.
    fn seed_raw_stamped_event(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        place_id: &str,
        id: &str,
        amount: f32,
        stamp: &str,
    ) {
        diesel::insert_into(economic_events::table)
            .values((
                economic_events::id.eq(id),
                economic_events::h_app_id.eq(ctx.h_app_id()),
                economic_events::action.eq("consume"),
                economic_events::provider.eq("agent-provider"),
                economic_events::receiver.eq("agent-receiver"),
                economic_events::resource_conforms_to.eq("water"),
                economic_events::resource_quantity_value.eq(amount),
                economic_events::has_point_in_time.eq(stamp),
                economic_events::state.eq("settled"),
                economic_events::created_at.eq(stamp),
                economic_events::at_location.eq(place_id),
            ))
            .execute(conn)
            .expect("insert raw-stamped event");
    }

    /// The fail-closed rule must survive the SQL layer, not just the Rust fold.
    ///
    /// `an_unparseable_timestamp_fails_closed_and_is_counted` passes on a fixture
    /// (`"not-a-timestamp"`) that happens to start with `n` (0x6E) and therefore sorts
    /// lexically ABOVE any `2xxx-…` floor — so it never exercised the SQL bound. These
    /// two do: epoch seconds (`"1754899200"`, leading `1`) and the empty string both
    /// sort BELOW a `2026-…` floor. A SQL `has_point_in_time >= floor` prefilter drops
    /// them before the fold ever sees them, `admits()` never runs, and usage reads 0 —
    /// which ALLOWS an allocation that the fail-closed rule says must be REFUSED.
    #[test]
    fn stamps_that_cannot_be_lexically_ordered_are_still_counted() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-unorderable-stamp",
            capacity_json("water", 1000.0, "liters/day"),
        );

        // Epoch seconds: a real rendering, unparseable here, and lexically BELOW a
        // 2026-dated floor because it starts with '1'.
        seed_raw_stamped_event(
            &mut conn,
            &ctx,
            "place-unorderable-stamp",
            "epoch-seconds-usage",
            1_500.0,
            "1754899200",
        );
        // The empty string sorts below every floor there is.
        seed_raw_stamped_event(
            &mut conn,
            &ctx,
            "place-unorderable-stamp",
            "empty-stamp-usage",
            250.0,
            "",
        );

        let result =
            check_carrying_capacity(&mut conn, &ctx, "place-unorderable-stamp", "water", 0.0)
                .expect("check unorderable-stamp place");

        assert!(
            (result.current_usage - 1_750.0).abs() < 1e-3,
            "a stamp that cannot be lexically ordered must reach the fold and be counted, \
             not excluded by the SQL bound; got {}",
            result.current_usage
        );
        assert!(
            !result.is_allowed,
            "1,750 against a 1,000/day yield must be refused, util_after={}",
            result.utilization_after
        );
    }

    /// Zone-less and `Z`-terminated renderings of the same instant read identically.
    #[test]
    fn timestamp_flavours_resolve_to_the_same_instant() {
        let z = parse_event_instant("2026-08-11T12:00:00Z").expect("Z-terminated");
        let offset = parse_event_instant("2026-08-11T05:00:00-07:00").expect("offset-bearing");
        let naive = parse_event_instant("2026-08-11T12:00:00").expect("zone-less");
        let fractional = parse_event_instant("2026-08-11T12:00:00.000Z").expect("fractional");

        assert_eq!(z, offset);
        assert_eq!(z, naive);
        assert_eq!(z, fractional);
        assert!(parse_event_instant("not-a-timestamp").is_none());
    }

    /// The window is anchored to `now`, so a Place that overshot in the past but has
    /// been within its yield since reads as within capacity again. Under the old
    /// all-time sum there was no state the system could reach that recovered.
    #[test]
    fn a_place_recovers_once_the_overshoot_leaves_the_window() {
        let mut conn = test_conn();
        let ctx = AppContext::new("test-app");

        seed_place(
            &mut conn,
            &ctx,
            "place-recovering",
            capacity_json("water", 1000.0, "liters/day"),
        );

        // A severe historical overshoot, 10 days ago.
        let long_ago = (Utc::now() - Duration::days(10))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        diesel::insert_into(economic_events::table)
            .values((
                economic_events::id.eq("historical-overshoot"),
                economic_events::h_app_id.eq(ctx.h_app_id()),
                economic_events::action.eq("consume"),
                economic_events::provider.eq("agent-provider"),
                economic_events::receiver.eq("agent-receiver"),
                economic_events::resource_conforms_to.eq("water"),
                economic_events::resource_quantity_value.eq(50_000.0f32),
                economic_events::has_point_in_time.eq(&long_ago),
                economic_events::state.eq("settled"),
                economic_events::created_at.eq(&long_ago),
                economic_events::at_location.eq("place-recovering"),
            ))
            .execute(&mut conn)
            .expect("insert historical overshoot");

        // Modest, within-yield consumption today.
        seed_daily_usage(&mut conn, &ctx, "place-recovering", "water", 100.0, 1);

        let result = check_carrying_capacity(&mut conn, &ctx, "place-recovering", "water", 100.0)
            .expect("check recovering place");

        assert!(
            result.is_allowed,
            "an overshoot outside the window must not refuse forever, util_after={}",
            result.utilization_after
        );
        assert!(
            !result.trigger_governance,
            "a recovered Place must stop tripping governance"
        );
    }
}
