//! Native unit tests for `post_commit`'s header-driven type resolution.
//!
//! # Why this file exists
//!
//! `post_commit` used to pick an entry's type by trying `to_app_option::<T>()`
//! against ~24 candidate types in a fixed order and taking the first success.
//! MessagePack struct decoding is structurally permissive, so a type whose
//! fields are a subset of a type appearing LATER in the chain silently stole
//! that type's signal. That is "Gap F": an `Agreement` decode succeeded on
//! `Commitment` bytes, `AgreementCommitted` fired, `ReaCommitmentCommitted`
//! never landed, and the elohim-storage REA projection never saw its entries.
//!
//! The dispatch now reads the type off the action header instead of guessing.
//! `resolve_entry_type` is the whole of that decision, so it is the whole of
//! what needs proving — and it is host-call free, so it proves natively with no
//! conductor, no sweettest, and no signal-capture harness.
//!
//! # What "natively" costs, and why the model is faithful
//!
//! In the live zome the scope table comes from `zome_info()?.zome_types.entries`,
//! which the conductor builds by scoping the DNA's global entry types to the
//! calling coordinator's declared integrity dependencies
//! (`ribosome.rs::zome_info` -> `in_scope_subset`). Here we build that same
//! `ScopedZomeTypes<EntryDefIndex>` by hand. Two things keep the model honest:
//!
//! 1. The per-variant `EntryDefIndex` is never hardcoded. It is derived from the
//!    variant's position in `UnitEntryTypes::iter()`, which is exactly what the
//!    `hdk_to_coordinates` derive uses for `From<UnitEntryTypes> for
//!    ZomeEntryTypesKey` (`type_index: <ordinal>.into()`). Adding, removing or
//!    reordering an entry type cannot silently invalidate these tests.
//! 2. `offset_scope_is_honoured_not_assumed` builds a scope table whose zome
//!    index and entry indices are both non-zero and non-identity, proving the
//!    resolver reads the indirection table rather than treating the header's
//!    `entry_index` as a raw ordinal.

use content_store_integrity::{Agreement, Commitment, EconomicEvent, EntryTypes, UnitEntryTypes};
use hdk::prelude::*;

use crate::resolve_entry_type;

// ---------------------------------------------------------------------------
// Scope-table construction (models the conductor's `in_scope_subset` output)
// ---------------------------------------------------------------------------

/// The `ZomeIndex` of `content_store_integrity`. The elohim DNA declares exactly
/// one integrity zome (`dna.yaml`), and `content_store` depends on only that one,
/// so in the live cell the scope table has a single row at index 0.
const OUR_ZOME: ZomeIndex = ZomeIndex(0);

/// How many app entry types the integrity zome defines.
fn entry_type_count() -> u8 {
    u8::try_from(UnitEntryTypes::iter().count()).expect("entry types fit in u8")
}

/// The zome-local declaration ordinal of a unit variant.
///
/// Mirrors the generated `From<UnitEntryTypes> for ZomeEntryTypesKey`, which sets
/// `type_index` to the variant's declaration position. Derived, never hardcoded.
fn ordinal_of(unit: UnitEntryTypes) -> u8 {
    let position = UnitEntryTypes::iter()
        .position(|candidate| candidate == unit)
        .unwrap_or_else(|| panic!("{unit:?} is not in UnitEntryTypes::iter()"));
    u8::try_from(position).expect("entry types fit in u8")
}

/// The single-integrity-zome scope table: zome-local ordinal *i* maps to global
/// `EntryDefIndex(i)`.
fn scope() -> ScopedZomeTypes<EntryDefIndex> {
    ScopedZomeTypes(vec![(
        OUR_ZOME,
        (0..entry_type_count()).map(EntryDefIndex).collect(),
    )])
}

/// The `EntryDefIndex` the action header would carry for `unit`, under `scope()`.
fn header_index_for(unit: UnitEntryTypes) -> EntryDefIndex {
    EntryDefIndex(ordinal_of(unit))
}

// ---------------------------------------------------------------------------
// Fixtures — the Gap F triple
// ---------------------------------------------------------------------------

/// `Agreement` is the type that stole the signal. Four fields, all of which also
/// appear (by name) on the two types below.
fn agreement() -> Agreement {
    Agreement {
        id: "agreement-gap-f".into(),
        name: Some("Gap F agreement".into()),
        note: Some("the type that used to win the race".into()),
        created_at: "2026-05-28T12:00:00Z".into(),
    }
}

