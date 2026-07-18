//! Property tests for the `AgentPubKey` ↔ `did:key` codec.
//!
//! For any valid 32-byte core, the round-trips are byte-identical. Because the
//! DHT location bytes are a deterministic function of the core, reconstructing
//! an agent_cid from its core always reproduces the exact original.

use did_bridge::{
    agent_cid_to_core32, agent_cid_to_did_key, core32_to_agent_cid, did_key_to_agent_cid,
    did_key_to_core32,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn core_to_agent_cid_and_back(core in proptest::array::uniform32(any::<u8>())) {
        let agent_cid = core32_to_agent_cid(&core);
        let recovered = agent_cid_to_core32(&agent_cid).unwrap();
        prop_assert_eq!(core, recovered);
    }

    #[test]
    fn core_to_did_key_and_back(core in proptest::array::uniform32(any::<u8>())) {
        let agent_cid = core32_to_agent_cid(&core);
        let did_key = agent_cid_to_did_key(&agent_cid).unwrap();
        let recovered = did_key_to_core32(&did_key).unwrap();
        prop_assert_eq!(core, recovered);
    }

    #[test]
    fn agent_cid_to_did_key_to_agent_cid_is_identical(
        core in proptest::array::uniform32(any::<u8>())
    ) {
        // Start from a canonical (correct-loc) agent_cid.
        let agent_cid = core32_to_agent_cid(&core);
        let did_key = agent_cid_to_did_key(&agent_cid).unwrap();
        let round = did_key_to_agent_cid(&did_key).unwrap();
        prop_assert_eq!(agent_cid, round);
    }
}
