//! sign_account_bind_challenge — graduating a hosted account onto this node's key.
//!
//! Coordinator-only (DNA-hash-neutral). Signs one purpose-bound text with the
//! cell's agent key so a doorway can verify the human holds the key they claim.
//! The purpose line is re-checked here so no caller of the cell can turn this
//! into a general signer (unlike `sign_for_agent`, which signs arbitrary bytes).
//!
//! Spec: genesis/docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md

use hdk::prelude::*;

/// First line of every account-bind challenge (wire contract v1).
pub const ACCOUNT_BIND_PURPOSE_LINE: &str = "elohim:doorway-account-bind:v1";
const MAX_CHALLENGE_BYTES: usize = 1024;
const CHALLENGE_LINES: usize = 7;

/// Output of `sign_account_bind_challenge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAccountBind {
    /// Raw 64-byte Ed25519 signature over the challenge UTF-8 bytes.
    pub signature: Vec<u8>,
    /// The agent key that signed.
    pub signer: AgentPubKey,
}

/// Longest a challenge may live ahead of now (the doorway issues 5 minutes).
pub const MAX_LIFETIME_SECS: i64 = 600;

const KEYS: [&str; 6] = [
    "audience",
    "identifier",
    "human",
    "from",
    "nonce",
    "expires",
];

/// Seconds since the epoch of `YYYY-MM-DDTHH:MM:SSZ` (exactly 20 bytes), else None. Pure.
pub fn parse_rfc3339_z_secs(v: &str) -> Option<i64> {
    let b = v.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let part = &v[r];
        if part.bytes().all(|c| c.is_ascii_digit()) {
            part.parse().ok()
        } else {
            None
        }
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    // days_from_civil (Howard Hinnant)
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss)
}

/// Pure check of the full v1 grammar: <= 1 KiB, no CR, exactly 7 LF-separated
/// lines (purpose line, then audience, identifier, human, from, nonce, expires),
/// nonce 64 lowercase hex, expires RFC3339 `Z` seconds, after `now_secs` and at
/// most [`MAX_LIFETIME_SECS`] ahead. Returns the refusal words on failure.
pub fn account_bind_challenge_shape(text: &str, now_secs: i64) -> Result<(), String> {
    if text.len() > MAX_CHALLENGE_BYTES {
        return Err(format!("challenge over {MAX_CHALLENGE_BYTES} bytes"));
    }
    if text.contains('\r') {
        return Err("challenge contains CR".into());
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() != CHALLENGE_LINES {
        return Err(format!(
            "challenge has {} lines, expected {CHALLENGE_LINES}",
            lines.len()
        ));
    }
    if lines[0] != ACCOUNT_BIND_PURPOSE_LINE {
        return Err("challenge is not an account-bind challenge".into());
    }
    let mut values: Vec<&str> = Vec::with_capacity(6);
    for (i, key) in KEYS.iter().enumerate() {
        let v = lines[i + 1]
            .strip_prefix(key)
            .and_then(|r| r.strip_prefix('='))
            .ok_or_else(|| format!("line {} is not `{key}=...`", i + 2))?;
        values.push(v);
    }
    let nonce = values[4];
    if nonce.len() != 64
        || !nonce
            .bytes()
            .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err("nonce is not 64 lowercase hex characters".into());
    }
    let expires = parse_rfc3339_z_secs(values[5])
        .ok_or_else(|| "expires is not RFC3339 UTC seconds (...Z)".to_string())?;
    if expires <= now_secs {
        return Err("challenge has expired".into());
    }
    if expires > now_secs + MAX_LIFETIME_SECS {
        return Err("challenge expires too far ahead".into());
    }
    Ok(())
}

