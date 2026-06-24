from pii_public_entry_lint import scan_source

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

def test_flags_pii_on_public_struct():
    hits = scan_source(LEAKING, visibility="public")
    assert {h.field for h in hits} == {"display_name", "bio"}

def test_ignores_pii_on_private_struct():
    assert scan_source(PRIVATE, visibility="private") == []

def test_passes_clean_anchor():
    assert scan_source(CLEAN, visibility="public") == []