/// The project-epr `Commitment` whose signal Gap F swallowed.
fn commitment() -> Commitment {
    Commitment {
        id: "project-epr-98f0d59051751497".into(),
        action: "project-epr".into(),
        provider: "agent:matthew".into(),
        receiver: "agent:matthew".into(),
        resource_conforms_to: None,
        resource_inventoried_as: None,
        resource_classified_as_json: "[]".into(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: None,
        has_beginning: None,
        has_end: None,
        due: None,
        clause_of: None,
        agreed_in: None,
        input_of: None,
        output_of: None,
        satisfies: None,
        in_scope_of_json: "[\"doorway:alpha-elohim-host\"]".into(),
        finished: false,
        state: "active".into(),
        note: None,
        metadata_json: "{}".into(),
        created_at: "2026-05-28T12:00:00Z".into(),
        updated_at: "2026-05-28T12:00:00Z".into(),
    }
}

/// The republish-epr `EconomicEvent` — the third member of the confusable set.
fn economic_event() -> EconomicEvent {
    EconomicEvent {
        id: "republish-epr-evt-1".into(),
        action: "republish-epr".into(),
        provider: "agent:matthew".into(),
        receiver: "agent:matthew".into(),
        resource_conforms_to: None,
        resource_inventoried_as: None,
        to_resource_inventoried_as: None,
        resource_classified_as_json: "[]".into(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: "2026-05-28T12:00:00Z".into(),
        has_duration: None,
        input_of: None,
        output_of: None,
        fulfills_json: "[]".into(),
        realization_of: None,
        satisfies_json: "[]".into(),
        in_scope_of_json: "[\"doorway:alpha-elohim-host\"]".into(),
        note: None,
        state: "settled".into(),
        triggered_by: None,
        at_location: None,
        image: None,
        lamad_event_type: None,
        metadata_json: "{}".into(),
        substrate_signal: None,
        created_at: "2026-05-28T12:00:00Z".into(),
    }
}

/// Serialize an entry struct into the `Entry::App` shape a committed record carries.
fn app_entry<T>(value: &T) -> Entry
where
    SerializedBytes: TryFrom<T, Error = SerializedBytesError>,
    T: Clone,
{
    let bytes = SerializedBytes::try_from(value.clone()).expect("entry serializes");
    Entry::app(bytes).expect("entry fits in an AppEntryBytes")
}

// ---------------------------------------------------------------------------
// MANDATORY: the Gap F triple resolves to its own variant
// ---------------------------------------------------------------------------

/// Each member of the confusable triple, given ITS OWN header index, resolves to
/// ITS OWN `EntryTypes` variant — and round-trips its payload intact.
///
/// The `Commitment` case is the literal Gap F regression: under the old ordered
/// chain these bytes yielded `EntryTypes::Agreement` (position 22 beat position
/// 23) and `ReaCommitmentCommitted` never fired.
#[test]
fn gap_f_triple_each_resolves_to_its_own_variant() {
    let scope = scope();

    let agreement = agreement();
    let resolved = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Agreement),
        &app_entry(&agreement),
    )
    .expect("Agreement resolves")
    .expect("Agreement is in scope");
    match resolved {
        EntryTypes::Agreement(decoded) => assert_eq!(
            decoded, agreement,
            "Agreement must round-trip through header dispatch unchanged"
        ),
        other => panic!("Agreement bytes + Agreement index resolved to {other:?}"),
    }

    let commitment = commitment();
    let resolved = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Commitment),
        &app_entry(&commitment),
    )
    .expect("Commitment resolves")
    .expect("Commitment is in scope");
    match resolved {
        EntryTypes::Commitment(decoded) => assert_eq!(
            decoded, commitment,
            "Commitment must round-trip through header dispatch unchanged"
        ),
        EntryTypes::Agreement(_) => panic!(
            "GAP F REGRESSION: a project-epr Commitment resolved to Agreement. \
             post_commit would emit AgreementCommitted and ReaCommitmentCommitted \
             would never fire — the storage REA projection never lands."
        ),
        other => panic!("Commitment bytes + Commitment index resolved to {other:?}"),
    }

    let event = economic_event();
    let resolved = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::EconomicEvent),
        &app_entry(&event),
    )
    .expect("EconomicEvent resolves")
    .expect("EconomicEvent is in scope");
    match resolved {
        EntryTypes::EconomicEvent(decoded) => assert_eq!(
            decoded, event,
            "EconomicEvent must round-trip through header dispatch unchanged"
        ),
        EntryTypes::Agreement(_) => panic!(
            "GAP F REGRESSION: a republish-epr EconomicEvent resolved to Agreement. \
             ReaEconomicEventCommitted would never fire."
        ),
        other => panic!("EconomicEvent bytes + EconomicEvent index resolved to {other:?}"),
    }
}

