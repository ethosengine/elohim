import { createHash } from 'node:crypto';
import {
  closeSync,
  constants,
  fstatSync,
  openSync,
  readSync,
  realpathSync,
  statSync,
} from 'node:fs';

const MAX_FILE_BYTES = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES = 64 * 1024 * 1024;
const MAX_INPUT_FILES = 16;
const MAX_LINES = 100_000;
const MAX_OUTPUT_ROWS = 100;
const SQL_TIMING_ARTIFACT_MAX_BYTES = 2 * 1024 * 1024;
const WORKFLOW_ARTIFACT_MAX_BYTES = 2 * 1024 * 1024;
const SQL_TIMING_MAX_WINDOW_SECONDS = 900;
const SQL_TIMING_MAX_EVENTS = 10_000;
const SQL_TIMING_MAX_IDENTITIES = 4_096;
const SQL_TIMING_MAX_STATEMENT_BYTES = 64 * 1024;
const SQL_TIMING_MAX_SOURCE_SITES = 4;
const SQL_TIMING_CLOCK_TOLERANCE_SECONDS = 5;
const SQL_TIMING_SOURCE_SITES = new Set([
  'dht.publish.select_ready',
  'dht.publish.pending_count',
  'dht.publish.record_sent',
  'dht.validation.sys.pending_ops',
  'dht.validation.app.pending_ops',
]);
const UNKNOWN_SOURCE = 'aggregate-unknown';
const ANSI = new RegExp(String.raw`\u001B\[[0-?]*[ -/]*[@-~]`, 'g');

export interface EvidenceOptions {
  sqlxLogs: string[];
  eventLogs: string[];
  networkBefore?: string;
  networkAfter?: string;
}
interface BoundedFile {
  path: string;
  sha256: string;
  text: string;
  lineCount: number;
}
export interface ExternalEvidence {
  sqlxSlow: unknown[];
  sqlTiming: unknown[];
  diagnosticEvents: unknown[];
  workflowSummary: unknown[];
  networkPair: unknown | null;
  omitted: {
    sqlxSlow: number;
    sqlTiming: number;
    sqlTimingStatements: number;
    diagnosticEvents: number;
    workflowGroups: number;
  };
  issues: string[];
  limitations: string[];
}
export const emptyEvidenceOptions = (): EvidenceOptions => ({ sqlxLogs: [], eventLogs: [] });

interface WorkflowGroup {
  producer: string;
  cell: string;
  workflow: string;
  triggerClass: string;
  notifications: number;
  wakeups: number;
  runStarts: number;
  runFinishes: number;
  censoredRuns: number;
  outcomes: Record<string, number>;
  observedDurationSeconds: { count: number; mean: number | null; max: number | null };
  observedEventsPerMinute: number;
  notificationsPerMinute: number;
  wakeupsPerMinute: number;
  runStartsPerMinute: number;
  runFinishesPerMinute: number;
  observedRateWindowSeconds: number;
  rateBasis: 'diagnostic-active-window' | 'report-wall-time';
  completeness: 'provisional';
}

interface InspectedFile {
  path: string;
  size: number;
  device: bigint;
  inode: bigint;
}
function inspectInputs(paths: string[]): Map<string, InspectedFile> {
  if (paths.length > MAX_INPUT_FILES) throw new Error(`at most ${MAX_INPUT_FILES} evidence files`);
  const output = new Map<string, InspectedFile>();
  let total = 0;
  for (const supplied of paths) {
    const path = realpathSync(supplied);
    if (output.has(path)) throw new Error(`duplicate evidence file: ${path}`);
    const stat = statSync(path, { bigint: true });
    if (!stat.isFile() || stat.size <= 0n || stat.size > BigInt(MAX_FILE_BYTES))
      throw new Error(`evidence must be a regular file of 1..${MAX_FILE_BYTES} bytes: ${path}`);
    const size = Number(stat.size);
    total += size;
    if (total > MAX_TOTAL_BYTES)
      throw new Error(`evidence exceeds aggregate ${MAX_TOTAL_BYTES} byte limit`);
    output.set(path, { path, size, device: stat.dev, inode: stat.ino });
  }
  return output;
}

function readBounded(input: InspectedFile): BoundedFile {
  const flags = constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW;
  const fd = openSync(input.path, flags);
  try {
    const opened = fstatSync(fd, { bigint: true });
    if (
      !opened.isFile() ||
      opened.size !== BigInt(input.size) ||
      opened.dev !== input.device ||
      opened.ino !== input.inode
    )
      throw new Error(`evidence identity changed before reading: ${input.path}`);
    const buffer = Buffer.alloc(Math.min(MAX_FILE_BYTES + 1, input.size + 1));
    let offset = 0;
    while (offset < buffer.length) {
      const count = readSync(fd, buffer, offset, buffer.length - offset, null);
      if (!count) break;
      offset += count;
    }
    if (offset !== input.size || offset > MAX_FILE_BYTES)
      throw new Error(`evidence size changed while reading: ${input.path}`);
    const completed = fstatSync(fd, { bigint: true });
    if (
      !completed.isFile() ||
      completed.size !== BigInt(input.size) ||
      completed.dev !== input.device ||
      completed.ino !== input.inode
    )
      throw new Error(`evidence identity changed while reading: ${input.path}`);
    const bytes = buffer.subarray(0, offset);
    const text = bytes.toString('utf8');
    return {
      path: input.path,
      sha256: createHash('sha256').update(bytes).digest('hex'),
      text,
      lineCount: text.split(/\r?\n/).length,
    };
  } finally {
    closeSync(fd);
  }
}

function operationClass(statement: string): string {
  return (
    /^\s*(SELECT|INSERT|UPDATE|DELETE|PRAGMA)\b/i.exec(statement)?.[1]?.toUpperCase() ?? 'OTHER'
  );
}
function sqlShape(statement: string): string {
  return statement
    .replace(/'(?:''|[^'])*'/g, '?')
    .replace(/\b\d+(?:\.\d+)?\b/g, '?')
    .replace(/\s+/g, ' ')
    .trim();
}
function optionalCount(line: string, name: string): number | null {
  const raw = new RegExp(String.raw`\b${name}=(\d+)`).exec(line)?.[1];
  if (raw === undefined) return null;
  const value = Number(raw);
  return Number.isSafeInteger(value) && value >= 0 ? value : null;
}
function jsonCount(value: unknown): number | null {
  const parsed =
    typeof value === 'number' || typeof value === 'string' ? Number(value) : Number.NaN;
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
}
function parseSqlx(file: BoundedFile, start: number, end: number): unknown[] {
  const output: unknown[] = [];
  for (const rawLine of file.text.split(/\r?\n/)) {
    const line = rawLine.replace(ANSI, '');
    if (line.startsWith('{')) {
      let parsed: unknown;
      try {
        parsed = JSON.parse(line);
      } catch {
        throw new Error(`invalid SQLx JSONL: ${file.path}`);
      }
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) continue;
      const row = parsed as { timestamp?: unknown; time?: unknown; fields?: unknown };
      if (!row.fields || typeof row.fields !== 'object' || Array.isArray(row.fields)) continue;
      const fields = row.fields as Record<string, unknown>;
      if (
        typeof fields.message !== 'string' ||
        !fields.message.includes('slow statement: execution time exceeded alert threshold')
      )
        continue;
      const atUnixMs = Date.parse(scalarString(row.timestamp ?? row.time, 'timestamp', 64));
      if (!Number.isFinite(atUnixMs) || atUnixMs < start || atUnixMs > end) continue;
      const statement = scalarString(fields['db.statement'], 'db.statement', 1_000_000);
      const elapsedSeconds = finiteNumber(fields.elapsed_secs, 'elapsed_secs');
      const shape = sqlShape(statement);
      output.push({
        sourceSha256: file.sha256,
        atUnixMs,
        operation: operationClass(shape),
        shapeSha256: createHash('sha256').update(shape).digest('hex'),
        elapsedSeconds,
        rowsAffected: jsonCount(fields.rows_affected),
        rowsReturned: jsonCount(fields.rows_returned),
        slowOnly: true,
        sourceIdentity: UNKNOWN_SOURCE,
      });
      continue;
    }
    if (!line.includes('slow statement: execution time exceeded alert threshold')) continue;
    const atUnixMs = Date.parse(/^\S+/.exec(line)?.[0] ?? '');
    if (!Number.isFinite(atUnixMs) || atUnixMs < start || atUnixMs > end) continue;
    const elapsedRaw = /\belapsed_secs=([^\s]+)/.exec(line)?.[1];
    const statement = /\bdb\.statement="((?:\\.|[^"\\])*)"/.exec(line)?.[1];
    if (elapsedRaw === undefined || statement === undefined) continue;
    const elapsedSeconds = Number(elapsedRaw);
    if (!Number.isFinite(elapsedSeconds) || elapsedSeconds < 0) continue;
    const shape = sqlShape(statement.replace(/\\n/g, ' ').replace(/\\"/g, '"'));
    output.push({
      sourceSha256: file.sha256,
      atUnixMs,
      operation: operationClass(shape),
      shapeSha256: createHash('sha256').update(shape).digest('hex'),
      elapsedSeconds,
      rowsAffected: optionalCount(line, 'rows_affected'),
      rowsReturned: optionalCount(line, 'rows_returned'),
      slowOnly: true,
      sourceIdentity: UNKNOWN_SOURCE,
    });
  }
  output.sort(
    (a, b) =>
      (b as { elapsedSeconds: number }).elapsedSeconds -
      (a as { elapsedSeconds: number }).elapsedSeconds
  );
  return output;
}

