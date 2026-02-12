//! Permission levels and operation whitelist for Holochain admin operations
//!
//! Ported from admin-proxy/src/permissions.ts

use serde::{Deserialize, Serialize};
use std::fmt;

/// Permission levels for admin operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[repr(u8)]
#[derive(Default)]
pub enum PermissionLevel {
    /// No authentication - read-only operations
    #[default]
    Public = 0,
    /// Authenticated user - normal dev workflow operations
    Authenticated = 1,
    /// Admin - destructive operations like install/uninstall
    Admin = 2,
}

impl fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionLevel::Public => write!(f, "PUBLIC"),
            PermissionLevel::Authenticated => write!(f, "AUTHENTICATED"),
            PermissionLevel::Admin => write!(f, "ADMIN"),
        }
    }
}

/// Get the required permission level for a Holochain admin operation.
/// Returns None for unknown operations (which should be blocked).
pub fn get_required_permission(operation: &str) -> Option<PermissionLevel> {
    match operation {
        // Public - read only status queries
        "list_apps"
        | "list_app_interfaces"
        | "agent_info"
        | "storage_info"
        | "dump_network_stats" => Some(PermissionLevel::Public),

        // Authenticated - normal dev workflow
        "generate_agent_pub_key"
        | "grant_zome_call_capability"
        | "revoke_zome_call_capability"
        | "authorize_signing_credentials"
        | "attach_app_interface"
        | "issue_app_authentication_token"
        | "list_capability_grants"
        | "list_dnas"
        | "list_cell_ids"
        | "get_dna_definition"
        | "dump_state"
        | "dump_full_state" => Some(PermissionLevel::Authenticated),

        // Admin - dangerous/destructive operations
        "install_app"
        | "enable_app"
        | "disable_app"
        | "uninstall_app"
        | "update_coordinators"
        | "delete_clone_cell"
        | "add_agent_info"
        | "revoke_agent_key" => Some(PermissionLevel::Admin),

        // Unknown operations are blocked
        _ => None,
    }
}

/// Check if an operation is allowed for the given permission level
pub fn is_operation_allowed(operation: &str, level: PermissionLevel) -> bool {
    match get_required_permission(operation) {
        Some(required) => level >= required,
        None => false, // Unknown operations are blocked
    }
}

/// Get a human-readable description of an operation for logging
pub fn get_operation_description(operation: &str) -> &'static str {
    match operation {
        // Public
        "list_apps" => "List installed apps",
        "list_app_interfaces" => "List app interfaces",
        "agent_info" => "Get agent info",
        "storage_info" => "Get storage info",
        "dump_network_stats" => "Dump network statistics",

        // Authenticated
        "generate_agent_pub_key" => "Generate agent public key",
        "grant_zome_call_capability" => "Grant zome call capability",
        "revoke_zome_call_capability" => "Revoke zome call capability",
        "authorize_signing_credentials" => "Authorize signing credentials",
        "attach_app_interface" => "Attach app interface",
        "issue_app_authentication_token" => "Issue app auth token",
        "list_capability_grants" => "List capability grants",
        "list_dnas" => "List DNAs",
        "list_cell_ids" => "List cell IDs",
        "get_dna_definition" => "Get DNA definition",
        "dump_state" => "Dump state",
        "dump_full_state" => "Dump full state",

        // Admin
        "install_app" => "Install app",
        "enable_app" => "Enable app",
        "disable_app" => "Disable app",
        "uninstall_app" => "Uninstall app",
        "update_coordinators" => "Update coordinators",
        "delete_clone_cell" => "Delete clone cell",
        "add_agent_info" => "Add agent info",
        "revoke_agent_key" => "Revoke agent key",

        _ => "Unknown operation",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_operations() {
        assert!(is_operation_allowed("list_apps", PermissionLevel::Public));
        assert!(is_operation_allowed(
            "list_apps",
            PermissionLevel::Authenticated
        ));
        assert!(is_operation_allowed("list_apps", PermissionLevel::Admin));
    }

    #[test]
    fn test_authenticated_operations() {
        assert!(!is_operation_allowed(
            "generate_agent_pub_key",
            PermissionLevel::Public
        ));
        assert!(is_operation_allowed(
            "generate_agent_pub_key",
            PermissionLevel::Authenticated
        ));
        assert!(is_operation_allowed(
            "generate_agent_pub_key",
            PermissionLevel::Admin
        ));
    }

    #[test]
    fn test_admin_operations() {
        assert!(!is_operation_allowed(
            "install_app",
            PermissionLevel::Public
        ));
        assert!(!is_operation_allowed(
            "install_app",
            PermissionLevel::Authenticated
        ));
        assert!(is_operation_allowed("install_app", PermissionLevel::Admin));
    }

    #[test]
    fn test_unknown_operations_blocked() {
        assert!(!is_operation_allowed(
            "unknown_operation",
            PermissionLevel::Admin
        ));
        assert!(!is_operation_allowed(
            "hack_the_planet",
            PermissionLevel::Admin
        ));
    }

    #[test]
    fn test_permission_ordering() {
        assert!(PermissionLevel::Admin > PermissionLevel::Authenticated);
        assert!(PermissionLevel::Authenticated > PermissionLevel::Public);
    }
}