/// The header index — not the byte shape — decides the variant.
///
/// The same `Commitment` bytes are presented under three different header
/// indices. Header dispatch keys off the index, so the identity of the result is
/// determined entirely by which index was supplied. This is the property the old
/// ordered-decode chain did not have.
#[test]
fn the_header_index_decides_the_variant_not_the_byte_shape() {
    let scope = scope();
    let entry = app_entry(&commitment());

    let as_commitment = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Commitment),
        &entry,
    );
    assert!(
        matches!(as_commitment, Ok(Some(EntryTypes::Commitment(_)))),
        "Commitment index must yield the Commitment variant, got {as_commitment:?}"
    );

    // Same bytes, Agreement's index. Agreement carries #[serde(deny_unknown_fields)]
    // (the original one-type Gap F patch), so the superset's extra keys are rejected.
    let as_agreement = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Agreement),
        &entry,
    );
    assert!(
        as_agreement.is_err(),
        "Commitment bytes under Agreement's index must Err, got {as_agreement:?}"
    );

    // Same bytes, Content's index. Content requires title/content/content_format,
    // which Commitment bytes do not carry — a structural rejection that holds
    // whether or not any deny_unknown_fields attribute is present.
    let as_content = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Content),
        &entry,
    );
    assert!(
        as_content.is_err(),
        "Commitment bytes under Content's index must Err, got {as_content:?}"
    );
}

/// The reverse mis-index, independent of `deny_unknown_fields`.
///
/// `Agreement` bytes under `Commitment`'s index must Err: `Commitment` requires
/// `action`, `provider`, `receiver` and more, none of which an `Agreement`
/// carries. This is the louder-than-baseline path — the old chain swallowed a
/// failed decode into "no signal at all", which is exactly the silence that let
/// Gap F run undetected. It is now a returned `Err` that `post_commit` logs.
#[test]
fn bytes_under_a_foreign_index_error_rather_than_decode_or_fall_silent() {
    let scope = scope();
    let entry = app_entry(&agreement());

    let mis_indexed = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Commitment),
        &entry,
    );
    assert!(
        mis_indexed.is_err(),
        "Agreement bytes under Commitment's index must Err, got {mis_indexed:?}"
    );

    // Sanity: the same bytes under the right index still resolve. The Err above
    // is about the index disagreeing with the bytes, not about bad fixtures.
    let correct = resolve_entry_type(
        &scope,
        OUR_ZOME,
        header_index_for(UnitEntryTypes::Agreement),
        &entry,
    );
    assert!(
        matches!(correct, Ok(Some(EntryTypes::Agreement(_)))),
        "Agreement bytes under Agreement's index must resolve, got {correct:?}"
    );
}

// ---------------------------------------------------------------------------
// MANDATORY: the two miss paths, exactly as the implementation documents them
// ---------------------------------------------------------------------------

/// An entry authored against an integrity zome this coordinator does not depend
/// on resolves to `Ok(None)` — not ours to project, and not an error.
///
/// This is the path that keeps `post_commit` quiet about other zomes' traffic.
#[test]
fn an_out_of_scope_zome_index_is_ok_none() {
    let scope = scope();
    let foreign_zome = ZomeIndex(99);
    assert!(
        !scope.dependencies().any(|z| z == foreign_zome),
        "test precondition: ZomeIndex(99) must not be a declared dependency"
    );

    let resolved = resolve_entry_type(
        &scope,
        foreign_zome,
        header_index_for(UnitEntryTypes::Commitment),
        &app_entry(&commitment()),
    );
    assert!(
        matches!(resolved, Ok(None)),
        "an out-of-scope zome index must be Ok(None), got {resolved:?}"
    );
}