interface SqlTimingStart {
  processId: number;
  producerId: string;
  startedUnixMs: number;
  requestedSeconds: number;
  eventLimit: number;
  identityLimit: number;
  statementByteLimit: number;
  sourceSiteLimit?: number;
}
interface SqlTimingMappingCounts {
  unattributedEvents: number;
  unmappedSourceSiteEvents: number;
  sourceSiteOverflowEvents: number;
}
interface SqlTimingMapping extends SqlTimingMappingCounts {
  sourceSiteIds: string[];
}
interface SqlTimingClosed {
  processId: number;
  producerId: string;
  startedUnixMs: number;
  requestedSeconds: number;
  endedUnixMs: number;
  observedSeconds: number;
  closureReason: string;
  coverageComplete: boolean;
  counters: Record<string, number>;
  sourceSiteLimit?: number;
  mappingCounts?: SqlTimingMappingCounts;
  statements: {
    hmac: string;
    count: number;
    totalElapsedSeconds: number;
    maxElapsedSeconds: number;
    mapping?: SqlTimingMapping;
  }[];
}

const SQL_TIMING_CLOSURES = new Set([
  'expiry',
  'event_limit',
  'arithmetic_overflow',
  'shutdown_before_expiry',
  'timer_error',
]);
const SQL_TIMING_COUNTERS = [
  'observed_events',
  'aggregated_events',
  'dropped_events',
  'missing_statement_events',
  'oversize_statement_events',
  'identity_overflow_events',
  'malformed_elapsed_events',
  'malformed_statement_events',
  'arithmetic_overflow_events',
] as const;

function directObject(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error(`invalid SQL timing ${label}`);
  return value as Record<string, unknown>;
}
function sqlTimingLimitations(value: unknown): void {
  if (!Array.isArray(value) || value.length > 16) throw new Error('invalid SQL timing limitations');
  for (const limitation of value) scalarString(limitation, 'sql timing limitation', 512);
}

/** Schema 1 supports both the original PID producer and the private re-arm native extension. */
function sqlTimingProducerV1(value: unknown, processId: number): string {
  const producerId = scalarString(value, 'sql timing producer_id', 128);
  const legacy = /^([0-9a-f]+)-[0-9a-f]+$/.exec(producerId);
  const privateRearm = /^sql-([0-9a-f]+)-[0-9a-f]+$/.exec(producerId);
  const encodedProcessId = (privateRearm ?? legacy)?.[1];
  if (!encodedProcessId || encodedProcessId !== processId.toString(16))
    throw new Error('invalid SQL timing producer identity');
  return producerId;
}

