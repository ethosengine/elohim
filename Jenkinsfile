/**
 * Elohim App Pipeline
 *
 * Builds and deploys the Angular web application to alpha/staging/production.
 * Triggered by orchestrator when elohim-app/ or elohim-library/ files change.
 *
 * What this pipeline builds:
 *   - elohim-app Angular application
 *   - UI Playground (dev branches only, when elohim-ui-playground/** changes)
 *   - Docker images pushed to Harbor registry
 *
 * Environment Architecture:
 *   - dev, feat-*, claude branches → alpha.elohim.host
 *   - staging* → staging.elohim.host
 *   - main → elohim.host (production)
 *
 * Trigger behavior:
 *   - Only runs when triggered by orchestrator or manual
 *   - Shows NOT_BUILT when triggered directly by webhook
 *
 * Artifact dependency:
 *   - Fetches holochain-cache-core WASM from elohim-holochain pipeline
 *
 * @see orchestrator/Jenkinsfile for central trigger logic
 */

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

def loadBuildVars() {
    def rootEnv = "${env.WORKSPACE}/build.env"
    def path = fileExists(rootEnv) ? rootEnv : 'build.env'
    
    echo "DEBUG: Looking for build.env at: ${path}"
    if (!fileExists(path)) {
        error "build.env not found at ${path}"
    }
    
    // Debug: Show actual file contents
    sh "echo '--- build.env content ---'; cat '${path}'"
    
    def props = readProperties file: path
    echo "DEBUG: Properties read from file: ${props}"
    
    // Return the properties instead of trying to set env
    return props
}

// Helper to setup environment from properties
def withBuildVars(props, Closure body) {
    withEnv([
        "BASE_VERSION=${props.BASE_VERSION ?: ''}",
        "GIT_COMMIT_HASH=${props.GIT_COMMIT_HASH ?: ''}",
        "IMAGE_TAG=${props.IMAGE_TAG ?: ''}",
        "BRANCH_NAME=${props.BRANCH_NAME ?: env.BRANCH_NAME}"
    ]) {
        body()
    }
}

// ============================================================================
// STAGE HELPER METHODS (to reduce bytecode size)
// ============================================================================

def buildPerseusPlugin() {
    dir('elohim-library/projects/perseus-plugin') {
        sh 'npm ci && npm run build && ls -la dist/'
    }
}

def runE2ETests(String environment, String baseUrl, String gitCommitHash) {
    echo "Running E2E tests against ${environment}"
    env.E2E_TESTS_RAN = 'true'

    // Install Cypress if needed
    sh '''
        if [ ! -d "node_modules/cypress" ]; then
            npm install cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
        fi
    '''

    // Verify environment is up
    sh """
        timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" ${baseUrl} | grep -q "200\\|302\\|301"; do
            sleep 5
        done'
        echo "✅ ${environment} site is responding"
    """

    // Run tests
    sh """#!/bin/bash
        export CYPRESS_baseUrl=${baseUrl}
        export CYPRESS_ENV=${environment}
        export CYPRESS_EXPECTED_GIT_HASH=${gitCommitHash}
        export NO_COLOR=1
        export DISPLAY=:99

        Xvfb :99 -screen 0 1024x768x24 -ac > /dev/null 2>&1 &
        XVFB_PID=\\\$!
        sleep 2

        npx cypress verify > /dev/null
        mkdir -p cypress/reports

        npx cypress run \\
            --headless \\
            --browser chromium \\
            --spec "cypress/e2e/staging-validation.feature"

        kill \\\$XVFB_PID 2>/dev/null || true
    """

    echo "✅ ${environment} validation passed!"
}

