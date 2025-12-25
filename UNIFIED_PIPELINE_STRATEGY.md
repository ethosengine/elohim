# Unified CI/CD Pipeline Strategy
## Multi-Project, Multi-Environment Architecture

## Overview

This document outlines a unified Jenkins pipeline strategy for the Elohim ecosystem projects:
- **elohim**: 3 environments (Production, Staging, Dev)
- **holochain**: 2 environments (Production, Dev)
- **doorway**: 2 environments (Production, Dev)

## Design Principles

1. **DRY (Don't Repeat Yourself)**: Single source of truth for pipeline logic
2. **Flexibility**: Support 2-environment and 3-environment deployments
3. **Consistency**: Same pipeline behavior across all projects
4. **Maintainability**: Shared library = single place to fix bugs
5. **Scalability**: Easy to add new projects or environments

---

## Architecture

### Jenkins Shared Library Structure

```
jenkins-shared-library/
├── vars/
│   ├── standardPipeline.groovy          # Main pipeline orchestrator
│   ├── buildAngularApp.groovy           # Angular build logic
│   ├── buildGoApp.groovy                # Go build logic (for holochain?)
│   ├── runTests.groovy                  # Test execution
│   ├── deployToEnvironment.groovy       # Generic deployment
│   ├── scanQuality.groovy               # SonarQube scanning
│   └── publishArtifacts.groovy          # Harbor/registry push
├── src/org/ethosengine/
│   ├── DeploymentConfig.groovy          # Environment configuration
│   ├── BuildConfig.groovy               # Build configuration
│   └── NotificationManager.groovy       # Slack/email notifications
└── resources/
    ├── pod-templates/
    │   ├── angular-builder.yaml         # For elohim, doorway
    │   └── go-builder.yaml              # For holochain
    └── deployment-configs/
        ├── elohim.yaml                  # 3-env config
        ├── holochain.yaml               # 2-env config
        └── doorway.yaml                 # 2-env config
```

---

## Environment Configuration Model

### Project Configuration Files

Each project defines its deployment targets:

**elohim.yaml:**
```yaml
project:
  name: elohim
  type: angular
  deploymentTargets:
    - name: dev
      branch: dev
      url: https://alpha.elohim.host
      namespace: ethosengine
      deployment: elohim-site-alpha
      configMap: elohim-config-alpha
      runE2E: true

    - name: staging
      branch: staging
      url: https://staging.elohim.host
      namespace: ethosengine
      deployment: elohim-site-staging
      configMap: elohim-config-staging
      runE2E: true
      requiresApproval: false

    - name: production
      branch: main
      url: https://elohim.host
      namespace: ethosengine
      deployment: elohim-site
      configMap: elohim-config-prod
      runE2E: false
      requiresApproval: true  # Manual approval for prod
```

**holochain.yaml:**
```yaml
project:
  name: holochain
  type: go
  deploymentTargets:
    - name: dev
      branch: dev
      url: https://dev.holochain.ethosengine.com
      namespace: ethosengine
      deployment: holochain-dev

    - name: production
      branch: main
      url: https://holochain.ethosengine.com
      namespace: ethosengine
      deployment: holochain-prod
      requiresApproval: true
```

**doorway.yaml:**
```yaml
project:
  name: doorway
  type: angular
  deploymentTargets:
    - name: dev
      branch: dev
      url: https://dev.doorway.ethosengine.com
      namespace: ethosengine
      deployment: doorway-dev

    - name: production
      branch: main
      url: https://doorway.ethosengine.com
      namespace: ethosengine
      deployment: doorway-prod
      requiresApproval: true
```

---

## Unified Jenkinsfile Template

All three projects use the SAME simplified Jenkinsfile:

**Jenkinsfile (elohim, holochain, doorway):**
```groovy
@Library('ethosengine-pipeline@main') _

// Load project-specific configuration
def config = readYaml file: "deployment-config.yaml"

standardPipeline {
    projectConfig = config

    // Optional: Project-specific overrides
    buildSteps = {
        // Custom build steps if needed
    }

    testSteps = {
        // Custom test steps if needed
    }
}
```

That's it! **~10 lines per project instead of 1,110 lines.**

---

## Shared Library Implementation

### vars/standardPipeline.groovy

```groovy
def call(Map pipelineConfig) {
    def config = pipelineConfig.projectConfig
    def projectType = config.project.type

    pipeline {
        agent {
            kubernetes {
                yaml libraryResource("pod-templates/${projectType}-builder.yaml")
            }
        }

        environment {
            PROJECT_NAME = "${config.project.name}"
            BUILD_CONFIG = buildConfig(this, config)
        }

        stages {
            stage('Checkout & Version') {
                steps {
                    script {
                        checkout scm
                        env.BUILD_VARS = setupVersioning(config)
                    }
                }
            }

            stage('Build') {
                steps {
                    script {
                        if (projectType == 'angular') {
                            buildAngularApp(config: env.BUILD_CONFIG)
                        } else if (projectType == 'go') {
                            buildGoApp(config: env.BUILD_CONFIG)
                        }
                    }
                }
            }

            stage('Test & Quality') {
                parallel {
                    stage('Unit Tests') {
                        steps { runTests(type: 'unit') }
                    }
                    stage('SonarQube') {
                        when {
                            anyOf {
                                branch 'main'
                                branch 'staging'
                                changeRequest target: 'main'
                            }
                        }
                        steps { scanQuality(enforce: true) }
                    }
                }
            }

            stage('Build & Publish Image') {
                steps {
                    script {
                        publishArtifacts(config: env.BUILD_CONFIG)
                    }
                }
            }

            stage('Deploy') {
                steps {
                    script {
                        // Determine which environment to deploy to
                        def target = getDeploymentTarget(env.BRANCH_NAME, config)

                        if (target) {
                            if (target.requiresApproval) {
                                input message: "Deploy to ${target.name}?",
                                      ok: 'Deploy'
                            }

                            deployToEnvironment(
                                target: target,
                                image: env.IMAGE_TAG,
                                config: config
                            )

                            if (target.runE2E) {
                                runE2ETests(
                                    url: target.url,
                                    environment: target.name
                                )
                            }
                        }
                    }
                }
            }
        }

        post {
            always {
                script {
                    notificationManager.send(
                        status: currentBuild.result,
                        project: config.project.name
                    )
                }
            }
        }
    }
}

// Helper function to determine deployment target
def getDeploymentTarget(branchName, config) {
    return config.project.deploymentTargets.find { target ->
        branchName == target.branch ||
        branchName =~ target.branchPattern ||
        (branchName.startsWith('claude/') && target.name == 'dev') ||
        (branchName.startsWith('feat-') && target.name == 'dev')
    }
}
```

---

## vars/deployToEnvironment.groovy

```groovy
def call(Map params) {
    def target = params.target
    def image = params.image
    def config = params.config

    container('builder') {
        echo "🚀 Deploying ${config.project.name} to ${target.name}"
        echo "   URL: ${target.url}"
        echo "   Image: ${image}"

        // Validate prerequisites
        sh """
            kubectl get configmap ${target.configMap} -n ${target.namespace} || {
                echo "❌ ERROR: ConfigMap ${target.configMap} missing"
                exit 1
            }
        """

        // Update deployment manifest
        def manifestPath = "manifests/${target.name}-deployment.yaml"
        sh """
            sed 's/BUILD_NUMBER_PLACEHOLDER/${image}/g' ${manifestPath} > /tmp/deployment.yaml

            echo '==== Deployment Preview ===='
            grep 'image:' /tmp/deployment.yaml
            echo '============================'
        """

        // Apply deployment
        sh """
            kubectl apply -f /tmp/deployment.yaml
            kubectl rollout restart deployment/${target.deployment} -n ${target.namespace}
        """

        // Wait for rollout with health check
        timeout(time: 5, unit: 'MINUTES') {
            sh """
                kubectl rollout status deployment/${target.deployment} -n ${target.namespace} --timeout=300s
            """
        }

        // Verify deployed image
        sh """
            echo '==== Verifying Deployment ===='
            DEPLOYED_IMAGE=\$(kubectl get deployment ${target.deployment} -n ${target.namespace} -o jsonpath='{.spec.template.spec.containers[0].image}')
            echo "Deployed: \$DEPLOYED_IMAGE"
            echo "Expected: ${image}"

            if [[ "\$DEPLOYED_IMAGE" != *"${image}"* ]]; then
                echo "❌ Image mismatch!"
                exit 1
            fi
            echo '✅ Deployment verified'
        """

        echo "✅ ${target.name} deployment completed!"
    }
}
```

---

## Benefits of This Architecture

### 1. Consistency Across Projects
- **Same pipeline logic** for elohim, holochain, doorway
- **Same deployment process** regardless of environment count
- **Same quality gates** enforced everywhere

### 2. Flexibility
```yaml
# Easy to add new environments
deploymentTargets:
  - name: review
    branchPattern: "review-.*"
    ttl: 24h
    ephemeral: true
```

### 3. Maintainability
- **One fix** → all projects benefit
- **Clear separation** of concerns
- **Version-controlled** pipeline library

### 4. Reduced Duplication
| Metric | Before | After |
|--------|--------|-------|
| Elohim Jenkinsfile | 1,110 lines | ~15 lines |
| Holochain Jenkinsfile | ~800 lines | ~15 lines |
| Doorway Jenkinsfile | ~800 lines | ~15 lines |
| **Total** | **~2,710 lines** | **~45 lines + shared library** |

### 5. Easy Onboarding
New project setup:
```bash
# 1. Copy template
cp templates/deployment-config.yaml new-project/

# 2. Edit config
vim new-project/deployment-config.yaml

# 3. Copy Jenkinsfile
cp templates/Jenkinsfile new-project/

# Done! Pipeline automatically inherits all features
```

---

## Migration Strategy

### Phase 1: Create Shared Library (Week 1)
1. Create `jenkins-shared-library` repository
2. Implement core functions (deploy, build, test)
3. Test with elohim on `claude/*` branches

### Phase 2: Migrate Elohim (Week 2)
1. Create `deployment-config.yaml` for elohim
2. Replace 1,110-line Jenkinsfile with 15-line version
3. Test on dev → staging → production

### Phase 3: Migrate Holochain & Doorway (Week 3)
1. Create configs for holochain and doorway
2. Copy standardized Jenkinsfile
3. Test deployments
4. Decommission old Jenkinsfiles

### Phase 4: Enhance (Week 4+)
1. Add deployment previews
2. Add automated rollback
3. Add performance monitoring
4. Add cost tracking

---

## Environment-Specific Behaviors

### Automatic Behavior Based on Config

```groovy
// In standardPipeline.groovy
stage('E2E Tests') {
    when {
        expression {
            def target = getDeploymentTarget(env.BRANCH_NAME, config)
            return target?.runE2E == true
        }
    }
    steps {
        runE2ETests()
    }
}

stage('Manual Approval') {
    when {
        expression {
            def target = getDeploymentTarget(env.BRANCH_NAME, config)
            return target?.requiresApproval == true
        }
    }
    steps {
        input message: "Deploy to ${target.name}?"
    }
}
```

### Branch to Environment Mapping

```yaml
# Elohim (3 environments)
claude/* → dev
feat-*   → dev
dev      → dev
staging  → staging
main     → production

# Holochain & Doorway (2 environments)
claude/* → dev
feat-*   → dev
dev      → dev
main     → production
```

---

## Next Steps

1. **Create shared library repository**
   ```bash
   cd /home/user
   mkdir jenkins-shared-library
   cd jenkins-shared-library
   git init
   ```

2. **Implement core functions** (deploy, build, test)

3. **Create deployment configs** for all 3 projects

4. **Test with elohim first** (lowest risk)

5. **Roll out to holochain and doorway**

---

## Questions for Alignment

1. **Holochain & Doorway Tech Stack**:
   - What technology? (Go, Node.js, Rust?)
   - Do they use Docker/Kubernetes deployments like elohim?

2. **Current Pain Points**:
   - Are holochain and doorway experiencing similar pipeline issues?
   - Do they have the same Jenkinsfile size problems?

3. **Deployment Approval**:
   - Should production deployments require manual approval?
   - Who approves? (Auto-approve from CI if all checks pass?)

4. **Notification Preferences**:
   - Slack? Email? Both?
   - Different channels per project or unified?

5. **Testing Strategy**:
   - Do holochain/doorway have E2E tests?
   - Should dev deployments automatically run E2E tests?