function sqlTimingStart(row: Record<string, unknown>): SqlTimingStart {
  if (integer(row.diagnostic_schema, 'sql timing diagnostic_schema') !== 1)
    throw new Error('unsupported SQL timing diagnostic_schema');
  if (
    scalarString(row.diagnostic_kind, 'sql timing diagnostic_kind', 64) !==
    'sql_timing_window_started'
  )
    throw new Error('invalid SQL timing start kind');
  const processId = integer(row.process_id, 'sql timing process_id');
  const producerId = sqlTimingProducerV1(row.producer_id, processId);
  const requestedSeconds = integer(row.requested_seconds, 'sql timing requested_seconds');
  const eventLimit = integer(row.event_limit, 'sql timing event_limit');
  const identityLimit = integer(row.identity_limit, 'sql timing identity_limit');
  const statementByteLimit = integer(row.statement_byte_limit, 'sql timing statement_byte_limit');
  const sourceSiteLimit =
    row.source_site_limit === undefined
      ? undefined
      : integer(row.source_site_limit, 'sql timing source_site_limit');
  if (
    processId < 1 ||
    requestedSeconds < 1 ||
    requestedSeconds > SQL_TIMING_MAX_WINDOW_SECONDS ||
    eventLimit < 1 ||
    eventLimit > SQL_TIMING_MAX_EVENTS ||
    identityLimit < 1 ||
    identityLimit > SQL_TIMING_MAX_IDENTITIES ||
    statementByteLimit < 1 ||
    statementByteLimit > SQL_TIMING_MAX_STATEMENT_BYTES ||
    (sourceSiteLimit !== undefined &&
      (sourceSiteLimit < 1 || sourceSiteLimit > SQL_TIMING_MAX_SOURCE_SITES))
  )
    throw new Error('SQL timing declared bounds exceed native limits');
  scalarString(row.scope, 'sql timing scope', 512);
  sqlTimingLimitations(row.limitations);
  return {
    processId,
    producerId,
    startedUnixMs: integer(row.started_unix_ms, 'sql timing started_unix_ms'),
    requestedSeconds,
    eventLimit,
    identityLimit,
    statementByteLimit,
    sourceSiteLimit,
  };
}
function sqlTimingMappingCounts(
  row: Record<string, unknown>,
  label: string
): SqlTimingMappingCounts | undefined {
  const fields = [
    'unattributed_events',
    'unmapped_source_site_events',
    'source_site_overflow_events',
  ] as const;
  const present = fields.map(field => row[field] !== undefined);
  if (!present.some(Boolean)) return undefined;
  if (!present.every(Boolean))
    throw new Error(`incomplete SQL timing ${label} source-site mapping`);
  return {
    unattributedEvents: integer(row.unattributed_events, `sql timing ${label} unattributed_events`),
    unmappedSourceSiteEvents: integer(
      row.unmapped_source_site_events,
      `sql timing ${label} unmapped_source_site_events`
    ),
    sourceSiteOverflowEvents: integer(
      row.source_site_overflow_events,
      `sql timing ${label} source_site_overflow_events`
    ),
  };
}
function sqlTimingMapping(
  row: Record<string, unknown>,
  label: string
): SqlTimingMapping | undefined {
  const counts = sqlTimingMappingCounts(row, label);
  const idsPresent = row.source_site_ids !== undefined;
  if (!counts && !idsPresent) return undefined;
  if (!counts || !idsPresent) throw new Error(`incomplete SQL timing ${label} source-site mapping`);
  const sourceSiteIds = row.source_site_ids;
  if (!Array.isArray(sourceSiteIds) || sourceSiteIds.length > SQL_TIMING_MAX_SOURCE_SITES)
    throw new Error(`invalid SQL timing ${label} source_site_ids`);
  const ids = sourceSiteIds.map(value =>
    scalarString(value, `sql timing ${label} source_site_id`, 128)
  );
  if (new Set(ids).size !== ids.length || ids.some(id => !SQL_TIMING_SOURCE_SITES.has(id)))
    throw new Error(`invalid SQL timing ${label} source_site_ids`);
  return {
    sourceSiteIds: ids,
    ...counts,
  };
}
function validateSqlTimingMappingCounts(
  mapping: SqlTimingMappingCounts,
  count: number,
  label: string
): number {
  const mapped =
    count -
    mapping.unattributedEvents -
    mapping.unmappedSourceSiteEvents -
    mapping.sourceSiteOverflowEvents;
  if (
    mapping.unattributedEvents > count ||
    mapping.unmappedSourceSiteEvents > count ||
    mapping.sourceSiteOverflowEvents > count ||
    mapped < 0
  )
    throw new Error(`SQL timing ${label} source-site counters exceed count`);
  return mapped;
}
function validateSqlTimingMapping(
  mapping: SqlTimingMapping,
  count: number,
  label: string,
  sourceSiteLimit = SQL_TIMING_MAX_SOURCE_SITES
): void {
  const mapped = validateSqlTimingMappingCounts(mapping, count, label);
  if (
    mapping.sourceSiteIds.length > sourceSiteLimit ||
    mapping.sourceSiteIds.length > mapped ||
    (mapped > 0 && mapping.sourceSiteIds.length === 0) ||
    (mapping.sourceSiteOverflowEvents > 0 &&
      mapping.sourceSiteIds.length !== SQL_TIMING_MAX_SOURCE_SITES)
  )
    throw new Error(`SQL timing ${label} source-site counters exceed count`);
}
function sqlTimingClosed(row: Record<string, unknown>): SqlTimingClosed {
  if (integer(row.diagnostic_schema, 'sql timing diagnostic_schema') !== 1)
    throw new Error('unsupported SQL timing diagnostic_schema');
  if (
    scalarString(row.diagnostic_kind, 'sql timing diagnostic_kind', 64) !==
    'sql_timing_window_closed'
  )
    throw new Error('invalid SQL timing close kind');
  const processId = integer(row.process_id, 'sql timing process_id');
  const producerId = sqlTimingProducerV1(row.producer_id, processId);
  if (processId < 1) throw new Error('invalid SQL timing producer identity');
  const requestedSeconds = integer(row.requested_seconds, 'sql timing requested_seconds');
  if (requestedSeconds < 1 || requestedSeconds > SQL_TIMING_MAX_WINDOW_SECONDS)
    throw new Error('SQL timing requested window exceeds native limit');
  scalarString(row.coverage_scope, 'sql timing coverage_scope', 512);
  sqlTimingLimitations(row.limitations);
  const closureReason = enumString(
    row.closure_reason,
    'sql timing closure_reason',
    SQL_TIMING_CLOSURES
  );
  const counters = Object.fromEntries(
    SQL_TIMING_COUNTERS.map(name => [name, integer(row[name], `sql timing ${name}`)])
  );
  const sourceSiteLimit =
    row.source_site_limit === undefined
      ? undefined
      : integer(row.source_site_limit, 'sql timing source_site_limit');
  if (
    sourceSiteLimit !== undefined &&
    (sourceSiteLimit < 1 || sourceSiteLimit > SQL_TIMING_MAX_SOURCE_SITES)
  )
    throw new Error('SQL timing declared source-site limit exceeds native limit');
  const mappingCounts = sqlTimingMappingCounts(row, 'window');
  const identities = row.identities;
  if (!Array.isArray(identities) || identities.length > SQL_TIMING_MAX_IDENTITIES)
    throw new Error('invalid SQL timing identities');
  const seen = new Set<string>();
  const statements = identities.map(value => {
    const identity = directObject(value, 'identity');
    const hmac = scalarString(
      identity.statement_hmac_sha256,
      'sql timing statement_hmac_sha256',
      64
    );
    if (!/^[0-9a-f]{64}$/.test(hmac) || seen.has(hmac))
      throw new Error('invalid or duplicate SQL timing statement identity');
    seen.add(hmac);
    const count = integer(identity.count, 'sql timing statement count');
    const totalElapsedSeconds = finiteNumber(
      identity.total_elapsed_seconds,
      'sql timing statement total_elapsed_seconds'
    );
    const maxElapsedSeconds = finiteNumber(
      identity.max_elapsed_seconds,
      'sql timing statement max_elapsed_seconds'
    );
    const statementMapping = sqlTimingMapping(identity, 'statement');
    for (const field of ['rows_affected', 'rows_returned']) {
      if (identity[field] !== null && identity[field] !== undefined)
        integer(identity[field], `sql timing statement ${field}`);
    }
    if (count < 1 || maxElapsedSeconds > totalElapsedSeconds)
      throw new Error('invalid SQL timing statement aggregate');
    if (maxElapsedSeconds < totalElapsedSeconds / count)
      throw new Error('invalid SQL timing statement max aggregate');
    if (statementMapping) validateSqlTimingMapping(statementMapping, count, 'statement');
    return { hmac, count, totalElapsedSeconds, maxElapsedSeconds, mapping: statementMapping };
  });
  const mappingPresent = statements.map(statement => statement.mapping !== undefined);
  if (mappingPresent.length > 0 && mappingPresent.some(Boolean) !== mappingPresent.every(Boolean))
    throw new Error('inconsistent SQL timing statement source-site mapping');
  if (statements.length > 0 && (mappingCounts !== undefined) !== mappingPresent.every(Boolean))
    throw new Error('inconsistent SQL timing window source-site mapping');
  if (mappingCounts) {
    validateSqlTimingMappingCounts(mappingCounts, counters.aggregated_events, 'window');
    for (const key of [
      'unattributedEvents',
      'unmappedSourceSiteEvents',
      'sourceSiteOverflowEvents',
    ] as const) {
      if (
        mappingCounts[key] !==
        statements.reduce((sum, statement) => sum + statement.mapping![key], 0)
      )
        throw new Error('SQL timing source-site counters do not balance');
    }
  }
  const accounted =
    counters.aggregated_events +
    counters.missing_statement_events +
    counters.oversize_statement_events +
    counters.identity_overflow_events +
    counters.malformed_elapsed_events +
    counters.malformed_statement_events +
    counters.arithmetic_overflow_events;
  if (
    accounted !== counters.observed_events ||
    statements.reduce((sum, statement) => sum + statement.count, 0) !== counters.aggregated_events
  )
    throw new Error('SQL timing event accounting does not balance');
  const coverageComplete = booleanValue(row.coverage_complete, 'sql timing coverage_complete');
  const noLoss =
    counters.observed_events === counters.aggregated_events &&
    SQL_TIMING_COUNTERS.slice(2).every(name => counters[name] === 0);
  if (coverageComplete !== (closureReason === 'expiry' && noLoss))
    throw new Error('SQL timing coverage_complete does not match terminal evidence');
  const endedUnixMs = integer(row.ended_unix_ms, 'sql timing ended_unix_ms');
  const startedUnixMs = integer(row.started_unix_ms, 'sql timing started_unix_ms');
  if (endedUnixMs < startedUnixMs) throw new Error('SQL timing close precedes start');
  const observedSeconds = finiteNumber(row.observed_seconds, 'sql timing observed_seconds');
  const wallSeconds = (endedUnixMs - startedUnixMs) / 1000;
  if (Math.abs(wallSeconds - observedSeconds) > SQL_TIMING_CLOCK_TOLERANCE_SECONDS)
    throw new Error('SQL timing clock witness exceeds tolerance');
  if (closureReason === 'expiry' && observedSeconds < requestedSeconds)
    throw new Error('SQL timing expiry closed before requested duration');
  return {
    processId,
    producerId,
    startedUnixMs,
    requestedSeconds,
    endedUnixMs,
    observedSeconds,
    closureReason,
    coverageComplete,
    counters,
    sourceSiteLimit,
    mappingCounts,
    statements,
  };
}
function sqlTimingKey(value: Pick<SqlTimingStart, 'processId' | 'producerId'>): string {
  return `${value.processId}:${value.producerId}`;
}
function parseSqlTiming(file: BoundedFile): unknown[] {
  const starts = new Map<string, SqlTimingStart>();
  const closes = new Map<string, SqlTimingClosed>();
  for (const rawLine of file.text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line.startsWith('{')) continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new Error(`invalid SQL timing JSONL: ${file.path}`);
    }
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) continue;
    const row = parsed as Record<string, unknown>;
    if (
      row.diagnostic_kind !== 'sql_timing_window_started' &&
      row.diagnostic_kind !== 'sql_timing_window_closed'
    )
      continue;
    if (row.diagnostic_kind === 'sql_timing_window_started') {
      const value = sqlTimingStart(row);
      const key = sqlTimingKey(value);
      if (closes.has(key)) throw new Error('SQL timing close precedes start record');
      if (starts.has(key))
        throw new Error('duplicate SQL timing lifecycle record for producer in file');
      starts.set(key, value);
    } else {
      const value = sqlTimingClosed(row);
      const key = sqlTimingKey(value);
      if (!starts.has(key)) throw new Error('SQL timing close precedes start record');
      if (closes.has(key))
        throw new Error('duplicate SQL timing lifecycle record for producer in file');
      closes.set(key, value);
    }
  }
  for (const key of new Set([...starts.keys(), ...closes.keys()])) {
    if (!starts.has(key) || !closes.has(key))
      throw new Error('unpaired SQL timing lifecycle record');
  }
  return [...starts.entries()].map(([key, start]) => {
    const closed = closes.get(key)!;
    if (
      closed.startedUnixMs !== start.startedUnixMs ||
      closed.requestedSeconds !== start.requestedSeconds ||
      closed.sourceSiteLimit !== start.sourceSiteLimit
    )
      throw new Error('SQL timing start and close do not match');
    if (closed.statements.length > start.identityLimit)
      throw new Error('SQL timing identities exceed declared identity limit');
    if (Object.values(closed.counters).some(value => value > start.eventLimit))
      throw new Error('SQL timing counter exceeds event limit');
    if (closed.mappingCounts && start.sourceSiteLimit === undefined)
      throw new Error('SQL timing source-site mapping lacks declared source-site limit');
    if (!closed.mappingCounts && start.sourceSiteLimit !== undefined)
      throw new Error('SQL timing declared source-site limit lacks mapping');
    if (closed.mappingCounts) {
      validateSqlTimingMappingCounts(
        closed.mappingCounts,
        closed.counters.aggregated_events,
        'window'
      );
      for (const statement of closed.statements)
        validateSqlTimingMapping(
          statement.mapping!,
          statement.count,
          'statement',
          start.sourceSiteLimit
        );
    }
    const statements = [...closed.statements]
      .sort(
        (left, right) =>
          right.totalElapsedSeconds - left.totalElapsedSeconds ||
          right.maxElapsedSeconds - left.maxElapsedSeconds ||
          right.count - left.count ||
          left.hmac.localeCompare(right.hmac)
      )
      .slice(0, MAX_OUTPUT_ROWS)
      .map(statement => ({
        statementRef: `capture-local-hmac-sha256:${statement.hmac}`,
        count: statement.count,
        totalElapsedSeconds: statement.totalElapsedSeconds,
        meanElapsedSeconds: statement.totalElapsedSeconds / statement.count,
        maxElapsedSeconds: statement.maxElapsedSeconds,
        ...(statement.mapping
          ? {
              sourceSiteIds: statement.mapping.sourceSiteIds,
              mappingCounts: {
                unattributedEvents: statement.mapping.unattributedEvents,
                unmappedSourceSiteEvents: statement.mapping.unmappedSourceSiteEvents,
                sourceSiteOverflowEvents: statement.mapping.sourceSiteOverflowEvents,
              },
            }
          : {}),
      }));
    return {
      requestedSeconds: closed.requestedSeconds,
      observedSeconds: closed.observedSeconds,
      closureReason: closed.closureReason,
      terminalState: closed.coverageComplete ? 'expiry-no-loss' : 'incomplete',
      completeness: 'provisional',
      counters: closed.counters,
      siteMapping: closed.mappingCounts ? 'available' : 'unavailable',
      statements,
      omittedStatements: closed.statements.length - statements.length,
      timingSemantics:
        'Elapsed time is SQLx QueryLogger lifetime and can include iterator or stream hold time.',
      limitations: [
        'Only sqlx::query logger events delivered to the native layer are represented.',
        'This capture identity is not bound to the report process or CPU sampling window.',
        'Producer coverage_complete is not full database-execution coverage.',
        'Statement references are capture-local HMACs and cannot be compared across windows.',
        ...(closed.mappingCounts
          ? [
              'Source-site identifiers are a bounded static allowlist; they do not expose SQL text or bind data.',
              'A statement HMAC can cover multiple source sites, so no per-site duration attribution is inferred.',
            ]
          : ['Source-site mapping is unavailable for this legacy capture.']),
      ],
    };
  });
}

