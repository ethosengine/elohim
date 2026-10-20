// Types for build-stage-task.mjs, for TypeScript requesters that name a pinned stage.
export interface BuildStageTaskInput {
  feature: string;
  requester: string;
  provider: string;
  out: string;
  runtimeImage?: string;
  executor?: string;
  repoRoot?: string;
  a2oDir?: string;
  nodeBin?: string;
  justBin?: string;
  writablePaths?: string[];
  cpuMillis?: number;
  memoryBytes?: number;
  timeoutSeconds?: number;
  env?: NodeJS.ProcessEnv;
  execute?: (program: string, args: string[]) => Promise<string>;
  probeRun?: boolean;
}

export declare function buildStageTask(input: BuildStageTaskInput): Promise<{
  envelope: Record<string, unknown>;
  taskPath: string;
  stagePath: string;
  runnerPath: string;
  dnaPath: string;
  featureSha256: string;
  scenarioNames: string[];
  scenarioSlugs: string[];
  expectedTests: string[];
  concern: string;
  sut: string;
  probe: { ok: boolean; message?: string; skipped?: string };
}>;
