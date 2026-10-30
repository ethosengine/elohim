//! Typed reach refusals and byte-stable HTTP renderings.

use bytes::Bytes;
use eprfs_core::{ReachReason, ReachRefusal};
use http_body_util::Full;
use hyper::{header, Response, StatusCode};

use crate::private_reach::WithholdReason;

/// The specific local reason a reach decision refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalDetail {
    /// No identity was presented at the boundary.
    IdentityAbsent,
    /// The presented identity did not resolve to a human with standing.
    IdentityUnresolved,
    /// The reach authorizer denied the resolved human.
    Denied(String),
    /// Custody facts withheld a private subject.
    CustodyWithheld(WithholdReason),
    /// The subject carries an unrecognized reach word.
    UnknownTier,
    /// No local record declares an audience for this subject.
    NoRecord,
    /// Required projection or authority records could not be read.
    RecordsUnavailable,
}

/// A uniform refusal shared by HTTP, P2P, and eprfs adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The transport-neutral eprfs reason.
    pub reason: ReachReason,
    /// The local diagnostic detail.
    pub detail: RefusalDetail,
    /// The reach the requester would need, when disclosing it is safe.
    pub required_reach: Option<String>,
}

impl Refusal {
    /// Refuse a request that presented no identity.
    pub fn identity_absent(reach: impl Into<String>) -> Self {
        Self {
            reason: ReachReason::IdentityRequired,
            detail: RefusalDetail::IdentityAbsent,
            required_reach: Some(reach.into()),
        }
    }

    /// Refuse a request whose presented identity did not resolve.
    pub fn identity_unresolved(reach: impl Into<String>) -> Self {
        Self {
            reason: ReachReason::IdentityRequired,
            detail: RefusalDetail::IdentityUnresolved,
            required_reach: Some(reach.into()),
        }
    }

    /// Refuse a request denied by the reach authorizer.
    pub fn denied(reach: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            reason: ReachReason::OutsideStanding,
            detail: RefusalDetail::Denied(reason.into()),
            required_reach: Some(reach.into()),
        }
    }

    /// Refuse because custody standing withheld a private subject.
    pub fn custody_withheld(reach: impl Into<String>, reason: WithholdReason) -> Self {
        Self {
            reason: ReachReason::OutsideStanding,
            detail: RefusalDetail::CustodyWithheld(reason),
            required_reach: Some(reach.into()),
        }
    }

    /// Refuse because no local record declares the subject.
    pub fn no_record() -> Self {
        Self {
            reason: ReachReason::NoRecord,
            detail: RefusalDetail::NoRecord,
            required_reach: None,
        }
    }

    /// Refuse because required authority records were unavailable.
    pub fn records_unavailable() -> Self {
        Self {
            reason: ReachReason::AuthorizerUnavailable,
            detail: RefusalDetail::RecordsUnavailable,
            required_reach: None,
        }
    }

    /// Render the exact JSON body used by content-row HTTP gates.
    pub fn http_row_response(&self) -> Response<Full<Bytes>> {
        let reach = self.required_reach.as_deref().unwrap_or("");
        let body = match &self.detail {
            RefusalDetail::IdentityAbsent => {
                format!(r#"{{"error":"Authentication required","requiredReach":"{reach}"}}"#)
            }
            RefusalDetail::IdentityUnresolved => {
                format!(r#"{{"error":"Reach authorization required","requiredReach":"{reach}"}}"#)
            }
            RefusalDetail::Denied(reason) => format!(
                r#"{{"error":"Reach denied","requiredReach":"{reach}","reason":"{reason}"}}"#
            ),
            RefusalDetail::CustodyWithheld(reason) => format!(
                r#"{{"error":"Reach denied","requiredReach":"{reach}","reason":"{reason}"}}"#
            ),
            RefusalDetail::UnknownTier => format!(
                r#"{{"error":"Reach denied","requiredReach":"{reach}","reason":"Unknown reach level: {reach}"}}"#
            ),
            RefusalDetail::NoRecord | RefusalDetail::RecordsUnavailable => {
                format!(r#"{{"error":"Reach authorization required","requiredReach":"{reach}"}}"#)
            }
        };
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .expect("static reach refusal response")
    }

    /// Render the exact JSON and headers used by the HTTP blob gate.
    pub fn http_blob_response(&self) -> Response<Full<Bytes>> {
        let mut response = self.http_row_response();
        response.headers_mut().insert(
            "X-Content-Type-Options",
            "nosniff".parse().expect("static header"),
        );
        response
    }

    /// Convert the refusal to the eprfs boundary type.
    pub fn into_eprfs(self) -> ReachRefusal {
        ReachRefusal::new(self.reason, self.required_reach)
    }
}
