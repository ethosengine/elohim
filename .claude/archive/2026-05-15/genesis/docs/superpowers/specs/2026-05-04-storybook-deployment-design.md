# Storybook Deployment — Design Spec

**Date**: 2026-05-04
**Status**: Approved for implementation
**Scope**: Deploy `app/elohim-library` Storybook 10 build to `storybook.elohim.host`, refactoring leftover `ui-playground` scaffolding in the process. Add a new `elohim-storybook` pipeline to the orchestrator, parallel to `elohim`.

---

## Motivation

Storybook is being elevated from a component playground to the project's **unified design surface**: the place where Angular component stories, A2O human stories (`.feature` files), device archetypes, hub archetypes, and frontmatter-bearing design briefs cohere into one browseable artifact. Future work will:

- Pull in A2O `.feature` files via a gherkin loader so user stories sit alongside the components that implement them
- Wire `@storybook/addon-mcp` so backend agents can query the design catalog
- Bootstrap a `frontend-designer` subagent off the resulting CLAUDE.md + design surface
- Long-term: become the foundation for an Elohim Protocol-native graphos design system

This spec is the deployment substrate that makes all of that addressable. It does not implement the A2O integration, the MCP exposure, or the subagent — those are downstream sprints that build on the deployed surface.

A precursor task (Storybook 8 → 10 upgrade) landed before this spec was written; the new pipeline assumes Storybook 10 conventions (ESM `main.ts`, consolidated addons, `storybook/manager-api` and `storybook/theming` import paths).

## Build graph

`elohim-storybook` slots into the orchestrator's `PIPELINES` map as a sibling of `elohim`. Both pipelines consume `app/elohim-library/**` source; neither produces an artifact the other consumes. They run in parallel.

```
elohim-sophia ────────────────┐
elohim-holochain (cascades) ──┴──► elohim ───────────────┐
                                                         ├──► elohim-genesis (A2O)
elohim-holochain ──► elohim-edge ────────────────────────┘

(changes to app/elohim-library trigger both elohim and elohim-storybook in parallel)

                                   elohim-storybook (NEW)
                                   (no downstream edge —
                                    A2O drivers do not
                                    exercise the docs surface)
```

Three properties:

1. **Parallel with elohim**: a single change to `app/elohim-library/projects/lamad-ui/...component.ts` matches the `changePatterns` of both `elohim` (because the app imports lamad-ui via `@app/lamad`) and `elohim-storybook`. They run concurrently. No shared artifact, no cascade edge.
2. **No genesis cascade**: `elohim-storybook` is not in `elohim-genesis.dependsOn`. A2O test drivers exercise the deployed app + doorway + storage, not the docs surface.
3. **Future reshape (not in scope)**: when `elohim-library/projects/lamad-ui` graduates from path-alias-only to ng-packagr publishing with external consumers, it should become its own upstream pipeline `elohim-library`, with both `elohim` and `elohim-storybook` declaring `dependsOn: ['elohim-library']`. The trigger pattern shape proposed below is intentionally compatible with that future move — only the graph edges change at that point.

## Repo layout

**New files**

```
app/elohim-library/
├── Jenkinsfile                          ← pipeline definition
└── images/
    ├── Dockerfile                       ← nginx:alpine + dist/storybook
    └── nginx.conf                       ← gzip + cache headers, no SPA rewrite needed

genesis/orchestrator/manifests/elohim-storybook/
├── alpha.yaml                           ← Deployment
└── alpha/
    ├── service.yaml                     ← ClusterIP service
    └── ingress.yaml                     ← storybook.elohim.host + cert-manager

genesis/orchestrator/scripts/
└── deploy-helpers.groovy                ← extracted deployAppToEnvironment()
```

No configmap. Storybook is fully static — no runtime configuration injection.

No `staging/` or `prod/` manifest dirs. Storybook is a developer surface, not a release-gated product. "Alpha is the latest from dev" is the only contract that makes sense for it. If a staging-pinned snapshot becomes useful later (offsite, design review), that is an additive change.

**Refactor / cleanup (the "ui-playground graduates into storybook" move)**

