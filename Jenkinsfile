def loadBuildVars() {
    def rootEnv = "${env.WORKSPACE}/build.env"
    def path = fileExists(rootEnv) ? rootEnv : 'build.env'

    if (!fileExists(path)) {
        error "build.env not found at ${path}"
    }

    return readProperties file: path
}

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
                    expression { return env.BRANCH_NAME ==~ /staging-.+/ }
                    expression { return env.BRANCH_NAME ==~ /feat-.+/ }
                    expression { return env.BRANCH_NAME ==~ /alpha-.+/ }
                    expression { return env.BRANCH_NAME ==~ /claude\/.+/ }
                    changeRequest()
                }
            }
            steps {
                container('builder'){
                    script {
                        sh 'git config --global --add safe.directory "*"'
                        checkout scm
                        sh 'git clean -fdx && git reset --hard HEAD'
                        echo "Building branch: ${env.BRANCH_NAME}"
                    }
                }
            }
        }

        stage('Setup Version') {
            steps {
                container('builder'){
                    script {
                        sh 'git config --global --add safe.directory "*"'

                        def baseVersion = readFile('VERSION').trim()
                        if (!baseVersion) error "VERSION file is empty"

                        def gitHash = sh(script: 'git rev-parse --short HEAD', returnStdout: true).trim()

                        dir('elohim-app') {
                            sh "npm version '${baseVersion}' --no-git-tag-version"
                        }

                        def sanitizedBranch = env.BRANCH_NAME.replaceAll('/', '-')
                        def imageTag = (env.BRANCH_NAME == 'main') ? baseVersion : "${baseVersion}-${sanitizedBranch}-${gitHash}"

                        def buildEnvContent = """BASE_VERSION=${baseVersion}
GIT_COMMIT_HASH=${gitHash}
IMAGE_TAG=${imageTag}
BRANCH_NAME=${env.BRANCH_NAME}"""

                        writeFile file: "${env.WORKSPACE}/build.env", text: buildEnvContent
                        archiveArtifacts artifacts: 'build.env', allowEmptyArchive: false
                        echo "Build variables: ${imageTag}"
                    }
                }
            }
        }

        stage('Install Dependencies') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        sh 'npm ci'
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
                                sh """
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.prod.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.staging.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.alpha.ts
                                    npm run build
                                """
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
                        sh 'npm run test -- --watch=false --browsers=ChromeHeadless --code-coverage'
                    }
                }
            }
        }

        stage('SonarQube Analysis') {
            when {
                anyOf {
                    branch 'main'
                    branch 'staging'
                    changeRequest target: 'staging'
                    changeRequest target: 'main'
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

                            timeout(time: 4, unit: 'MINUTES') {
                                def qg = waitForQualityGate()
                                if (qg.status != 'OK') {
                                    echo "⚠️ SonarQube Quality Gate status: ${qg.status}"
                                }
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
                        if (!props.IMAGE_TAG || !props.GIT_COMMIT_HASH) {
                            error "Missing required build variables"
                        }

                        withBuildVars(props) {
                            sh "bash jenkins/scripts/build-image.sh ${IMAGE_TAG} ${GIT_COMMIT_HASH} ${BRANCH_NAME}"
                            env.DOCKER_BUILD_COMPLETED = 'true'
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
                                sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'
                                sh "bash jenkins/scripts/push-to-harbor.sh ${IMAGE_TAG} ${GIT_COMMIT_HASH} ${BRANCH_NAME}"
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
                                sh "bash jenkins/scripts/harbor-security-scan.sh ${IMAGE_TAG}"
                            }
                        }
                    }
                }
            }
        }

        stage('Install UI Playground Dependencies') {
            when {
                allOf {
                    not { branch 'main' }
                    not { branch 'staging' }
                    not { expression { return env.BRANCH_NAME ==~ /staging-.+/ } }
                    not { expression { return env.BRANCH_NAME ==~ /review-.+/ } }
                    anyOf {
                        changeset "elohim-library/**"
                        changeset "elohim-ui-playground/**"
                        changeset "images/Dockerfile.ui-playground"
                    }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-library') {
                        sh 'npm ci'
                    }
                }
            }
        }

        stage('Build UI Playground') {
            when {
                allOf {
                    not { branch 'main' }
                    not { branch 'staging' }
                    not { expression { return env.BRANCH_NAME ==~ /staging-.+/ } }
                    not { expression { return env.BRANCH_NAME ==~ /review-.+/ } }
                    anyOf {
                        changeset "elohim-library/**"
                        changeset "elohim-ui-playground/**"
                        changeset "images/Dockerfile.ui-playground"
                    }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-library') {
                        sh 'npm run build lamad-ui'
                        sh 'npm run build elohim-ui-playground -- --base-href=/ui-playground/'
                    }
                }
            }
        }

        stage('Build UI Playground Image') {
            when {
                allOf {
                    not { branch 'main' }
                    not { branch 'staging' }
                    not { expression { return env.BRANCH_NAME ==~ /staging-.+/ } }
                    not { expression { return env.BRANCH_NAME ==~ /review-.+/ } }
                    anyOf {
                        changeset "elohim-library/**"
                        changeset "elohim-ui-playground/**"
                        changeset "images/Dockerfile.ui-playground"
                    }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            sh "bash jenkins/scripts/build-ui-playground-image.sh ${IMAGE_TAG} ${GIT_COMMIT_HASH} ${BRANCH_NAME}"
                        }
                    }
                }
            }
        }

        stage('Push UI Playground to Harbor Registry') {
            when {
                allOf {
                    not { branch 'main' }
                    not { branch 'staging' }
                    not { expression { return env.BRANCH_NAME ==~ /staging-.+/ } }
                    not { expression { return env.BRANCH_NAME ==~ /review-.+/ } }
                    anyOf {
                        changeset "elohim-library/**"
                        changeset "elohim-ui-playground/**"
                        changeset "images/Dockerfile.ui-playground"
                    }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'
                                sh "bash jenkins/scripts/push-ui-playground-to-harbor.sh ${IMAGE_TAG} ${GIT_COMMIT_HASH} ${BRANCH_NAME}"
                            }
                        }
                    }
                }
            }
        }

        stage('Deploy to Staging') {
            when {
                anyOf {
                    branch 'staging'
                    expression { return env.BRANCH_NAME ==~ /staging-.+/ }
                    expression { return env.BRANCH_NAME ==~ /review-.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            sh '''
                                kubectl get configmap elohim-config-staging -n ethosengine
                                sed "s/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g" manifests/staging-deployment.yaml > manifests/staging-deployment-${IMAGE_TAG}.yaml
                                kubectl apply -f manifests/staging-deployment-${IMAGE_TAG}.yaml
                                kubectl rollout restart deployment/elohim-site-staging -n ethosengine
                                kubectl rollout status deployment/elohim-site-staging -n ethosengine --timeout=300s
                            '''
                            echo 'Staging deployment completed!'
                        }
                    }
                }
            }
        }

        stage('Deploy to Alpha') {
            when {
                anyOf {
                    branch 'dev'
                    expression { return env.BRANCH_NAME ==~ /feat-.+/ }
                    expression { return env.BRANCH_NAME ==~ /claude\/.+/ }
                    expression { return env.BRANCH_NAME.contains('alpha') }
                    expression { return env.CHANGE_BRANCH && env.CHANGE_BRANCH ==~ /claude\/.+/ }
                    expression { return env.CHANGE_BRANCH && env.CHANGE_BRANCH ==~ /feat-.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            sh '''
                                kubectl get configmap elohim-config-alpha -n ethosengine
                                sed "s/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g" manifests/alpha-deployment.yaml > manifests/alpha-deployment-${IMAGE_TAG}.yaml
                                kubectl apply -f manifests/alpha-deployment-${IMAGE_TAG}.yaml
                                kubectl rollout restart deployment/elohim-site-alpha -n ethosengine
                                kubectl rollout status deployment/elohim-site-alpha -n ethosengine --timeout=300s
                            '''
                            echo 'Alpha deployment completed!'
                        }
                    }
                }
            }
        }

        stage('Deploy UI Playground to Alpha') {
            when {
                allOf {
                    anyOf {
                        branch 'dev'
                        expression { return env.BRANCH_NAME ==~ /feat-.+/ }
                        expression { return env.BRANCH_NAME ==~ /claude\/.+/ }
                        expression { return env.BRANCH_NAME.contains('alpha') }
                        expression { return env.CHANGE_BRANCH && env.CHANGE_BRANCH ==~ /claude\/.+/ }
                    }
                    anyOf {
                        changeset "elohim-library/**"
                        changeset "elohim-ui-playground/**"
                        changeset "images/Dockerfile.ui-playground"
                    }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            sh '''
                                sed "s/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g" manifests/alpha-deployment-ui-playground.yaml > manifests/alpha-deployment-ui-playground-${IMAGE_TAG}.yaml
                                kubectl apply -f manifests/alpha-deployment-ui-playground-${IMAGE_TAG}.yaml
                                kubectl rollout restart deployment/elohim-ui-playground-alpha -n ethosengine
                                kubectl rollout status deployment/elohim-ui-playground-alpha -n ethosengine --timeout=300s
                            '''
                        }
                    }
                }
            }
        }

        stage('E2E Testing - Alpha') {
            when {
                anyOf {
                    branch 'dev'
                    expression { return env.BRANCH_NAME ==~ /feat-.+/ }
                    expression { return env.BRANCH_NAME ==~ /claude\/.+/ }
                    expression { return env.BRANCH_NAME ==~ /alpha-.+/ }
                    expression { return env.CHANGE_BRANCH && env.CHANGE_BRANCH ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                env.E2E_TESTS_RAN = 'true'
                                sh '''
                                    if [ ! -d "node_modules/cypress" ]; then
                                        npm install cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
                                    fi
                                    timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" https://alpha.elohim.host | grep -q "200\\|302\\|301"; do sleep 5; done'
                                '''
                                sh "bash ../jenkins/scripts/run-e2e-tests.sh https://alpha.elohim.host alpha ${GIT_COMMIT_HASH}"
                            }
                        }
                    }
                }
            }
            post {
                always {
                    dir('elohim-app') {
                        script {
                            if (env.E2E_TESTS_RAN == 'true' && fileExists('cypress/reports/cucumber-report.json')) {
                                cucumber([
                                    reportTitle: 'E2E Test Results (Alpha)',
                                    fileIncludePattern: 'cucumber-report.json',
                                    jsonReportDirectory: 'cypress/reports'
                                ])
                            }
                            archiveArtifacts artifacts: 'cypress/**/*.png,cypress/**/*.mp4,cypress/reports/*.json', allowEmptyArchive: true
                        }
                    }
                }
            }
        }

        stage('E2E Testing - Staging') {
            when {
                anyOf {
                    branch 'staging'
                    expression { return env.BRANCH_NAME ==~ /staging-.+/ }
                    expression { return env.BRANCH_NAME ==~ /review-.+/ }
                }
            }
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                env.E2E_TESTS_RAN = 'true'
                                sh '''
                                    if [ ! -d "node_modules/cypress" ]; then
                                        npm install cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
                                    fi
                                    timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" https://staging.elohim.host | grep -q "200\\|302\\|301"; do sleep 5; done'
                                '''
                                sh "bash ../jenkins/scripts/run-e2e-tests.sh https://staging.elohim.host staging ${GIT_COMMIT_HASH}"
                            }
                        }
                    }
                }
            }
            post {
                always {
                    dir('elohim-app') {
                        script {
                            if (env.E2E_TESTS_RAN == 'true' && fileExists('cypress/reports/cucumber-report.json')) {
                                cucumber([
                                    reportTitle: 'E2E Test Results (Staging)',
                                    fileIncludePattern: 'cucumber-report.json',
                                    jsonReportDirectory: 'cypress/reports'
                                ])
                            }
                            archiveArtifacts artifacts: 'cypress/**/*.png,cypress/**/*.mp4,cypress/reports/*.json', allowEmptyArchive: true
                        }
                    }
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
                            sh '''
                                kubectl get configmap elohim-config-prod -n ethosengine
                                sed "s/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g" manifests/prod-deployment.yaml > manifests/prod-deployment-${IMAGE_TAG}.yaml
                                kubectl apply -f manifests/prod-deployment-${IMAGE_TAG}.yaml
                                kubectl rollout restart deployment/elohim-site -n ethosengine
                                kubectl rollout status deployment/elohim-site -n ethosengine --timeout=300s
                            '''
                            echo 'Production deployment completed!'
                        }
                    }
                }
            }
        }

        stage('Cleanup') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        sh 'rm -rf node_modules || true'
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
                        echo "Pipeline completed - Image: ${props.IMAGE_TAG}"
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
                                sh "bash jenkins/scripts/cleanup-images.sh ${IMAGE_TAG} ${GIT_COMMIT_HASH} ${BRANCH_NAME}"
                            }
                        }
                    } catch (Exception e) {
                        echo "Cleanup failed: ${e.message}"
                    }
                }
            }
        }
    }
}
