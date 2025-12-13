/**
 * Permission levels for admin operations
 */
export enum PermissionLevel {
  /** No authentication - read-only operations */
  PUBLIC = 0,
  /** Authenticated user - normal dev workflow operations */
  AUTHENTICATED = 1,
  /** Admin - destructive operations like install/uninstall */
  ADMIN = 2,
}

/**
 * Maps Holochain admin operations to required permission levels.
 * Operations not in this list are blocked by default.
 */
export const OPERATION_WHITELIST: Record<string, PermissionLevel> = {
  // Public - read only status queries
  list_apps: PermissionLevel.PUBLIC,
  list_app_interfaces: PermissionLevel.PUBLIC,
  agent_info: PermissionLevel.PUBLIC,
  storage_info: PermissionLevel.PUBLIC,
  dump_network_stats: PermissionLevel.PUBLIC,

  // Authenticated - normal dev workflow
  generate_agent_pub_key: PermissionLevel.AUTHENTICATED,
  grant_zome_call_capability: PermissionLevel.AUTHENTICATED,
  revoke_zome_call_capability: PermissionLevel.AUTHENTICATED,
  authorize_signing_credentials: PermissionLevel.AUTHENTICATED,
  attach_app_interface: PermissionLevel.AUTHENTICATED,
  issue_app_authentication_token: PermissionLevel.AUTHENTICATED,
  list_capability_grants: PermissionLevel.AUTHENTICATED,
  list_dnas: PermissionLevel.AUTHENTICATED,
  list_cell_ids: PermissionLevel.AUTHENTICATED,
  get_dna_definition: PermissionLevel.AUTHENTICATED,
  dump_state: PermissionLevel.AUTHENTICATED,
  dump_full_state: PermissionLevel.AUTHENTICATED,

  // Admin - dangerous/destructive operations
  install_app: PermissionLevel.ADMIN,
  enable_app: PermissionLevel.ADMIN,
  disable_app: PermissionLevel.ADMIN,
  uninstall_app: PermissionLevel.ADMIN,
  update_coordinators: PermissionLevel.ADMIN,
  delete_clone_cell: PermissionLevel.ADMIN,
  add_agent_info: PermissionLevel.ADMIN,
  revoke_agent_key: PermissionLevel.ADMIN,
};

/**
 * Check if an operation is allowed for the given permission level
 */
export function isOperationAllowed(
  operation: string,
  level: PermissionLevel
): boolean {
  const requiredLevel = OPERATION_WHITELIST[operation];

  // Unknown operations are blocked
  if (requiredLevel === undefined) {
    return false;
  }

  return level >= requiredLevel;
}

/**
 * Get the name of a permission level for logging
 */
export function getPermissionLevelName(level: PermissionLevel): string {
  switch (level) {
    case PermissionLevel.PUBLIC:
      return 'PUBLIC';
    case PermissionLevel.AUTHENTICATED:
      return 'AUTHENTICATED';
    case PermissionLevel.ADMIN:
      return 'ADMIN';
    default:
      return 'UNKNOWN';
  }
}
