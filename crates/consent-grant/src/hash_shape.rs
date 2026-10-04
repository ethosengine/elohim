//! Shape checks for the hashes and keys that travel in a request.
//!
//! These are shape checks, not proofs. Possession of a device key is proven
//! where it has to be: the enrollment zome refuses a binding the device key did
//! not sign. Checking shapes here only keeps a malformed value from reaching a
//! consent screen.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

/// Multihash prefix of an `AgentPubKey` (`uhCAk…` once encoded).
const AGENT_PREFIX: [u8; 3] = [0x84, 0x20, 0x24];
/// 3 prefix bytes, 32 key bytes, 4 location bytes.
const AGENT_KEY_LEN: usize = 39;

/// Multihash prefix of a `DnaHash` (`uhC0k…` once encoded).
const DNA_PREFIX: [u8; 3] = [0x84, 0x2d, 0x24];

fn has_shape(text: &str, prefix: [u8; 3]) -> bool {
    let Some(encoded) = text.strip_prefix('u') else {
        return false;
    };
    match URL_SAFE_NO_PAD.decode(encoded) {
        Ok(bytes) => bytes.len() == AGENT_KEY_LEN && bytes[..3] == prefix,
        Err(_) => false,
    }
}

pub fn is_agent_key(text: &str) -> bool {
    has_shape(text, AGENT_PREFIX)
}

/// A DNA hash names the network a device is on, so a controller never signs
/// for a network the device did not ask about.
pub fn is_dna_hash(text: &str) -> bool {
    has_shape(text, DNA_PREFIX)
}

/// Multihash prefix of an `ActionHash` (`uhCkk…` once encoded).
const ACTION_PREFIX: [u8; 3] = [0x84, 0x29, 0x24];

/// An action hash names a notarized record, such as the identity a device asks
/// to be bound to.
pub fn is_action_hash(text: &str) -> bool {
    has_shape(text, ACTION_PREFIX)
}

/// A device's cryptographic root as local tooling names it: an ed25519
/// `did:key`, which is `z6Mk` followed by 44 base58 characters.
pub fn is_device_root_key(text: &str) -> bool {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    match text.strip_prefix("did:key:z6Mk") {
        Some(rest) => rest.len() == 44 && rest.bytes().all(|b| BASE58.contains(&b)),
        None => false,
    }
}

/// Short form for a consent screen: enough for a person to compare against
/// what their terminal printed, not enough to mistake for the key.
pub fn fingerprint(agent_key: &str) -> String {
    let tail_start = agent_key.len().saturating_sub(6);
    let head_end = agent_key.len().min(9);
    if agent_key.len() <= 15 {
        return agent_key.to_string();
    }
    format!("{}…{}", &agent_key[..head_end], &agent_key[tail_start..])
}

#[cfg(test)]
fn sample(prefix: [u8; 3], fill: u8) -> String {
    let mut bytes = Vec::with_capacity(AGENT_KEY_LEN);
    bytes.extend_from_slice(&prefix);
    bytes.extend_from_slice(&[fill; 36]);
    format!("u{}", URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
pub(crate) fn sample_key(fill: u8) -> String {
    sample(AGENT_PREFIX, fill)
}

#[cfg(test)]
pub(crate) fn sample_action(fill: u8) -> String {
    sample(ACTION_PREFIX, fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_agent_key_passes() {
        assert!(is_agent_key(
            "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl"
        ));
        assert!(is_agent_key(&sample_key(7)));
    }

    #[test]
    fn other_hash_kinds_and_garbage_fail() {
        // A DNA hash and an action hash have the right length and the wrong prefix.
        assert!(!is_agent_key(
            "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt"
        ));
        assert!(!is_agent_key(
            "uhCkkppgLq-1oNhvKmEHUvnM_r9Jew4d9ynyWymbmU_p4-m4nG3gn"
        ));
        assert!(!is_agent_key(""));
        assert!(!is_agent_key("uhCAk"));
        assert!(!is_agent_key(
            "hCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl"
        ));
        assert!(!is_agent_key("uhCAk iqczp"));
    }

    #[test]
    fn a_dna_hash_is_told_from_an_agent_key() {
        let dna = "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt";
        assert!(is_dna_hash(dna));
        assert!(!is_dna_hash(
            "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl"
        ));
    }

    #[test]
    fn an_action_hash_is_told_from_the_other_kinds() {
        assert!(is_action_hash(
            "uhCkkppgLq-1oNhvKmEHUvnM_r9Jew4d9ynyWymbmU_p4-m4nG3gn"
        ));
        assert!(!is_action_hash(
            "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl"
        ));
        assert!(!is_action_hash(
            "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt"
        ));
    }

    #[test]
    fn a_device_root_key_is_an_ed25519_did_key() {
        let key = format!("did:key:z6Mk{}", "h".repeat(44));
        assert!(is_device_root_key(&key));
        assert!(!is_device_root_key(&key[..key.len() - 1]));
        assert!(!is_device_root_key(&key.replace("z6Mk", "z6LS")));
        // 0, O, I and l are not base58.
        assert!(!is_device_root_key(&format!(
            "did:key:z6Mk{}",
            "0".repeat(44)
        )));
        assert!(!is_device_root_key(
            "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl"
        ));
    }

    #[test]
    fn the_fingerprint_keeps_both_ends() {
        let key = "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl";
        assert_eq!(fingerprint(key), "uhCAkiqcz…TGMzzl");
        assert_eq!(fingerprint("short"), "short");
    }
}
