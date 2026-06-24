from pii_public_entry_lint import scan_source, _collect_private_struct_names

LEAKING = '''
pub struct Human {
    pub id: String,
    pub display_name: String,   // PII
    pub bio: String,            // PII
}
'''
PRIVATE = '''
#[entry_type(visibility = "private")]
pub struct HumanProfile {
    pub display_name: String,
    pub bio: String,
}
'''
CLEAN = '''
pub struct Human {
    pub id: String,
    pub agent_key: AgentPubKey,
    pub created_at: Timestamp,
}
'''

# ── NEW fixture: enum-variant-private exemption ─────────────────────────────
# Simulates the real Holochain pattern: visibility is declared on the ENUM
# VARIANT, not on the pub struct.  The struct itself only has #[hdk_entry_helper].
ENUM_PRIVATE_VARIANT_SOURCE = '''
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    PublicRecord(PublicRecord),
    #[entry_type(visibility = "private")]
    AttentionTending(AttentionTending),
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct AttentionTending {
    pub context_json: String,
    pub bio: String,
}

pub struct PublicRecord {
    pub id: String,
    pub name: String,  // PII — public, should be flagged
}
'''

# ── NEW fixture: short-token word-boundary guard ─────────────────────────────
# `related_node_ids` contains "lat" as a substring — must NOT be flagged.
# `flat_rate` contains "lat" — must NOT be flagged.
# `latitude` is the real thing — SHOULD be flagged.
SUBSTRING_FALSE_POSITIVE_SOURCE = '''
pub struct Content {
    pub id: String,
    pub related_node_ids: Vec<String>,
    pub flat_rate: f32,
    pub template_id: String,
    pub latitude: f32,
}
'''


def test_flags_pii_on_public_struct():
    hits = scan_source(LEAKING, visibility="public")
    assert {h.field for h in hits} == {"display_name", "bio"}


def test_ignores_pii_on_private_struct():
    assert scan_source(PRIVATE, visibility="private") == []


def test_passes_clean_anchor():
    assert scan_source(CLEAN, visibility="public") == []


def test_enum_variant_private_exempts_struct():
    """A struct whose EntryTypes variant is marked visibility="private" must be
    exempted even though the struct itself carries no visibility attribute."""
    private_names = _collect_private_struct_names(ENUM_PRIVATE_VARIANT_SOURCE)
    assert "AttentionTending" in private_names
    assert "PublicRecord" not in private_names

    hits = scan_source(ENUM_PRIVATE_VARIANT_SOURCE, private_structs=private_names)
    hit_structs = {h.struct for h in hits}
    # AttentionTending is private — must NOT appear
    assert "AttentionTending" not in hit_structs
    # PublicRecord.name is public PII — MUST appear
    assert "PublicRecord" in hit_structs


def test_short_token_word_boundary():
    """Fields whose names merely CONTAIN a PII token as a substring (related_node_ids
    contains 'lat', flat_rate contains 'lat', template_id contains 'lat') must NOT be
    flagged.  Only the exact word-boundary match (latitude) should be flagged."""
    hits = scan_source(SUBSTRING_FALSE_POSITIVE_SOURCE)
    hit_fields = {h.field for h in hits}
    assert "related_node_ids" not in hit_fields, "substring 'lat' in related_node_ids must not fire"
    assert "flat_rate" not in hit_fields, "substring 'lat' in flat_rate must not fire"
    assert "template_id" not in hit_fields, "substring 'lat' in template_id must not fire"
    assert "latitude" in hit_fields, "latitude is real PII and must be flagged"
