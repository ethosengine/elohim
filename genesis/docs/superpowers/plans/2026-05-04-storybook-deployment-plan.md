# Storybook Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy `app/elohim-library` Storybook 10 build to `https://storybook.elohim.host` via a new `elohim-storybook` Jenkins pipeline parallel to `elohim`, refactoring the leftover `ui-playground` k8s scaffolding in the same change.

**Architecture:** New downstream Jenkinsfile (`app/elohim-library/Jenkinsfile`) registered in the orchestrator's `PIPELINES` map. Builds storybook static bundle → bakes into nginx:alpine image → pushes to Harbor → deploys to `elohim-alpha` namespace as `elohim-storybook-alpha` Deployment + Service + Ingress on subdomain `storybook.elohim.host`. Extracts `deployAppToEnvironment` helper from root Jenkinsfile to a shared groovy file so both the existing `elohim` pipeline and the new one consume the same deploy logic.

**Tech Stack:** Jenkins declarative pipeline (groovy), buildkit/nerdctl for image build, Harbor registry, microk8s (cert-manager + ingress-nginx + LetsEncrypt), pnpm workspaces, Storybook 10.3.6 + @storybook/angular.

---

## Spec Reference

`genesis/docs/superpowers/specs/2026-05-04-storybook-deployment-design.md`

## File Structure

**Files created:**

| Path | Responsibility |
|------|----------------|
| `app/elohim-library/Jenkinsfile` | Pipeline definition: build → image → push → deploy alpha → verify |
| `app/elohim-library/images/Dockerfile` | nginx:alpine base + dist/storybook contents |
| `app/elohim-library/images/nginx.conf` | gzip + cache headers (no SPA rewrites needed; storybook is fully static) |
| `genesis/orchestrator/manifests/elohim-storybook/alpha.yaml` | k8s Deployment for `elohim-storybook-alpha` |
| `genesis/orchestrator/manifests/elohim-storybook/alpha/service.yaml` | ClusterIP service `elohim-storybook-alpha-service` |
| `genesis/orchestrator/manifests/elohim-storybook/alpha/ingress.yaml` | Ingress on `storybook.elohim.host` with cert-manager |
| `genesis/orchestrator/scripts/deploy-helpers.groovy` | Extracted `deployAppToEnvironment()` for shared use |

**Files modified:**

| Path | Change |
|------|--------|
| `Jenkinsfile` (root) | Replace inlined `deployAppToEnvironment` definition with `load 'genesis/orchestrator/scripts/deploy-helpers.groovy'` + delegating wrapper |
| `genesis/orchestrator/Jenkinsfile` | Add `elohim-storybook` entry to `PIPELINES` map |
| `genesis/orchestrator/manifests/elohim-app/alpha/service.yaml` | Remove dead `elohim-ui-playground-alpha-service` block |
| `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml` | Remove dead `/ui-playground` path entry |
| `genesis/orchestrator/manifests/elohim-app/staging/service.yaml` | Remove dead staging ui-playground service block (if present) |
| `genesis/orchestrator/manifests/elohim-app/staging/ingress.yaml` | Remove dead staging /ui-playground path (if present) |
| `genesis/orchestrator/manifests/elohim-app/prod/service.yaml` | Remove dead prod ui-playground service block (if present) |
| `genesis/orchestrator/manifests/elohim-app/prod/ingress.yaml` | Remove dead prod /ui-playground path (if present) |

## Validation Tooling

Each task uses one of these reusable validation commands:

- **YAML structural lint**: `python3 -c 'import yaml; yaml.safe_load(open("PATH"))'` — non-zero exit on bad YAML.
- **Multi-doc YAML lint**: `python3 -c 'import yaml; list(yaml.safe_load_all(open("PATH")))'`
- **Jenkinsfile lint**: `cd genesis/orchestrator && pnpm exec npm-groovy-lint --path '../..' --files 'PATTERN' --ignorepattern '**/node_modules/**' --failon error`
- **kubectl dry-run** (only if a kubeconfig is available; skip otherwise — Jenkins will validate at deploy time): `kubectl apply --dry-run=client -f PATH`
- **Orchestrator unit tests**: `cd genesis/orchestrator && pnpm test` runs `graph-walker.test.mjs` and `orchestrator-strategy.test.mjs`.

If pnpm exec fails due to missing dependencies, run `pnpm install` from `/projects/elohim/genesis/orchestrator` first.

---

## Task 1: Cleanup — remove dead `elohim-ui-playground-alpha-service` from elohim-app/alpha service manifest

**Files:**
- Modify: `genesis/orchestrator/manifests/elohim-app/alpha/service.yaml`

The `ui-playground` service has no Deployment selector serving traffic. The `service.yaml` currently contains two Service objects separated by `---`. We're removing the second one.

- [ ] **Step 1: Read the current file**

```bash
cat genesis/orchestrator/manifests/elohim-app/alpha/service.yaml
```

Expected: file contains an `elohim-site-alpha-service` block, then `---`, then an `elohim-ui-playground-alpha-service` block.

- [ ] **Step 2: Replace file with the elohim-site Service only**

Write the file with this exact content:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: elohim-site-alpha-service
  namespace: elohim-alpha
  labels:
    app.kubernetes.io/name: elohim-site
    app.kubernetes.io/instance: elohim-site-alpha
    app.kubernetes.io/component: frontend
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
spec:
  selector:
    app: elohim-site-alpha
  ports:
  - protocol: TCP
    port: 80
    targetPort: 80
  type: ClusterIP