export function summarizeSqlTimingArtifact(bytes: Buffer): unknown[] {
  if (bytes.length < 1 || bytes.length > SQL_TIMING_ARTIFACT_MAX_BYTES)
    throw new Error(`SQL timing artifact must contain 1..${SQL_TIMING_ARTIFACT_MAX_BYTES} bytes`);
  const text = bytes.toString('utf8');
  const lineCount = text.split(/\r?\n/).length;
  if (lineCount > MAX_LINES) throw new Error(`SQL timing artifact exceeds ${MAX_LINES} lines`);
  return parseSqlTiming({
    path: '<private SQL timing artifact>',
    sha256: createHash('sha256').update(bytes).digest('hex'),
    text,
    lineCount,
  });
}

const RUN_START = 'workflow.run.start';
const RUN_FINISH = 'workflow.run.finish';
const TRIGGER_NOTIFICATION = 'workflow.trigger.notification';
const TRIGGER_WAKEUP = 'workflow.trigger.wakeup';
const WORKFLOW_TRUNCATED = 'workflow.diagnostics.truncated';
const WORKFLOW_WINDOW_STARTED = 'workflow.diagnostics.window_started';
const WORKFLOW_WINDOW_CLOSED = 'workflow.diagnostics.window_closed';
const WORKFLOW_SCOPE = 'queue_consumer_workflow_tracing_attempts';
const WORKFLOW_LIMITATIONS = 'not_persisted_proof;not_cpu_coverage;not_full_workflow_coverage';
const WORKFLOW_MAX_SECONDS = 15 * 60;
const WORKFLOW_MAX_EVENTS = 10_000;
const WORKFLOW_CLOCK_TOLERANCE_SECONDS = 5;
const WORKFLOW_NATIVE_IDENTITY_FIELDS = [
  'native_process_schema',
  'process_start_ticks',
  'boot_id',
  'executable',
] as const;
const WORKFLOW_MAX_PRODUCERS_PER_FILE = 16;
const WORKFLOW_EVENTS = new Set([
  TRIGGER_NOTIFICATION,
  TRIGGER_WAKEUP,
  RUN_START,
  RUN_FINISH,
  WORKFLOW_TRUNCATED,
  WORKFLOW_WINDOW_STARTED,
  WORKFLOW_WINDOW_CLOSED,
]);
const STORAGE_KINDS = new Set([
  'db_query_start',
  'db_query_finish',
  'conductor_attempt_start',
  'conductor_attempt_finish',
  'events_dropped',
  'window_closed',
]);
const TRIGGER_CLASSES = new Set(['init', 'retrigger', 'retry', 'resume', 'external', 'loop']);
const WAKE_SOURCES = new Set(['notification', 'loop']);
const WORKFLOW_OUTCOMES = new Set([
  'complete',
  'incomplete',
  'retryable_error',
  'fatal_error',
  'dropped',
]);
const TRUNCATION_REASONS = new Set(['expiry', 'event_limit']);
const STORAGE_OUTCOMES = new Set(['success', 'error', 'caller_dropped']);
const DB_OPERATIONS = new Set(['capacity_report', 'unattributed']);
const DB_SITES = new Set([
  'capacity_resolve_custodian',
  'capacity_measure',
  'capacity_upsert',
  'unattributed',
]);

function scalarString(value: unknown, field: string, max = 128): string {
  if (
    typeof value !== 'string' ||
    !value.length ||
    value.length > max ||
    [...value].some(character => (character.codePointAt(0) ?? 0) < 32)
  )
    throw new Error(`invalid diagnostic ${field}`);
  return value;
}
function enumString(value: unknown, field: string, allowed: Set<string>): string {
  const parsed = scalarString(value, field, 64);
  if (!allowed.has(parsed)) throw new Error(`invalid diagnostic ${field}`);
  return parsed;
}
function finiteNumber(value: unknown, field: string): number {
  if ((typeof value !== 'number' && typeof value !== 'string') || value === '')
    throw new Error(`invalid diagnostic ${field}`);
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) throw new Error(`invalid diagnostic ${field}`);
  return parsed;
}
function integer(value: unknown, field: string): number {
  const parsed = finiteNumber(value, field);
  if (!Number.isSafeInteger(parsed)) throw new Error(`invalid diagnostic ${field}`);
  return parsed;
}
function booleanValue(value: unknown, field: string): boolean {
  if (value === true || value === 'true') return true;
  if (value === false || value === 'false') return false;
  throw new Error(`invalid diagnostic ${field}`);
}
function identifier(value: unknown, field: string): string {
  const parsed = scalarString(value, field);
  if (!/^[A-Za-z0-9_.:-]+$/.test(parsed)) throw new Error(`invalid diagnostic ${field}`);
  return parsed;
}

