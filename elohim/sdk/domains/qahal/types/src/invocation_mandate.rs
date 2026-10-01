//! Private invocation bounds. The selected, author-signed CapGrant carries these
//! terms; a client-supplied copy is never authority. Public content grants remain
//! the receiver-verifiable mandate. No additional content head is created.
use serde::{Deserialize, Serialize};

pub const MANDATE_TAG: &str = "elohim:invocation-mandate:v1:";
pub const MAX_SUBJECTS: usize = 128;
pub const MAX_TAG_BYTES: usize = 32768;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InvocationSubject {
    pub id: String,
    pub root: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationMandate {
    pub issuer: String,
    pub requester: String,
    pub dna: String,
    pub delegate: Option<String>,
    pub subjects: Vec<InvocationSubject>,
    pub operations: Vec<String>,
    pub valid_until: i64,
    /// Existing notarized device relationship; not an administrative role.
    pub binding: Option<String>,
    pub policy: String,
    /// Exact canonical JSON for an identity signing ceremony. Content operations
    /// instead constrain independently resolved roots, delegate and expiry.
    pub exact_payload_json: Option<String>,
}

pub struct InvocationContext<'a> {
    pub issuer: &'a str,
    pub requester: &'a str,
    pub dna: &'a str,
    pub operation: &'a str,
    pub now: i64,
    pub subject: Option<(&'a str, &'a str)>,
    pub delegate: Option<&'a str>,
    pub grant_expiry: Option<i64>,
    pub payload_json: Option<&'a str>,
}

impl InvocationMandate {
    pub fn check(&self, c: &InvocationContext<'_>) -> Result<(), &'static str> {
        if self.issuer != c.issuer || self.requester != c.requester || self.dna != c.dna {
            return Err("invocation issuer, requester or DNA differs");
        }
        if self.valid_until <= c.now {
            return Err("invocation mandate expired");
        }
        if self.policy.trim().is_empty()
            || self.subjects.len() > MAX_SUBJECTS
            || self.operations.is_empty()
            || self.operations.len() > 8
            || !self.operations.iter().any(|op| op == c.operation)
        {
            return Err("invocation operation or policy is not authorized");
        }
        if let Some((id, root)) = c.subject {
            if id.is_empty()
                || id.contains('*')
                || !self.subjects.iter().any(|s| s.id == id && s.root == root)
                || self.delegate.as_deref() != c.delegate
                || self.binding.as_deref().is_none_or(str::is_empty)
            {
                return Err("invocation subject, delegate or binding is outside mandate");
            }
        } else if self.exact_payload_json.as_deref() != c.payload_json || c.payload_json.is_none() {
            return Err("identity ceremony requires the exact authorized payload");
        }
        if c.grant_expiry
            .is_some_and(|expiry| expiry > self.valid_until)
        {
            return Err("grant expiry exceeds invocation mandate");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mandate() -> InvocationMandate {
        InvocationMandate {
            issuer: "author".into(),
            requester: "signing-key".into(),
            dna: "network".into(),
            delegate: Some("device".into()),
            subjects: vec![InvocationSubject {
                id: "lesson".into(),
                root: "root".into(),
            }],
            operations: vec!["grant_head_delegation".into()],
            valid_until: 100,
            binding: Some("binding".into()),
            policy: "fct-commons-v1".into(),
            exact_payload_json: None,
        }
    }
    fn context() -> InvocationContext<'static> {
        InvocationContext {
            issuer: "author",
            requester: "signing-key",
            dna: "network",
            operation: "grant_head_delegation",
            now: 99,
            subject: Some(("lesson", "root")),
            delegate: Some("device"),
            grant_expiry: Some(100),
            payload_json: None,
        }
    }
    #[test]
    fn stolen_invocation_cannot_expand_payload_or_context() {
        let m = mandate();
        assert!(m.check(&context()).is_ok());
        for changed in 0..9 {
            let mut c = context();
            match changed {
                0 => c.requester = "another-key",
                1 => c.issuer = "another-author",
                2 => c.dna = "another-holon",
                3 => c.operation = "revoke_identity_device",
                4 => c.subject = Some(("private", "root")),
                5 => c.subject = Some(("lesson", "other-root")),
                6 => c.delegate = Some("intruder"),
                7 => c.grant_expiry = Some(101),
                _ => c.now = 100,
            }
            assert!(m.check(&c).is_err(), "escape {changed}");
        }
    }
    #[test]
    fn enrollment_does_not_implicitly_authorize_another_payload() {
        let mut m = mandate();
        m.operations = vec!["sign_device_enrollment".into()];
        m.exact_payload_json = Some("{\"device\":1}".into());
        let mut c = context();
        c.operation = "sign_device_enrollment";
        c.subject = None;
        c.grant_expiry = None;
        c.payload_json = Some("{\"device\":1}");
        assert!(m.check(&c).is_ok());
        c.payload_json = Some("{\"device\":2}");
        assert!(m.check(&c).is_err());
    }
}
