---
name: ethosengine-wedge-nfs-hardmount
title: ethosengine I/O wedge = NFS hard-mount deadlock
description: ethosengine wedge root cause = hard NFS4 mounts to in-cluster ClusterIP (server pod on SAME node); bites when diagnosing node hangs or rebooting ethosengine
metadata: 
  node_type: memory
  title: ethosengine I/O wedge = NFS hard-mount deadlock
  type: project
  originSessionId: 5a20ed95-5975-4026-a560-7bef7ffcb4f1
  modified: 2026-08-17T16:37:23.401Z
---

Root cause captured 2026-08-17 (operator node-debugging, stack trace in shutdown journal):
ethosengine holds ~11 `hard,timeo=600` NFS4 mounts to in-cluster ClusterIPs
(openebs-rwx servers for SonarQube + its Postgres, scheduled on ethosengine itself).
When kubelite stops (shutdown) or the NFS server pod dies/reschedules (live), the VIP
vanishes, hard mounts retry forever, `sync()` blocks in `folio_wait_writeback` (D-state,
holds `s_sync_lock`), and `ksys_sync`'s `iterate_supers` stalls flushes of the LOCAL
ext4 too — the node can't even flush its own disks to reboot. Power button is the only exit.

**Live hazard, not just shutdown**: the NFS server pod reschedules on its own; a restart
while the box is up starts the same D-state cascade under a running system. Wedge windows
correlated with CI wave dispatch — pod churn at wave start is a plausible perturbation.

Ruled OUT (each was a prior suspect): NVMe drives (no MCE/EDAC/AER; empty error log
because drives never hung), APST wake failure, kernel 6.8.0-137 regression, AMD-Vi
IO_PAGE_FAULT at boot (3x re-confirmed red herring), alpha dataplane write churn.

Still true and separate: (1) household peers' ~25 MB/s continuous idle churn was the
heal-starvation retry loop, fixed in the bf8819048 batch — watch the per-peer write floor
collapse post-deploy; feeds (2) NVMe wear at 122%/100% rated TBW — planned replacement,
not urgent. Mitigations kept: APST off, vm.dirty_bytes 1GiB/256MiB bound.

RESOLVED 2026-08-17: operator moved SonarQube's PVC from openebs-rwx (NFS) to
openebs-hostpath — no NFS mounts, no ClusterIP dependency, hazard removed at source.
Residual watch: any FUTURE openebs-rwx consumer scheduled on a node reintroduces the
class; the systemd shutdown unit (`After=snap.microk8s.daemon-kubelite.service`,
unmount kubelet nfs4 in ExecStop) remains the class-level backstop if rwx returns.

Related: [[cargo-pvc-disk-discipline]], [[ci-storage-topology]]
