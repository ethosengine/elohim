# Runbook — Self-hosted MinIO for sccache + future quilt swap (2026-05-09)

**Target cluster:** the K8s cluster hosting Jenkins build pods (`jenkins` ns) and Eclipse Che workspaces (per-user Che ns).
**Target namespace for MinIO:** `ethosengine`
**Storage node:** hp-micro10 (4TB disk, openebs-jiva-csi-default).
**Risk to running services:** None during install. Existing Jenkins builds and Che workspaces continue using local target/ caches until images are updated to point at sccache (separate follow-up).

## Why MinIO, not AWS S3, not anything-else

We need an S3-API-shaped blob store available cluster-internally. Constraints:

1. **Stay on our hardware.** External S3 (AWS, R2, GCS) couples our CI to a rented substrate the protocol exists to subsume. Out.
2. **Single endpoint shared between Jenkins build pods and Che workspaces.** Two consumers, one cache surface. Cross-namespace network is fine; cross-namespace PVC mounting is not.
3. **Match the future quilt API surface.** Quilt is the elohim-native S3-shape over content-addressed blobs (per the `project_quilt_as_native_s3_surface` memory). Today's MinIO bucket-key model maps to tomorrow's CID-prefix model with an endpoint-URL swap; sccache's S3 v4 client doesn't care.

MinIO satisfies all three. Helm-installable. Backing storage on hp-micro10 via openebs-jiva-csi-default (same pattern as the nix-cache PVCs from the previous runbook).

## What this runbook does NOT do

- Update the ci-builder-nix image (Jenkins) — that's an `ethosengine/che-devworkspaces` repo change, separate follow-up after MinIO is verified up.
- Update the Eclipse Che workspace image — same separate follow-up.
- Delete or redirect any existing local caches — those keep working until images are explicitly switched.

## Apply

### Step 1 — Add Bitnami helm repo (skip if already added)

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo update bitnami
```

### Step 2 — Generate root credentials and store as a K8s secret

These will be created BEFORE the MinIO install so the chart can read them rather than auto-generating ephemeral creds we can't recover.

```bash
ROOT_USER="elohim-sccache-admin"
ROOT_PASSWORD=$(openssl rand -base64 32 | tr -d '\n=' | tr '/+' '-_' | head -c 32)

kubectl create namespace ethosengine --dry-run=client -o yaml | kubectl apply -f -

kubectl -n ethosengine create secret generic minio-root-credentials \
    --from-literal=root-user="${ROOT_USER}" \
    --from-literal=root-password="${ROOT_PASSWORD}" \
    --dry-run=client -o yaml | kubectl apply -f -

# Save the password somewhere durable (1Password / vault). Other secrets
# below need it; if it's lost MinIO can be recreated but cached blobs go
# with it.
echo "ROOT_USER=${ROOT_USER}"
echo "ROOT_PASSWORD=${ROOT_PASSWORD}"
```

### Step 3 — Helm install MinIO

Single-replica MinIO is fine for sccache (build-cache loss is recoverable; first-build-after-loss just becomes a cold compile, no business risk). Distributed mode is overkill for this use case.

```bash
helm install minio bitnami/minio \
    --namespace ethosengine \
    --version 17.0.20 \
    --set mode=standalone \
    --set persistence.enabled=true \
    --set persistence.storageClass=openebs-jiva-csi-default \
    --set persistence.size=200Gi \
    --set auth.existingSecret=minio-root-credentials \
    --set auth.rootUserSecretKey=root-user \
    --set auth.rootPasswordSecretKey=root-password \
    --set service.type=ClusterIP \
    --set service.ports.api=9000 \
    --set service.ports.console=9001 \
    --set provisioning.enabled=true \
    --set provisioning.buckets[0].name=sccache-elohim \
    --set provisioning.buckets[0].versioning=false \
    --set provisioning.buckets[0].withLock=false