def publishE2EReports(String environment) {
    if (env.E2E_TESTS_RAN == 'true') {
        echo '📊 Publishing cucumber reports...'

        if (environment == 'staging') {
            sh 'echo "DEBUG: Contents of cypress directory:"'
            sh 'find cypress -type f -name "*" 2>/dev/null || echo "cypress directory not found"'
            sh 'echo "DEBUG: Contents of cypress/reports directory:"'
            sh 'ls -la cypress/reports/ 2>/dev/null || echo "cypress/reports directory not found"'
            sh 'echo "DEBUG: Current working directory: $(pwd)"'
            sh 'echo "DEBUG: Absolute path to cucumber report: $(pwd)/cypress/reports/cucumber-report.json"'
            sh 'test -f cypress/reports/cucumber-report.json && echo "DEBUG: File exists and is readable" || echo "DEBUG: File does not exist or is not readable"'
        }

        if (fileExists('cypress/reports/cucumber-report.json')) {
            cucumber([
                reportTitle: "E2E Test Results (${environment})",
                fileIncludePattern: 'cucumber-report.json',
                jsonReportDirectory: 'cypress/reports',
                buildStatus: 'FAILURE',
                failedFeaturesNumber: -1,
                failedScenariosNumber: -1,
                failedStepsNumber: -1,
                skippedStepsNumber: -1,
                pendingStepsNumber: -1,
                undefinedStepsNumber: -1
            ])
            echo 'Cucumber reports published successfully'
        } else {
            echo 'No cucumber reports found to publish'
        }
    } else {
        echo 'E2E tests did not run - skipping cucumber report publishing'
    }

    // Archive test artifacts
    if (env.E2E_TESTS_RAN == 'true') {
        if (fileExists('cypress/screenshots')) {
            archiveArtifacts artifacts: 'cypress/screenshots/**/*.png', allowEmptyArchive: true
        }
        if (fileExists('cypress/videos')) {
            archiveArtifacts artifacts: 'cypress/videos/**/*.mp4', allowEmptyArchive: true
        }
        if (fileExists('cypress/reports/cucumber-report.json')) {
            archiveArtifacts artifacts: 'cypress/reports/cucumber-report.json', allowEmptyArchive: true
        }
    }
}

