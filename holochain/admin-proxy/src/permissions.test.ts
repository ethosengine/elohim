import { describe, it, expect } from 'vitest';
import {
  PermissionLevel,
  isOperationAllowed,
  OPERATION_WHITELIST,
} from './permissions.js';

describe('permissions', () => {
  describe('isOperationAllowed', () => {
    it('allows public operations without authentication', () => {
      expect(isOperationAllowed('list_apps', PermissionLevel.PUBLIC)).toBe(
        true
      );
      expect(
        isOperationAllowed('list_app_interfaces', PermissionLevel.PUBLIC)
      ).toBe(true);
      expect(isOperationAllowed('agent_info', PermissionLevel.PUBLIC)).toBe(
        true
      );
      expect(isOperationAllowed('storage_info', PermissionLevel.PUBLIC)).toBe(
        true
      );
    });

    it('blocks authenticated operations for public users', () => {
      expect(
        isOperationAllowed('generate_agent_pub_key', PermissionLevel.PUBLIC)
      ).toBe(false);
      expect(
        isOperationAllowed('attach_app_interface', PermissionLevel.PUBLIC)
      ).toBe(false);
      expect(
        isOperationAllowed(
          'issue_app_authentication_token',
          PermissionLevel.PUBLIC
        )
      ).toBe(false);
    });

    it('allows authenticated operations for authenticated users', () => {
      expect(
        isOperationAllowed(
          'generate_agent_pub_key',
          PermissionLevel.AUTHENTICATED
        )
      ).toBe(true);
      expect(
        isOperationAllowed(
          'attach_app_interface',
          PermissionLevel.AUTHENTICATED
        )
      ).toBe(true);
      expect(
        isOperationAllowed(
          'issue_app_authentication_token',
          PermissionLevel.AUTHENTICATED
        )
      ).toBe(true);
      expect(
        isOperationAllowed(
          'grant_zome_call_capability',
          PermissionLevel.AUTHENTICATED
        )
      ).toBe(true);
    });

    it('blocks admin operations for authenticated users', () => {
      expect(
        isOperationAllowed('install_app', PermissionLevel.AUTHENTICATED)
      ).toBe(false);
      expect(
        isOperationAllowed('uninstall_app', PermissionLevel.AUTHENTICATED)
      ).toBe(false);
      expect(
        isOperationAllowed('enable_app', PermissionLevel.AUTHENTICATED)
      ).toBe(false);
      expect(
        isOperationAllowed('disable_app', PermissionLevel.AUTHENTICATED)
      ).toBe(false);
    });

    it('allows all operations for admin users', () => {
      expect(isOperationAllowed('install_app', PermissionLevel.ADMIN)).toBe(
        true
      );
      expect(isOperationAllowed('uninstall_app', PermissionLevel.ADMIN)).toBe(
        true
      );
      expect(isOperationAllowed('enable_app', PermissionLevel.ADMIN)).toBe(
        true
      );
      expect(isOperationAllowed('disable_app', PermissionLevel.ADMIN)).toBe(
        true
      );
      // Admin can also do authenticated and public operations
      expect(
        isOperationAllowed('generate_agent_pub_key', PermissionLevel.ADMIN)
      ).toBe(true);
      expect(isOperationAllowed('list_apps', PermissionLevel.ADMIN)).toBe(true);
    });

    it('blocks unknown operations', () => {
      expect(
        isOperationAllowed('unknown_operation', PermissionLevel.ADMIN)
      ).toBe(false);
      expect(
        isOperationAllowed('hack_the_planet', PermissionLevel.ADMIN)
      ).toBe(false);
    });
  });

  describe('OPERATION_WHITELIST', () => {
    it('has all expected public operations', () => {
      const publicOps = ['list_apps', 'list_app_interfaces', 'agent_info', 'storage_info', 'dump_network_stats'];
      for (const op of publicOps) {
        expect(OPERATION_WHITELIST[op]).toBe(PermissionLevel.PUBLIC);
      }
    });

    it('has all expected admin operations', () => {
      const adminOps = ['install_app', 'enable_app', 'disable_app', 'uninstall_app'];
      for (const op of adminOps) {
        expect(OPERATION_WHITELIST[op]).toBe(PermissionLevel.ADMIN);
      }
    });
  });
});