```

The chart auto-creates the `sccache-elohim` bucket via the provisioning job. Versioning is OFF — sccache content is content-addressed (cargo computes the hash), so versions add cost without adding value.

Notes on chart values:
- `mode=standalone` → one replica, simpler ops. Distributed mode requires ≥4 PVCs and adds replication overhead we don't need for cache data.
- `persistence.size=200Gi` → headroom for ~6 months of dense Rust build artifacts across all branches. Adjust if growth telemetry says otherwise.
- `service.type=ClusterIP` → MinIO is internal-only. No ingress; no public exposure of the cache.

### Step 4 — Distribute consumer credentials (read/write to cache bucket only, NOT root)

Root creds should never be used by build pods. Create a scoped IAM-style user with access only to `sccache-elohim`.

```bash
# Wait for MinIO to be ready before client commands
kubectl -n ethosengine rollout status deployment/minio --timeout=120s

# Generate consumer creds
SCCACHE_KEY=$(openssl rand -hex 12)
SCCACHE_SECRET=$(openssl rand -base64 24 | tr -d '\n=' | tr '/+' '-_')

# Use mc (MinIO client) inside the chart's ephemeral provisioning pod, OR
# port-forward + run mc from this workspace. Below is the port-forward pattern.
kubectl -n ethosengine port-forward svc/minio 9000:9000 >/dev/null 2>&1 &
PF_PID=$!
sleep 3

mc alias set local-minio http://127.0.0.1:9000 "${ROOT_USER}" "${ROOT_PASSWORD}"
mc admin user add local-minio "${SCCACHE_KEY}" "${SCCACHE_SECRET}"

# Policy: only read/write within sccache-elohim bucket
cat <<EOF > /tmp/sccache-policy.json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:ListBucket"],
      "Resource": ["arn:aws:s3:::sccache-elohim", "arn:aws:s3:::sccache-elohim/*"]
    }
  ]
}
EOF
mc admin policy create local-minio sccache-rw /tmp/sccache-policy.json
mc admin policy attach local-minio sccache-rw --user "${SCCACHE_KEY}"

kill $PF_PID
rm -f /tmp/sccache-policy.json

# Distribute creds to BOTH Jenkins and Che namespaces.
# Jenkins build pods need it for ci-builder-nix.
# Che workspaces need it for the dev image (see follow-up runbook).
for ns in jenkins ethosengine; do
    kubectl -n "${ns}" create secret generic sccache-credentials \
        --from-literal=AWS_ACCESS_KEY_ID="${SCCACHE_KEY}" \
        --from-literal=AWS_SECRET_ACCESS_KEY="${SCCACHE_SECRET}" \
        --from-literal=SCCACHE_BUCKET=sccache-elohim \
        --from-literal=SCCACHE_ENDPOINT=http://minio.ethosengine.svc.cluster.local:9000 \
        --from-literal=SCCACHE_S3_USE_SSL=false \
        --from-literal=SCCACHE_REGION=us-east-1 \
        --dry-run=client -o yaml | kubectl apply -f -
done

# Per-Che-user namespaces follow whatever Che provisioning template you use;
# the sccache-credentials secret may need to be templated in there too.
# If unclear, leave it for the Che image-update follow-up.
```

`SCCACHE_REGION=us-east-1` is a placeholder MinIO accepts; sccache requires the env var be set even though MinIO doesn't enforce regions.

## Verify

### Verify MinIO is up

```bash
kubectl -n ethosengine get pods -l app.kubernetes.io/name=minio
# expected: minio-0 1/1 Running