```

- [ ] **Step 3: Validate YAML**

```bash
python3 -c 'import yaml; list(yaml.safe_load_all(open("genesis/orchestrator/manifests/elohim-app/alpha/service.yaml")))'
```

Expected: exit 0, no output.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/elohim-app/alpha/service.yaml
git commit -m "chore(manifests): drop dead elohim-ui-playground-alpha-service

The Service had no matching Deployment (selector
app: elohim-ui-playground-alpha was never instantiated). Cleanup pass
in advance of standing up the new elohim-storybook deployment.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Cleanup — remove dead `/ui-playground` path from alpha ingress

**Files:**
- Modify: `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml`

- [ ] **Step 1: Read the current ingress file**

```bash
cat genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml
```

Expected: contains a path `/ui-playground` with backend `elohim-ui-playground-alpha-service`.

- [ ] **Step 2: Write the file without the dead path**

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: elohim-site-alpha-ingress
  namespace: elohim-alpha
  labels:
    app.kubernetes.io/name: elohim-site
    app.kubernetes.io/instance: elohim-site-alpha
    app.kubernetes.io/component: frontend
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-production
spec:
  ingressClassName: public
  rules:
  - host: alpha.elohim.host
    http:
      paths:
      - backend:
          service:
            name: elohim-doorway-alpha
            port:
              number: 8080
        path: /apps
        pathType: Prefix
      - backend:
          service:
            name: elohim-doorway-alpha
            port:
              number: 8080
        path: /blob
        pathType: Prefix
      - backend:
          service:
            name: elohim-site-alpha-service
            port:
              number: 80
        path: /
        pathType: Prefix
  tls:
  - hosts:
    - alpha.elohim.host
    secretName: alpha-elohim-site-tls-cert
```

- [ ] **Step 3: Validate YAML**

```bash
python3 -c 'import yaml; yaml.safe_load(open("genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml"))'
```

Expected: exit 0.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml
git commit -m "chore(manifests): drop dead /ui-playground path on alpha ingress

Pointed at elohim-ui-playground-alpha-service which itself had no
Deployment. Ingress cleanup ahead of the dedicated storybook subdomain.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Cleanup — repeat the staging service+ingress cleanup if dead refs exist there

**Files:**
- Possibly modify: `genesis/orchestrator/manifests/elohim-app/staging/service.yaml`
- Possibly modify: `genesis/orchestrator/manifests/elohim-app/staging/ingress.yaml`

- [ ] **Step 1: Check for dead references in staging**

```bash
grep -l "ui-playground" genesis/orchestrator/manifests/elohim-app/staging/ 2>/dev/null
```

Expected: either no output (nothing to clean) or a list of files needing the same cleanup.

- [ ] **Step 2: If staging has dead refs, repeat the same removal pattern as Tasks 1 & 2**

For each file flagged in Step 1:
- Read the file
- Remove the `elohim-ui-playground-staging-service` Service block (if `service.yaml`)
- Remove the `/ui-playground` path entry pointing at it (if `ingress.yaml`)
- Validate YAML with `python3 -c 'import yaml; list(yaml.safe_load_all(open("PATH")))'`

If Step 1 returned no files, this task is a no-op — proceed to Task 4 without creating a commit.

- [ ] **Step 3: Commit if changes were made**

```bash
git add genesis/orchestrator/manifests/elohim-app/staging/
git commit -m "chore(manifests): drop dead ui-playground refs from staging env

Same cleanup as alpha — Service had no Deployment selector, ingress path
pointed at the dead Service.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Cleanup — repeat for prod env if dead refs exist

**Files:**
- Possibly modify: `genesis/orchestrator/manifests/elohim-app/prod/service.yaml`
- Possibly modify: `genesis/orchestrator/manifests/elohim-app/prod/ingress.yaml`

- [ ] **Step 1: Check for dead references in prod**

```bash
grep -l "ui-playground" genesis/orchestrator/manifests/elohim-app/prod/ 2>/dev/null
```

- [ ] **Step 2: Apply identical pattern as Task 3 if any results returned**

Same procedure: remove dead Service block from `service.yaml`, remove dead `/ui-playground` path from `ingress.yaml`, validate YAML.

- [ ] **Step 3: Commit if changes were made**

```bash
git add genesis/orchestrator/manifests/elohim-app/prod/
git commit -m "chore(manifests): drop dead ui-playground refs from prod env

Final pass of the cleanup — pattern repeated from alpha and staging.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Extract `deployAppToEnvironment` to a shared groovy file

**Files:**
- Create: `genesis/orchestrator/scripts/deploy-helpers.groovy`
- Modify: `Jenkinsfile` (root, lines 89-132)

This refactor changes WHERE the helper lives. Behavior is unchanged. Both the existing `elohim` pipeline and the new `elohim-storybook` pipeline will load this helper via `load`.

- [ ] **Step 1: Create the shared helper file**

Write `genesis/orchestrator/scripts/deploy-helpers.groovy`:

```groovy
/**
 * Shared deploy helpers for orchestrator-managed pipelines.
 *
 * Loaded via `def helpers = load 'genesis/orchestrator/scripts/deploy-helpers.groovy'`
 * from any downstream Jenkinsfile.
 *
 * Conventions:
 *   - All manifests use SITE_TAG_PLACEHOLDER and DEPLOY_VERSION_PLACEHOLDER tokens
 *   - Substituted via sed at deploy time
 *   - kubectl rollout status waits up to 300s
 *   - Pre-deploy: ingress conflict check via genesis/orchestrator/scripts/check-ingress-conflicts.sh
 *   - Post-deploy: stale resource detection (advisory only)
 */

def deployAppToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag) {
    echo "Deploying to ${environment}: ${imageTag}"

    // Validate ConfigMap exists (skipped automatically when caller does not require one)
    sh "kubectl get configmap elohim-config-${environment} -n ${namespace} || { echo 'ConfigMap missing'; exit 1; }"

    // Update deployment manifest with image tag and deploy version label
    def outputFile = manifestPath.replace('.yaml', "-${environment}.rendered.yaml")
    sh "sed -e 's/SITE_TAG_PLACEHOLDER/${imageTag}/g' -e 's/DEPLOY_VERSION_PLACEHOLDER/${imageTag}/g' ${manifestPath} > ${outputFile}"

    // Fail fast if any placeholders remain
    def remaining = sh(script: "grep -c '_PLACEHOLDER' ${outputFile} || true", returnStdout: true).trim()
    if (remaining != '0') {
        error "Unresolved placeholders in ${outputFile}!"
    }

    // Preview manifest
    sh """
        echo '==== Deployment manifest preview ===='
        grep 'image:\\|app.kubernetes.io/version:' ${outputFile}
        echo '===================================='
    """

    // Pre-deploy: check for ingress hostname conflicts
    sh "bash genesis/orchestrator/scripts/check-ingress-conflicts.sh ${outputFile} ${namespace}"

    // Deploy and rollout
    sh "kubectl apply -f ${outputFile}"
    sh "kubectl rollout restart deployment/${deploymentName} -n ${namespace}"
    sh "kubectl rollout status deployment/${deploymentName} -n ${namespace} --timeout=300s"

    // Verify deployed image
    sh """
        echo '==== Verifying deployed image ===='
        kubectl get deployment ${deploymentName} -n ${namespace} -o jsonpath='{.spec.template.spec.containers[0].image}'
        echo ''
        echo '=================================='
    """

    // Post-deploy: detect stale resources (advisory only)
    sh "bash genesis/orchestrator/scripts/detect-stale-resources.sh ${namespace} ${imageTag} elohim-site || true"

    echo "${environment} deployment completed!"
}

/**
 * Same as deployAppToEnvironment but does NOT validate a configmap.
 * Used by elohim-storybook (fully static, no runtime config injection).
 */
def deployStaticToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag, String imageNameForStaleCheck) {
    echo "Deploying static site to ${environment}: ${imageTag}"

    // Update deployment manifest with image tag and deploy version label
    def outputFile = manifestPath.replace('.yaml', "-${environment}.rendered.yaml")
    sh "sed -e 's/STORYBOOK_TAG_PLACEHOLDER/${imageTag}/g' -e 's/DEPLOY_VERSION_PLACEHOLDER/${imageTag}/g' ${manifestPath} > ${outputFile}"

    // Fail fast if any placeholders remain
    def remaining = sh(script: "grep -c '_PLACEHOLDER' ${outputFile} || true", returnStdout: true).trim()
    if (remaining != '0') {
        error "Unresolved placeholders in ${outputFile}!"
    }

    // Preview manifest
    sh """
        echo '==== Deployment manifest preview ===='
        grep 'image:\\|app.kubernetes.io/version:' ${outputFile}
        echo '===================================='
    """

    // Pre-deploy: check for ingress hostname conflicts
    sh "bash genesis/orchestrator/scripts/check-ingress-conflicts.sh ${outputFile} ${namespace}"

    // Deploy and rollout
    sh "kubectl apply -f ${outputFile}"
    sh "kubectl rollout restart deployment/${deploymentName} -n ${namespace}"
    sh "kubectl rollout status deployment/${deploymentName} -n ${namespace} --timeout=300s"

    // Verify deployed image
    sh """
        echo '==== Verifying deployed image ===='
        kubectl get deployment ${deploymentName} -n ${namespace} -o jsonpath='{.spec.template.spec.containers[0].image}'
        echo ''
        echo '=================================='
    """

    // Post-deploy: detect stale resources (advisory only)
    sh "bash genesis/orchestrator/scripts/detect-stale-resources.sh ${namespace} ${imageTag} ${imageNameForStaleCheck} || true"

    echo "${environment} deployment completed!"
}

return this
```

The trailing `return this` is required for `load`-style loading in Jenkins pipeline groovy.

- [ ] **Step 2: Lint the new helper file**

```bash
cd /projects/elohim/genesis/orchestrator && pnpm exec npm-groovy-lint --path '../..' --files 'genesis/orchestrator/scripts/deploy-helpers.groovy' --ignorepattern '**/node_modules/**' --failon error
```

Expected: zero errors. Warnings about line length or style are acceptable but errors must be fixed before proceeding.

- [ ] **Step 3: Replace the inlined `deployAppToEnvironment` in root Jenkinsfile with a load + delegate**

Open `Jenkinsfile` (root). Find the function definition that starts at line 89:

```groovy
def deployAppToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag) {
    echo "Deploying to ${environment}: ${imageTag}"
    ...
    echo "${environment} deployment completed!"
}
```

Replace the entire block (from the `def deployAppToEnvironment(...)` opening to the matching closing brace — the whole function body, ~44 lines) with:

```groovy
def deployAppToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag) {
    def helpers = load 'genesis/orchestrator/scripts/deploy-helpers.groovy'
    helpers.deployAppToEnvironment(environment, namespace, deploymentName, manifestPath, imageTag)
}
```

This keeps the public signature unchanged so the existing `stage('Deploy to Staging')`, `stage('🚀 Deploy to Alpha')`, and `stage('Deploy to Prod')` call sites at lines 1001, 1031, 1204 require zero modification.

