import type { AppSyncProgressView } from "./generated/AppSyncProgressView";
/** Shared client contracts. Adapters implement these; declaration is not live delivery. */
import type { AppReleaseView } from "./generated/AppReleaseView";
import type { AppOpenInput } from "./generated/AppOpenInput";
import type { AppInstanceView } from "./generated/AppInstanceView";
import type { AppResidencyView } from "./generated/AppResidencyView";
import type { AppContinuationView } from "./generated/AppContinuationView";
import type { AppAttachmentInput } from "./generated/AppAttachmentInput";
import type { AppRouteView } from "./generated/AppRouteView";
import type { AppSyncDemandView } from "./generated/AppSyncDemandView";
import type { AppSyncUrgency } from "./generated/AppSyncUrgency";
import type { AppSyncTargetView } from "./generated/AppSyncTargetView";
import type { AppReleaseAnswer } from "./generated/AppReleaseAnswer";
import type { AppInstanceAnswer } from "./generated/AppInstanceAnswer";
import type { ObservationIntentView } from "./generated/ObservationIntentView";
import type { ObservationAcceptedView } from "./generated/ObservationAcceptedView";

export type {
  AppReleaseView,
  AppOpenInput,
  AppInstanceView,
  AppResidencyView,
  AppContinuationView,
  AppAttachmentInput,
  AppRouteView,
  AppSyncDemandView,
  AppSyncUrgency,
  AppSyncTargetView,
  AppSyncProgressView,
  AppReleaseAnswer,
  AppInstanceAnswer,
};

export interface AppRuntime {
  resolve(appId: string, target: string): Promise<AppReleaseAnswer>;
  ensureInstance(request: AppOpenInput): Promise<AppInstanceView>;
  observe(instanceId: string): Promise<AppInstanceAnswer>;
}
export interface AppResidency {
  acquire(release: AppReleaseView): Promise<AppResidencyView>;
  pin(releaseCid: string, keep: boolean): Promise<AppResidencyView>;
  archive(releaseCid: string): Promise<AppResidencyView>;
}
export interface AppContinuation {
  prepare(instanceId: string): Promise<AppContinuationView>;
  activate(
    target: AppInstanceView,
    checkpoint: AppContinuationView,
    operationId: string,
  ): Promise<void>;
}
export interface AppAttachment {
  attach(request: AppAttachmentInput): Promise<AppRouteView>;
  detach(instanceId: string): Promise<void>;
}
/** Append resolves only after checked local persistence. Receipt retains unsigned/as-asserted status. */
export interface AppObservationJournal {
  append(intent: ObservationIntentView): Promise<string>;
  acknowledge(
    pendingId: string,
    receipt: ObservationAcceptedView,
  ): Promise<void>;
}
/** Promotes existing authorized reconciliation, never a separate network path. */
export interface AppSync {
  progress(demandId: string): Promise<AppSyncProgressView[]>;
  request(demand: AppSyncDemandView): Promise<string>;
  release(demandId: string): Promise<void>;
}

export class UnsupportedCapabilityError extends Error {
  constructor(readonly capability: string) {
    super(`Unsupported capability: ${capability}`);
    this.name = "UnsupportedCapabilityError";
  }
}

/** A default adapter refuses. It does not fake a save, successful route, or empty observation. */
export class UnsupportedLifecycle
  implements
    AppRuntime,
    AppResidency,
    AppContinuation,
    AppAttachment,
    AppObservationJournal,
    AppSync
{
  async resolve(_appId: string, _target: string): Promise<AppReleaseAnswer> {
    throw new UnsupportedCapabilityError("app.resolve");
  }
  async ensureInstance(_request: AppOpenInput): Promise<AppInstanceView> {
    throw new UnsupportedCapabilityError("app.ensureInstance");
  }
  async observe(_instanceId: string): Promise<AppInstanceAnswer> {
    throw new UnsupportedCapabilityError("app.observe");
  }
  async acquire(_release: AppReleaseView): Promise<AppResidencyView> {
    throw new UnsupportedCapabilityError("app.acquire");
  }
  async pin(_releaseCid: string, _keep: boolean): Promise<AppResidencyView> {
    throw new UnsupportedCapabilityError("app.pin");
  }
  async archive(_releaseCid: string): Promise<AppResidencyView> {
    throw new UnsupportedCapabilityError("app.archive");
  }
  async prepare(_instanceId: string): Promise<AppContinuationView> {
    throw new UnsupportedCapabilityError("app.prepareContinuation");
  }
  async activate(
    _target: AppInstanceView,
    _checkpoint: AppContinuationView,
    _operationId: string,
  ): Promise<void> {
    throw new UnsupportedCapabilityError("app.activateContinuation");
  }
  async attach(_request: AppAttachmentInput): Promise<AppRouteView> {
    throw new UnsupportedCapabilityError("app.attach");
  }
  async detach(_instanceId: string): Promise<void> {
    throw new UnsupportedCapabilityError("app.detach");
  }
  async append(_intent: ObservationIntentView): Promise<string> {
    throw new UnsupportedCapabilityError("observation.append");
  }
  async acknowledge(
    _pendingId: string,
    _receipt: ObservationAcceptedView,
  ): Promise<void> {
    throw new UnsupportedCapabilityError("observation.acknowledge");
  }
  async request(_demand: AppSyncDemandView): Promise<string> {
    throw new UnsupportedCapabilityError("sync.request");
  }
  async progress(_demandId: string): Promise<AppSyncProgressView[]> {
    throw new UnsupportedCapabilityError("sync.progress");
  }
  async release(_demandId: string): Promise<void> {
    throw new UnsupportedCapabilityError("sync.release");
  }
}

export function availableOffline(residency: AppResidencyView): boolean {
  return (
    residency.artifactsVerified === true && residency.dataVerified === true
  );
}
