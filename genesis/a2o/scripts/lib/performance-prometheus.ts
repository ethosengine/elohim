export interface PrometheusSummary {
  /** True only when parsing, identity verification, and every requested delta are issue-free. */
  valid: boolean;
  producerIdentity: { verified: boolean; changed: boolean; reason: string | null };
  counters: {
    name: string;
    labels: Record<string, string>;
    delta: number;
    perMinute: number;
  }[];
  histograms: {
    name: string;
    labels: Record<string, string>;
    count: number;
    totalMs?: number;
    mean: number | null;
    p50: number | null;
    p95: number | null;
    p99: number | null;
    unit: 'ms' | 'seconds' | 'unknown';
    quantilesApproximate: true;
    /** Quantiles are the upper bound of the first cumulative bucket reaching the rank. */
    quantileMethod: 'bucket-upper-bound';
  }[];
  gauges: {
    name: string;
    labels: Record<string, string>;
    before: number;
    after: number;
  }[];
  issues: string[];
}

interface Sample {
  name: string;
  labels: Record<string, string>;
  value: number;
}

type MetricType = 'counter' | 'gauge' | 'histogram' | 'summary' | 'untyped';

const NUMBER = String.raw`[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?`;
const SAMPLE = new RegExp(
  String.raw`^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{(.*)\})?\s+(${NUMBER}|NaN|[+-]?Inf)(?:\s+(-?\d+))?\s*$`
);
const LABEL = /\s*([a-zA-Z_]\w*)\s*=\s*"((?:\\[\\"n]|[^"\\])*)"\s*(?:,|$)/y;

function parseLabels(raw: string, line: number): Record<string, string> {
  if (!raw) return {};
  const labels: Record<string, string> = {};
  let position = 0;
  while (position < raw.length) {
    LABEL.lastIndex = position;
    const match = LABEL.exec(raw);
    if (!match) throw new Error(`line ${line}: invalid label set`);
    if (Object.hasOwn(labels, match[1]))
      throw new Error(`line ${line}: duplicate label ${match[1]}`);
    labels[match[1]] = match[2].replace(/\\([\\"n])/g, (_all, escaped: string) =>
      escaped === 'n' ? '\n' : escaped
    );
    position = LABEL.lastIndex;
  }
  return labels;
}

function parse(
  text: string,
  side: string,
  issues: string[],
  types: Map<string, MetricType>
): Map<string, Sample> {
  const samples = new Map<string, Sample>();
  for (const [offset, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (!line) continue;
    if (line.startsWith('# TYPE')) {
      const typeMatch =
        /^# TYPE ([a-zA-Z_:][a-zA-Z0-9_:]*) (counter|gauge|histogram|summary|untyped)$/.exec(line);
      if (!typeMatch) {
        issues.push(`${side} line ${offset + 1}: invalid Prometheus TYPE metadata`);
        continue;
      }
      const name = typeMatch[1];
      const type = typeMatch[2] as MetricType;
      const existing = types.get(name);
      if (existing && existing !== type)
        issues.push(`${side} line ${offset + 1}: conflicting TYPE metadata for ${name}`);
      else types.set(name, type);
      continue;
    }
    if (line.startsWith('#')) continue;
    const match = SAMPLE.exec(line);
    if (!match) {
      issues.push(`${side} line ${offset + 1}: invalid Prometheus sample`);
      continue;
    }
    const value = Number(match[4]);
    if (!Number.isFinite(value)) {
      issues.push(`${side} line ${offset + 1}: ${match[1]} is not finite`);
      continue;
    }
    try {
      const labels = parseLabels(match[3] ?? '', offset + 1);
      const key = seriesKey(match[1], labels);
      if (samples.has(key)) issues.push(`${side} line ${offset + 1}: duplicate series ${key}`);
      else samples.set(key, { name: match[1], labels, value });
    } catch (error) {
      issues.push(`${side} ${String(error)}`);
    }
  }
  return samples;
}

function seriesKey(name: string, labels: Record<string, string>): string {
  return `${name}{${Object.entries(labels)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, value]) => `${key}=${JSON.stringify(value)}`)
    .join(',')}}`;
}

function withoutLe(labels: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(labels).filter(([key]) => key !== 'le'));
}

function paired(
  before: Map<string, Sample>,
  after: Map<string, Sample>,
  predicate: (sample: Sample) => boolean,
  issues: string[]
): [Sample, Sample][] {
  const pairs: [Sample, Sample][] = [];
  const keys = new Set([
    ...[...before].filter(([, sample]) => predicate(sample)).map(([key]) => key),
    ...[...after].filter(([, sample]) => predicate(sample)).map(([key]) => key),
  ]);
  for (const key of keys) {
    const left = before.get(key);
    const right = after.get(key);
    if (!left) issues.push(`new series in after scrape: ${key}`);
    else if (right) {
      pairs.push([left, right]);
    } else {
      issues.push(`series missing from after scrape: ${key}`);
    }
  }
  return pairs;
}

function histogramUnit(name: string): 'ms' | 'seconds' | 'unknown' {
  if (name.endsWith('_ms')) return 'ms';
  if (name.endsWith('_seconds')) return 'seconds';
  return 'unknown';
}

function quantile(buckets: [number, number][], count: number, fraction: number): number | null {
  if (count <= 0) return null;
  const target = count * fraction;
  for (const [bound, cumulative] of buckets) {
    if (cumulative >= target) return Number.isFinite(bound) ? bound : null;
  }
  return null;
}

export function summarizePrometheusWindow(
  beforeText: string,
  afterText: string,
  elapsedSeconds: number
): PrometheusSummary {
  const issues: string[] = [];
  const result: PrometheusSummary = {
    valid: false,
    producerIdentity: { verified: false, changed: false, reason: 'not evaluated' },
    counters: [],
    histograms: [],
    gauges: [],
    issues,
  };
  if (!Number.isFinite(elapsedSeconds) || elapsedSeconds <= 0) {
    issues.push('elapsedSeconds must be finite and greater than zero');
    return result;
  }
  const beforeTypes = new Map<string, MetricType>();
  const afterTypes = new Map<string, MetricType>();
  const before = parse(beforeText, 'before', issues, beforeTypes);
  const after = parse(afterText, 'after', issues, afterTypes);
  for (const name of new Set([...beforeTypes.keys(), ...afterTypes.keys()])) {
    const left = beforeTypes.get(name);
    const right = afterTypes.get(name);
    if (left && right && left !== right)
      issues.push(`TYPE metadata changed for ${name}: ${left} to ${right}`);
  }
  const declaredType = (name: string): MetricType | undefined =>
    beforeTypes.get(name) ?? afterTypes.get(name);
  const isHistogramBucket = (sample: Sample): boolean => {
    if (!sample.name.endsWith('_bucket')) return false;
    const baseName = sample.name.slice(0, -'_bucket'.length);
    const directType = declaredType(sample.name);
    const baseType = declaredType(baseName);
    if (directType !== undefined) return directType === 'histogram';
    if (baseType !== undefined) return baseType === 'histogram';
    return true;
  };
  const beforeStarts = [...before.values()].filter(
    sample => sample.name === 'process_start_time_seconds'
  );
  const afterStarts = [...after.values()].filter(
    sample => sample.name === 'process_start_time_seconds'
  );
  const starts = paired(
    before,
    after,
    sample => sample.name === 'process_start_time_seconds',
    issues
  );
  const sameMembership =
    beforeStarts.length === afterStarts.length && starts.length === beforeStarts.length;
  const restarted = starts.some(([left, right]) => left.value !== right.value);
  if (restarted) {
    result.producerIdentity = {
      verified: true,
      changed: true,
      reason: 'process_start_time_seconds changed',
    };
    issues.push('process_start_time_seconds changed; counter and histogram deltas suppressed');
  } else if (beforeStarts.length > 0 && sameMembership) {
    result.producerIdentity = { verified: true, changed: false, reason: null };
  } else {
    const reason = 'process_start_time_seconds missing or unpaired; deltas are provisional';
    result.producerIdentity = { verified: false, changed: false, reason };
    issues.push(reason);
  }

  if (!restarted) {
    for (const [left, right] of paired(
      before,
      after,
      sample =>
        declaredType(sample.name) === 'counter' ||
        (declaredType(sample.name) === undefined && sample.name.endsWith('_total')),
      issues
    )) {
      if (left.value < 0 || right.value < 0) {
        issues.push(`counter has negative value: ${seriesKey(left.name, left.labels)}`);
        continue;
      }
      const delta = right.value - left.value;
      if (delta < 0) {
        issues.push(`counter reset: ${seriesKey(left.name, left.labels)}`);
        continue;
      }
      result.counters.push({
        name: left.name,
        labels: left.labels,
        delta,
        perMinute: (delta * 60) / elapsedSeconds,
      });
    }

    const groups = new Map<string, { name: string; labels: Record<string, string> }>();
    for (const sample of [...before.values(), ...after.values()]) {
      if (!isHistogramBucket(sample)) continue;
      const name = sample.name.slice(0, -'_bucket'.length);
      const labels = withoutLe(sample.labels);
      groups.set(seriesKey(name, labels), { name, labels });
    }
    for (const group of groups.values()) {
      const countKey = seriesKey(`${group.name}_count`, group.labels);
      const leftCount = before.get(countKey);
      const rightCount = after.get(countKey);
      if (!leftCount || !rightCount) {
        issues.push(`histogram missing before/after count: ${seriesKey(group.name, group.labels)}`);
        continue;
      }
      if (leftCount.value < 0 || rightCount.value < 0) {
        issues.push(`histogram count is negative: ${seriesKey(group.name, group.labels)}`);
        continue;
      }
      const count = rightCount.value - leftCount.value;
      if (count < 0) {
        issues.push(`histogram count reset: ${seriesKey(group.name, group.labels)}`);
        continue;
      }
      const bucketPairs = paired(
        before,
        after,
        sample =>
          sample.name === `${group.name}_bucket` &&
          seriesKey(group.name, withoutLe(sample.labels)) === seriesKey(group.name, group.labels),
        issues
      );
      const leftBuckets = [...before.values()].filter(
        sample =>
          sample.name === `${group.name}_bucket` &&
          seriesKey(group.name, withoutLe(sample.labels)) === seriesKey(group.name, group.labels)
      );
      const rightBuckets = [...after.values()].filter(
        sample =>
          sample.name === `${group.name}_bucket` &&
          seriesKey(group.name, withoutLe(sample.labels)) === seriesKey(group.name, group.labels)
      );
      if (
        leftBuckets.length !== rightBuckets.length ||
        bucketPairs.length !== leftBuckets.length ||
        issues.some(
          issue => issue.includes('duplicate series') && issue.includes(`${group.name}_bucket{`)
        )
      ) {
        issues.push(`histogram bucket membership changed: ${seriesKey(group.name, group.labels)}`);
        continue;
      }
      const buckets: [number, number][] = [];
      const numericBounds = new Set<number>();
      let valid = true;
      for (const [left, right] of bucketPairs) {
        const delta = right.value - left.value;
        const rawBound = left.labels.le;
        const bound = rawBound === '+Inf' ? Infinity : Number(rawBound);
        if (
          left.value < 0 ||
          right.value < 0 ||
          delta < 0 ||
          Number.isNaN(bound) ||
          numericBounds.has(bound)
        )
          valid = false;
        numericBounds.add(bound);
        buckets.push([bound, delta]);
      }
      buckets.sort(([a], [b]) => a - b);
      for (let index = 1; index < buckets.length; index += 1) {
        if (buckets[index][1] < buckets[index - 1][1]) valid = false;
      }
      const infinity = buckets.find(([bound]) => bound === Infinity);
      if (infinity?.[1] !== count) valid = false;
      if (!valid) {
        issues.push(`invalid histogram bucket deltas: ${seriesKey(group.name, group.labels)}`);
        continue;
      }
      const leftSum = before.get(seriesKey(`${group.name}_sum`, group.labels));
      const rightSum = after.get(seriesKey(`${group.name}_sum`, group.labels));
      let sum: number | undefined;
      if (leftSum && rightSum) {
        sum = rightSum.value - leftSum.value;
        if (sum < 0 || !Number.isFinite(sum)) {
          issues.push(`histogram sum reset or invalid: ${seriesKey(group.name, group.labels)}`);
          continue;
        }
      } else {
        issues.push(`histogram sum missing from scrape: ${seriesKey(group.name, group.labels)}`);
      }
      const unit = histogramUnit(group.name);
      result.histograms.push({
        name: group.name,
        labels: group.labels,
        count,
        ...(sum !== undefined && unit !== 'unknown'
          ? { totalMs: unit === 'seconds' ? sum * 1000 : sum }
          : {}),
        mean: sum === undefined || count === 0 ? null : sum / count,
        p50: quantile(buckets, count, 0.5),
        p95: quantile(buckets, count, 0.95),
        p99: quantile(buckets, count, 0.99),
        unit,
        quantilesApproximate: true,
        quantileMethod: 'bucket-upper-bound',
      });
    }
  }

  const histogramParts = /_(?:bucket|count|sum)$/;
  for (const [left, right] of paired(
    before,
    after,
    sample =>
      declaredType(sample.name) === 'gauge' ||
      (declaredType(sample.name) === undefined &&
        !sample.name.endsWith('_total') &&
        !histogramParts.test(sample.name) &&
        /(?:_in_flight|_capacity|_queue_depth|_active|_usage|_utilization|_bytes)$/.test(
          sample.name
        )),
    issues
  )) {
    result.gauges.push({
      name: left.name,
      labels: left.labels,
      before: left.value,
      after: right.value,
    });
  }
  result.valid = issues.length === 0;
  return result;
}
