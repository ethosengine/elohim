pipeline {
    agent {
        kubernetes {
            cloud 'kubernetes'
            yaml '''
apiVersion: v1
kind: Pod
spec:
 nodeSelector:
    node-type: edge
 containers:
 - name: node
   image: zenika/alpine-chrome:with-node
   command:
   - cat
   tty: true
 - name: docker
   image: docker:dind
   command:
   - cat
   tty: true
   volumeMounts:
   - name: docker-sock
     mountPath: /var/run/docker.sock
 volumes:
  - name: docker-sock
    hostPath:
     path: /var/run/docker.sock
'''
        }
    }
    
    stages {

        stage('Checkout') {
            steps {
                container('node'){
                    script {
                        checkout scm      
                    }
                }
            }
        }
        
        stage('Install Dependencies') {
            steps {
                container('node'){
                    dir('elohim-app') {
                        script {
                            echo 'Installing npm dependencies'
                            sh 'npm ci'
                        }
                    }
                }
            }
        }
        
        stage('Build') {
            steps {
                container('node'){
                    dir('elohim-app') {
                        script {
                            echo 'Building Angular application'
                            sh 'npm run build'
                            echo 'Build output:'
                            sh 'ls -la dist/'
                        }
                    }
                }
            }
        }

        stage('Test') {
            steps {
                container('node'){
                    dir('elohim-app') {
                        script {
                            echo 'Running Angular tests'
                            sh 'npm run test -- --watch=false --browsers=ChromeHeadless'
                        }
                    }
                }
            }
        }

        stage('Build Docker Image') {
            steps {
                container('docker'){
                    script {
                        echo 'Building Docker image'
                        sh 'docker build -t elohim-app:${BUILD_NUMBER} -f images/Dockerfile .'
                        sh 'docker tag elohim-app:${BUILD_NUMBER} elohim-app:latest'
                        echo 'Docker image built successfully'
                    }
                }
            }
        }

    }

    post {
        success {
            echo 'Pipeline completed successfully. Docker image elohim-app:${BUILD_NUMBER} is ready.'
        }
        failure {
            echo 'Pipeline failed. Check the logs for details.'
        }
        always {
            container('node'){
                dir('elohim-app') {
                    script {
                        // Clean up node_modules to save space
                        sh 'rm -rf node_modules || true'
                    }
                }
            }
            container('docker'){
                script {
                    // Clean up local Docker images to save space
                    echo 'Cleaning up Docker images'
                    sh 'docker rmi elohim-app:${BUILD_NUMBER} || true'
                    sh 'docker rmi elohim-app:latest || true'
                    sh 'docker system prune -f || true'
                }
            }
        }
    }
}