/// An entry index outside our integrity zome's range, on a zome we DO depend on,
/// is a real inconsistency and resolves to `Err`.
///
/// The two miss paths are deliberately different: unknown zome is routine
/// (`Ok(None)`), in-range zome with an out-of-range index cannot happen in a
/// coherent DNA and is surfaced.
#[test]
fn an_out_of_range_entry_index_on_our_own_zome_is_err() {
    let scope = scope();
    let out_of_range = EntryDefIndex(entry_type_count());
    assert!(
        scope.dependencies().any(|z| z == OUR_ZOME),
        "test precondition: OUR_ZOME must be a declared dependency"
    );

    let resolved = resolve_entry_type(&scope, OUR_ZOME, out_of_range, &app_entry(&commitment()));
    assert!(
        resolved.is_err(),
        "an out-of-range entry index on a depended-on zome must Err, got {resolved:?}"
    );
}

// ---------------------------------------------------------------------------
// Universal properties over every entry type (no per-type fixtures needed)
// ---------------------------------------------------------------------------

/// Every entry type in the integrity zome has a DISTINCT, resolvable header index.
///
/// This is the broad version of the Gap F guard. Building byte fixtures for all
/// ~80 entry types would be a large, high-rot hand-written surface; this proves
/// the index -> variant half of the mapping is total and injective across ALL of
/// them, with zero fixtures and nothing to keep in sync. Any aliasing — two types
/// resolving from one index, or a type unreachable by index — fails here.
#[test]
fn every_entry_type_has_a_distinct_resolvable_index() {
    let scope = scope();
    let mut seen: Vec<(EntryDefIndex, UnitEntryTypes)> = Vec::new();

    for unit in UnitEntryTypes::iter() {
        let index = header_index_for(unit);
        let scoped = ScopedEntryDefIndex {
            zome_index: OUR_ZOME,
            zome_type: index,
        };
        let found = scope.find(UnitEntryTypes::iter(), scoped);
        assert_eq!(
            found,
            Some(unit),
            "index {index:?} must resolve to {unit:?}, resolved to {found:?}"
        );

        if let Some((_, collided)) = seen.iter().find(|(seen_index, _)| *seen_index == index) {
            panic!("index collision: {unit:?} and {collided:?} share {index:?}");
        }
        seen.push((index, unit));
    }

    assert_eq!(
        seen.len(),
        usize::from(entry_type_count()),
        "every entry type must be reachable by exactly one header index"
    );
}

/// The resolver reads the scope table's indirection; it does not treat the
/// header's `entry_index` as a raw declaration ordinal.
///
/// Builds a scope table where the integrity zome sits at `ZomeIndex(7)` and its
/// entries are mapped to global indices offset by 100. A resolver that ignored
/// the table and used the ordinal directly would fail every assertion here.
/// This is what keeps the hand-built scope table above from being a tautology.
#[test]
fn offset_scope_is_honoured_not_assumed() {
    const OFFSET_ZOME: ZomeIndex = ZomeIndex(7);
    const INDEX_OFFSET: u8 = 100;

    let offset_scope = ScopedZomeTypes(vec![(
        OFFSET_ZOME,
        (0..entry_type_count())
            .map(|i| EntryDefIndex(INDEX_OFFSET + i))
            .collect(),
    )]);

    let commitment = commitment();
    let entry = app_entry(&commitment);

    // The header index is the GLOBAL one from the table, not the ordinal.
    let global_index = EntryDefIndex(INDEX_OFFSET + ordinal_of(UnitEntryTypes::Commitment));
    let resolved = resolve_entry_type(&offset_scope, OFFSET_ZOME, global_index, &entry)
        .expect("Commitment resolves under an offset scope")
        .expect("Commitment is in scope");
    match resolved {
        EntryTypes::Commitment(decoded) => assert_eq!(decoded, commitment),
        other => panic!("offset scope resolved Commitment to {other:?}"),
    }

    // The bare ordinal is meaningless under this table: it is out of range for a
    // zome we depend on, so it takes the Err path rather than silently aliasing
    // to some other type.
    let bare_ordinal = EntryDefIndex(ordinal_of(UnitEntryTypes::Commitment));
    let resolved = resolve_entry_type(&offset_scope, OFFSET_ZOME, bare_ordinal, &entry);
    assert!(
        resolved.is_err(),
        "a raw ordinal must not resolve under an offset scope table, got {resolved:?}"
    );

    // And the zome index is honoured too: the right entry index under the wrong
    // zome is not ours.
    let resolved = resolve_entry_type(&offset_scope, OUR_ZOME, global_index, &entry);
    assert!(
        matches!(resolved, Ok(None)),
        "the wrong zome index must be Ok(None), got {resolved:?}"
    );
}