function workflowFields(fields: Record<string, unknown>, event: string): Record<string, unknown> {
  if (String(fields.telemetry_version ?? '') !== '1')
    throw new Error('unsupported workflow telemetry_version');
  const common = {
    telemetry_version: 1,
    process_id: integer(fields.process_id, 'process_id'),
    producer_id: identifier(fields.producer_id, 'producer_id'),
  };
  if (event === WORKFLOW_WINDOW_STARTED || event === WORKFLOW_WINDOW_CLOSED) {
    if (integer(fields.diagnostic_schema, 'workflow diagnostic_schema') !== 1)
      throw new Error('unsupported workflow diagnostic_schema');
    const processId = common.process_id;
    const producerId = common.producer_id;
    const producerMatch = /^workflow-([0-9a-f]+)-([0-9a-f]+)$/.exec(producerId);
    if (processId < 1 || producerMatch?.[1] !== processId.toString(16))
      throw new Error('invalid workflow producer identity');
    const requestedSeconds = integer(fields.requested_seconds, 'workflow requested_seconds');
    const eventLimit = integer(fields.event_limit, 'workflow event_limit');
    const generation = integer(fields.generation, 'workflow generation');
    const nonce = scalarString(fields.nonce, 'workflow nonce', 64);
    const outputBasename = scalarString(fields.output_basename, 'workflow output_basename', 166);
    if (
      requestedSeconds < 1 ||
      requestedSeconds > WORKFLOW_MAX_SECONDS ||
      eventLimit < 1 ||
      eventLimit > WORKFLOW_MAX_EVENTS ||
      generation < 1 ||
      generation > 16 ||
      !/^[A-Za-z0-9_-]{1,64}$/.test(nonce) ||
      outputBasename !== `workflow-g${String(generation).padStart(2, '0')}-${nonce}.jsonl`
    )
      throw new Error('workflow declared bounds exceed native limits');
    if (fields.scope !== WORKFLOW_SCOPE || fields.limitations !== WORKFLOW_LIMITATIONS)
      throw new Error('invalid workflow diagnostic scope');
    const nativePresent = WORKFLOW_NATIVE_IDENTITY_FIELDS.map(name => fields[name] !== undefined);
    let nativeIdentity: Record<string, unknown> | undefined;
    if (nativePresent.some(Boolean)) {
      if (!nativePresent.every(Boolean))
        throw new Error('incomplete workflow native process identity');
      const nativeProcessSchema = integer(
        fields.native_process_schema,
        'workflow native_process_schema'
      );
      const processStartTicks = integer(fields.process_start_ticks, 'workflow process_start_ticks');
      if (nativeProcessSchema !== 1 || processStartTicks < 1)
        throw new Error('invalid workflow native process identity');
      nativeIdentity = {
        native_process_schema: nativeProcessSchema,
        process_start_ticks: processStartTicks,
        boot_id: scalarString(fields.boot_id, 'workflow boot_id', 128),
        executable: scalarString(fields.executable, 'workflow executable', 4096),
      };
    }
    const startedClockPresent = fields.started_monotonic_ms !== undefined;
    const clockNamePresent = fields.monotonic_clock !== undefined;
    const endedClockPresent = fields.ended_monotonic_ms !== undefined;
    let monotonicWitness: Record<string, unknown> | undefined;
    if (startedClockPresent || clockNamePresent || endedClockPresent) {
      if (!startedClockPresent || !clockNamePresent || fields.monotonic_clock !== 'CLOCK_MONOTONIC')
        throw new Error('incomplete workflow monotonic clock witness');
      const startedMonotonicMs = finiteNumber(
        fields.started_monotonic_ms,
        'workflow started_monotonic_ms'
      );
      if (startedMonotonicMs <= 0) throw new Error('invalid workflow monotonic window');
      monotonicWitness = {
        monotonic_clock: 'CLOCK_MONOTONIC',
        started_monotonic_ms: startedMonotonicMs,
      };
      if (endedClockPresent) {
        if (event !== WORKFLOW_WINDOW_CLOSED)
          throw new Error('invalid workflow monotonic close witness');
        const endedMonotonicMs = finiteNumber(
          fields.ended_monotonic_ms,
          'workflow ended_monotonic_ms'
        );
        if (
          endedMonotonicMs <= 0 ||
          endedMonotonicMs < startedMonotonicMs ||
          endedMonotonicMs - startedMonotonicMs > requestedSeconds * 1000
        )
          throw new Error('invalid workflow monotonic window');
        monotonicWitness.ended_monotonic_ms = endedMonotonicMs;
      }
    }
    const lifecycle = {
      event,
      diagnostic_schema: 1,
      ...common,
      started_unix_ms: integer(fields.started_unix_ms, 'workflow started_unix_ms'),
      requested_seconds: requestedSeconds,
      event_limit: eventLimit,
      generation,
      nonce,
      output_basename: outputBasename,
      scope: WORKFLOW_SCOPE,
      limitations: WORKFLOW_LIMITATIONS,
      ...(nativeIdentity ? { _native_identity: nativeIdentity } : {}),
      ...(monotonicWitness ? { _monotonic_witness: monotonicWitness } : {}),
    };
    if (event === WORKFLOW_WINDOW_STARTED) return lifecycle;
    return {
      ...lifecycle,
      ended_unix_ms: integer(fields.ended_unix_ms, 'workflow ended_unix_ms'),
      observed_seconds: finiteNumber(fields.observed_seconds, 'workflow observed_seconds'),
      closure_reason: enumString(
        fields.closure_reason,
        'workflow closure_reason',
        TRUNCATION_REASONS
      ),
      reserved_detail_events: integer(fields.reserved_detail_events, 'reserved_detail_events'),
      emitted_detail_events: integer(fields.emitted_detail_events, 'emitted_detail_events'),
      remaining_detail_events: integer(fields.remaining_detail_events, 'remaining_detail_events'),
      rejected_detail_events_at_close: integer(
        fields.rejected_detail_events_at_close,
        'rejected_detail_events_at_close'
      ),
      admitted_runs: integer(fields.admitted_runs, 'admitted_runs'),
      finished_runs_before_close: integer(
        fields.finished_runs_before_close,
        'finished_runs_before_close'
      ),
      in_flight_runs_at_close: integer(fields.in_flight_runs_at_close, 'in_flight_runs_at_close'),
      censored_in_flight_runs: integer(fields.censored_in_flight_runs, 'censored_in_flight_runs'),
    };
  }
  if (event === WORKFLOW_TRUNCATED)
    return {
      event,
      ...common,
      reason: enumString(fields.reason, 'reason', TRUNCATION_REASONS),
      event_limit: integer(fields.event_limit, 'event_limit'),
      reserved_events: integer(fields.reserved_events, 'reserved_events'),
      dropped_events: integer(fields.dropped_events, 'dropped_events'),
      incomplete_runs: integer(fields.incomplete_runs, 'incomplete_runs'),
    };
  const output: Record<string, unknown> = {
    event,
    ...common,
    workflow: identifier(fields.workflow, 'workflow'),
    dna_hash: scalarString(fields.dna_hash, 'dna_hash', 256),
    cell_token:
      fields.cell_token === null || fields.cell_token === undefined
        ? null
        : integer(fields.cell_token, 'cell_token'),
    trigger_class: enumString(fields.trigger_class, 'trigger_class', TRIGGER_CLASSES),
  };
  if (event === TRIGGER_NOTIFICATION) {
    output.delivered = booleanValue(fields.delivered, 'delivered');
    return output;
  }
  output.wake_source = enumString(fields.wake_source, 'wake_source', WAKE_SOURCES);
  if (event === TRIGGER_WAKEUP)
    output.coalesced_observed = booleanValue(fields.coalesced_observed, 'coalesced_observed');
  if (event === RUN_START || event === RUN_FINISH) output.run_id = integer(fields.run_id, 'run_id');
  if (event === RUN_FINISH) {
    output.outcome = enumString(fields.outcome, 'outcome', WORKFLOW_OUTCOMES);
    output.duration_seconds = finiteNumber(fields.duration_seconds, 'duration_seconds');
  }
  return output;
}

interface DiagnosticEvent {
  sourceSha256: string;
  sourceIdentity: string;
  atUnixMs: number;
  fields: Record<string, unknown>;
}
interface ParsedEventFile {
  events: DiagnosticEvent[];
  strictGateSeconds: Map<string, number>;
  censoredRunKeys: Set<string>;
  censoredRuns: number;
}

export interface WorkflowArtifactWitness {
  nonce: string;
  generation: number;
  outputBasename: string;
  producerId: string;
  requestedSeconds: number;
  eventLimit: number;
  nativeProcess: {
    schema: 1;
    processId: number;
    processStartTicks: number;
    bootId: string;
    executable: string;
    clock: 'CLOCK_MONOTONIC';
  };
  startedMonotonicMs: number;
  endedMonotonicMs: number;
  closureReason: string;
}
const workflowProducerKey = (fields: Record<string, unknown>): string =>
  `${String(fields.process_id)}:${String(fields.producer_id)}`;
const workflowRunKey = (fields: Record<string, unknown>): string =>
  `${workflowProducerKey(fields)}:${String(fields.run_id)}`;

