//! Reach-level access control
//!
//! The Elohim Protocol uses "reach" levels to control content visibility.
//! Content can be restricted to different spheres of trust.

use crate::error::{Result, SdkError};
use serde::{Deserialize, Serialize};

/// Reach levels from most public to most private.
///
/// The hierarchy is (most accessible to least accessible):
/// `commons` → `regional` → `bioregional` → `municipal` → `neighborhood` → `local` → `invited` → `private`
///
/// Higher numeric values = more restricted access.
/// Content at level N is accessible to requesters with reach level >= N.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReachLevel {
    /// Public commons - accessible to all (lowest barrier)
    Commons = 0,
    /// Regional level
    Regional = 1,
    /// Bioregional level
    Bioregional = 2,
    /// Municipal level
    Municipal = 3,
    /// Trusted neighborhood clusters
    Neighborhood = 4,
    /// Local family/cluster only
    Local = 5,
    /// Explicitly invited agents only
    Invited = 6,
    /// Only the owner can access (highest barrier)
    Private = 7,
}

impl ReachLevel {
    /// Parse from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "private" => Some(Self::Private),
            "invited" => Some(Self::Invited),
            "local" => Some(Self::Local),
            "neighborhood" => Some(Self::Neighborhood),
            "municipal" => Some(Self::Municipal),
            "bioregional" => Some(Self::Bioregional),
            "regional" => Some(Self::Regional),
            "commons" | "public" => Some(Self::Commons),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Invited => "invited",
            Self::Local => "local",
            Self::Neighborhood => "neighborhood",
            Self::Municipal => "municipal",
            Self::Bioregional => "bioregional",
            Self::Regional => "regional",
            Self::Commons => "commons",
        }
    }

    /// Check if this reach level is public (commons)
    pub fn is_public(&self) -> bool {
        *self == Self::Commons
    }

    /// Check if the given reach level can access content at this level
    pub fn can_access(&self, requester_reach: ReachLevel) -> bool {
        requester_reach >= *self
    }
}

impl Default for ReachLevel {
    fn default() -> Self {
        Self::Commons
    }
}

impl std::fmt::Display for ReachLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Enforces reach-based access control.
///
/// In Phase A (no DHT), this performs basic reach checks.
/// In Phase B, this will integrate with DHT attestations.
#[derive(Debug, Clone)]
pub struct ReachEnforcer {
    /// Current agent's reach level
    agent_reach: ReachLevel,
}

impl ReachEnforcer {
    /// Create a new enforcer with the agent's reach level
    pub fn new(agent_reach: ReachLevel) -> Self {
        Self { agent_reach }
    }

    /// Create an enforcer for anonymous access (commons only)
    pub fn anonymous() -> Self {
        Self {
            agent_reach: ReachLevel::Commons,
        }
    }

    /// Create an enforcer for authenticated access (regional by default)
    pub fn authenticated() -> Self {
        Self {
            agent_reach: ReachLevel::Regional,
        }
    }

    /// Check if the agent can access content at the given reach level
    pub fn can_access(&self, content_reach: ReachLevel) -> bool {
        content_reach.can_access(self.agent_reach)
    }

    /// Check access and return an error if denied
    pub fn check_access(&self, content_reach: ReachLevel) -> Result<()> {
        if self.can_access(content_reach) {
            Ok(())
        } else {
            Err(SdkError::AccessDenied {
                required: content_reach.to_string(),
                actual: self.agent_reach.to_string(),
            })
        }
    }

    /// Get the agent's reach level
    pub fn agent_reach(&self) -> ReachLevel {
        self.agent_reach
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reach_hierarchy() {
        // Higher value = more restricted
        assert!(ReachLevel::Private > ReachLevel::Commons);
        assert!(ReachLevel::Local > ReachLevel::Regional);
        assert!(ReachLevel::Invited > ReachLevel::Neighborhood);
    }

    #[test]
    fn test_can_access() {
        // Commons content (0) accessible to all - any reach >= 0
        assert!(ReachLevel::Commons.can_access(ReachLevel::Private));
        assert!(ReachLevel::Commons.can_access(ReachLevel::Commons));
        assert!(ReachLevel::Commons.can_access(ReachLevel::Regional));

        // Private content (7) only accessible to Private reach (7)
        assert!(ReachLevel::Private.can_access(ReachLevel::Private));
        assert!(!ReachLevel::Private.can_access(ReachLevel::Local)); // Local (5) < Private (7)
        assert!(!ReachLevel::Private.can_access(ReachLevel::Commons)); // Commons (0) < Private (7)

        // Regional content (1) accessible to Regional (1) and above
        assert!(ReachLevel::Regional.can_access(ReachLevel::Regional));
        assert!(ReachLevel::Regional.can_access(ReachLevel::Private)); // Private (7) >= Regional (1)
        assert!(!ReachLevel::Regional.can_access(ReachLevel::Commons)); // Commons (0) < Regional (1)
    }

    #[test]
    fn test_enforcer() {
        // Agent with Regional reach (1) can access content at Regional (1) or lower
        let enforcer = ReachEnforcer::new(ReachLevel::Regional);

        assert!(enforcer.can_access(ReachLevel::Commons)); // Commons (0) <= Regional (1)
        assert!(enforcer.can_access(ReachLevel::Regional)); // Regional (1) <= Regional (1)
        assert!(!enforcer.can_access(ReachLevel::Local)); // Local (5) > Regional (1)
        assert!(!enforcer.can_access(ReachLevel::Private)); // Private (7) > Regional (1)
    }
}