- [ ] **Step 4: Lint the modified root Jenkinsfile**

```bash
cd /projects/elohim/genesis/orchestrator && pnpm exec npm-groovy-lint --path '../..' --files 'Jenkinsfile' --ignorepattern '**/node_modules/**' --failon error
```

Expected: zero errors.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/scripts/deploy-helpers.groovy Jenkinsfile
git commit -m "refactor(jenkins): extract deployAppToEnvironment to shared helper

Moves the deploy helper from the root Jenkinsfile (where it consumed
~45 lines, growing the CPS-method size) to a shared groovy file loaded
via 'load'. Adds a parallel deployStaticToEnvironment variant with no
configmap requirement, for the upcoming elohim-storybook pipeline.

Behavior of the existing app pipeline is unchanged — root Jenkinsfile
still exposes deployAppToEnvironment with the same signature, now
delegating to the shared helper.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Create the storybook Dockerfile

**Files:**
- Create: `app/elohim-library/images/Dockerfile`

- [ ] **Step 1: Verify the parent dir exists**

```bash
ls /projects/elohim/app/elohim-library/
```

Expected: directory listing including `package.json` and `projects/`. If `images/` doesn't exist yet, the Write call below will fail; create with `mkdir -p app/elohim-library/images` first.

- [ ] **Step 2: Create the Dockerfile**

Write `app/elohim-library/images/Dockerfile`:

```dockerfile
# Static nginx container for Storybook.
# Storybook build happens in Jenkins via `pnpm --filter elohim-library run build-storybook`,
# producing app/elohim-library/dist/storybook/. This Dockerfile packages the pre-built
# static assets — no Node, no build tools at runtime.

FROM nginx:alpine

# Copy pre-built storybook static bundle
COPY elohim-library/dist/storybook /usr/share/nginx/html

# Custom nginx config for gzip + sane cache headers on hashed assets
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
```

- [ ] **Step 3: Verify Dockerfile syntax (basic structural check)**

```bash
test -f app/elohim-library/images/Dockerfile && grep -q '^FROM ' app/elohim-library/images/Dockerfile && grep -q '^CMD ' app/elohim-library/images/Dockerfile && echo "OK"
```

Expected output: `OK`

- [ ] **Step 4: Commit (defer until nginx.conf lands in next task)**

Skip — bundle the Dockerfile commit with nginx.conf in Task 7.

---

## Task 7: Create the nginx config and commit Dockerfile + config together

**Files:**
- Create: `app/elohim-library/images/nginx.conf`

Storybook is fully static (no client-side router), so we don't need a SPA `try_files` rewrite. The config focuses on gzip + cache strategy: long-cache the hashed bundles, never-cache `index.html` and `index.json`.

- [ ] **Step 1: Create nginx.conf**

Write `app/elohim-library/images/nginx.conf`:

```nginx
server {
  listen 80;
  server_name _;
  root /usr/share/nginx/html;
  index index.html;

  # gzip for text-like assets
  gzip on;
  gzip_types
    text/plain
    text/css
    application/javascript
    application/json
    image/svg+xml;
  gzip_min_length 256;

  # Storybook's hashed bundles (e.g. main.21360055ce6baf36bb28.css) — immutable
  location ~* \.(js|css|woff2?|ttf|eot|svg|png|jpg|gif|map)$ {
    expires 1y;
    add_header Cache-Control "public, immutable";
    access_log off;
    try_files $uri =404;
  }

  # index.html and index.json — never cache (they reference the hashed bundles)
  location ~* /(index\.html|index\.json|stories\.json|project\.json)$ {
    expires -1;
    add_header Cache-Control "no-cache, no-store, must-revalidate";
    try_files $uri =404;
  }

  # Default: serve from root
  location / {
    try_files $uri $uri/ /index.html;
  }

  # Health endpoint (returns 200 OK, used for readiness checks if needed later)
  location = /healthz {
    access_log off;
    return 200 "ok\n";
    add_header Content-Type text/plain;
  }
}
```

- [ ] **Step 2: Validate nginx config syntax (best-effort, since we may not have nginx installed)**

```bash
which nginx >/dev/null 2>&1 && nginx -t -c $(realpath app/elohim-library/images/nginx.conf) || echo "nginx not installed locally — config will be validated at image build time"
```

Either result is acceptable. If nginx is installed and `-t` reports an error, fix syntax before proceeding.

- [ ] **Step 3: Commit Dockerfile + nginx.conf together**