/// Sign an account-bind challenge with this cell's agent key.
#[hdk_extern]
pub fn sign_account_bind_challenge(challenge: String) -> ExternResult<SignedAccountBind> {
    let now_secs = sys_time()?.as_seconds_and_nanos().0;
    account_bind_challenge_shape(&challenge, now_secs)
        .map_err(|why| wasm_error!(WasmErrorInner::Guest(why)))?;
    let signer = agent_info()?.agent_initial_pubkey;
    let signature = hdk::ed25519::sign_raw(signer.clone(), challenge.into_bytes())?;
    Ok(SignedAccountBind {
        signature: signature.0.to_vec(),
        signer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    // 2026-10-16T20:00:00Z
    const NOW: i64 = 1_792_180_800;

    fn with(expires: &str) -> String {
        format!(
            "{ACCOUNT_BIND_PURPOSE_LINE}\naudience=doorway.example\nidentifier=alice\nhuman=h1\nfrom=uhCAkfrom\nnonce={NONCE}\nexpires={expires}"
        )
    }
    fn good() -> String {
        with("2026-10-16T20:05:00Z")
    }

    #[test]
    fn epoch_math_is_right() {
        assert_eq!(parse_rfc3339_z_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339_z_secs("2026-10-16T20:00:00Z"), Some(NOW));
        assert_eq!(parse_rfc3339_z_secs("2026-10-16T20:00:00+00:00"), None);
        assert_eq!(parse_rfc3339_z_secs("2026-13-16T20:00:00Z"), None);
    }
    #[test]
    fn accepts_well_formed() {
        assert!(account_bind_challenge_shape(&good(), NOW).is_ok());
    }
    #[test]
    fn refuses_wrong_purpose() {
        let t = good().replacen("v1", "v2", 1);
        assert!(account_bind_challenge_shape(&t, NOW).is_err());
    }
    #[test]
    fn refuses_wrong_line_count() {
        assert!(account_bind_challenge_shape(&format!("{}\nextra", good()), NOW).is_err());
        assert!(account_bind_challenge_shape(ACCOUNT_BIND_PURPOSE_LINE, NOW).is_err());
        assert!(account_bind_challenge_shape(&format!("{}\n", good()), NOW).is_err());
    }
    #[test]
    fn refuses_cr() {
        assert!(account_bind_challenge_shape(&good().replace('\n', "\r\n"), NOW).is_err());
    }
    #[test]
    fn refuses_oversize() {
        let t = good().replace("alice", &"a".repeat(1100));
        assert!(account_bind_challenge_shape(&t, NOW).is_err());
    }
    #[test]
    fn refuses_wrong_or_misordered_keys() {
        let t = good().replace("identifier=alice\nhuman=h1", "human=h1\nidentifier=alice");
        assert!(account_bind_challenge_shape(&t, NOW).is_err());
        assert!(account_bind_challenge_shape(&good().replace("audience=", "aud="), NOW).is_err());
    }
    #[test]
    fn refuses_bad_nonce() {
        assert!(account_bind_challenge_shape(&good().replace(NONCE, "aa"), NOW).is_err());
        assert!(
            account_bind_challenge_shape(&good().replace(NONCE, &NONCE.to_uppercase()), NOW)
                .is_err()
        );
    }
    #[test]
    fn refuses_bad_expires_shape() {
        assert!(account_bind_challenge_shape(&with("2026-10-16T20:05:00.5Z"), NOW).is_err());
        assert!(account_bind_challenge_shape(&with("2026-10-16T20:05:00+00:00"), NOW).is_err());
    }
    #[test]
    fn refuses_expired_and_too_far_ahead() {
        assert!(account_bind_challenge_shape(&with("2026-10-16T20:00:00Z"), NOW).is_err());
        assert!(account_bind_challenge_shape(&with("2026-10-16T19:59:00Z"), NOW).is_err());
        // exactly 10 minutes ahead is allowed, one second more is not
        assert!(account_bind_challenge_shape(&with("2026-10-16T20:10:00Z"), NOW).is_ok());
        assert!(account_bind_challenge_shape(&with("2026-10-16T20:10:01Z"), NOW).is_err());
    }
}
