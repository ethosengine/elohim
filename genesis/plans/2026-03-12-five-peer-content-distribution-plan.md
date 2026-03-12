# Five-Peer Content Distribution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the single anonymous alpha StatefulSet with 5 per-human StatefulSets, add stewardship filtering to `seed-sqlite.ts`, and update the genesis Jenkinsfile to seed each conductor with its human's content.

**Architecture:** Each of the 5 genesis humans (Matthew, Susan, Pete, Timothy, Frank) gets a standalone StatefulSet with a `HUMAN_ID` env var, per-human resource limits, and trust-topology bootstrap peers. The SQLite seeder gains `--conductor-for` filtering (ported from `seed.ts`). The Jenkins seeding stage loops over all 5 conductors, querying each for its identity and seeding accordingly.

**Tech Stack:** Kubernetes YAML (StatefulSets, Services, ConfigMap), TypeScript (seed-sqlite.ts), Groovy (Jenkinsfile)

---

### Task 1: Add `--conductor-for` stewardship filtering to seed-sqlite.ts

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts:33-39` (arg parsing)
- Modify: `genesis/seeder/src/seed-sqlite.ts:427-459` (loadContentFiles + filtering)

**Step 1: Add CLI arg and stewardship types**

After line 39 (`const ACCOUNT_PACKAGES_DIR = ...`), add:

```typescript
const CONDUCTOR_FOR = args.find(a => a.startsWith('--conductor-for='))?.split('=')[1];
```

Before `loadContentFiles()` (line 427), add the filter function:

```typescript
// ============================================================================
// Stewardship Filtering
// ============================================================================

interface StewardAnnotation {
  humanId: string;
  affinity: number;
  role: string;
}

/**
 * Filter content nodes to those stewarded by a specific human.
 * Returns content where the given humanId is the highest-affinity steward.
 * If no stewardedBy field exists, defaults to the operator (backwards compat).
 */