function validateWorkflowLifecycle(rows: DiagnosticEvent[]): Omit<ParsedEventFile, 'events'> {
  const controls = new Map<
    string,
    {
      start?: { row: DiagnosticEvent; index: number };
      close?: { row: DiagnosticEvent; index: number };
    }
  >();
  for (const [index, row] of rows.entries()) {
    const event = String(row.fields.event);
    if (event !== WORKFLOW_WINDOW_STARTED && event !== WORKFLOW_WINDOW_CLOSED) continue;
    const key = workflowProducerKey(row.fields);
    const pair = controls.get(key) ?? {};
    if (event === WORKFLOW_WINDOW_STARTED) {
      if (pair.start) throw new Error('duplicate workflow lifecycle start record');
      if (pair.close) throw new Error('workflow lifecycle close precedes start record');
      pair.start = { row, index };
    } else {
      if (!pair.start) throw new Error('workflow lifecycle close precedes start record');
      if (pair.close) throw new Error('duplicate workflow lifecycle close record');
      pair.close = { row, index };
    }
    controls.set(key, pair);
  }

  if (controls.size > WORKFLOW_MAX_PRODUCERS_PER_FILE)
    throw new Error(
      `at most ${WORKFLOW_MAX_PRODUCERS_PER_FILE} workflow lifecycle producers per file`
    );

  const strictGateSeconds = new Map<string, number>();
  const censoredRunKeys = new Set<string>();
  let censoredRuns = 0;
  for (const [key, pair] of controls) {
    if (!pair.start || !pair.close) throw new Error('missing workflow lifecycle terminal record');
    const start = pair.start.row.fields;
    const close = pair.close.row.fields;
    for (const field of [
      'process_id',
      'producer_id',
      'started_unix_ms',
      'requested_seconds',
      'event_limit',
      'generation',
      'nonce',
      'output_basename',
      'scope',
      'limitations',
    ]) {
      if (start[field] !== close[field])
        throw new Error('workflow lifecycle start and close do not match');
    }
    const startNative = start._native_identity as Record<string, unknown> | undefined;
    const closeNative = close._native_identity as Record<string, unknown> | undefined;
    if (Boolean(startNative) !== Boolean(closeNative))
      throw new Error('workflow lifecycle native identity does not match');
    if (startNative && closeNative && JSON.stringify(startNative) !== JSON.stringify(closeNative))
      throw new Error('workflow lifecycle native identity does not match');
    const startClock = start._monotonic_witness as Record<string, unknown> | undefined;
    const closeClock = close._monotonic_witness as Record<string, unknown> | undefined;
    if (Boolean(startClock) !== Boolean(closeClock))
      throw new Error('workflow lifecycle monotonic witness does not match');
    if (startClock && closeClock) {
      const closeBase = { ...closeClock };
      delete closeBase.ended_monotonic_ms;
      if (JSON.stringify(startClock) !== JSON.stringify(closeBase))
        throw new Error('workflow lifecycle monotonic witness does not match');
    }
    const startedUnixMs = Number(start.started_unix_ms);
    const endedUnixMs = Number(close.ended_unix_ms);
    const observedSeconds = Number(close.observed_seconds);
    const requestedSeconds = Number(close.requested_seconds);
    const wallSeconds = (endedUnixMs - startedUnixMs) / 1000;
    if (
      endedUnixMs < startedUnixMs ||
      Math.abs(wallSeconds - observedSeconds) > WORKFLOW_CLOCK_TOLERANCE_SECONDS
    )
      throw new Error('workflow lifecycle clock witness exceeds tolerance');
    if (close.closure_reason === 'expiry' && observedSeconds < requestedSeconds)
      throw new Error('workflow lifecycle expiry closed before requested duration');
    if (observedSeconds <= 0) throw new Error('workflow lifecycle has zero observed duration');

    const eventLimit = Number(close.event_limit);
    const reserved = Number(close.reserved_detail_events);
    const emitted = Number(close.emitted_detail_events);
    const remaining = Number(close.remaining_detail_events);
    const rejected = Number(close.rejected_detail_events_at_close);
    const admitted = Number(close.admitted_runs);
    const finished = Number(close.finished_runs_before_close);
    const inFlight = Number(close.in_flight_runs_at_close);
    const censored = Number(close.censored_in_flight_runs);
    if (
      [reserved, emitted, remaining, admitted, finished, inFlight, censored].some(
        value => value > eventLimit
      ) ||
      rejected > 2 ||
      reserved + remaining !== eventLimit ||
      emitted + censored !== reserved ||
      admitted !== finished + inFlight ||
      censored !== inFlight ||
      (close.closure_reason === 'event_limit' && remaining !== 0 && rejected <= remaining)
    )
      throw new Error('workflow lifecycle counters do not balance');

    const details = rows.filter((row, index) => {
      if (workflowProducerKey(row.fields) !== key) return false;
      const event = String(row.fields.event);
      if (event === WORKFLOW_WINDOW_STARTED || event === WORKFLOW_WINDOW_CLOSED) return false;
      if (event === WORKFLOW_TRUNCATED)
        throw new Error('legacy workflow truncation cannot be mixed with lifecycle controls');
      if (index <= pair.start!.index || index >= pair.close!.index)
        throw new Error('workflow lifecycle detail occurs outside controls');
      return true;
    });
    const starts = new Map<string, Record<string, unknown>>();
    const finishes = new Set<string>();
    let notifications = 0;
    let wakeups = 0;
    for (const detail of details) {
      const event = String(detail.fields.event);
      if (event === TRIGGER_NOTIFICATION) notifications += 1;
      else if (event === TRIGGER_WAKEUP) wakeups += 1;
      else if (event === RUN_START) {
        const runKey = workflowRunKey(detail.fields);
        if (starts.has(runKey)) throw new Error('duplicate workflow run start');
        starts.set(runKey, detail.fields);
      } else if (event === RUN_FINISH) {
        const runKey = workflowRunKey(detail.fields);
        const runStart = starts.get(runKey);
        if (!runStart || finishes.has(runKey)) throw new Error('unmatched workflow run finish');
        for (const field of [
          'workflow',
          'dna_hash',
          'cell_token',
          'trigger_class',
          'wake_source',
        ]) {
          if (runStart[field] !== detail.fields[field])
            throw new Error('workflow run start and finish do not match');
        }
        finishes.add(runKey);
      }
    }
    const unmatched = [...starts.keys()].filter(runKey => !finishes.has(runKey));
    if (
      details.length !== emitted ||
      starts.size !== admitted ||
      finishes.size !== finished ||
      unmatched.length !== censored ||
      notifications + wakeups + 2 * starts.size !== reserved ||
      notifications + wakeups + starts.size + finishes.size !== emitted
    )
      throw new Error('workflow lifecycle detail counters do not balance');
    unmatched.forEach(runKey => censoredRunKeys.add(runKey));
    censoredRuns += unmatched.length;
    // The terminal may be emitted late, but admission stops at the deadline.
    // Timer/sink scheduling delay must not dilute the active-window rate.
    strictGateSeconds.set(key, Math.min(observedSeconds, requestedSeconds));
  }
  return { strictGateSeconds, censoredRunKeys, censoredRuns };
}

/**
 * Validate one private workflow artifact and return only its native binding witness.
 * Detail rows are validated by the same lifecycle authority used for imported evidence,
 * but they are never copied into the resource capture's public summary.
 */
export function inspectWorkflowArtifact(bytes: Buffer): WorkflowArtifactWitness {
  if (bytes.length < 1 || bytes.length > WORKFLOW_ARTIFACT_MAX_BYTES)
    throw new Error(`workflow artifact must contain 1..${WORKFLOW_ARTIFACT_MAX_BYTES} bytes`);
  const text = bytes.toString('utf8');
  const lines = text.split(/\r?\n/);
  if (lines.length > MAX_LINES) throw new Error(`workflow artifact exceeds ${MAX_LINES} lines`);
  const sourceSha256 = createHash('sha256').update(bytes).digest('hex');
  const rows: DiagnosticEvent[] = [];
  for (const [index, line] of lines.entries()) {
    if (!line.trim()) continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new Error(`invalid workflow artifact JSONL at line ${index + 1}`);
    }
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
      throw new Error(`invalid workflow artifact row at line ${index + 1}`);
    const envelope = parsed as { time?: unknown; timestamp?: unknown; fields?: unknown };
    if (!envelope.fields || typeof envelope.fields !== 'object' || Array.isArray(envelope.fields))
      throw new Error(`invalid workflow artifact fields at line ${index + 1}`);
    const fields = envelope.fields as Record<string, unknown>;
    const event = typeof fields.event === 'string' ? fields.event : '';
    if (!WORKFLOW_EVENTS.has(event))
      throw new Error(
        `unexpected non-workflow row in private workflow artifact at line ${index + 1}`
      );
    const atUnixMs = Date.parse(scalarString(envelope.time ?? envelope.timestamp, 'timestamp', 64));
    if (!Number.isFinite(atUnixMs)) throw new Error('invalid workflow artifact timestamp');
    rows.push({
      sourceSha256,
      sourceIdentity: UNKNOWN_SOURCE,
      atUnixMs,
      fields: workflowFields(fields, event),
    });
  }
  validateWorkflowLifecycle(rows);
  const starts = rows.filter(row => row.fields.event === WORKFLOW_WINDOW_STARTED);
  const closes = rows.filter(row => row.fields.event === WORKFLOW_WINDOW_CLOSED);
  if (starts.length !== 1 || closes.length !== 1)
    throw new Error('workflow artifact must contain exactly one lifecycle start and close');
  const start = starts[0].fields;
  const close = closes[0].fields;
  const native = start._native_identity as Record<string, unknown> | undefined;
  const clock = start._monotonic_witness as Record<string, unknown> | undefined;
  const closeClock = close._monotonic_witness as Record<string, unknown> | undefined;
  if (!native || !clock || closeClock?.ended_monotonic_ms === undefined)
    throw new Error('workflow artifact lacks native identity or terminal monotonic witness');
  return {
    nonce: String(start.nonce),
    generation: Number(start.generation),
    outputBasename: String(start.output_basename),
    producerId: String(start.producer_id),
    requestedSeconds: Number(start.requested_seconds),
    eventLimit: Number(start.event_limit),
    nativeProcess: {
      schema: 1,
      processId: Number(start.process_id),
      processStartTicks: Number(native.process_start_ticks),
      bootId: String(native.boot_id),
      executable: String(native.executable),
      clock: 'CLOCK_MONOTONIC',
    },
    startedMonotonicMs: Number(clock.started_monotonic_ms),
    endedMonotonicMs: Number(closeClock.ended_monotonic_ms),
    closureReason: String(close.closure_reason),
  };
}