kubectl -n ethosengine get pvc
# expected: data-minio-0 Bound 200Gi RWO openebs-jiva-csi-default
```

### Verify the bucket exists

```bash
kubectl -n ethosengine port-forward svc/minio 9000:9000 >/dev/null 2>&1 &
PF_PID=$!
sleep 3
mc alias set verify http://127.0.0.1:9000 "${ROOT_USER}" "${ROOT_PASSWORD}"
mc ls verify/
# expected: sccache-elohim/  (and nothing else yet)
kill $PF_PID
```

### Verify endpoint is reachable from Jenkins ns

```bash
kubectl -n jenkins run --rm -it sccache-test \
    --image=curlimages/curl --restart=Never \
    --command -- curl -sf http://minio.ethosengine.svc.cluster.local:9000/minio/health/live
# expected: HTTP 200, no body
```

### Verify the secret distributed correctly

```bash
for ns in jenkins ethosengine; do
    echo "=== ${ns} ==="
    kubectl -n "${ns}" get secret sccache-credentials -o jsonpath='{.data.SCCACHE_BUCKET}' | base64 -d
    echo
done
# expected: sccache-elohim   (twice)
```

## Rollback

MinIO is consumed only by build pods and Che workspaces — and only AFTER their images are updated in the follow-up runbook. So this runbook's deployment is fully reversible without affecting any running workload.

```bash
# Remove the consumer credential secrets
for ns in jenkins ethosengine; do
    kubectl -n "${ns}" delete secret sccache-credentials --ignore-not-found
done

# Uninstall MinIO (releases the 200Gi PVC unless you set persistence.resourcePolicy=keep)
helm uninstall minio --namespace ethosengine

# Remove the root credentials secret
kubectl -n ethosengine delete secret minio-root-credentials --ignore-not-found
```

The 200Gi PVC will be automatically released by helm uninstall. If you want to preserve cache contents across a reinstall, add `--set persistence.resourcePolicy=keep` to the helm install above.

## What happens AFTER this runbook is applied

1. **Verify the secrets are in both namespaces and the endpoint is reachable** (steps above).
2. **Confirm back to the Jenkins pipeline owner** that MinIO is ready. The image-update follow-up:
   - `ci-builder-nix` image gets sccache binary + env-var injection from the K8s secret. PR against the `ethosengine/che-devworkspaces` repo.
   - The Eclipse Che dev image gets the same. Same repo.
   - Once both image rebuilds are out, set `RUSTC_WRAPPER=sccache` on the build pods and Che workspaces. cargo invocations transparently consult MinIO; cache hits are ~milliseconds, misses do the normal compile and populate the cache.
3. **Disk pressure on /projects in Che workspaces drops** because target/release artifacts no longer need to be retained across sessions — sccache hits regenerate them faster than a fresh compile.

## Forward path: swap to quilt when iroh-blobs matures

This deployment is **specifically designed to swap cleanly to quilt** when the elohim-native S3 surface graduates. The swap is endpoint-only:

- Today: `SCCACHE_ENDPOINT=http://minio.ethosengine.svc.cluster.local:9000` + bucket `sccache-elohim`
- Tomorrow: `SCCACHE_ENDPOINT=http://quilt.elohim.svc.cluster.local:<port>` + bucket = some CID-prefix mapping

sccache's S3 v4 client doesn't care about the backend implementation. The cache contents repopulate on the new endpoint over time. The interim MinIO step lets us dogfood the S3 shape now without waiting for iroh substrate readiness.

## Files referenced

- This runbook: `genesis/manifests/RUNBOOK-minio-sccache-2026-05-09.md`
- Prior runbook (storage-class pattern): `genesis/manifests/RUNBOOK-dna-caching-2026-05-09.md`
- Prior PVC manifest (same storage class pattern): `genesis/manifests/nix-cache-pvc.yaml`
- Architectural memory: `project_quilt_as_native_s3_surface.md` (in claude-config memory)

## Confirm back

Once `kubectl -n ethosengine get pods -l app.kubernetes.io/name=minio` shows `minio-0 1/1 Running` and `kubectl -n jenkins get secret sccache-credentials` returns the credential keys, signal back. The Jenkins/Che image-update PRs land next.
