import { decode, encode } from '@msgpack/msgpack';

/**
 * Envelope format used by @holochain/client
 * The client wraps all requests in this structure:
 * { id: number, type: "request", data: <encoded AdminRequest> }
 */
interface ClientEnvelope {
  id: number;
  type: string;
  data: Uint8Array;
}

/**
 * Tagged message format used by Holochain admin API
 * The inner AdminRequest has: { type: "list_apps", data: {...} }
 */
export interface TaggedMessage {
  type: string;
  data: unknown;
}

/**
 * Parse a MessagePack-encoded Holochain admin message.
 * The @holochain/client double-encodes messages:
 * - Outer: { id, type: "request", data: <encoded inner> }
 * - Inner: { type: "list_apps", data: {...} }
 *
 * Returns null if parsing fails.
 */
export function parseMessage(data: Buffer): TaggedMessage | null {
  try {
    const envelope = decode(data) as ClientEnvelope;

    // Check for client envelope format
    if (
      typeof envelope === 'object' &&
      envelope !== null &&
      envelope.type === 'request' &&
      envelope.data instanceof Uint8Array
    ) {
      // Decode the inner AdminRequest
      const innerRequest = decode(envelope.data);

      if (
        typeof innerRequest === 'object' &&
        innerRequest !== null &&
        'type' in innerRequest &&
        typeof (innerRequest as Record<string, unknown>).type === 'string'
      ) {
        return innerRequest as TaggedMessage;
      }
    }

    // Fallback: try direct parsing (for other clients)
    if (
      typeof envelope === 'object' &&
      envelope !== null &&
      'type' in envelope &&
      typeof (envelope as unknown as Record<string, unknown>).type === 'string'
    ) {
      return envelope as unknown as TaggedMessage;
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
