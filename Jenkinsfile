// Best practice: Return values instead of setting env directly
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
       ephemeral-storage: "1Gi"
     limits:
       ephemeral-storage: "2.5Gi"
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
    
    stages {
        stage('Checkout') {
            when {
                anyOf {
                    branch 'main'
                    branch 'staging'
                    branch 'dev'
                    expression { return env.BRANCH_NAME ==~ /review-.+/ }
                    expression { return env.BRANCH_NAME ==~ /feat-.+/ }
                    changeRequest()
                }
            }
            steps {
                container('builder'){
                    script {
                        checkout scm
                        echo "Building branch: ${env.BRANCH_NAME}"
                        echo "Change request: ${env.CHANGE_ID ?: 'None'}"
                    }
                }
            }
        }

        stage('Setup Version') {
            steps {
                container('builder'){
                    script {
                        // Fix git safe.directory issue
                        sh 'git config --global --add safe.directory $(pwd)'
                        
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
                        
                        // Create image tag
                        def imageTag = (env.BRANCH_NAME == 'main') 
                            ? baseVersion 
                            : "${baseVersion}-${env.BRANCH_NAME}-${gitHash}"
                        
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
        
        stage('Install Dependencies') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Installing npm dependencies'
                            sh 'npm ci'
                        }
                    }
                }
            }
        }
        
        stage('Build App') {
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
                                """
                                
                                sh 'npm run build'
                                sh 'ls -la dist/'
                            }
                        }
                    }
                }
            }
        }

        stage('Unit Test') {
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
                                    -Dsonar.javascript.lcov.reportPaths=coverage/elohim-app/lcov.info
                                '''
                            }
                            
                            echo "Waiting for SonarQube quality gate..."
                            timeout(time: 4, unit: 'MINUTES') {
                                def qg = waitForQualityGate()
                                if (qg.status != 'OK') {
                                    error "SonarQube Quality Gate failed: ${qg.status}"
                                }
                                echo "✅ SonarQube Quality Gate passed"
                            }
                        }
                    }
                }
            }
        }

        stage('Build Image') {
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        
                        // Validate required variables
                        if (!props.IMAGE_TAG || !props.GIT_COMMIT_HASH || !props.BASE_VERSION) {
                            error "Missing required build variables: IMAGE_TAG='${props.IMAGE_TAG}', GIT_COMMIT_HASH='${props.GIT_COMMIT_HASH}', BASE_VERSION='${props.BASE_VERSION}'"
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
                                cp images/Dockerfile /tmp/build-context/
                                
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

        stage('Deploy to Staging') {
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
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' manifests/staging-deployment.yaml > manifests/staging-deployment-${IMAGE_TAG}.yaml"
                            
                            // Deploy
                            sh "kubectl apply -f manifests/staging-deployment-${IMAGE_TAG}.yaml"
                            sh 'kubectl rollout status deployment/elohim-site-staging -n ethosengine --timeout=300s'
                            
                            echo 'Staging deployment completed!'
                        }
                    }
                }
            }
        }

        stage('E2E Testing - Staging Validation') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()
                            
                            withBuildVars(props) {
                                echo 'Running E2E tests against staging'
                                env.E2E_TESTS_RAN = 'true'
                                
                                // Install Cypress if needed
                                sh '''
                                    if [ ! -d "node_modules/cypress" ]; then
                                        npm install cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
                                    fi
                                '''
                                
                                // Verify staging is up
                                sh '''
                                    timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" https://staging.elohim.host | grep -q "200\\|302\\|301"; do 
                                        sleep 5
                                    done'
                                    echo "✅ Staging site is responding"
                                '''
                                
                                // Run tests
                                sh """#!/bin/bash
                                    export CYPRESS_baseUrl=https://staging.elohim.host
                                    export CYPRESS_ENV=staging
                                    export CYPRESS_EXPECTED_GIT_HASH=${GIT_COMMIT_HASH}
                                    export NO_COLOR=1
                                    export DISPLAY=:99
                                    
                                    Xvfb :99 -screen 0 1024x768x24 -ac > /dev/null 2>&1 &
                                    XVFB_PID=\$!
                                    sleep 2
                                    
                                    npx cypress verify > /dev/null
                                    mkdir -p cypress/reports
                                    
                                    npx cypress run \\
                                        --headless \\
                                        --browser chromium \\
                                        --spec "cypress/e2e/staging-validation.feature"
                                    
                                    kill \$XVFB_PID 2>/dev/null || true
                                """
                                
                                echo '✅ Staging validation passed!'
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
                            if (env.E2E_TESTS_RAN == 'true') {
                                echo '📊 Publishing cucumber reports...'
                                
                                // Debug: Show what files exist in cypress directory
                                sh 'echo "DEBUG: Contents of cypress directory:"'
                                sh 'find cypress -type f -name "*" 2>/dev/null || echo "cypress directory not found"'
                                
                                // Debug: Show specifically what's in reports directory
                                sh 'echo "DEBUG: Contents of cypress/reports directory:"'
                                sh 'ls -la cypress/reports/ 2>/dev/null || echo "cypress/reports directory not found"'
                                
                                // Debug: Show absolute paths for cucumber plugin
                                sh 'echo "DEBUG: Current working directory: $(pwd)"'
                                sh 'echo "DEBUG: Absolute path to cucumber report: $(pwd)/cypress/reports/cucumber-report.json"'
                                sh 'test -f cypress/reports/cucumber-report.json && echo "DEBUG: File exists and is readable" || echo "DEBUG: File does not exist or is not readable"'
                                
                                // Publish cucumber reports using cucumber plugin
                                if (fileExists('cypress/reports/cucumber-report.json')) {
                                cucumber([
                                    reportTitle: 'E2E Test Results',
                                    fileIncludePattern: 'cucumber-report.json',
                                    jsonReportDirectory: 'cypress/reports',
                                    buildStatus: 'UNSTABLE',
                                    failedFeaturesNumber: -1,
                                    failedScenariosNumber: -1,
                                    failedStepsNumber: -1,
                                    skippedStepsNumber: -1,
                                    pendingStepsNumber: -1,
                                    undefinedStepsNumber: -1
                                ])
                                    echo 'Cucumber reports published successfully with cucumber plugin'
                                } else {
                                    echo 'No cucumber reports found to publish'
                                }
                            } else {
                                echo 'E2E tests did not run - skipping cucumber report publishing'
                            }
                            
                            // Archive test artifacts if E2E tests ran
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
                    }
                }
                failure {
                    echo '❌ E2E tests failed - staging deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('Deploy to Prod') {
            when {
                branch 'main'
            }
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
                            sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' manifests/prod-deployment.yaml > manifests/prod-deployment-${IMAGE_TAG}.yaml" 
                            sh "kubectl apply -f manifests/prod-deployment-${IMAGE_TAG}.yaml"
                            sh "kubectl rollout restart deployment/elohim-site -n ethosengine"
                            sh 'kubectl rollout status deployment/elohim-site -n ethosengine --timeout=300s'
                            
                            echo 'Production deployment completed!'
                        }
                    }
                }
            }
        }

        stage('Cleanup') {
            steps {
                container('builder'){
                    script {
                        echo 'Cleaning up workspace'
                        dir('elohim-app') {
                            sh 'rm -rf node_modules || true'
                        }
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
                                """
                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io rmi elohim-app:latest || true
                                        nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true
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
