//! The reader an answer is shaped for: who (an agent CID, when one is claimed) and at what tier.
use elohim_epr_index::reader::Reader;

#[test]
fn an_anonymous_reader_names_no_agent() {
    let anonymous = Reader::default();
    assert_eq!(anonymous.agent_cid, None);
    let named = Reader {
        agent_cid: Some("uhCAk-agent".into()),
        tier: "standard".into(),
    };
    assert_ne!(anonymous, named);
}