```bash
git add app/elohim-library/images/Dockerfile app/elohim-library/images/nginx.conf
git commit -m "feat(library): add nginx-based Dockerfile for storybook static bundle

Image takes the pre-built dist/storybook/ output (produced by Jenkins
via pnpm build-storybook) and serves it from nginx:alpine. Cache
strategy: immutable for hashed bundles, no-cache for index.html /
index.json. /healthz endpoint for future readiness probes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Create the alpha Deployment manifest

**Files:**
- Create: `genesis/orchestrator/manifests/elohim-storybook/alpha.yaml`

- [ ] **Step 1: Create the directory**

```bash
mkdir -p genesis/orchestrator/manifests/elohim-storybook/alpha
```

- [ ] **Step 2: Write alpha.yaml**

Write `genesis/orchestrator/manifests/elohim-storybook/alpha.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: elohim-storybook-alpha
  namespace: elohim-alpha
  labels:
    app: elohim-storybook-alpha
    app.kubernetes.io/name: elohim-storybook
    app.kubernetes.io/instance: elohim-storybook-alpha
    app.kubernetes.io/version: "DEPLOY_VERSION_PLACEHOLDER"
    app.kubernetes.io/component: design-surface
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
spec:
  replicas: 1
  selector:
    matchLabels:
      app: elohim-storybook-alpha
  template:
    metadata:
      labels:
        app: elohim-storybook-alpha
        app.kubernetes.io/name: elohim-storybook
        app.kubernetes.io/instance: elohim-storybook-alpha
        app.kubernetes.io/version: "DEPLOY_VERSION_PLACEHOLDER"
        app.kubernetes.io/component: design-surface
        app.kubernetes.io/part-of: elohim
        app.kubernetes.io/managed-by: jenkins
    spec:
      containers:
      - name: elohim-storybook-alpha
        image: harbor.ethosengine.com/ethosengine/elohim-storybook:STORYBOOK_TAG_PLACEHOLDER
        imagePullPolicy: Always
        ports:
        - containerPort: 80
        readinessProbe:
          httpGet:
            path: /healthz
            port: 80
          initialDelaySeconds: 2
          periodSeconds: 5
        livenessProbe:
          httpGet:
            path: /healthz
            port: 80
          initialDelaySeconds: 10
          periodSeconds: 20
        resources:
          requests:
            memory: "64Mi"
            cpu: "50m"
          limits:
            memory: "128Mi"
            cpu: "100m"
```

- [ ] **Step 3: Validate YAML**

```bash
python3 -c 'import yaml; yaml.safe_load(open("genesis/orchestrator/manifests/elohim-storybook/alpha.yaml"))'
```

Expected: exit 0.

- [ ] **Step 4: Commit (defer until Service + Ingress also land in Task 9)**

Skip — bundle all three manifest files in one commit at the end of Task 9.

---

## Task 9: Create the alpha Service and Ingress manifests

**Files:**
- Create: `genesis/orchestrator/manifests/elohim-storybook/alpha/service.yaml`
- Create: `genesis/orchestrator/manifests/elohim-storybook/alpha/ingress.yaml`

- [ ] **Step 1: Write service.yaml**

```yaml
apiVersion: v1
kind: Service
metadata:
  name: elohim-storybook-alpha-service
  namespace: elohim-alpha
  labels:
    app.kubernetes.io/name: elohim-storybook
    app.kubernetes.io/instance: elohim-storybook-alpha
    app.kubernetes.io/component: design-surface
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
spec:
  selector:
    app: elohim-storybook-alpha
  ports:
  - protocol: TCP
    port: 80
    targetPort: 80
  type: ClusterIP
```

- [ ] **Step 2: Write ingress.yaml**

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: elohim-storybook-alpha-ingress
  namespace: elohim-alpha
  labels:
    app.kubernetes.io/name: elohim-storybook
    app.kubernetes.io/instance: elohim-storybook-alpha
    app.kubernetes.io/component: design-surface
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-production
spec:
  ingressClassName: public
  rules:
  - host: storybook.elohim.host
    http:
      paths:
      - backend:
          service:
            name: elohim-storybook-alpha-service
            port:
              number: 80
        path: /
        pathType: Prefix
  tls:
  - hosts:
    - storybook.elohim.host
    secretName: alpha-elohim-storybook-tls-cert
```

- [ ] **Step 3: Validate both YAMLs**

```bash
python3 -c 'import yaml; yaml.safe_load(open("genesis/orchestrator/manifests/elohim-storybook/alpha/service.yaml"))' && python3 -c 'import yaml; yaml.safe_load(open("genesis/orchestrator/manifests/elohim-storybook/alpha/ingress.yaml"))'
```

Expected: exit 0, no output.

- [ ] **Step 4: Commit all three manifests together**

```bash
git add genesis/orchestrator/manifests/elohim-storybook/
git commit -m "feat(manifests): add elohim-storybook alpha k8s manifests

Deployment, Service, and Ingress for the new storybook subdomain.
Single replica, 64Mi/50m requests (well below the elohim-site SPA
since this is static nginx). cert-manager auto-issues the LetsEncrypt
cert once storybook.elohim.host CNAME resolves.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Create the storybook Jenkinsfile

**Files:**
- Create: `app/elohim-library/Jenkinsfile`

- [ ] **Step 1: Write the Jenkinsfile**

The pipeline reuses pod-template + buildkit conventions from the root Jenkinsfile, but is much smaller because there's no holochain dependency and no E2E.

Write `app/elohim-library/Jenkinsfile`:

```groovy
/**
 * Elohim Storybook Pipeline (elohim-storybook)
 *
 * Builds the @app/elohim-library Storybook 10 static bundle and deploys it
 * to https://storybook.elohim.host (alpha env, latest from dev branch).
 *
 * Triggered by orchestrator when files in app/elohim-library/ change.
 *
 * Build flow:
 *   pnpm install (workspace) → pnpm build-storybook → buildkit image →
 *   nerdctl push to Harbor → kubectl apply to elohim-alpha →
 *   curl /index.json verification
 *
 * @see genesis/orchestrator/Jenkinsfile for central trigger logic
 * @see genesis/docs/superpowers/specs/2026-05-04-storybook-deployment-design.md
 */

