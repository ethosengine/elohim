//! SLA (Service Level Agreement) computation for governance challenges.
//!
//! Tracks whether challenge responses are on time, approaching deadline,
//! or overdue. The default response window is 3 days from filing.

use crate::db::models::Challenge;

/// SLA status for a challenge response deadline
pub enum SlaStatus {
    /// Response is still within the deadline window
    OnTime,
    /// Less than 1 day remaining before deadline
    Warning,
    /// Deadline has passed without a response
    Overdue,
    /// Challenge has been responded to or resolved
    Resolved,
}

impl SlaStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::OnTime => "on_time",
            Self::Warning => "warning",
            Self::Overdue => "overdue",
            Self::Resolved => "resolved",
        }
    }
}

/// Compute the SLA status for a challenge based on its response deadline
pub fn compute_sla_status(challenge: &Challenge) -> SlaStatus {
    if challenge.responded_at.is_some() || challenge.resolved_at.is_some() {
        return SlaStatus::Resolved;
    }
    let now = chrono::Utc::now().naive_utc();
    let deadline = chrono::NaiveDateTime::parse_from_str(
        &challenge.response_deadline,
        "%Y-%m-%dT%H:%M:%SZ",
    )
    .unwrap_or(now);
    let warning = deadline - chrono::Duration::days(1);
    if now > deadline {
        SlaStatus::Overdue
    } else if now > warning {
        SlaStatus::Warning
    } else {
        SlaStatus::OnTime
    }
}

/// Compute the response deadline given a filing timestamp (3-day window)
pub fn compute_response_deadline(filed_at: &str) -> String {
    let filed = chrono::NaiveDateTime::parse_from_str(filed_at, "%Y-%m-%dT%H:%M:%SZ")
        .unwrap_or_else(|_| chrono::Utc::now().naive_utc());
    let deadline = filed + chrono::Duration::days(3);
    deadline.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
