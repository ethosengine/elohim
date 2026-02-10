#!/usr/bin/env npx tsx
/**
 * Debug Stream Client
 *
 * Connects to doorway's /debug/stream WebSocket endpoint and displays
 * real-time debug events from doorway and elohim-storage.
 *
 * Usage:
 *   npx tsx src/debug-stream.ts
 *   npm run debug:stream
 *
 * Environment:
 *   DOORWAY_URL - Doorway HTTP/WS URL (required)
 *   DOORWAY_API_KEY - API key for authentication
 *
 * Examples:
 *   # Connect to dev environment
 *   DOORWAY_URL='https://doorway-alpha.elohim.host' \
 *   DOORWAY_API_KEY='dev-elohim-auth-2024' \
 *   npm run debug:stream
 */

import WebSocket from 'ws';

// =============================================================================
// Configuration
// =============================================================================

const DOORWAY_URL = process.env.DOORWAY_URL;
const API_KEY = process.env.DOORWAY_API_KEY;

// Color codes for terminal
const colors = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
};

// =============================================================================
// Event Formatting
// =============================================================================

interface DebugEvent {
  timestamp: string;
  source: string;
  event_type: string;
  level: string;
  message: string;
  data?: Record<string, unknown>;
}

function formatTimestamp(iso: string): string {
  const date = new Date(iso);
  return date.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }) + '.' + date.getMilliseconds().toString().padStart(3, '0');
}

function getLevelColor(level: string): string {
  switch (level) {
    case 'error': return colors.red;
    case 'warn': return colors.yellow;
    case 'info': return colors.green;
    case 'debug': return colors.dim;
    default: return colors.white;
  }
}

function getSourceColor(source: string): string {
  switch (source) {
    case 'doorway': return colors.cyan;
    case 'storage': return colors.magenta;
    case 'conductor': return colors.blue;
    default: return colors.white;
  }
}

function formatEvent(event: DebugEvent): string {
  const time = colors.dim + formatTimestamp(event.timestamp) + colors.reset;
  const source = getSourceColor(event.source) + `[${event.source}]` + colors.reset;
  const level = getLevelColor(event.level);
  const message = level + event.message + colors.reset;

  let output = `${time} ${source} ${message}`;

  // Add data if present and not too large
  if (event.data && Object.keys(event.data).length > 0) {
    const dataStr = JSON.stringify(event.data);
    if (dataStr.length < 100) {
      output += colors.dim + ` ${dataStr}` + colors.reset;
    }
  }

  return output;
}

// =============================================================================
// WebSocket Connection
// =============================================================================

function connect() {
  if (!DOORWAY_URL) {
    console.error('Error: DOORWAY_URL not set');
    console.error('Usage: DOORWAY_URL=https://doorway-alpha.elohim.host npm run debug:stream');
    process.exit(1);
  }

  // Build WebSocket URL
  const wsBase = DOORWAY_URL
    .replace('https://', 'wss://')
    .replace('http://', 'ws://');

  const wsUrl = API_KEY
    ? `${wsBase}/debug/stream?apiKey=${encodeURIComponent(API_KEY)}`
    : `${wsBase}/debug/stream`;

  console.log(colors.cyan + '═'.repeat(60) + colors.reset);
  console.log(colors.cyan + '  DEBUG STREAM' + colors.reset);
  console.log(colors.cyan + '═'.repeat(60) + colors.reset);
  console.log(`${colors.dim}Connecting to: ${wsBase}/debug/stream${colors.reset}`);
  console.log(`${colors.dim}Press Ctrl+C to disconnect${colors.reset}`);
  console.log('');

  const ws = new WebSocket(wsUrl, {
    headers: {
      'Origin': 'http://localhost',
    },
  });

  ws.on('open', () => {
    console.log(colors.green + '✓ Connected to debug stream' + colors.reset);
    console.log('');

    // Send ping periodically to keep connection alive
    const pingInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ command: 'ping' }));
      }
    }, 30000);

    ws.on('close', () => {
      clearInterval(pingInterval);
    });
  });

  ws.on('message', (data) => {
    try {
      const event = JSON.parse(data.toString()) as DebugEvent;
      console.log(formatEvent(event));
    } catch (e) {
      // Raw message
      console.log(colors.dim + data.toString() + colors.reset);
    }
  });

  ws.on('error', (error) => {
    console.error(colors.red + `WebSocket error: ${error.message}` + colors.reset);
  });

  ws.on('close', (code, reason) => {
    console.log('');
    console.log(colors.yellow + `Disconnected (code: ${code}, reason: ${reason || 'none'})` + colors.reset);

    // Reconnect after 5 seconds
    console.log(colors.dim + 'Reconnecting in 5 seconds...' + colors.reset);
    setTimeout(connect, 5000);
  });
}

// =============================================================================
// Main
// =============================================================================

// Handle Ctrl+C gracefully
process.on('SIGINT', () => {
  console.log('\n' + colors.dim + 'Disconnecting...' + colors.reset);
  process.exit(0);
});

// Start connection
connect();
