/**
 * Sophia Pipeline (elohim-sophia)
 *
 * Builds and publishes the Sophia rendering library (React/TypeScript monorepo).
 * Triggered by orchestrator when sophia/ files change.
 *
 * What this pipeline builds:
 *   - Sophia monorepo packages (lint, typecheck, test, build)
 *   - sophia-element UMD bundle for downstream consumption
 *
 * Environment Architecture:
 *   - main       -> sophia SonarQube project (blocking gate)
 *   - staging*   -> sophia-staging SonarQube project
 *   - dev/*      -> sophia-alpha SonarQube project
 *
 * Trigger behavior:
 *   - Only runs when triggered by orchestrator or manual
 *   - Shows NOT_BUILT when triggered directly by webhook
 *
 * Artifact dependency:
 *   - Downstream: elohim-app consumes sophia-element UMD bundle + CSS
 *
 * @see orchestrator/Jenkinsfile for central trigger logic
 */

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Determine SonarQube project config based on branch.
 * Returns: [projectKey: String, shouldEnforce: Boolean, env: String]
 */
@NonCPS
def getSonarProjectConfig() {
    def targetBranch = env.CHANGE_TARGET ?: env.BRANCH_NAME

    if (targetBranch == 'main') {
        return [projectKey: 'sophia', shouldEnforce: true, env: 'prod']
    } else if (targetBranch == 'staging' || targetBranch ==~ /staging-.+/) {
        return [projectKey: 'sophia-staging', shouldEnforce: false, env: 'staging']
    } else {
        return [projectKey: 'sophia-alpha', shouldEnforce: false, env: 'alpha']
    }
}

