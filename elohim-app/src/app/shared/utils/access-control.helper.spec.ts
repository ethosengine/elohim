import { canView, canEdit, canDelete, AccessControlEntity } from './access-control.helper';

describe('access-control helper', () => {
  const currentAgentId = 'agent-123';
  const otherAgentId = 'agent-456';

  describe('canView', () => {
    it('should allow viewing public entities', () => {
      const entity: AccessControlEntity = { visibility: 'public' };
      expect(canView(entity, currentAgentId)).toBe(true);
    });

    it('should allow owner to view their entity', () => {
      const entity: AccessControlEntity = {
        visibility: 'private',
        ownerId: currentAgentId
      };
      expect(canView(entity, currentAgentId)).toBe(true);
    });

    it('should allow viewing shared entities', () => {
      const entity: AccessControlEntity = {
        visibility: 'shared',
        ownerId: otherAgentId,
        sharedWith: [currentAgentId]
      };
      expect(canView(entity, currentAgentId)).toBe(true);
    });

    it('should allow collaborators to view', () => {
      const entity: AccessControlEntity = {
        visibility: 'private',
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'viewer' }]
      };
      expect(canView(entity, currentAgentId)).toBe(true);
    });

    it('should deny viewing private entities by non-owners', () => {
      const entity: AccessControlEntity = {
        visibility: 'private',
        ownerId: otherAgentId
      };
      expect(canView(entity, currentAgentId)).toBe(false);
    });

    it('should handle entities with extendedBy instead of ownerId', () => {
      const entity: AccessControlEntity = {
        visibility: 'private',
        extendedBy: currentAgentId
      };
      expect(canView(entity, currentAgentId)).toBe(true);
    });
  });

  describe('canEdit', () => {
    it('should allow owner to edit', () => {
      const entity: AccessControlEntity = {
        ownerId: currentAgentId
      };
      expect(canEdit(entity, currentAgentId)).toBe(true);
    });

    it('should allow editor collaborators to edit', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'editor' }]
      };
      expect(canEdit(entity, currentAgentId)).toBe(true);
    });

    it('should allow admin collaborators to edit', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'admin' }]
      };
      expect(canEdit(entity, currentAgentId)).toBe(true);
    });

    it('should deny viewers from editing', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'viewer' }]
      };
      expect(canEdit(entity, currentAgentId)).toBe(false);
    });

    it('should deny non-collaborators from editing', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId
      };
      expect(canEdit(entity, currentAgentId)).toBe(false);
    });

    it('should handle entities with extendedBy', () => {
      const entity: AccessControlEntity = {
        extendedBy: currentAgentId
      };
      expect(canEdit(entity, currentAgentId)).toBe(true);
    });
  });

  describe('canDelete', () => {
    it('should allow owner to delete', () => {
      const entity: AccessControlEntity = {
        ownerId: currentAgentId
      };
      expect(canDelete(entity, currentAgentId)).toBe(true);
    });

    it('should allow admin collaborators to delete', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'admin' }]
      };
      expect(canDelete(entity, currentAgentId)).toBe(true);
    });

    it('should deny editors from deleting', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'editor' }]
      };
      expect(canDelete(entity, currentAgentId)).toBe(false);
    });

    it('should deny viewers from deleting', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId,
        collaborators: [{ agentId: currentAgentId, role: 'viewer' }]
      };
      expect(canDelete(entity, currentAgentId)).toBe(false);
    });

    it('should deny non-collaborators from deleting', () => {
      const entity: AccessControlEntity = {
        ownerId: otherAgentId
      };
      expect(canDelete(entity, currentAgentId)).toBe(false);
    });

    it('should handle entities with extendedBy', () => {
      const entity: AccessControlEntity = {
        extendedBy: currentAgentId
      };
      expect(canDelete(entity, currentAgentId)).toBe(true);
    });
  });
});
