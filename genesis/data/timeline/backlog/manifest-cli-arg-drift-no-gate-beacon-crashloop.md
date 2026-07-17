---
title: Manifest args can drift from a binary's CLI with no gate — beacon-init CrashLooped 93× on --state-dir vs --state-file
created: 2026-07-17
status: OPEN
domain: D-ci
source: overnight shift p2p-dataplane-resiliency-convergence (C1 root-cause, 2026-07-17)
severity: medium
---

The relay-addr-beacon image built green in CI while its own deployment manifests
could not start it: `alpha-coturn-{operations,shem}.yaml` passed `--state-dir`
but the binary's clap surface takes `--state-file`. The init container CrashLooped
~93 times (kubelet backoff), coturn never started, TURN DNS never appeared — and
nothing red pointed at the manifests: the edge deploy's `kubectl apply` printed
"unchanged" and moved on. Found only via Loki pod logs. Fixed at `ecf2b3d43`
(explicit `--state-file /var/lib/relay-addr-beacon/state.json` ×4).

Durable fix candidate: a CI check that validates manifest `args:` against the
built binary's `--help` for images this repo builds and deploys (the beacon is
the first instance; any future sidecar/init binary has the same exposure).
Cheap shape: in the image-build stage, run `<binary> --help`, extract long flags,
grep the repo's manifests that reference the image for `--` args not in the set.

Related lesson (same night, same class): a green image + "unchanged" apply is a
silent non-deploy — rollout-triggering requires a manifest diff.
