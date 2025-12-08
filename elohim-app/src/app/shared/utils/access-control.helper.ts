/**
 * Utility for common access control patterns across services
 */

export interface AccessControlEntity {
  visibility?: 'public' | 'private' | 'shared' | 'default';
  ownerId?: string;
  extendedBy?: string; // for PathExtension
  sharedWith?: string[];
  collaborators?: Array<{ agentId: string; role: string }>;
}

/**
 * Check if an agent can view an entity based on visibility and ownership
 * @param entity - Entity to check access for
 * @param currentAgentId - ID of the current agent
 * @returns True if the agent can view the entity
 */
export function canView(entity: AccessControlEntity, currentAgentId: string): boolean {
  // Public entities are viewable by everyone
  if (entity.visibility === 'public') {
    return true;
  }

  // Owner can always view
  const ownerId = entity.ownerId ?? entity.extendedBy;
  if (ownerId === currentAgentId) {
    return true;
  }

  // Shared entities can be viewed by shared agents
  if (entity.visibility === 'shared' && entity.sharedWith?.includes(currentAgentId)) {
    return true;
  }

  // Collaborators can view
  if (entity.collaborators?.some(c => c.agentId === currentAgentId)) {
    return true;
  }

  return false;
}

/**
 * Check if an agent can edit an entity
 * @param entity - Entity to check access for
 * @param currentAgentId - ID of the current agent
 * @returns True if the agent can edit the entity
 */
export function canEdit(entity: AccessControlEntity, currentAgentId: string): boolean {
  // Owner can always edit
  const ownerId = entity.ownerId ?? entity.extendedBy;
  if (ownerId === currentAgentId) {
    return true;
  }

  // Collaborators with 'editor' or 'admin' role can edit
  if (entity.collaborators?.some(c =>
    c.agentId === currentAgentId && (c.role === 'editor' || c.role === 'admin')
  )) {
    return true;
  }

  return false;
}

/**
 * Check if an agent can delete an entity
 * @param entity - Entity to check access for
 * @param currentAgentId - ID of the current agent
 * @returns True if the agent can delete the entity
 */
export function canDelete(entity: AccessControlEntity, currentAgentId: string): boolean {
  // Only owner can delete
  const ownerId = entity.ownerId ?? entity.extendedBy;
  if (ownerId === currentAgentId) {
    return true;
  }

  // Collaborators with 'admin' role can delete
  if (entity.collaborators?.some(c =>
    c.agentId === currentAgentId && c.role === 'admin'
  )) {
    return true;
  }

  return false;
}