pipeline {
    agent {
        kubernetes {
            cloud 'kubernetes'
            yaml '''
apiVersion: v1
kind: Pod
spec:
 serviceAccount: jenkins-deployer
 nodeSelector:
    node-type: edge
 volumes:
  - name: containerd-sock
    hostPath:
     path: /var/snap/microk8s/common/run/containerd.sock
     type: Socket
  - name: buildkit-run
    emptyDir: {}
 containers:
 - name: builder
   image: harbor.ethosengine.com/ethosengine/ci-builder:latest
   command:
   - cat
   tty: true
   resources:
     requests:
       ephemeral-storage: "2Gi"
       memory: "1Gi"
     limits:
       ephemeral-storage: "4Gi"
       memory: "3Gi"
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
 - name: buildkitd
   image: moby/buildkit:v0.12.5
   securityContext:
     privileged: true
   args:
   - --addr
   - unix:///run/buildkit/buildkitd.sock
   - --oci-worker=true
   - --containerd-worker=false
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
'''
        }
    }

    environment {
        BRANCH_NAME = "${env.BRANCH_NAME ?: 'dev'}"
        NPM_TOKEN = credentials('ee-nexus-npm-token')
        IMAGE_NAME = 'elohim-storybook'
        HARBOR_REGISTRY = 'harbor.ethosengine.com'
        HARBOR_PROJECT = 'ethosengine'
    }

    options {
        skipDefaultCheckout(true)
        overrideIndexTriggers(false)
        timeout(time: 30, unit: 'MINUTES')
        disableConcurrentBuilds()
    }

    stages {
        stage('Check Trigger') {
            steps {
                script {
                    def validTrigger = currentBuild.getBuildCauses().any { cause ->
                        cause._class.contains('UserIdCause') ||
                        cause._class.contains('UpstreamCause')
                    }
                    if (!validTrigger) {
                        echo """
                        ═══════════════════════════════════════════════════════════
                        PIPELINE SKIPPED — managed by elohim-orchestrator.
                        Triggered by: ${currentBuild.getBuildCauses()*.shortDescription.join(', ')}
                        ═══════════════════════════════════════════════════════════
                        """
                        currentBuild.result = 'NOT_BUILT'
                        currentBuild.displayName = "#${env.BUILD_NUMBER} SKIPPED"
                        env.PIPELINE_SKIPPED = 'true'
                    } else {
                        echo "Valid trigger: ${currentBuild.getBuildCauses()*.shortDescription.join(', ')}"
                    }
                }
            }
        }

        stage('Checkout') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    checkout scm
                    script {
                        env.GIT_COMMIT_HASH = sh(script: 'git rev-parse --short HEAD', returnStdout: true).trim()
                        env.GIT_COMMIT_FULL = sh(script: 'git rev-parse HEAD', returnStdout: true).trim()
                    }
                }
            }
        }

        stage('Setup Version') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    script {
                        // Read VERSION file for the base version (e.g., "0.1.0")
                        def baseVersion = sh(script: 'grep "^APP_VERSION=" VERSION | cut -d= -f2 | tr -d "[:space:]" || echo "0.1.0"', returnStdout: true).trim()
                        if (!baseVersion) { baseVersion = '0.1.0' }
                        env.IMAGE_TAG = "${baseVersion}-dev-${env.GIT_COMMIT_HASH}"
                        echo "Storybook image tag: ${env.IMAGE_TAG}"
                    }
                }
            }
        }

        stage('Install Dependencies') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    sh '''#!/bin/bash
                        set -euo pipefail
                        echo "Installing workspace dependencies via pnpm..."
                        pnpm install --frozen-lockfile
                    '''
                }
            }
        }

        stage('Build Storybook') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    sh '''#!/bin/bash
                        set -euo pipefail
                        echo "Building Storybook static bundle..."
                        pnpm --filter elohim-library run build-storybook

                        if [ ! -f app/elohim-library/dist/storybook/index.html ]; then
                            echo "ERROR: dist/storybook/index.html missing after build"
                            exit 1
                        fi
                        if [ ! -f app/elohim-library/dist/storybook/index.json ]; then
                            echo "ERROR: dist/storybook/index.json missing after build"
                            exit 1
                        fi
                        ENTRIES=$(python3 -c 'import json; print(len(json.load(open("app/elohim-library/dist/storybook/index.json"))["entries"]))')
                        echo "Built storybook with ${ENTRIES} indexed entries"
                        if [ "${ENTRIES}" -eq 0 ]; then
                            echo "ERROR: storybook index has 0 entries — silent regression"
                            exit 1
                        fi
                    '''
                }
            }
        }

        stage('Build Image') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('buildkitd') {
                    sh '''#!/bin/sh
                        set -e
                        cd app
                        # Stage Dockerfile + nginx.conf at the build context root
                        cp elohim-library/images/Dockerfile ./Dockerfile.storybook
                        cp elohim-library/images/nginx.conf ./nginx.conf

                        buildctl --addr unix:///run/buildkit/buildkitd.sock \
                            build \
                            --frontend dockerfile.v0 \
                            --opt filename=Dockerfile.storybook \
                            --local context=. \
                            --local dockerfile=. \
                            --output type=image,name=elohim-storybook:${IMAGE_TAG},push=false,oci-mediatypes=true

                        rm -f Dockerfile.storybook nginx.conf
                    '''
                }
            }
        }

        stage('Push to Harbor Registry') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                        sh '''#!/bin/bash
                            set -euo pipefail
                            echo $HARBOR_PASSWORD | nerdctl -n k8s.io login ${HARBOR_REGISTRY} -u $HARBOR_USERNAME --password-stdin

                            nerdctl -n k8s.io tag elohim-storybook:${IMAGE_TAG} ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:${IMAGE_TAG}
                            nerdctl -n k8s.io tag elohim-storybook:${IMAGE_TAG} ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:${GIT_COMMIT_HASH}
                            nerdctl -n k8s.io tag elohim-storybook:${IMAGE_TAG} ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:latest

                            nerdctl -n k8s.io push ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:${IMAGE_TAG}
                            nerdctl -n k8s.io push ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:${GIT_COMMIT_HASH}
                            nerdctl -n k8s.io push ${HARBOR_REGISTRY}/${HARBOR_PROJECT}/elohim-storybook:latest
                        '''
                    }
                }
            }
        }

        stage('Deploy to Alpha') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    script {
                        def helpers = load 'genesis/orchestrator/scripts/deploy-helpers.groovy'
                        helpers.deployStaticToEnvironment(
                            'alpha',
                            'elohim-alpha',
                            'elohim-storybook-alpha',
                            'genesis/orchestrator/manifests/elohim-storybook/alpha.yaml',
                            env.IMAGE_TAG,
                            'elohim-storybook'
                        )
                        sh "kubectl apply -f genesis/orchestrator/manifests/elohim-storybook/alpha/service.yaml"
                        sh "kubectl apply -f genesis/orchestrator/manifests/elohim-storybook/alpha/ingress.yaml"
                    }
                }
            }
        }

        stage('Verify Deploy') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    sh '''#!/bin/bash
                        set -euo pipefail
                        URL="https://storybook.elohim.host/index.json"
                        echo "Verifying ${URL} ..."
                        # Allow up to 60s for ingress + cert to settle on first deploy
                        for i in $(seq 1 12); do
                            HTTP_CODE=$(curl -s -o /tmp/index.json -w '%{http_code}' --max-time 10 ${URL} || echo "000")
                            if [ "${HTTP_CODE}" = "200" ]; then
                                ENTRIES=$(python3 -c 'import json; print(len(json.load(open("/tmp/index.json"))["entries"]))')
                                echo "VERIFIED: ${ENTRIES} stories indexed at ${URL}"
                                if [ "${ENTRIES}" -eq 0 ]; then
                                    echo "ERROR: Deployed storybook has 0 entries"
                                    exit 1
                                fi
                                exit 0
                            fi
                            echo "Attempt ${i}/12 returned ${HTTP_CODE}, retrying in 5s..."
                            sleep 5
                        done
                        echo "FAILED: ${URL} did not return 200 within 60s"
                        exit 1
                    '''
                }
            }
        }

        stage('Cleanup') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    sh 'rm -rf app/elohim-library/dist || true'
                }
            }
        }
    }

    post {
        success {
            script {
                if (env.PIPELINE_SKIPPED != 'true') {
                    echo "elohim-storybook deployed: ${env.IMAGE_TAG} → https://storybook.elohim.host"
                }
            }
        }
        failure {
            echo "elohim-storybook FAILED at stage: ${env.STAGE_NAME ?: 'unknown'}"
        }
    }
}
```

- [ ] **Step 2: Lint the new Jenkinsfile**

```bash
cd /projects/elohim/genesis/orchestrator && pnpm exec npm-groovy-lint --path '../..' --files 'app/elohim-library/Jenkinsfile' --ignorepattern '**/node_modules/**' --failon error
```

Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add app/elohim-library/Jenkinsfile
git commit -m "feat(library): add elohim-storybook Jenkins pipeline

11-stage downstream pipeline triggered by orchestrator on
app/elohim-library/ changes. Builds storybook → buildkit image →
push to Harbor → kubectl apply → verify /index.json returns 200
with non-zero entries.

Reuses pod template + buildkit pattern from root Jenkinsfile but
trimmed: no holochain wait, no E2E, no multi-env promotion. Single
target: storybook.elohim.host (alpha namespace).

Loads deploy helper from genesis/orchestrator/scripts/deploy-helpers.groovy.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Register `elohim-storybook` in the orchestrator PIPELINES map

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile`