function filterBySteward(
  concepts: ConceptJson[],
  humanId: string,
  operatorId: string = 'human-matthew-manager',
): ConceptJson[] {
  return concepts.filter(concept => {
    const stewards = (concept as Record<string, unknown>).stewardedBy as
      | StewardAnnotation[]
      | undefined;

    if (!stewards || stewards.length === 0) {
      return humanId === operatorId;
    }

    const primary = stewards.reduce((max, s) => (s.affinity > max.affinity ? s : max), stewards[0]);
    return primary.humanId === humanId;
  });
}
```

**Step 2: Wire the filter into `main()`**

In the `main()` function, after `loadContentFiles()` is called (around line 956), add filtering before the LIMIT check:

Find this block in the Phase 1 section:
```typescript
    let content = loadContentFiles();
    console.log(`   Loaded ${formatCount(content.length)} content items`);

    if (LIMIT > 0 && content.length > LIMIT) {
```

Replace with:
```typescript
    let content = loadContentFiles();
    console.log(`   Loaded ${formatCount(content.length)} content items`);

    if (CONDUCTOR_FOR) {
      const beforeCount = content.length;
      content = filterBySteward(content, CONDUCTOR_FOR);
      console.log(`   [stewardship] Filtered to ${content.length}/${beforeCount} content nodes for ${CONDUCTOR_FOR}`);
    }

    if (LIMIT > 0 && content.length > LIMIT) {
```

Also add the `--conductor-for` to the configuration output in `main()`. Find:
```typescript
  console.log(`   Skip blob upload: ${SKIP_BLOB_UPLOAD}`);
```

Add after it:
```typescript
  if (CONDUCTOR_FOR) {
    console.log(`   Conductor for: ${CONDUCTOR_FOR}`);
  }
```

**Step 3: Test locally**

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed-sqlite.ts --dry-run --conductor-for=human-pete-pastor 2>&1 | head -30`
Expected: Shows "Conductor for: human-pete-pastor" and "[stewardship] Filtered to ~300-400/3525"

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed-sqlite.ts --dry-run --conductor-for=human-matthew-manager 2>&1 | head -30`
Expected: Matthew gets the largest share (1000+ as founder default)

Run: `cd /projects/elohim/genesis/seeder && npx tsx src/seed-sqlite.ts --dry-run 2>&1 | head -20`
Expected: No filtering message, loads all 3525 (backwards compat)

**Step 4: Commit**

```bash
git add genesis/seeder/src/seed-sqlite.ts
git commit -m "feat(seeder): add --conductor-for stewardship filtering to seed-sqlite

Ports filterBySteward from seed.ts to the SQLite seeder that CI
actually uses. Filters content to highest-affinity steward match.
Backwards compatible — without the flag, seeds all content.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Create per-human StatefulSet for Matthew (template)

Matthew's YAML is the template — we'll derive the other 4 from it.

**Files:**
- Create: `genesis/manifests/humans/matthew-manager.yaml`

**Step 1: Create the directory**

Run: `ls /projects/elohim/genesis/manifests/`
Then: `mkdir -p /projects/elohim/genesis/manifests/humans`

**Step 2: Create Matthew's StatefulSet**

Derive from `genesis/orchestrator/manifests/edgenode/alpha.yaml`, with these changes:
- Name: `elohim-matthew-alpha` (not `elohim-edgenode-alpha`)
- `replicas: 1`
- `HUMAN_ID` env var on storage container
- Labels include `elohim-human: matthew-manager`
- Headless service named `elohim-matthew-alpha-headless`
- ClusterIP service named `elohim-matthew-alpha`
- Bootstrap peers point to Susan (household) and Pete (congregation)
- Resources: conductor 1Gi/500m, storage 512Mi/500m (founder gets more)
- PVC names scoped: `matthew-holochain-data`, `matthew-storage-data`

```yaml
# Matthew Dowell — Protocol Founder
# Stewards: governance, protocol core, family learning
# Trust topology: household with Susan, congregation with Pete
# Resource profile: founder node, higher capacity
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: elohim-matthew-alpha-config
  namespace: elohim-alpha
  labels:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
data:
  happ-version: "HAPP_TAG_PLACEHOLDER"
  conductor-config.yaml: |
    admin_interfaces:
      - driver:
          type: websocket
          port: 4444
          allowed_origins: "*"
    network:
      bootstrap_url: "https://doorway-alpha.elohim.host/bootstrap"
      signal_url: "wss://signal.doorway-alpha.elohim.host"
      enable_mdns: false
      enable_relaying: true
      webrtc_config:
        ice_servers:
          - urls: ["stun:stun.cloudflare.com:3478"]
          - urls: ["stun:stun.l.google.com:19302"]
    data_root_path: "/var/local/lib/holochain"
    keystore:
      type: lair_server_in_proc
      lair_root: "/var/local/lib/holochain/ks"
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: elohim-matthew-alpha
  namespace: elohim-alpha
  labels:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
    app.kubernetes.io/name: edgenode
    app.kubernetes.io/instance: matthew-alpha
    app.kubernetes.io/version: DEPLOY_VERSION_PLACEHOLDER
    app.kubernetes.io/component: data
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
spec:
  serviceName: elohim-matthew-alpha-headless
  replicas: 1
  podManagementPolicy: Parallel
  updateStrategy:
    type: RollingUpdate
  selector:
    matchLabels:
      app: elohim-edgenode
      elohim-human: matthew-manager
      environment: alpha
  template:
    metadata:
      labels:
        app: elohim-edgenode
        elohim-human: matthew-manager
        environment: alpha
        app.kubernetes.io/name: edgenode
        app.kubernetes.io/instance: matthew-alpha
        app.kubernetes.io/version: DEPLOY_VERSION_PLACEHOLDER
        app.kubernetes.io/component: data
        app.kubernetes.io/part-of: elohim
        app.kubernetes.io/managed-by: jenkins
    spec:
      securityContext:
        fsGroup: 1000
      affinity:
        nodeAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 80
              preference:
                matchExpressions:
                  - key: node-type
                    operator: In
                    values:
                      - operations
            - weight: 70
              preference:
                matchExpressions:
                  - key: node-type
                    operator: In
                    values:
                      - performance
      initContainers:
        - name: happ-fetcher
          image: ghcr.io/oras-project/oras:v1.1.0
          command: ['sh', '-c']
          args:
            - |
              set -e
              echo "Fetching hApp from Harbor registry..."
              oras pull harbor.ethosengine.com/ethosengine/elohim-happ:${HAPP_VERSION} -o /happ
              if [ -f /happ/elohim.happ ]; then
                echo "hApp downloaded successfully"
              else
                echo "Failed to download hApp"
                exit 1
              fi
          env:
            - name: HAPP_VERSION
              valueFrom:
                configMapKeyRef:
                  name: elohim-matthew-alpha-config
                  key: happ-version
          volumeMounts:
            - name: happ-volume
              mountPath: /happ
          resources:
            requests:
              memory: "32Mi"
              cpu: "10m"
            limits:
              memory: "64Mi"
              cpu: "100m"
      containers:
        - name: edgenode
          image: harbor.ethosengine.com/ethosengine/elohim-edgenode:EDGENODE_TAG_PLACEHOLDER
          env:
            - name: CONDUCTOR_MODE
              value: "true"
          resources:
            requests:
              memory: "1Gi"
              cpu: "500m"
            limits:
              memory: "2Gi"
              cpu: "2000m"
          volumeMounts:
            - name: holochain-data
              mountPath: /data
            - name: conductor-config
              mountPath: /usr/local/share/holochain/conductor-config.template.yaml
              subPath: conductor-config.yaml
            - name: happ-volume
              mountPath: /opt/holochain
          livenessProbe:
            exec:
              command: ["bash", "-c", "echo > /dev/tcp/127.0.0.1/4444"]
            initialDelaySeconds: 60
            periodSeconds: 30
            failureThreshold: 5
          readinessProbe:
            exec:
              command: ["bash", "-c", "echo > /dev/tcp/127.0.0.1/4444"]
            initialDelaySeconds: 30
            periodSeconds: 10
            failureThreshold: 3
        - name: ws-proxy
          image: alpine/socat:latest
          command: ['sh', '-c']
          args:
            - |
              socat TCP-LISTEN:8444,fork,reuseaddr TCP:127.0.0.1:4444 &
              socat TCP-LISTEN:8445,fork,reuseaddr TCP:127.0.0.1:4445 &
              wait
          ports:
            - name: admin-ws-int
              containerPort: 8444
            - name: app-ws-int
              containerPort: 8445
          resources:
            requests:
              memory: "16Mi"
              cpu: "10m"
            limits:
              memory: "64Mi"
              cpu: "100m"
          readinessProbe:
            tcpSocket:
              port: 8444
            initialDelaySeconds: 5
            periodSeconds: 5
        - name: elohim-storage
          image: harbor.ethosengine.com/ethosengine/elohim-storage:STORAGE_TAG_PLACEHOLDER
          imagePullPolicy: Always
          env:
            - name: HUMAN_ID
              value: "human-matthew-manager"
            - name: RUST_LOG
              value: "info,elohim_storage=debug"
            - name: HOLOCHAIN_ADMIN_URL
              value: "ws://localhost:4444"
            - name: HOLOCHAIN_APP_URL
              value: "ws://localhost:4445"
            - name: HOLOCHAIN_APP_ID
              value: "elohim"
            - name: ENABLE_IMPORT_API
              value: "true"
            - name: ENABLE_CONTENT_DB
              value: "true"
            - name: IMPORT_CHUNK_SIZE
              value: "50"
            - name: IMPORT_CHUNK_DELAY_MS
              value: "300"
            - name: ENABLE_P2P
              value: "true"
            - name: P2P_PORT
              value: "9876"
            - name: DISABLE_MDNS
              value: "true"
            - name: P2P_BOOTSTRAP_NODES
              value: "/dns4/elohim-susan-alpha-0.elohim-susan-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876,/dns4/elohim-pete-alpha-0.elohim-pete-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876"
            - name: RELAY_MODE
              value: "server"
          ports:
            - name: storage-http
              containerPort: 8090
            - name: p2p
              containerPort: 9876
          resources:
            requests:
              memory: "256Mi"
              cpu: "200m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          volumeMounts:
            - name: storage-data
              mountPath: /data
          readinessProbe:
            httpGet:
              path: /health
              port: 8090
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: 8090
            initialDelaySeconds: 10
            periodSeconds: 30
        - name: happ-installer
          image: harbor.ethosengine.com/ethosengine/elohim-happ-installer:HAPP_INSTALLER_TAG_PLACEHOLDER
          imagePullPolicy: Always
          env:
            - name: CONDUCTOR_URL
              value: "ws://localhost:4444"
            - name: APP_ID
              value: "elohim"
            - name: MAX_RETRIES
              value: "60"
            - name: RETRY_DELAY_MS
              value: "2000"
            - name: HAPP_PATH
              value: "/opt/holochain/elohim.happ"
          volumeMounts:
            - name: happ-volume
              mountPath: /opt/holochain
          resources:
            requests:
              memory: "64Mi"
              cpu: "50m"
            limits:
              memory: "256Mi"
              cpu: "200m"
      volumes:
        - name: happ-volume
          emptyDir: {}
        - name: conductor-config
          configMap:
            name: elohim-matthew-alpha-config
  volumeClaimTemplates:
    - metadata:
        name: holochain-data
      spec:
        accessModes: [ReadWriteOnce]
        resources:
          requests:
            storage: 10Gi
    - metadata:
        name: storage-data
      spec:
        accessModes: [ReadWriteOnce]
        resources:
          requests:
            storage: 5Gi
---
apiVersion: v1
kind: Service
metadata:
  name: elohim-matthew-alpha-headless
  namespace: elohim-alpha
  labels:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
spec:
  clusterIP: None
  selector:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
  ports:
    - name: admin-ws
      port: 4444
      targetPort: 8444
    - name: app-ws
      port: 4445
      targetPort: 8445
    - name: p2p
      port: 9876
      targetPort: 9876
---
apiVersion: v1
kind: Service
metadata:
  name: elohim-matthew-alpha
  namespace: elohim-alpha
  labels:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
spec:
  type: ClusterIP
  ports:
    - name: admin-ws
      port: 4444
      targetPort: 8444
    - name: app-ws
      port: 4445
      targetPort: 8445
    - name: storage-db
      port: 8090
      targetPort: 8090
    - name: p2p
      port: 9876
      targetPort: 9876
  selector:
    app: elohim-edgenode
    elohim-human: matthew-manager
    environment: alpha
```

**Step 3: Validate YAML syntax**

Run: `python3 -c "import yaml; list(yaml.safe_load_all(open('genesis/manifests/humans/matthew-manager.yaml')))" && echo "Valid YAML"`
Expected: Valid YAML

**Step 4: Commit**

```bash
git add genesis/manifests/humans/matthew-manager.yaml
git commit -m "infra(genesis): add Matthew's per-human StatefulSet for alpha

Founder node: 1Gi RAM conductor, 512Mi storage, bootstraps to Susan
and Pete. Template for the other 4 human StatefulSets.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Create StatefulSets for Susan, Pete, Timothy, Frank

**Files:**
- Create: `genesis/manifests/humans/susan-spouse.yaml`
- Create: `genesis/manifests/humans/pete-pastor.yaml`
- Create: `genesis/manifests/humans/timothy-tutor.yaml`
- Create: `genesis/manifests/humans/frank-farmer.yaml`

**Step 1: Create Susan's StatefulSet**

Copy Matthew's template with these changes:

| Field | Matthew | Susan |
|-------|---------|-------|
| Names | `elohim-matthew-alpha` | `elohim-susan-alpha` |
| Label | `elohim-human: matthew-manager` | `elohim-human: susan-spouse` |
| ConfigMap | `elohim-matthew-alpha-config` | `elohim-susan-alpha-config` |
| `HUMAN_ID` | `human-matthew-manager` | `human-susan-spouse` |
| Conductor memory request | `1Gi` | `768Mi` |
| Conductor CPU request | `500m` | `400m` |
| Storage memory request | `256Mi` | `128Mi` |
| Storage CPU request | `200m` | `100m` |
| `P2P_BOOTSTRAP_NODES` | Susan + Pete | Matthew + Timothy |
| Header comment | Protocol Founder | Household partner, family curriculum |

Bootstrap for Susan:
```
/dns4/elohim-matthew-alpha-0.elohim-matthew-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876,/dns4/elohim-timothy-alpha-0.elohim-timothy-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876
```

**Step 2: Create Pete's StatefulSet**

| Field | Value |
|-------|-------|
| Names | `elohim-pete-alpha` |
| Label | `elohim-human: pete-pastor` |
| `HUMAN_ID` | `human-pastor-pete-pastor` |
| Conductor memory | `768Mi` request, `1536Mi` limit |
| Conductor CPU | `400m` request |
| Storage memory | `128Mi` request |
| Bootstrap | Matthew + Frank |
| Comment | Faith community primary, pastoral care |

Bootstrap for Pete:
```
/dns4/elohim-matthew-alpha-0.elohim-matthew-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876,/dns4/elohim-frank-alpha-0.elohim-frank-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876
```

**Step 3: Create Timothy's StatefulSet**

| Field | Value |
|-------|-------|
| Names | `elohim-timothy-alpha` |
| Label | `elohim-human: timothy-tutor` |
| `HUMAN_ID` | `human-timothy-tutor` |
| Conductor memory | `512Mi` request, `1Gi` limit |
| Conductor CPU | `300m` request |
| Storage memory | `128Mi` request |
| Bootstrap | Susan + Pete |
| Comment | Learning steward, tutorials, mentorship |

Bootstrap for Timothy:
```
/dns4/elohim-susan-alpha-0.elohim-susan-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876,/dns4/elohim-pete-alpha-0.elohim-pete-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876
```

**Step 4: Create Frank's StatefulSet**

| Field | Value |
|-------|-------|
| Names | `elohim-frank-alpha` |
| Label | `elohim-human: frank-farmer` |
| `HUMAN_ID` | `human-frank-farmer` |
| Conductor memory | `512Mi` request, `1Gi` limit |
| Conductor CPU | `300m` request |
| Storage memory | `128Mi` request |
| Bootstrap | Pete + Matthew |
| Comment | Economy cluster, agriculture, local economy |

Bootstrap for Frank:
```
/dns4/elohim-pete-alpha-0.elohim-pete-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876,/dns4/elohim-matthew-alpha-0.elohim-matthew-alpha-headless.elohim-alpha.svc.cluster.local/tcp/9876
```

**Step 5: Validate all YAML files**

Run: `for f in genesis/manifests/humans/*.yaml; do python3 -c "import yaml; list(yaml.safe_load_all(open('$f')))" && echo "OK: $f" || echo "FAIL: $f"; done`
Expected: All 5 OK

**Step 6: Commit**

```bash
git add genesis/manifests/humans/susan-spouse.yaml genesis/manifests/humans/pete-pastor.yaml genesis/manifests/humans/timothy-tutor.yaml genesis/manifests/humans/frank-farmer.yaml
git commit -m "infra(genesis): add per-human StatefulSets for Susan, Pete, Timothy, Frank

Each human gets their own conductor with story-appropriate resources
and trust-topology bootstrap peers. Together with Matthew, these 5
peers model real P2P content distribution.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: Update genesis Jenkinsfile for per-conductor seeding

**Files:**
- Modify: `genesis/Jenkinsfile:48-59` (add per-human storage URL resolver)
- Modify: `genesis/Jenkinsfile:415-448` (replace single-target seed with per-conductor loop)

**Step 1: Add per-human storage URL map**

After the `resolveInternalStorageUrl` function (line 59), add a new function:

```groovy
def getHumanStorageUrls(String environment) {
    // Per-human StatefulSet storage endpoints
    // Each human's elohim-storage runs on port 8090
    def namespace = "elohim-${environment}"
    def humans = [
        [humanId: 'human-matthew-manager',     service: 'elohim-matthew-alpha'],
        [humanId: 'human-susan-spouse',         service: 'elohim-susan-alpha'],
        [humanId: 'human-pastor-pete-pastor',   service: 'elohim-pete-alpha'],
        [humanId: 'human-timothy-tutor',        service: 'elohim-timothy-alpha'],
        [humanId: 'human-frank-farmer',         service: 'elohim-frank-alpha'],
    ]
    return humans.collect { h ->
        h + [storageUrl: "${h.service}.${namespace}.svc.cluster.local:8090"]
    }
}
```

**Step 2: Replace the Seed Database stage**

Replace the existing `stage('Seed Database')` block (lines 415-448) with a per-conductor loop:

```groovy
        stage('Seed Database') {
            when { allOf { expression { env.PIPELINE_SKIPPED != 'true' }; expression { params.SEED_DATA } } }
            steps {
                container('builder') {
                    script {
                        def humans = getHumanStorageUrls('alpha')

                        dir('genesis/seeder') {
                            // Seed each conductor with its human's stewardship content
                            for (human in humans) {
                                def humanId = human.humanId
                                def storageUrl = human.storageUrl
                                def service = human.service

                                sh """#!/bin/bash
                                    set -euo pipefail

                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "🌱 SEEDING ${humanId}"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo "Storage: ${storageUrl}"
                                    echo "═══════════════════════════════════════════════════════════"
                                    echo ""

                                    # Wait for this conductor's storage to be ready
                                    for i in \$(seq 1 30); do
                                        if curl -sf "http://${storageUrl}/health" > /dev/null 2>&1; then
                                            echo "✅ ${service} storage is ready"
                                            break
                                        fi
                                        if [ \$i -eq 30 ]; then
                                            echo "❌ ${service} storage not ready after 30 attempts"
                                            exit 1
                                        fi
                                        echo "   Waiting for ${service} storage... (\$i/30)"
                                        sleep 2
                                    done

                                    STORAGE_URL="http://${storageUrl}" \\
                                        npx tsx src/seed-sqlite.ts --conductor-for=${humanId}

                                    echo ""
                                    echo "✅ ${humanId} seeded successfully"
                                    echo ""
                                """
                            }

                            // Summary: query all conductors for content counts
                            sh '''#!/bin/bash
                                echo "═══════════════════════════════════════════════════════════"
                                echo "📊 SEEDING SUMMARY"
                                echo "═══════════════════════════════════════════════════════════"
                            '''
                            for (human in humans) {
                                sh """#!/bin/bash
                                    STATS=\$(curl -sf "http://${human.storageUrl}/db/stats" 2>/dev/null || echo '{}')
                                    COUNT=\$(echo "\$STATS" | jq -r '.content_count // 0')
                                    echo "   ${human.humanId}: \$COUNT content nodes"
                                """
                            }
                            sh '''#!/bin/bash
                                echo "═══════════════════════════════════════════════════════════"
                                echo "✅ ALL CONDUCTORS SEEDED"
                                echo "═══════════════════════════════════════════════════════════"
                            '''
                        }
                    }
                }
            }
        }
```

**Step 3: Update the Verify Seeding stage**

Find the existing `stage('Verify Seeding')` (around line 451) and update it to verify all 5 conductors. Replace the single `STORAGE_URL` check with a loop:

Update the verification `sh` block to loop over humans:

```groovy
                        def humans = getHumanStorageUrls('alpha')
                        def totalContent = 0

                        for (human in humans) {
                            sh """#!/bin/bash
                                set -euo pipefail
                                STATS=\$(curl -sf "http://${human.storageUrl}/db/stats")
                                COUNT=\$(echo "\$STATS" | jq -r '.content_count')
                                echo "   ${human.humanId}: \$COUNT content nodes"
                                if [ "\$COUNT" -eq 0 ]; then
                                    echo "❌ ${human.humanId} has no content!"
                                    exit 1
                                fi
                            """
                        }
```

**Step 4: Update INTERNAL_STORAGE_URL resolution**

Find where `INTERNAL_STORAGE_URL` is set in the environment block (around line 107-130). The single storage URL is still needed for preflight health checks. Keep it pointing to Matthew (the founder/primary conductor):

In the `resolveInternalStorageUrl` function, update the alpha mapping to point to Matthew's service:
```groovy
    if (doorwayHost.contains('alpha.elohim.host')) {
        return 'elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090'
    }
```

**Step 5: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "feat(genesis): seed 5 conductors per-human in pipeline

Jenkins loops over Matthew, Susan, Pete, Timothy, and Frank,
seeding each conductor with only its steward's content via
--conductor-for. Pipeline output shows per-human content counts.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Update doorway to route to multiple storage backends

**Files:**
- Modify: `doorway/doorway-service/src/services/route_registry.rs`

**Context:** Doorway currently proxies to a single storage backend. With 5 per-human storage instances, doorway needs to know about all of them for content resolution. However, this is a larger change that affects the route registry design.

**Step 1: Check current route registry state**

Run: `grep -n "storage\|edgenode\|CONDUCTOR" doorway/doorway-service/src/services/route_registry.rs | head -20`

Review how the route registry currently discovers storage backends. The `/manifest` endpoint on each storage instance already declares its routes.

**Step 2: Add multi-backend awareness to route registry**

This step depends on the current route registry implementation. The minimal change: doorway's `CONDUCTOR_URLS` environment variable (or equivalent) needs to list all 5 storage endpoints. When a content request comes in, doorway tries each storage backend until it finds the content.

For now, the simplest approach: update the doorway alpha deployment's storage URL env vars to include all 5:

```yaml
- name: STORAGE_URLS
  value: "http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090,http://elohim-susan-alpha.elohim-alpha.svc.cluster.local:8090,http://elohim-pete-alpha.elohim-alpha.svc.cluster.local:8090,http://elohim-timothy-alpha.elohim-alpha.svc.cluster.local:8090,http://elohim-frank-alpha.elohim-alpha.svc.cluster.local:8090"
```

**Step 3: This task is intentionally underspecified**

The route registry was just redesigned (the recent `route_registry.rs` changes). The exact implementation depends on how the self-registration and `/admin/routes` endpoint works. Read the current state before implementing.

**Step 4: Commit**

```bash
git add doorway/
git commit -m "feat(doorway): route to 5 per-human storage backends

Doorway discovers all human conductors and routes content requests
to the appropriate storage instance.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Verify end-to-end — dry run then real deployment

**Step 1: Dry-run the seeder for all 5 humans**

Run each in sequence, verify content distribution:

```bash
cd /projects/elohim/genesis/seeder
for human in human-matthew-manager human-susan-spouse human-pastor-pete-pastor human-timothy-tutor human-frank-farmer; do
  echo "=== $human ==="
  npx tsx src/seed-sqlite.ts --dry-run --conductor-for=$human 2>&1 | grep -E "stewardship|Loaded|Filtered"
done
```

Expected: Each human gets a subset. Matthew gets the most (founder default). Total across all 5 should cover the full 3,525 corpus (some content counted multiple times if shared stewardship, but each content's primary steward is unique).

**Step 2: Validate YAML can be applied (dry-run)**

Run: `for f in genesis/manifests/humans/*.yaml; do echo "--- $f ---"; python3 -c "import yaml; docs=list(yaml.safe_load_all(open('$f'))); print(f'  {len(docs)} documents'); [print(f'  {d[\"kind\"]}: {d[\"metadata\"][\"name\"]}') for d in docs if d]"; done`

Expected: Each file has 4 documents (ConfigMap, StatefulSet, headless Service, ClusterIP Service)

**Step 3: Commit any fixes**

If anything needed adjustment, commit fixes.

**Step 4: Final commit — update plan status**

No code change — just mark the design as implemented when alpha deployment confirms 5 peers with distributed content.

---

## Summary

| Task | What | Files | Est. |
|------|------|-------|------|
| 1 | `--conductor-for` in seed-sqlite.ts | `seed-sqlite.ts` | 10 min |
| 2 | Matthew's StatefulSet (template) | `matthew-manager.yaml` (new) | 15 min |
| 3 | Susan, Pete, Timothy, Frank StatefulSets | 4 new YAML files | 20 min |
| 4 | Jenkins per-conductor seeding loop | `genesis/Jenkinsfile` | 15 min |
| 5 | Doorway multi-backend routing | `route_registry.rs` | 15 min |
| 6 | End-to-end verification | dry-run + deploy | 10 min |

**Total: ~85 minutes, 6 commits**

## What This Enables Next

With 5 real peers running distributed content:
1. **P2P replication testing** — assert that Pete discovers Matthew's community-reach content via protocol sync
2. **Reach-gated filtering** — verify private content stays private across trust boundaries
3. **Resilience profile** — compute real ShardHealthSummary from actual multi-peer distribution
4. **Schema version bridging** — when feature branches change wire format, peers at different versions negotiate
5. **Compute budget assertions** — shefa EconomicEvents recording real per-conductor resource usage
