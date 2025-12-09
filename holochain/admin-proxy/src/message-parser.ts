import { decode, encode } from '@msgpack/msgpack';

/**
 * Tagged message format used by Holochain admin API
 */
export interface TaggedMessage {
  type: string;
  data: unknown;
}

/**
 * Parse a MessagePack-encoded Holochain admin message.
 * Returns null if parsing fails.
 */
export function parseMessage(data: Buffer): TaggedMessage | null {
  try {
    const decoded = decode(data);

    // Holochain messages are objects with 'type' field
    if (
      typeof decoded === 'object' &&
      decoded !== null &&
      'type' in decoded &&
      typeof (decoded as Record<string, unknown>).type === 'string'
    ) {
      return decoded as TaggedMessage;
    }

    return null;
  } catch {
    return null;
  }
}

/**
 * Encode an error response in MessagePack format
 */
export function encodeError(message: string): Buffer {
  return Buffer.from(
    encode({
      type: 'error',
      data: { message },
    })
  );
}

/**
 * Get a human-readable description of the operation
 */
export function getOperationDescription(type: string): string {
  const descriptions: Record<string, string> = {
    list_apps: 'List installed apps',
    install_app: 'Install a new app',
    uninstall_app: 'Uninstall an app',
    enable_app: 'Enable an app',
    disable_app: 'Disable an app',
    generate_agent_pub_key: 'Generate agent public key',
    grant_zome_call_capability: 'Grant zome call capability',
    attach_app_interface: 'Attach app interface',
    issue_app_authentication_token: 'Issue app auth token',
  };

  return descriptions[type] ?? type;
}
