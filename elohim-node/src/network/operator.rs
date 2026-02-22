//! Operator management
//!
//! The operator is the human who manages this node:
//! - Created the join key
//! - Can suspend/remove the node
//! - Sees this node in their imagodei/shefa dashboard
//! - Controls what data syncs to this node

use serde::{Deserialize, Serialize};

/// Operator who manages this node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    /// Operator's agent public key (from imagodei)
    pub agent_pub_key: String,

    /// Human-readable display name
    pub display_name: Option<String>,

    /// Avatar URL (from imagodei profile)
    pub avatar_url: Option<String>,

    /// Operator's relationship to this node
    pub relationship: OperatorRelationship,

    /// Permissions granted to this operator
    pub permissions: OperatorPermissions,

    /// When this operator was associated
    pub associated_at: u64,

    /// Last action by operator
    pub last_action: Option<OperatorAction>,
}

/// Relationship of operator to this node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperatorRelationship {
    /// Primary owner/creator
    Owner,
    /// Family admin with full control
    Admin,
    /// Steward with limited management rights
    Steward,
    /// Read-only observer
    Observer,
}

/// Permissions granted to the operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorPermissions {
    /// Can invite new nodes to cluster
    pub can_invite: bool,
    /// Can remove nodes from cluster
    pub can_remove: bool,
    /// Can modify sync settings
    pub can_configure_sync: bool,
    /// Can view node metrics
    pub can_view_metrics: bool,
    /// Can trigger manual sync
    pub can_trigger_sync: bool,
    /// Can approve updates
    pub can_approve_updates: bool,
    /// Can access stored content
    pub can_access_content: bool,
}

/// Recent action by operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorAction {
    pub action_type: String,
    pub timestamp: u64,
    pub details: Option<String>,
}

impl Default for OperatorPermissions {
    fn default() -> Self {
        Self {
            can_invite: false,
            can_remove: false,
            can_configure_sync: false,
            can_view_metrics: true,
            can_trigger_sync: true,
            can_approve_updates: false,
            can_access_content: false,
        }
    }
}

#[allow(dead_code)]
impl OperatorPermissions {
    /// Full permissions for owner
    pub fn owner() -> Self {
        Self {
            can_invite: true,
            can_remove: true,
            can_configure_sync: true,
            can_view_metrics: true,
            can_trigger_sync: true,
            can_approve_updates: true,
            can_access_content: true,
        }
    }

    /// Admin permissions
    pub fn admin() -> Self {
        Self {
            can_invite: true,
            can_remove: true,
            can_configure_sync: true,
            can_view_metrics: true,
            can_trigger_sync: true,
            can_approve_updates: true,
            can_access_content: true,
        }
    }

    /// Steward permissions (limited)
    pub fn steward() -> Self {
        Self {
            can_invite: true,
            can_remove: false,
            can_configure_sync: false,
            can_view_metrics: true,
            can_trigger_sync: true,
            can_approve_updates: false,
            can_access_content: false,
        }
    }

    /// Observer permissions (read-only)
    pub fn observer() -> Self {
        Self {
            can_invite: false,
            can_remove: false,
            can_configure_sync: false,
            can_view_metrics: true,
            can_trigger_sync: false,
            can_approve_updates: false,
            can_access_content: false,
        }
    }
}

#[allow(dead_code)]
impl Operator {
    /// Create owner operator from join key data
    pub fn from_join_key(agent_pub_key: String, display_name: Option<String>) -> Self {
        Self {
            agent_pub_key,
            display_name,
            avatar_url: None,
            relationship: OperatorRelationship::Owner,
            permissions: OperatorPermissions::owner(),
            associated_at: now(),
            last_action: None,
        }
    }

    /// Check if operator has a specific permission
    pub fn can(&self, permission: &str) -> bool {
        match permission {
            "invite" => self.permissions.can_invite,
            "remove" => self.permissions.can_remove,
            "configure_sync" => self.permissions.can_configure_sync,
            "view_metrics" => self.permissions.can_view_metrics,
            "trigger_sync" => self.permissions.can_trigger_sync,
            "approve_updates" => self.permissions.can_approve_updates,
            "access_content" => self.permissions.can_access_content,
            _ => false,
        }
    }

    /// Record an action by this operator
    pub fn record_action(&mut self, action_type: &str, details: Option<String>) {
        self.last_action = Some(OperatorAction {
            action_type: action_type.to_string(),
            timestamp: now(),
            details,
        });
    }
}

#[allow(dead_code)]
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