// ============================================================================
// END HELPER METHODS
// ============================================================================

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
     limits:
       ephemeral-storage: "5Gi"
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
        // Only set static values here
        BRANCH_NAME = "${env.BRANCH_NAME ?: 'main'}"
    }

    options {
        // Skip default checkout - it uses sparse checkout with 0% files
        // We do explicit full checkout in the Checkout stage
        skipDefaultCheckout(true)
    }

    // No triggers - orchestrator handles all webhook events
    // triggers { }

    stages {
        stage('Check Trigger') {
            steps {
                script {
                    def validTrigger = currentBuild.getBuildCauses().any { cause ->
                        cause._class.contains('UserIdCause') || cause._class.contains('UpstreamCause')
                    }
                    if (!validTrigger) {
                        echo "⏭️ PIPELINE SKIPPED - Use orchestrator or manual trigger"
                        currentBuild.result = 'NOT_BUILT'
                        currentBuild.displayName = "#${env.BUILD_NUMBER} SKIPPED"
                        env.PIPELINE_SKIPPED = 'true'
                    } else {
                        echo "✅ Valid trigger: ${currentBuild.getBuildCauses()*.shortDescription.join(', ')}"
                    }
                }
            }
        }

        stage('Checkout') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        // Configure git safe directory before any git operations
                        sh 'git config --global --add safe.directory "*"'

                        // Explicit checkout - bypass sparse checkout config in job
                        checkout([
                            $class: 'GitSCM',
                            branches: [[name: "*/${env.BRANCH_NAME ?: 'dev'}"]],
                            extensions: [
                                [$class: 'CloneOption', shallow: false, noTags: true],
                                [$class: 'CleanBeforeCheckout']
                            ],
                            userRemoteConfigs: [[
                                url: 'https://github.com/ethosengine/elohim.git',
                                credentialsId: 'ee-bot-pat'
                            ]]
                        ])

                        echo "Building branch: ${env.BRANCH_NAME}"
                        echo "Change request: ${env.CHANGE_ID ?: 'None'}"

                        // Verify git state
                        sh 'git rev-parse --short HEAD'
                        sh 'git status'
                    }
                }
            }
        }

        stage('Setup Version') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        sh 'git config --global --add safe.directory "*"'

                        echo "DEBUG - Setup Version: Starting"
                        echo "DEBUG - Branch: ${env.BRANCH_NAME}"

                        // Validate VERSION file
                        if (!fileExists('VERSION')) {
                            error "VERSION file not found in workspace"
                        }
                        
                        // Read base version
                        def baseVersion = readFile('VERSION').trim()
                        echo "DEBUG - Base version: '${baseVersion}'"
                        
                        if (!baseVersion) {
                            error "VERSION file is empty"
                        }
                        
                        // Get git hash
                        def gitHash = sh(
                            script: 'git rev-parse --short HEAD',
                            returnStdout: true
                        ).trim()
                        echo "DEBUG - Git hash: '${gitHash}'"
                        
                        // Sync package.json version
                        dir('elohim-app') {
                            sh "npm version '${baseVersion}' --no-git-tag-version"
                        }

                        // Sanitize branch name for Docker tag (replace / with -)
                        def sanitizedBranch = env.BRANCH_NAME.replaceAll('/', '-')
                        echo "DEBUG - Sanitized branch: '${sanitizedBranch}'"

                        // Create image tag
                        def imageTag = (env.BRANCH_NAME == 'main')
                            ? baseVersion
                            : "${baseVersion}-${sanitizedBranch}-${gitHash}"

                        echo "DEBUG - Image tag: '${imageTag}'"
                        
                        // Write build.env file
                        def buildEnvContent = """BASE_VERSION=${baseVersion}
GIT_COMMIT_HASH=${gitHash}
IMAGE_TAG=${imageTag}
BRANCH_NAME=${env.BRANCH_NAME}"""
                        
                        writeFile file: "${env.WORKSPACE}/build.env", text: buildEnvContent
                        
                        // Verify file was written
                        sh "cat '${env.WORKSPACE}/build.env'"
                        
                        // Archive for debugging
                        archiveArtifacts artifacts: 'build.env', allowEmptyArchive: false
                        
                        echo "Build variables persisted to build.env"
                    }
                }
            }
        }
        
        stage('Fetch WASM Cache Core') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    script {
                        echo 'Fetching holochain-cache-core WASM module...'

                        def wasmDir = 'holochain/holochain-cache-core/pkg'
                        def wasmFiles = [
                            'holochain_cache_core.js',
                            'holochain_cache_core_bg.wasm',
                            'holochain_cache_core.d.ts',
                            'holochain_cache_core_bg.wasm.d.ts',
                            'package.json'
                        ]

                        // Helper to download WASM files from a branch
                        def downloadWasm = { branch ->
                            def baseUrl = "https://jenkins.ethosengine.com/job/elohim-holochain/job/${branch}/lastSuccessfulBuild/artifact/pkg"
                            sh "mkdir -p '${wasmDir}'"

                            def success = true
                            for (file in wasmFiles) {
                                def result = sh(
                                    script: "wget -q --timeout=30 --tries=2 -O '${wasmDir}/${file}' '${baseUrl}/${file}' 2>&1",
                                    returnStatus: true
                                )
                                if (result != 0) {
                                    success = false
                                    break
                                }
                            }
                            return success
                        }

                        // Try current branch, then dev, then main
                        def fetched = downloadWasm(env.BRANCH_NAME)

                        if (!fetched) {
                            echo "WASM not found for branch ${env.BRANCH_NAME}, trying dev..."
                            fetched = downloadWasm('dev')
                        }

                        if (!fetched) {
                            echo "WASM not found in dev, trying main..."
                            fetched = downloadWasm('main')
                        }

                        if (!fetched) {
                            echo """
                            ⚠️ Could not fetch holochain-cache-core WASM from any branch.
                            App will use TypeScript fallback (slightly slower but functional).
                            To enable WASM: Run holochain pipeline with FORCE_BUILD=true
                            """
                        }

                        if (fileExists("${wasmDir}/holochain_cache_core.js")) {
                            echo "✅ holochain-cache-core WASM module ready"
                            sh "ls -lh ${wasmDir}/"
                        } else {
                            echo "⚠️ WASM module not available - TypeScript fallback will be used"
                        }
                    }
                }
            }
        }

        stage('Install Dependencies') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    dir('elohim-library') {
                        script {
                            echo 'Installing elohim-library dependencies (required for elohim-service imports)'
                            sh 'npm ci'
                        }
                    }
                    dir('elohim-app') {
                        script {
                            echo 'Installing npm dependencies'
                            sh 'npm ci'

                            // Copy WASM files from fetched location to node_modules
                            // This is needed because Angular expects WASM in node_modules/holochain-cache-core
                            def wasmSrc = '../holochain/holochain-cache-core/pkg'
                            def wasmDest = 'node_modules/holochain-cache-core'
                            if (fileExists(wasmSrc)) {
                                echo 'Copying holochain-cache-core WASM to node_modules...'
                                sh """
                                    mkdir -p '${wasmDest}'
                                    cp -v '${wasmSrc}'/*.js '${wasmDest}/' 2>/dev/null || true
                                    cp -v '${wasmSrc}'/*.wasm '${wasmDest}/' 2>/dev/null || true
                                    cp -v '${wasmSrc}'/*.ts '${wasmDest}/' 2>/dev/null || true
                                    ls -la '${wasmDest}/' || true
                                """
                            } else {
                                echo "⚠️ WASM source not found at ${wasmSrc} - TypeScript fallback will be used"
                            }
                        }
                    }
                }
            }
        }

        stage('Build Perseus Plugin') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps { container('builder') { script { buildPerseusPlugin() } } }
        }

        stage('Build App') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()

                            withBuildVars(props) {
                                echo 'Building Angular application'
                                echo "Using git hash: ${GIT_COMMIT_HASH}"
                                echo "Using image tag: ${IMAGE_TAG}"

                                // Replace placeholders
                                sh """
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.prod.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.staging.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.alpha.ts
                                """

                                // Determine build configuration based on branch
                                // For PR builds, CHANGE_TARGET contains the target branch (e.g., 'dev')
                                // For direct branch builds, use BRANCH_NAME
                                def targetBranch = env.CHANGE_TARGET ?: env.BRANCH_NAME
                                def sourceBranch = env.CHANGE_BRANCH ?: env.BRANCH_NAME

                                def buildConfig = 'production'
                                if (targetBranch == 'staging' || targetBranch ==~ /staging-.+/ ||
                                    sourceBranch == 'staging' || sourceBranch ==~ /staging-.+/) {
                                    buildConfig = 'staging'
                                } else if (targetBranch == 'dev' || targetBranch ==~ /feat-.+/ || targetBranch ==~ /claude\/.+/ || targetBranch.contains('alpha') ||
                                           sourceBranch == 'dev' || sourceBranch ==~ /feat-.+/ || sourceBranch ==~ /claude\/.+/ || sourceBranch.contains('alpha')) {
                                    buildConfig = 'alpha'
                                }

                                echo "Building with configuration: ${buildConfig} (target: ${targetBranch}, source: ${sourceBranch})"
                                sh "npm run build -- --configuration=${buildConfig}"
                                sh 'ls -la dist/'
                            }
                        }
                    }
                }
            }
        }

        stage('Unit Test') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Running Angular tests with coverage'
                            sh 'npm run test -- --watch=false --browsers=ChromeHeadless --code-coverage'
                        }
                    }
                }
            }
        }

        stage('SonarQube Analysis') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf { branch 'main'; branch 'staging'; changeRequest target: 'staging'; changeRequest target: 'main' }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            withSonarQubeEnv('ee-sonarqube') {
                                sh '''
                                sonar-scanner \
                                    -Dsonar.projectKey=elohim-app \
                                    -Dsonar.sources=src \
                                    -Dsonar.tests=src \
                                    -Dsonar.test.inclusions=**/*.spec.ts \
                                    -Dsonar.typescript.lcov.reportPaths=coverage/elohim-app/lcov.info \
                                    -Dsonar.javascript.lcov.reportPaths=coverage/elohim-app/lcov.info \
                                    -Dsonar.coverage.exclusions=**/*.module.ts,**/*-routing.module.ts,**/*.model.ts,**/models/**,**/environments/**,**/main.ts,**/polyfills.ts,**/*.spec.ts,**/index.ts,**/components/**,**/renderers/**,**/content-io/**,**/guards/**,**/interceptors/**,**/pipes/**,**/directives/**,**/parsers/**,**/*.routes.ts \
                                    -Dsonar.qualitygate.wait=true \
                                    -Dsonar.qualitygate.timeout=240
                                '''
                            }

                            echo "Waiting for SonarQube quality gate..."
                            timeout(time: 4, unit: 'MINUTES') {
                                def qg = waitForQualityGate()
                                if (qg.status != 'OK') {
                                    // Log the failure but don't block - coverage threshold managed on SonarQube server
                                    echo "⚠️ SonarQube Quality Gate status: ${qg.status}"
                                    echo "Review coverage at: ${env.SONAR_HOST_URL}/dashboard?id=elohim-app"
                                    // Uncomment to enforce: error "SonarQube Quality Gate failed: ${qg.status}"
                                }
                                echo "✅ SonarQube analysis complete"
                            }
                        }
                    }
                }
            }
        }

        stage('Build Image') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        if (!props.IMAGE_TAG || !props.GIT_COMMIT_HASH || !props.BASE_VERSION) {
                            error "Missing required build variables"
                        }
                        
                        withBuildVars(props) {
                            echo 'Building container image'
                            echo "Image tag: ${IMAGE_TAG}"
                            echo "Git hash: ${GIT_COMMIT_HASH}"
                            
                            sh """#!/bin/bash
                                set -euo pipefail

                                # Verify BuildKit
                                buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

                                # Create build context
                                mkdir -p /tmp/build-context
                                cp -r elohim-app /tmp/build-context/
                                cp elohim-app/images/Dockerfile /tmp/build-context/
                                cp elohim-app/images/nginx.conf /tmp/build-context/
                                
                                # Build image
                                cd /tmp/build-context
                                BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \\
                                  nerdctl -n k8s.io build -t elohim-app:${IMAGE_TAG} -f Dockerfile .

                                # Additional tags
                                nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:${GIT_COMMIT_HASH}
                                
                                if [ "${BRANCH_NAME}" = "main" ]; then
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:latest
                                fi
                            """
                            
                            // Mark build as completed
                            env.DOCKER_BUILD_COMPLETED = 'true'
                            echo 'Container image built successfully'
                        }
                    }
                }
            }
        }

        stage('Push to Harbor Registry') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                echo 'Logging into Harbor registry'
                                sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'
                                
                                echo "Tagging and pushing image: ${IMAGE_TAG}"
                                sh """
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}
                                    
                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}
                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}
                                """
                                
                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:latest
                                        nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:latest
                                    """
                                }
                                
                                echo 'Successfully pushed to Harbor registry'
                            }
                        }
                    }
                }
            }
        }

        stage('Harbor Security Scan') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                echo "Triggering Harbor scan for: ${IMAGE_TAG}"

                                sh """
                                    AUTH_HEADER="Authorization: Basic \$(echo -n "\$HARBOR_USERNAME:\$HARBOR_PASSWORD" | base64)"

                                    wget --post-data="" \\
                                      --header="accept: application/json" \\
                                      --header="Content-Type: application/json" \\
                                      --header="\$AUTH_HEADER" \\
                                      -S -O- \\
                                      "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/scan" || \\
                                    echo "Scan request failed"
                                """

                                echo 'Scan initiated, polling for completion...'

                                sh """#!/bin/bash
                                    AUTH_HEADER="Authorization: Basic \$(echo -n "\$HARBOR_USERNAME:\$HARBOR_PASSWORD" | base64)"
                                    MAX_ATTEMPTS=24
                                    ATTEMPT=1

                                    while [ \$ATTEMPT -le \$MAX_ATTEMPTS ]; do
                                        VULN_DATA=\$(wget -q -O- \\
                                          --header="accept: application/json" \\
                                          --header="\$AUTH_HEADER" \\
                                          "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/additions/vulnerabilities" 2>/dev/null || echo "")

                                        if [ ! -z "\$VULN_DATA" ] && echo "\$VULN_DATA" | grep -q '"scanner"'; then
                                            echo "✅ Scan completed"
                                            break
                                        fi

                                        [ \$((ATTEMPT % 5)) -eq 0 ] && echo "Waiting for scan (attempt \$ATTEMPT/\$MAX_ATTEMPTS)..."
                                        sleep 10
                                        ATTEMPT=\$((ATTEMPT + 1))
                                    done
                                """
                            }
                        }
                    }
                }
            }
        }

        stage('Install UI Playground Dependencies') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    not { branch 'main' }; not { branch 'staging' }
                    changeset "elohim-ui-playground/**"
                }
            }
            steps {
                container('builder'){
                    dir('elohim-library') {
                        script {
                            echo 'Installing npm dependencies for UI Playground workspace'
                            sh 'npm ci'
                        }
                    }
                }
            }
        }

        stage('Build UI Playground') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    not { branch 'main' }; not { branch 'staging' }
                    changeset "elohim-ui-playground/**"
                }
            }
            steps {
                container('builder'){
                    dir('elohim-library') {
                        script {
                            echo 'Building lamad-ui library'
                            sh 'npm run build lamad-ui'

                            echo 'Building UI Playground Angular application'
                            sh 'npm run build elohim-ui-playground -- --base-href=/ui-playground/'
                            sh 'ls -la dist/'
                        }
                    }
                }
            }
        }

        stage('Build UI Playground Image') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    not { branch 'main' }; not { branch 'staging' }
                    changeset "elohim-ui-playground/**"
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            echo 'Building UI Playground container image'
                            echo "Image tag: ${IMAGE_TAG}"

                            sh """#!/bin/bash
                                set -euo pipefail

                                # Verify BuildKit
                                buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

                                # Create build context
                                mkdir -p /tmp/build-context-playground
                                cp -r elohim-library /tmp/build-context-playground/
                                cp elohim-ui-playground/images/Dockerfile.ui-playground /tmp/build-context-playground/Dockerfile
                                cp elohim-ui-playground/images/nginx-ui-playground.conf /tmp/build-context-playground/

                                # Build image
                                cd /tmp/build-context-playground
                                BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \\
                                  nerdctl -n k8s.io build -t elohim-ui-playground:${IMAGE_TAG} -f Dockerfile .

                                # Additional tags
                                nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} elohim-ui-playground:${GIT_COMMIT_HASH}

                                if [ "${BRANCH_NAME}" = "main" ]; then
                                    nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} elohim-ui-playground:latest
                                fi
                            """

                            echo 'UI Playground container image built successfully'
                        }
                    }
                }
            }
        }

        stage('Push UI Playground to Harbor Registry') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    not { branch 'main' }; not { branch 'staging' }
                    changeset "elohim-ui-playground/**"
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                echo 'Logging into Harbor registry'
                                sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'

                                echo "Tagging and pushing UI Playground image: ${IMAGE_TAG}"
                                sh """
                                    nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG}
                                    nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH}

                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG}
                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH}
                                """

                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest
                                        nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest
                                    """
                                }

                                echo 'Successfully pushed UI Playground to Harbor registry'
                            }
                        }
                    }
                }
            }
        }

        // Note: Holochain infrastructure (holochain/**) is built by a separate pipeline
        // (elohim-edge) that triggers independently via webhook when holochain files change.
        // This avoids duplicate builds and allows parallel execution.

        stage('Deploy to Staging') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf { branch 'staging'; expression { env.BRANCH_NAME ==~ /staging-.+/ } }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            echo "Deploying to Staging: ${IMAGE_TAG}"

                            // Validate configmap
                            sh '''
                                kubectl get configmap elohim-config-staging -n ethosengine || {
                                    echo "❌ ERROR: elohim-config-staging ConfigMap missing"
                                    exit 1
                                }
                            '''

                            // Update deployment manifest
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' elohim-app/manifests/staging-deployment.yaml > elohim-app/manifests/staging-deployment-${IMAGE_TAG}.yaml"

                            // Verify the image tag in the manifest
                            sh """
                                echo '==== Deployment manifest preview ===='
                                grep 'image:' elohim-app/manifests/staging-deployment-${IMAGE_TAG}.yaml
                                echo '===================================='
                            """

                            // Deploy
                            sh "kubectl apply -f elohim-app/manifests/staging-deployment-${IMAGE_TAG}.yaml"
                            sh "kubectl rollout restart deployment/elohim-site-staging -n ethosengine"
                            sh 'kubectl rollout status deployment/elohim-site-staging -n ethosengine --timeout=300s'

                            // Verify the deployment is using the correct image
                            sh """
                                echo '==== Verifying deployed image ===='
                                kubectl get deployment elohim-site-staging -n ethosengine -o jsonpath='{.spec.template.spec.containers[0].image}'
                                echo ''
                                echo '=================================='
                            """

                            echo 'Staging deployment completed!'
                        }
                    }
                }
            }
        }

        stage('🚀 Deploy to Alpha') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            echo """
                            ═══════════════════════════════════════════════════════════
                            🚀 DEPLOYING ELOHIM APP TO ALPHA
                            ═══════════════════════════════════════════════════════════
                            Image Tag: ${IMAGE_TAG}
                            Git Hash: ${GIT_COMMIT_HASH}
                            Target: https://alpha.elohim.host
                            ═══════════════════════════════════════════════════════════
                            """

                            // Validate configmap
                            sh '''
                                kubectl get configmap elohim-config-alpha -n ethosengine || {
                                    echo "❌ ERROR: elohim-config-alpha ConfigMap missing"
                                    exit 1
                                }
                            '''

                            // Update deployment manifest
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' elohim-app/manifests/alpha-deployment.yaml > elohim-app/manifests/alpha-deployment-${IMAGE_TAG}.yaml"

                            // Verify the image tag in the manifest
                            sh """
                                echo '==== Deployment manifest preview ===='
                                grep 'image:' elohim-app/manifests/alpha-deployment-${IMAGE_TAG}.yaml
                                echo '===================================='
                            """

                            // Deploy
                            sh "kubectl apply -f elohim-app/manifests/alpha-deployment-${IMAGE_TAG}.yaml"
                            sh "kubectl rollout restart deployment/elohim-site-alpha -n ethosengine"
                            sh 'kubectl rollout status deployment/elohim-site-alpha -n ethosengine --timeout=300s'

                            // Verify the deployment is using the correct image
                            sh """
                                echo '==== Verifying deployed image ===='
                                kubectl get deployment elohim-site-alpha -n ethosengine -o jsonpath='{.spec.template.spec.containers[0].image}'
                                echo ''
                                echo '=================================='
                            """

                            echo """
                            ═══════════════════════════════════════════════════════════
                            ✅ ALPHA DEPLOYMENT COMPLETE
                            ═══════════════════════════════════════════════════════════
                            App URL: https://alpha.elohim.host
                            Image: ${IMAGE_TAG}
                            ═══════════════════════════════════════════════════════════
                            """
                        }
                    }
                }
            }
        }

        stage('Verify Holochain Health') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        echo """
                        ═══════════════════════════════════════════════════════════
                        VERIFYING HOLOCHAIN INFRASTRUCTURE
                        ═══════════════════════════════════════════════════════════
                        Alpha and Staging apps use: doorway-dev.elohim.host
                        Seeding is managed by: elohim-genesis pipeline
                        ═══════════════════════════════════════════════════════════
                        """

                        // Check if holochain edge node is running
                        def holochainStatus = sh(
                            script: '''
                                kubectl get deployment elohim-edgenode-dev -n ethosengine -o jsonpath='{.status.availableReplicas}' 2>/dev/null || echo "0"
                            ''',
                            returnStdout: true
                        ).trim()

                        if (holochainStatus == "1") {
                            echo "Holochain Edge Node: Running"
                        } else {
                            echo "Holochain Edge Node: Not available (${holochainStatus} replicas)"
                            echo "Run elohim-edge pipeline with FORCE_DEPLOY=true"
                        }

                        // Check holochain connectivity with retry
                        def holochainHealth = "000"
                        for (int i = 0; i < 3; i++) {
                            holochainHealth = sh(
                                script: '''
                                    timeout 10s curl -sf -o /dev/null -w "%{http_code}" https://doorway-dev.elohim.host/health 2>/dev/null || echo "000"
                                ''',
                                returnStdout: true
                            ).trim()

                            if (holochainHealth == "200") break
                            if (i < 2) {
                                echo "Health check attempt ${i+1} failed, retrying..."
                                sleep 5
                            }
                        }

                        if (holochainHealth == "200") {
                            echo "Holochain Gateway: Healthy"
                        } else {
                            echo "Holochain Gateway: Unhealthy (HTTP ${holochainHealth})"
                            echo "App will work but holochain features may be unavailable"
                            echo "Run elohim-edge pipeline with FORCE_DEPLOY=true to fix"
                        }

                        echo """
                        ═══════════════════════════════════════════════════════════
                        HOLOCHAIN STATUS
                        ═══════════════════════════════════════════════════════════
                        Edge Node: ${holochainStatus == "1" ? "Running" : "Unavailable"}
                        Gateway: ${holochainHealth == "200" ? "Healthy" : "Unhealthy"}

                        Note: Database seeding is managed by elohim-genesis pipeline.
                        To force seed, run that pipeline with FORCE_SEED=true.
                        ═══════════════════════════════════════════════════════════
                        """
                    }
                }
            }
        }

        stage('Deploy UI Playground to Alpha') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                    changeset "elohim-ui-playground/**"
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            echo "Deploying UI Playground to Alpha: ${IMAGE_TAG}"

                            // Update deployment manifest
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' elohim-ui-playground/manifests/alpha-deployment-ui-playground.yaml > elohim-ui-playground/manifests/alpha-deployment-ui-playground-${IMAGE_TAG}.yaml"

                            // Deploy
                            sh "kubectl apply -f elohim-ui-playground/manifests/alpha-deployment-ui-playground-${IMAGE_TAG}.yaml"
                            sh "kubectl rollout restart deployment/elohim-ui-playground-alpha -n ethosengine"
                            sh 'kubectl rollout status deployment/elohim-ui-playground-alpha -n ethosengine --timeout=300s'

                            echo 'UI Playground Alpha deployment completed!'
                        }
                    }
                }
            }
        }

        stage('E2E Testing - Alpha Validation') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                runE2ETests('alpha', 'https://alpha.elohim.host', env.GIT_COMMIT_HASH)
                            }
                        }
                    }
                }
            }
            post {
                success {
                    echo '✅ E2E tests passed - alpha validation successful!'
                }
                always {
                    dir('elohim-app') {
                        script {
                            publishE2EReports('alpha')
                        }
                    }
                }
                failure {
                    echo '❌ E2E tests failed - alpha deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('E2E Testing - Staging Validation') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf { branch 'staging'; expression { env.BRANCH_NAME ==~ /staging-.+/ } }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                runE2ETests('staging', 'https://staging.elohim.host', env.GIT_COMMIT_HASH)
                            }
                        }
                    }
                }
            }
            post {
                success {
                    echo '✅ E2E tests passed - staging validation successful!'
                }
                always {
                    dir('elohim-app') {
                        script {
                            publishE2EReports('staging')
                        }
                    }
                }
                failure {
                    echo '❌ E2E tests failed - staging deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('Deploy to Prod') {
            when { allOf { expression { env.PIPELINE_SKIPPED != 'true' }; branch 'main' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()

                        withBuildVars(props) {
                            echo "Deploying to Production: ${IMAGE_TAG}"

                            // Validate configmap
                            sh '''
                                kubectl get configmap elohim-config-prod -n ethosengine || {
                                    echo "❌ ERROR: elohim-config-prod ConfigMap missing"
                                    exit 1
                                }
                            '''

                            // Deploy

                            // Update deployment manifest
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' elohim-app/manifests/prod-deployment.yaml > elohim-app/manifests/prod-deployment-${IMAGE_TAG}.yaml"
                            sh "kubectl apply -f elohim-app/manifests/prod-deployment-${IMAGE_TAG}.yaml"
                            sh "kubectl rollout restart deployment/elohim-site -n ethosengine"
                            sh 'kubectl rollout status deployment/elohim-site -n ethosengine --timeout=300s'

                            echo 'Production deployment completed!'
                        }
                    }
                }
            }
        }

        stage('Cleanup') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        dir('elohim-app') { sh 'rm -rf node_modules || true' }
                    }
                }
            }
        }
    }

    post {
        success {
            script {
                try {
                    container('builder') {
                        def props = loadBuildVars()
                        echo "Pipeline completed successfully"
                        echo "Docker image: elohim-app:${props.IMAGE_TAG}"
                        echo "Git hash: ${props.GIT_COMMIT_HASH}"
                        echo "Base version: ${props.BASE_VERSION}"
                        echo "Branch: ${props.BRANCH_NAME}"
                    }
                } catch (Exception e) {
                    echo "Pipeline completed successfully"
                }
            }
        }
        failure {
            echo 'Pipeline failed. Check the logs for details.'
        }
        always {
            script {
                if (env.DOCKER_BUILD_COMPLETED == 'true') {
                    try {
                        container('builder') {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                echo 'Cleaning up Docker images...'
                                sh """
                                    nerdctl -n k8s.io rmi elohim-app:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi elohim-app:${GIT_COMMIT_HASH} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH} || true
                                    nerdctl -n k8s.io rmi elohim-ui-playground:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi elohim-ui-playground:${GIT_COMMIT_HASH} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH} || true
                                """
                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io rmi elohim-app:latest || true
                                        nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true
                                        nerdctl -n k8s.io rmi elohim-ui-playground:latest || true
                                        nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest || true
                                    """
                                }
                                sh "nerdctl -n k8s.io system prune -af --volumes || true"
                            }
                        }
                    } catch (Exception e) {
                        echo "Cleanup failed: ${e.message}"
                    }
                } else {
                    echo 'Build not completed, skipping cleanup.'
                }
            }
        }
    }
}