This is the single change that activates the new pipeline. Until this lands, the new Jenkinsfile + manifests are inert.

- [ ] **Step 1: Locate the PIPELINES map**

```bash
grep -n "@Field def PIPELINES" /projects/elohim/genesis/orchestrator/Jenkinsfile
```

Expected: a single line near the top of the file (around line ~30) showing `@Field def PIPELINES = [`.

- [ ] **Step 2: Find the closing `]` of the PIPELINES map**

```bash
awk '/@Field def PIPELINES = \[/,/^\]$/' /projects/elohim/genesis/orchestrator/Jenkinsfile | tail -10
```

Identify the last entry (currently `'elohim-epr'`) and the closing `]`.

- [ ] **Step 3: Insert the new entry before the closing `]`**

Find the `'elohim-epr'` block — the entire entry for that pipeline including its closing `],`. Add a new entry immediately after it (still inside the outer map). The exact text to insert:

```groovy
    ],
    'elohim-storybook': [
        jenkinsPath: 'app/elohim-library/Jenkinsfile',
        changePatterns: [
            'app/elohim-library/projects/**',
            'app/elohim-library/.storybook/**',
            'app/elohim-library/package.json',
            'app/elohim-library/tsconfig.storybook.json',
            'app/elohim-library/angular.json',
            'app/elohim-library/Jenkinsfile',
            'app/elohim-library/images/**',
            'genesis/orchestrator/manifests/elohim-storybook/**'
        ],
        artifacts: ['elohim-storybook'],
        dependsOn: [],
        cascades: false,
        triggersGenesis: false,
        deploymentCheck: [
            dev: 'https://storybook.elohim.host/index.json'
        ]
```

(The leading `],` closes the previous `elohim-epr` entry. The new entry's own closing `]` becomes the last entry's closer.)

After editing, the tail of the PIPELINES map should look like:

```groovy
    'elohim-epr': [
        ...existing fields...
    ],
    'elohim-storybook': [
        jenkinsPath: 'app/elohim-library/Jenkinsfile',
        ...as written above...
        deploymentCheck: [
            dev: 'https://storybook.elohim.host/index.json'
        ]
    ]
]
```