```
genesis/orchestrator/manifests/elohim-app/{alpha,staging,prod}/
├── service.yaml      ← REMOVE the elohim-ui-playground-{env}-service block
                       (dead Service across all envs — no Deployment selector ever existed)
└── ingress.yaml      ← REMOVE the /ui-playground path entry on {alpha,staging,prod}.elohim.host
                       (dead route — pointed at the dead service)
```

This is in-place graduation, not delete-then-recreate: `ui-playground` was the nascent name for what is now `storybook`. The k8s-resource rename + pulling out the dead subpath in the elohim-app ingress is what lets us call this a refactor.

## Pipeline (`app/elohim-library/Jenkinsfile`)

Lean stages, ~250 LOC target. No holochain dependency, no E2E, no multi-env promotion.

```
1. Check Trigger          validate UpstreamCause from orchestrator (matches existing downstream pattern)
2. Checkout               repo at the orchestrator-supplied commit
3. Setup Version          IMAGE_TAG = "0.1.0-dev-<short-sha>", same scheme as elohim-site
4. Install Dependencies   pnpm install (workspace install from repo root, per existing pattern)
5. Build Storybook        pnpm --filter elohim-library run build-storybook
                          → app/elohim-library/dist/storybook/
6. Build Image            buildkit/nerdctl, app/elohim-library/images/Dockerfile
                          COPY dist/storybook → /usr/share/nginx/html
7. Push to Harbor         harbor.ethosengine.com/ethosengine/elohim-storybook:{IMAGE_TAG, sha, latest}
8. Harbor Security Scan   advisory scan via shared helper
9. Deploy to Alpha        deployStorybookToAlpha() helper — sed placeholders, kubectl apply, rollout status
10. Verify Deploy         curl https://storybook.elohim.host/index.json
                          assert HTTP 200 and entries count > 0
11. Cleanup               workspace cleanup
```

**Trigger guards** (matches existing downstream pattern):

```groovy
overrideIndexTriggers(false)   // orchestrator owns webhooks
// Stage 1 validates: cause must be UpstreamCause (orchestrator) or UserIdCause (manual rebuild)
```

**Helper extraction (the "little c" — directly serves the decoupling goal)**: the `deployAppToEnvironment()` helper currently inlined at root `Jenkinsfile:89` is moved to `genesis/orchestrator/scripts/deploy-helpers.groovy` and loaded via `library` directive. Three callers will benefit (root, new storybook pipeline, eventually doorway-app). Net effect: root Jenkinsfile shrinks ~45 LOC (more headroom under the 64KB CPS limit), the new pipeline picks up the helper for free, future pipelines inherit it. Behavior unchanged.

## Manifests

**`alpha.yaml` (Deployment)**

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
    app.kubernetes.io/component: design-surface     # distinct from "frontend"
    app.kubernetes.io/part-of: elohim
    app.kubernetes.io/managed-by: jenkins
spec:
  replicas: 1
  selector: { matchLabels: { app: elohim-storybook-alpha } }
  template:
    metadata: { labels: <same as above> }
    spec:
      containers:
      - name: elohim-storybook-alpha
        image: harbor.ethosengine.com/ethosengine/elohim-storybook:STORYBOOK_TAG_PLACEHOLDER
        imagePullPolicy: Always
        ports: [{ containerPort: 80 }]
        resources:
          requests: { memory: "64Mi",  cpu: "50m"  }   # static nginx, smaller than the SPA
          limits:   { memory: "128Mi", cpu: "100m" }
```

**`alpha/service.yaml`** — single ClusterIP `elohim-storybook-alpha-service` on port 80, selector `app: elohim-storybook-alpha`.

**`alpha/ingress.yaml`**

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: elohim-storybook-alpha-ingress
  namespace: elohim-alpha
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-production
spec:
  ingressClassName: public
  rules:
  - host: storybook.elohim.host
    http:
      paths:
      - backend: { service: { name: elohim-storybook-alpha-service, port: { number: 80 } } }
        path: /
        pathType: Prefix
  tls:
  - hosts: [storybook.elohim.host]
    secretName: alpha-elohim-storybook-tls-cert
```

## Change-detection rules

New entry in `genesis/orchestrator/Jenkinsfile`'s `PIPELINES` map:

```groovy
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
        'genesis/orchestrator/manifests/elohim-storybook/**',
    ],
    artifacts: ['elohim-storybook'],
    dependsOn: [],
    cascades: false,
    triggersGenesis: false,
    deploymentCheck: [
        dev: 'https://storybook.elohim.host/index.json'
    ]
]
```

Existing `elohim` entry's `changePatterns` already includes `app/elohim-library/`. Unchanged — the app still consumes lamad-ui via path aliases, so any lamad-ui change rebuilds both pipelines in parallel.

**A2O integration extension point (planned, not in scope here)**: when storybook gains a gherkin loader for `.feature` files, that PR adds `genesis/a2o/features/**` to `changePatterns` above. The loader, MDX adapters for archetypes, and trigger expansion ship together — they are coupled and pointless in isolation. No graph edges change at that point either; both `elohim-genesis` and `elohim-storybook` become *readers* of the same `genesis/a2o/features/**` source, neither produces it.

**Build manifest decision**: skip a per-pipeline `build-manifest.json` for v1. The 11 stages do not split into independently-skippable steps in a way that justifies the manifest's overhead. Add later if topological skip optimization becomes worthwhile.

## Pre-deploy prerequisites

**DNS** (one-time, manual): create CNAME

```
storybook   IN   CNAME   <same target as alpha.elohim.host>
```

Cert-manager will auto-issue the LetsEncrypt cert once DNS resolves. Until then, the deploy succeeds at the kubectl level but the ingress will serve a self-signed default cert, and `https://storybook.elohim.host/index.json` will fail TLS verification — the pipeline's `Verify Deploy` stage will fail until DNS propagates and cert-manager completes the challenge. This is expected on the first run; rerun the deploy after DNS confirms.

## Failure modes & rollback

| Failure | Detection | Action |
|---------|-----------|--------|
| Build error (TS / webpack) | `pnpm build-storybook` exit ≠ 0 | Pipeline fails, no image push, no deploy. Last good deploy stays live. |
| Image push fails (Harbor down or auth) | nerdctl push exit ≠ 0 | Pipeline fails, no deploy. Last good deploy stays live. |
| Deploy timeout | `kubectl rollout status --timeout=120s` exit ≠ 0 | Pipeline fails. Operator runs `kubectl rollout undo deployment/elohim-storybook-alpha -n elohim-alpha`. |
| Verify-deploy fails (HTTP ≠ 200 or empty index) | post-deploy step | Pipeline fails. Same rollback. New replica continues serving — failure is a regression-detected signal, not service-down. |

K8s revision history retains previous image:tag, so `kubectl rollout undo` is the standard rollback path. No bespoke tooling.

No alerting in v1. Storybook is a developer surface; outage detection via "I tried to load it." Adding to the existing health-check sweep is a future cheap addition.

## Observability

- **Jenkins pipeline page**: canonical source for build/deploy history.
- **`Verify Deploy` log line**: emits `verified <N> stories indexed` — quick eyeball signal that no stories were silently dropped (the failure mode the precursor 8 → 10 upgrade just hit).
- **Harbor image tags**: `latest`, `<short-sha>`, and full `IMAGE_TAG` per build provide rollback-target lookup.

## Out of scope

- A2O `.feature` integration (needs gherkin loader + MDX adapters; future PR)
- `@storybook/addon-mcp` integration (needs MCP server config + auth)
- `frontend-designer` subagent (needs the design surface deployed first to point at)
- ng-packagr library publishing for lamad-ui (no external consumer yet)
- Multi-env promotion (alpha-only is the contract)
- PR preview deploys (no per-PR ingress provisioning)
- Migration to graphos / EPR-native design system (long-term future)

## References

- Storybook 10 upgrade precursor: changes to `app/elohim-library/.storybook/`, `package.json`, story files (uncommitted at spec time)
- Orchestrator pattern: `genesis/orchestrator/Jenkinsfile`, `graph-walker.mjs`
- Existing deploy helper to extract: root `Jenkinsfile:89` `deployAppToEnvironment()`
- Existing manifest pattern to mirror: `genesis/orchestrator/manifests/elohim-app/alpha*`
- Image build pattern to mirror: root `Jenkinsfile:845` `Build Image` and `:899` `Push to Harbor Registry` stages