function storageFields(fields: Record<string, unknown>, kind: string): Record<string, unknown> {
  if (String(fields.diagnostic_schema ?? '') !== '1')
    throw new Error('unsupported diagnostic_schema');
  const common = {
    diagnostic_schema: 1,
    diagnostic_kind: kind,
    producer_pid: integer(fields.producer_pid, 'producer_pid'),
    producer_id: identifier(fields.producer_id, 'producer_id'),
  };
  if (kind === 'events_dropped')
    return {
      ...common,
      dropped_events: integer(fields.dropped_events, 'dropped_events'),
      event_limit: integer(fields.event_limit, 'event_limit'),
    };
  if (kind === 'window_closed')
    return {
      ...common,
      suppressed_terminals: integer(fields.suppressed_terminals, 'suppressed_terminals'),
    };
  const output: Record<string, unknown> = {
    ...common,
    correlation: integer(fields.correlation, 'correlation'),
  };
  if (kind.startsWith('db_query_')) {
    output.operation = enumString(fields.operation, 'operation', DB_OPERATIONS);
    output.query_ordinal = integer(fields.query_ordinal, 'query_ordinal');
    output.statement_site = enumString(fields.statement_site, 'statement_site', DB_SITES);
  } else {
    if (fields.operation !== 'conductor_call') throw new Error('invalid diagnostic operation');
    output.operation = 'conductor_call';
    output.attempt = integer(fields.attempt, 'attempt');
    output.zome = identifier(fields.zome, 'zome');
    output.function = identifier(fields.function, 'function');
    output.class = identifier(fields.class, 'class');
  }
  if (kind.endsWith('_finish')) {
    output.outcome = enumString(fields.outcome, 'outcome', STORAGE_OUTCOMES);
    output.elapsed_ms = finiteNumber(fields.elapsed_ms, 'elapsed_ms');
  }
  return output;
}
function parseEvents(file: BoundedFile, start: number, end: number): ParsedEventFile {
  const parsedRows: DiagnosticEvent[] = [];
  for (const [index, line] of file.text.split(/\r?\n/).entries()) {
    if (!line.trim()) continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new Error(`invalid diagnostic JSONL at ${file.path}:${index + 1}`);
    }
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
      throw new Error(`invalid diagnostic row at ${file.path}:${index + 1}`);
    const row = parsed as { time?: unknown; timestamp?: unknown; fields?: unknown };
    if (!row.fields || typeof row.fields !== 'object' || Array.isArray(row.fields))
      throw new Error(`invalid diagnostic fields at ${file.path}:${index + 1}`);
    const fields = row.fields as Record<string, unknown>;
    const event = typeof fields.event === 'string' ? fields.event : '';
    const kind = typeof fields.diagnostic_kind === 'string' ? fields.diagnostic_kind : '';
    if (!WORKFLOW_EVENTS.has(event) && !STORAGE_KINDS.has(kind)) continue;
    const atUnixMs = Date.parse(scalarString(row.time ?? row.timestamp, 'timestamp', 64));
    if (!Number.isFinite(atUnixMs)) throw new Error('invalid diagnostic timestamp');
    const safe = WORKFLOW_EVENTS.has(event)
      ? workflowFields(fields, event)
      : storageFields(fields, kind);
    parsedRows.push({
      sourceSha256: file.sha256,
      sourceIdentity: UNKNOWN_SOURCE,
      atUnixMs,
      fields: safe,
    });
  }
  const workflowRows = parsedRows.filter(row => WORKFLOW_EVENTS.has(String(row.fields.event)));
  const lifecycle = validateWorkflowLifecycle(workflowRows);
  const events = parsedRows
    .filter(row => {
      if (row.atUnixMs >= start && row.atUnixMs <= end) return true;
      return lifecycle.strictGateSeconds.has(workflowProducerKey(row.fields));
    })
    .map(row => {
      const fields = { ...row.fields };
      delete fields._native_identity;
      delete fields._monotonic_witness;
      delete fields.nonce;
      delete fields.output_basename;
      if (typeof fields.dna_hash === 'string') {
        const localIdentity = `${fields.dna_hash}\0${String(fields.cell_token ?? '')}`;
        fields.cell_ref = `capture-local-sha256:${createHash('sha256')
          .update(`${file.sha256}\0${localIdentity}`)
          .digest('hex')}`;
        delete fields.dna_hash;
        delete fields.cell_token;
      }
      return { ...row, fields };
    });
  return { events, ...lifecycle };
}

function finiteNonnegative(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
}
interface NetworkSnapshot {
  summary: Record<string, number | string | null>;
  membership: string | null;
}
function networkSummary(parsed: unknown): NetworkSnapshot {
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed))
    throw new Error('network evidence must be a JSON object');
  const root = parsed as Record<string, unknown>;
  const transport = root.transport_stats;
  if (transport && typeof transport === 'object' && !Array.isArray(transport)) {
    const value = transport as Record<string, unknown>;
    const connections = Array.isArray(value.connections) ? value.connections : [];
    const totals = [0, 0, 0, 0];
    const members: string[] = [];
    for (const connection of connections) {
      if (!connection || typeof connection !== 'object' || Array.isArray(connection))
        throw new Error('invalid network connection stats');
      const row = connection as Record<string, unknown>;
      const publicKey = scalarString(row.pub_key, 'network pub_key', 512);
      const openedAt = finiteNonnegative(row.opened_at_s);
      if (openedAt === null) throw new Error('invalid network connection opened_at_s');
      members.push(`${publicKey}:${openedAt}`);
      const values = [
        row.send_message_count,
        row.send_bytes,
        row.recv_message_count,
        row.recv_bytes,
      ].map(finiteNonnegative);
      if (values.some(item => item === null)) throw new Error('invalid network connection counter');
      values.forEach((item, index) => {
        totals[index] += item!;
      });
    }
    let blockedIncoming = 0;
    let blockedOutgoing = 0;
    const blocked = root.blocked_message_counts;
    if (blocked && typeof blocked === 'object' && !Array.isArray(blocked))
      for (const byDna of Object.values(blocked)) {
        if (!byDna || typeof byDna !== 'object' || Array.isArray(byDna)) continue;
        for (const count of Object.values(byDna as Record<string, unknown>)) {
          if (!count || typeof count !== 'object' || Array.isArray(count)) continue;
          const entry = count as Record<string, unknown>;
          const incoming = finiteNonnegative(entry.incoming);
          const outgoing = finiteNonnegative(entry.outgoing);
          if (incoming === null || outgoing === null)
            throw new Error('invalid blocked-message counter');
          blockedIncoming += incoming;
          blockedOutgoing += outgoing;
        }
      }
    members.sort();
    return {
      membership: createHash('sha256').update(members.join('\n')).digest('hex'),
      summary: {
        backend: typeof value.backend === 'string' ? value.backend.slice(0, 32) : null,
        connectionCount: connections.length,
        sendMessages: totals[0],
        sendBytes: totals[1],
        recvMessages: totals[2],
        recvBytes: totals[3],
        blockedIncoming,
        blockedOutgoing,
        pendingFetches: null,
      },
    };
  }
  let pendingFetches = 0;
  let spaces = 0;
  for (const metrics of Object.values(root)) {
    if (!metrics || typeof metrics !== 'object' || Array.isArray(metrics)) continue;
    const fetch = (metrics as Record<string, unknown>).fetch_state_summary;
    if (!fetch || typeof fetch !== 'object' || Array.isArray(fetch)) continue;
    const pending = (fetch as Record<string, unknown>).pending_requests;
    if (!pending || typeof pending !== 'object' || Array.isArray(pending)) continue;
    pendingFetches += Object.keys(pending).length;
    spaces += 1;
  }
  return {
    membership: null,
    summary: {
      backend: null,
      connectionCount: null,
      sendMessages: null,
      sendBytes: null,
      recvMessages: null,
      recvBytes: null,
      blockedIncoming: null,
      blockedOutgoing: null,
      pendingFetches: spaces ? pendingFetches : null,
    },
  };
}
function parseNetwork(before: BoundedFile, after: BoundedFile): unknown {
  const left = networkSummary(JSON.parse(before.text));
  const right = networkSummary(JSON.parse(after.text));
  const membershipMatched = left.membership !== null && left.membership === right.membership;
  const delta = Object.fromEntries(
    [
      'sendMessages',
      'sendBytes',
      'recvMessages',
      'recvBytes',
      'blockedIncoming',
      'blockedOutgoing',
    ].map(key => {
      const a = left.summary[key];
      const b = right.summary[key];
      return [
        key,
        membershipMatched && typeof a === 'number' && typeof b === 'number' && b >= a
          ? b - a
          : null,
      ];
    })
  );
  return {
    before: { sha256: before.sha256, summary: left.summary },
    after: { sha256: after.sha256, summary: right.summary },
    matchedConnectionAggregateDelta: delta,
    connectionMembershipMatched: membershipMatched,
    contemporaneityVerified: false,
    sourceIdentityMatched: false,
    coverageEligible: false,
    reason: 'dump bytes expose neither capture timestamps nor matchable process/binary identity',
  };
}

function summarizeWorkflow(
  events: unknown[],
  elapsedSeconds: number,
  strictGateSeconds = new Map<string, number>(),
  censoredRunKeys = new Set<string>()
): WorkflowGroup[] {
  const groups = new Map<
    string,
    Omit<
      WorkflowGroup,
      | 'observedDurationSeconds'
      | 'observedEventsPerMinute'
      | 'notificationsPerMinute'
      | 'wakeupsPerMinute'
      | 'runStartsPerMinute'
      | 'runFinishesPerMinute'
      | 'observedRateWindowSeconds'
      | 'rateBasis'
    > & {
      durations: number[];
    }
  >();
  for (const row of events as { fields?: Record<string, unknown> }[]) {
    const fields = row.fields ?? {};
    const event = String(fields.event ?? '');
    if (
      !WORKFLOW_EVENTS.has(event) ||
      event === WORKFLOW_TRUNCATED ||
      event === WORKFLOW_WINDOW_STARTED ||
      event === WORKFLOW_WINDOW_CLOSED
    )
      continue;
    const producer = `${String(fields.process_id)}:${String(fields.producer_id)}`;
    const cell = String(fields.cell_ref ?? 'capture-local-unknown');
    const workflow = String(fields.workflow);
    const triggerClass = String(fields.trigger_class);
    const key = `${producer}\0${cell}\0${workflow}\0${triggerClass}`;
    const group = groups.get(key) ?? {
      producer,
      cell,
      workflow,
      triggerClass,
      notifications: 0,
      wakeups: 0,
      runStarts: 0,
      runFinishes: 0,
      censoredRuns: 0,
      outcomes: {} as Record<string, number>,
      durations: [] as number[],
      completeness: 'provisional' as const,
    };
    if (event === TRIGGER_NOTIFICATION) group.notifications += 1;
    else if (event === TRIGGER_WAKEUP) group.wakeups += 1;
    else if (event === RUN_START) {
      group.runStarts += 1;
      if (censoredRunKeys.has(workflowRunKey(fields))) group.censoredRuns += 1;
    } else if (event === RUN_FINISH) {
      group.runFinishes += 1;
      const outcome = String(fields.outcome);
      group.outcomes[outcome] = (group.outcomes[outcome] ?? 0) + 1;
      if (typeof fields.duration_seconds === 'number')
        group.durations.push(fields.duration_seconds);
    }
    groups.set(key, group);
  }
  return [...groups.values()]
    .map(group => {
      const totalEvents = group.notifications + group.wakeups + group.runStarts + group.runFinishes;
      const durationTotal = group.durations.reduce((sum, value) => sum + value, 0);
      const rateWindowSeconds = strictGateSeconds.get(group.producer) ?? elapsedSeconds;
      return {
        producer: group.producer,
        cell: group.cell,
        workflow: group.workflow,
        triggerClass: group.triggerClass,
        notifications: group.notifications,
        wakeups: group.wakeups,
        runStarts: group.runStarts,
        runFinishes: group.runFinishes,
        censoredRuns: group.censoredRuns,
        outcomes: group.outcomes,
        observedDurationSeconds: {
          count: group.durations.length,
          mean: group.durations.length ? durationTotal / group.durations.length : null,
          max: group.durations.length ? Math.max(...group.durations) : null,
        },
        observedEventsPerMinute: (totalEvents * 60) / rateWindowSeconds,
        notificationsPerMinute: (group.notifications * 60) / rateWindowSeconds,
        wakeupsPerMinute: (group.wakeups * 60) / rateWindowSeconds,
        runStartsPerMinute: (group.runStarts * 60) / rateWindowSeconds,
        runFinishesPerMinute: (group.runFinishes * 60) / rateWindowSeconds,
        observedRateWindowSeconds: rateWindowSeconds,
        rateBasis: strictGateSeconds.has(group.producer)
          ? ('diagnostic-active-window' as const)
          : ('report-wall-time' as const),
        completeness: group.completeness,
      };
    })
    .sort((a, b) =>
      b.observedDurationSeconds.mean === a.observedDurationSeconds.mean
        ? b.observedEventsPerMinute - a.observedEventsPerMinute
        : (b.observedDurationSeconds.mean ?? -1) - (a.observedDurationSeconds.mean ?? -1)
    );
}