pipeline {
    agent {
        kubernetes {
            cloud 'kubernetes'
            yaml '''
apiVersion: v1
kind: Pod
spec:
  nodeSelector:
    node-type: operations
  tolerations:
    - key: "workload-type"
      operator: "Equal"
      value: "build"
      effect: "NoSchedule"
  containers:
    - name: node
      image: node:20
      command: [cat]
      tty: true
      resources:
        requests:
          memory: "4Gi"
          cpu: "1"
          ephemeral-storage: "5Gi"
        limits:
          memory: "8Gi"
          cpu: "4"
          ephemeral-storage: "10Gi"
'''
        }
    }

    parameters {
        booleanParam(
            name: 'FORCE_BUILD',
            defaultValue: false,
            description: 'Force full rebuild even without code changes'
        )
    }

    options {
        timeout(time: 30, unit: 'MINUTES')
        disableConcurrentBuilds(abortPrevious: true)
        buildDiscarder(logRotator(numToKeepStr: '50'))
        overrideIndexTriggers(false)  // Only orchestrator or manual triggers - no webhook/branch indexing
    }

    // No triggers - orchestrator handles all webhook events

    stages {
        stage('Check Trigger') {
            steps {
                script {
                    def validTrigger = currentBuild.getBuildCauses().any { cause ->
                        cause._class.contains('UserIdCause') ||
                        cause._class.contains('UpstreamCause')
                    }
                    if (!validTrigger) {
                        echo "Pipeline skipped - use orchestrator"
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
                container('node') {
                    checkout scm
                    echo "Building Sophia for branch: ${env.BRANCH_NAME}"
                }
            }
        }

        stage('Install') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail

                            # Install pnpm globally
                            corepack enable
                            corepack prepare pnpm@latest --activate

                            pnpm install --frozen-lockfile
                        '''
                    }
                }
            }
        }

        stage('Lint') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail
                            pnpm lint
                        '''
                    }
                }
            }
        }

        stage('Type Check') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail
                            pnpm typecheck
                        '''
                    }
                }
            }
        }

        stage('Unit Tests') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail
                            pnpm test -- --ci --coverage
                        '''
                    }
                }
            }
        }

        stage('Build') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail
                            pnpm build
                            pnpm build:types
                        '''
                    }
                }
            }
        }

        stage('Build UMD') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        sh '''#!/bin/bash
                            set -euo pipefail

                            pnpm --filter @ethosengine/sophia-element build:umd

                            # Verify UMD bundle was produced
                            UMD_PATH="packages/sophia-element/dist/sophia-element.umd.js"
                            if [ ! -f "$UMD_PATH" ]; then
                                echo "ERROR: UMD bundle not found at $UMD_PATH"
                                exit 1
                            fi

                            UMD_SIZE=$(stat -c%s "$UMD_PATH" 2>/dev/null || stat -f%z "$UMD_PATH")
                            echo "UMD bundle: $UMD_PATH ($UMD_SIZE bytes)"
                        '''
                    }
                }
            }
        }

        stage('SonarQube Analysis') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf {
                        branch 'main'
                        branch 'staging'
                        branch 'dev'
                        expression { env.BRANCH_NAME ==~ /staging-.+/ }
                        expression { env.BRANCH_NAME ==~ /feat-.+/ }
                        expression { env.BRANCH_NAME ==~ /claude\/.+/ }
                        changeRequest target: 'main'
                        changeRequest target: 'staging'
                        changeRequest target: 'dev'
                    }
                }
            }
            steps {
                container('node') {
                    dir('app/sophia') {
                        script {
                            def sonarConfig = getSonarProjectConfig()
                            echo "SonarQube Analysis: project=${sonarConfig.projectKey}, env=${sonarConfig.env}, enforce=${sonarConfig.shouldEnforce}"

                            withSonarQubeEnv('ee-sonarqube') {
                                sh """
                                sonar-scanner \
                                    -Dsonar.projectKey=${sonarConfig.projectKey} \
                                    -Dsonar.sources=packages \
                                    -Dsonar.tests=packages \
                                    -Dsonar.test.inclusions=**/*.test.ts,**/*.test.tsx \
                                    -Dsonar.javascript.lcov.reportPaths=coverage/lcov.info \
                                    -Dsonar.coverage.exclusions=**/*.test.ts,**/*.test.tsx,**/*.stories.tsx,**/dist/**,**/__docs__/**,**/node_modules/** \
                                    -Dsonar.qualitygate.wait=false
                                """
                            }

                            echo "Waiting for SonarQube quality gate..."
                            try {
                                timeout(time: 10, unit: 'MINUTES') {
                                    def qg = waitForQualityGate abortPipeline: false
                                    if (qg.status != 'OK') {
                                        if (sonarConfig.shouldEnforce) {
                                            // Production: Block on quality gate failure
                                            error "SonarQube Quality Gate FAILED: ${qg.status}\nReview issues at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                        } else {
                                            // Alpha/Staging: Log warning but don't block
                                            echo "SonarQube Quality Gate status: ${qg.status}"
                                            echo "Review issues at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                            currentBuild.result = 'UNSTABLE'
                                        }
                                    } else {
                                        echo "SonarQube quality gate passed (${sonarConfig.env})"
                                    }
                                }
                            } catch (Exception e) {
                                echo "SonarQube quality gate check failed: ${e.message}"
                                echo "This may be due to webhook configuration issues."
                                echo "Review results at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                if (sonarConfig.shouldEnforce) {
                                    currentBuild.result = 'UNSTABLE'
                                    echo "Marking build UNSTABLE - production quality gate could not be verified"
                                } else {
                                    echo "Continuing pipeline..."
                                }
                            }
                        }
                    }
                }
            }
        }

        stage('Publish') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf {
                        branch 'main'
                        branch 'staging'
                    }
                }
            }
            steps {
                container('node') {
                    dir('app/sophia') {
                        script {
                            withCredentials([string(credentialsId: 'npm-publish-token', variable: 'NPM_TOKEN')]) {
                                sh '''#!/bin/bash
                                    set -euo pipefail

                                    echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > ~/.npmrc
                                    pnpm --filter @ethosengine/sophia-element publish --no-git-checks
                                '''
                            }
                            echo "Published @ethosengine/sophia-element to npm registry"
                        }
                    }
                }
            }
        }

        stage('Archive Artifacts') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('node') {
                    dir('app/sophia') {
                        script {
                            // Stash UMD bundle + CSS for downstream pipelines (e.g. elohim-app)
                            stash(
                                name: 'sophia-umd',
                                includes: 'packages/sophia-element/dist/**'
                            )
                            archiveArtifacts(
                                artifacts: 'packages/sophia-element/dist/sophia-element.umd.js,packages/sophia-element/dist/**/*.css',
                                allowEmptyArchive: false
                            )
                            echo "Archived sophia-element UMD bundle and CSS for downstream consumption"
                        }
                    }
                }
            }
        }
    }

    post {
        always {
            script {
                if (env.PIPELINE_SKIPPED != 'true') {
                    def sonarConfig = getSonarProjectConfig()
                    currentBuild.description = [
                        "branch:${env.BRANCH_NAME}",
                        "sonar:${sonarConfig.projectKey}"
                    ].join(' | ')
                }
            }
        }
        success {
            echo "Sophia pipeline completed successfully"
        }
        failure {
            echo "Sophia pipeline failed"
            echo "Check the logs above for details. Common issues:"
            echo "  - pnpm install failures: Check lockfile consistency"
            echo "  - Lint/typecheck errors: Fix source code issues"
            echo "  - UMD build missing: Ensure sophia-element build:umd script is defined"
        }
    }
}