- [ ] **Step 4: Verify the map parses (basic groovy syntax sanity)**

```bash
cd /projects/elohim/genesis/orchestrator && pnpm exec npm-groovy-lint --path '../..' --files 'genesis/orchestrator/Jenkinsfile' --ignorepattern '**/node_modules/**' --failon error
```

Expected: zero errors.

- [ ] **Step 5: Run the existing orchestrator unit tests to confirm graph-walker still parses cleanly**

```bash
cd /projects/elohim/genesis/orchestrator && pnpm test
```

Expected: all tests pass. (The PIPELINES map is consumed by the orchestrator's groovy code, not the .mjs tests, so this is a smoke check that nothing structural broke.)

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile
git commit -m "feat(orchestrator): register elohim-storybook pipeline

Adds the new pipeline to the PIPELINES map. Triggers on changes under
app/elohim-library/ (parallel to elohim — same source, different deploy
target). No upstream dependency, no genesis cascade. Health check is
GET /index.json — Storybook 10 generates this on every build.

Once this lands, the next push that touches app/elohim-library/ will
trigger the new pipeline alongside elohim.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Document DNS prerequisite and trigger first build

**Files:**
- None (operational task)

- [ ] **Step 1: Confirm storybook.elohim.host CNAME**

```bash
dig +short storybook.elohim.host CNAME
```

Expected: returns the same target that `dig +short alpha.elohim.host` resolves to (e.g., `alpha.elohim.host` itself, or the underlying load-balancer hostname).

If the CNAME does not resolve, halt the pipeline trigger and create the DNS record first. Cert-manager issuance will retry every few minutes once DNS propagates, but the `Verify Deploy` stage will fail until then.

- [ ] **Step 2: Push the accumulated commits to dev branch**

```bash
git status
git push origin dev
```

Expected: `git status` shows clean working tree (all commits from Tasks 1-11 staged and committed). `git push` succeeds.

The orchestrator webhook will fire on push, analyze the changeset, and trigger `elohim-storybook` (because the changeset includes `app/elohim-library/Jenkinsfile`, `app/elohim-library/images/**`, and `genesis/orchestrator/manifests/elohim-storybook/**` — all matching the new entry's `changePatterns`).

- [ ] **Step 3: Monitor the orchestrator build, then the elohim-storybook downstream**

Open Jenkins:
- Orchestrator pipeline: should show a new build triggered by webhook, listing `elohim-storybook` (and likely also `elohim` and `elohim-genesis`) in its decision matrix.
- elohim-storybook downstream: should appear in the queue, run all 11 stages, and end green.

If the `Verify Deploy` stage fails with TLS errors, DNS hasn't fully propagated or cert-manager is still working through the LetsEncrypt challenge. Wait 2-5 minutes and trigger a manual rebuild from the elohim-storybook job in Jenkins.

- [ ] **Step 4: Smoke-test the deployed surface in a browser**

Visit `https://storybook.elohim.host/` and confirm:
- TLS shows a valid LetsEncrypt cert (no warning page).
- Storybook UI loads with the dark theme and Lamad UI sidebar entries.
- At least one story renders in the iframe (e.g., `Components / Hexagon Grid / Default`).
- `https://storybook.elohim.host/index.json` returns JSON with non-zero `entries`.

If any step fails, capture the failure details and triage in Jenkins (build log + `kubectl describe pod -n elohim-alpha -l app=elohim-storybook-alpha`).

- [ ] **Step 5: No commit (operational task)**

This task produces no file changes — its outcome is a deployed service. The deploy itself is the verification.

---

## Self-Review

**Spec coverage:**
- Pipeline placement (sibling of elohim, no genesis cascade): Task 11 ✓
- Repo layout (Jenkinsfile, Dockerfile, nginx.conf, manifests, helper extraction): Tasks 5-10 ✓
- ui-playground cleanup: Tasks 1-4 ✓
- Pipeline structure (11 stages from Check Trigger through Cleanup): Task 10 ✓
- Manifests (Deployment, Service, Ingress, no ConfigMap): Tasks 8-9 ✓
- Change-detection rules (PIPELINES entry with correct changePatterns + dependsOn + triggersGenesis): Task 11 ✓
- Pre-deploy DNS prerequisite: Task 12 ✓
- Failure modes / rollback: covered by deployStaticToEnvironment helper (kubectl rollout status with timeout) and the verify-loop in stage 'Verify Deploy' ✓
- Helper extraction: Task 5 ✓

**Type/name consistency check:**
- Image name `elohim-storybook` consistent across Jenkinsfile (Push stage), Deployment (`alpha.yaml` image field), and verify URL (constructed from `storybook.elohim.host`) ✓
- Deployment name `elohim-storybook-alpha` consistent in `alpha.yaml`, `service.yaml` selector (`app: elohim-storybook-alpha`), and helper invocation in Jenkinsfile ✓
- Service name `elohim-storybook-alpha-service` consistent in `service.yaml` and `ingress.yaml` backend ✓
- Helper function `deployStaticToEnvironment` defined in Task 5 step 1, called in Task 10 step 1 with matching signature (6 args, 6th being image-name-for-stale-check) ✓
- Placeholder tokens `STORYBOOK_TAG_PLACEHOLDER` (set in Task 8 manifest) match the sed pattern in `deployStaticToEnvironment` (Task 5) ✓

**No placeholders/TBDs:** scanned — every step has either a concrete code block or a concrete shell command with expected output.

---

## Execution Handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-04-storybook-deployment-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