export function loadExternalEvidence(
  options: EvidenceOptions,
  window: { startUnixMs: number; endUnixMs: number }
): ExternalEvidence {
  if (Boolean(options.networkBefore) !== Boolean(options.networkAfter))
    throw new Error('--network-before and --network-after must be supplied together');
  const supplied = [...options.sqlxLogs, ...options.eventLogs];
  if (options.networkBefore) supplied.push(options.networkBefore, options.networkAfter!);
  const inspected = inspectInputs(supplied);
  const files = new Map<string, BoundedFile>();
  for (const path of supplied) files.set(path, readBounded(inspected.get(realpathSync(path))!));
  const unique = [...new Map([...files.values()].map(file => [file.path, file])).values()];
  if (unique.reduce((sum, file) => sum + file.lineCount, 0) > MAX_LINES)
    throw new Error(`evidence exceeds aggregate ${MAX_LINES} lines`);
  const sqlx = options.sqlxLogs.flatMap(path =>
    parseSqlx(files.get(path)!, window.startUnixMs, window.endUnixMs)
  );
  const sqlTiming = options.sqlxLogs.flatMap(path => parseSqlTiming(files.get(path)!)) as {
    statements: unknown[];
    omittedStatements: number;
  }[];
  const visibleSqlTiming = sqlTiming.slice(0, MAX_OUTPUT_ROWS);
  const sqlTimingStatementCount = sqlTiming.reduce(
    (sum, timing) =>
      sum +
      (timing as { statements: unknown[]; omittedStatements: number }).statements.length +
      (timing as { omittedStatements: number }).omittedStatements,
    0
  );
  const visibleSqlTimingStatementCount = visibleSqlTiming.reduce(
    (sum, timing) => sum + timing.statements.length,
    0
  );
  const parsedEventFiles = options.eventLogs.map(path =>
    parseEvents(files.get(path)!, window.startUnixMs, window.endUnixMs)
  );
  const events = parsedEventFiles.flatMap(parsed => parsed.events);
  const strictGateSeconds = new Map<string, number>();
  const censoredRunKeys = new Set<string>();
  let censoredRuns = 0;
  for (const parsed of parsedEventFiles) {
    for (const [producer, seconds] of parsed.strictGateSeconds) {
      if (strictGateSeconds.has(producer))
        throw new Error('duplicate workflow lifecycle producer across evidence files');
      strictGateSeconds.set(producer, seconds);
    }
    parsed.censoredRunKeys.forEach(key => censoredRunKeys.add(key));
    censoredRuns += parsed.censoredRuns;
  }
  const workflowRuns = new Map<string, { starts: number; finishes: number }>();
  const storageAttempts = new Map<string, { starts: number; finishes: number }>();
  let truncated = false;
  for (const row of events as {
    fields?: Record<string, unknown>;
  }[]) {
    const fields = row.fields ?? {};
    if (
      fields.event === WORKFLOW_TRUNCATED ||
      (fields.event === WORKFLOW_WINDOW_CLOSED && fields.closure_reason === 'event_limit') ||
      fields.diagnostic_kind === 'events_dropped' ||
      fields.diagnostic_kind === 'window_closed'
    )
      truncated = true;
    if (fields.event === RUN_START || fields.event === RUN_FINISH) {
      if (strictGateSeconds.has(workflowProducerKey(fields))) continue;
      const key = `${String(fields.process_id)}:${String(fields.producer_id)}:${String(fields.run_id)}`;
      const counts = workflowRuns.get(key) ?? { starts: 0, finishes: 0 };
      if (fields.event === RUN_START) counts.starts += 1;
      else counts.finishes += 1;
      workflowRuns.set(key, counts);
    }
    const kind = String(fields.diagnostic_kind ?? '');
    if (kind.endsWith('_start') || kind.endsWith('_finish')) {
      const family = kind.startsWith('db_query_') ? 'db_query' : 'conductor_attempt';
      const key = `${family}:${String(fields.producer_pid)}:${String(fields.producer_id)}:${String(fields.correlation)}:${String(fields.query_ordinal ?? fields.attempt)}`;
      const counts = storageAttempts.get(key) ?? { starts: 0, finishes: 0 };
      if (kind.endsWith('_start')) counts.starts += 1;
      else counts.finishes += 1;
      storageAttempts.set(key, counts);
    }
  }
  const invalidWorkflowRuns = [...workflowRuns.values()].filter(
    value => value.starts !== 1 || value.finishes !== 1
  ).length;
  const invalidStorageAttempts = [...storageAttempts.values()].filter(
    value => value.starts !== 1 || value.finishes !== 1
  ).length;
  const workflowGroups = summarizeWorkflow(
    events,
    (window.endUnixMs - window.startUnixMs) / 1000,
    strictGateSeconds,
    censoredRunKeys
  );
  return {
    sqlxSlow: sqlx.slice(0, MAX_OUTPUT_ROWS),
    sqlTiming: visibleSqlTiming,
    diagnosticEvents: events.slice(0, MAX_OUTPUT_ROWS),
    workflowSummary: workflowGroups.slice(0, MAX_OUTPUT_ROWS),
    networkPair: options.networkBefore
      ? parseNetwork(files.get(options.networkBefore)!, files.get(options.networkAfter!)!)
      : null,
    omitted: {
      sqlxSlow: Math.max(0, sqlx.length - MAX_OUTPUT_ROWS),
      sqlTiming: Math.max(0, sqlTiming.length - MAX_OUTPUT_ROWS),
      sqlTimingStatements: sqlTimingStatementCount - visibleSqlTimingStatementCount,
      diagnosticEvents: Math.max(0, events.length - MAX_OUTPUT_ROWS),
      workflowGroups: Math.max(0, workflowGroups.length - MAX_OUTPUT_ROWS),
    },
    issues: [
      ...(truncated ? ['diagnostics reported truncation; lifecycle evidence is incomplete'] : []),
      ...(invalidWorkflowRuns
        ? [`${invalidWorkflowRuns} workflow run(s) have duplicate or unmatched lifecycle evidence`]
        : []),
      ...(censoredRuns
        ? [`${censoredRuns} workflow run(s) were right-censored at bounded window close`]
        : []),
      ...(invalidStorageAttempts
        ? [
            `${invalidStorageAttempts} storage attempt(s) have duplicate or unmatched lifecycle evidence`,
          ]
        : []),
    ],
    limitations: [
      'SQLx input contains threshold-selected slow events only; it cannot satisfy complete SQL timing coverage.',
      'Diagnostic logs are not bound to the report process/binary identity and remain aggregate unknown-source evidence.',
      'Network dumps expose no capture timestamp or matchable process/binary identity and cannot satisfy network-watermark coverage.',
      'Statement sites are source call-site labels, not atomic SQL statement fingerprints.',
      'Workflow ranks use the bounded active gate duration (excluding late terminal-emission delay) when strict lifecycle controls are present, otherwise the supplied capture wall window; neither duration is sampled CPU time.',
      'Strict workflow imports retain the whole diagnostic gate even when it extends outside the report wall window; their rates must not be attributed to that report window.',
      'Workflow lifecycle evidence is startup-armed and cannot establish a later settled-household interval without a separately authorized bounded rearm control.',
      'No causal joins are inferred across evidence sources.',
    ],
  };
}
