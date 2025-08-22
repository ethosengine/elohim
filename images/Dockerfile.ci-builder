#podman build --pull --no-cache -t harbor.ethosengine.com/ethosengine/ci-builder:latest .
#podman push harbor.ethosengine.com/ethosengine/ci-builder:latest

# Multi-tool CI Builder Image - Using Debian for better Cypress compatibility
FROM node:20-bookworm

# Install Cypress dependencies as per official documentation
RUN apt-get update && apt-get install -y \
    libgtk2.0-0 \
    libgtk-3-0 \
    libgbm-dev \
    libnotify-dev \
    libnss3 \
    libxss1 \
    libasound2 \
    libxtst6 \
    xauth \
    xvfb \
    && rm -rf /var/lib/apt/lists/*

# Install additional system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    bash \
    unzip \
    sed \
    coreutils \
    ca-certificates \
    openssh-client \
    openjdk-17-jre \
    chromium \
    x11-utils \
    && rm -rf /var/lib/apt/lists/*

# Install kubectl
RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" && \
    chmod +x kubectl && \
    mv kubectl /usr/local/bin/

# Install runc
RUN wget https://github.com/opencontainers/runc/releases/download/v1.1.7/runc.amd64 && \
    chmod +x runc.amd64 && \
    mv runc.amd64 /usr/local/bin/runc

# Set Chrome environment for headless testing with sandbox disabled for root
ENV CHROME_BIN=/usr/bin/chromium \
    CHROME_PATH=/usr/bin/chromium

# Create chromium wrapper script that adds --no-sandbox flag
RUN echo '#!/bin/sh' > /usr/bin/chromium-wrapper && \
    echo 'exec /usr/bin/chromium --no-sandbox --disable-dev-shm-usage --disable-gpu "$@"' >> /usr/bin/chromium-wrapper && \
    chmod +x /usr/bin/chromium-wrapper

# Update Chrome environment to use wrapper
ENV CHROME_BIN=/usr/bin/chromium-wrapper \
    CHROME_PATH=/usr/bin/chromium-wrapper

# Set JAVA_HOME for system JRE
ENV JAVA_HOME=/usr/lib/jvm/java-17-openjdk

# Install nerdctl for container operations
RUN NERDCTL_VERSION=1.7.7 && \
    curl -fsSL https://github.com/containerd/nerdctl/releases/download/v${NERDCTL_VERSION}/nerdctl-${NERDCTL_VERSION}-linux-amd64.tar.gz | \
    tar -xz -C /usr/local/bin/ nerdctl && \
    chmod +x /usr/local/bin/nerdctl

# Install BuildKit client for nerdctl build support
RUN BUILDKIT_VERSION=0.12.5 && \
    curl -fsSL https://github.com/moby/buildkit/releases/download/v${BUILDKIT_VERSION}/buildkit-v${BUILDKIT_VERSION}.linux-amd64.tar.gz | \
    tar -xz -C /tmp/ && \
    mv /tmp/bin/buildctl /usr/local/bin/ && \
    chmod +x /usr/local/bin/buildctl && \
    rm -rf /tmp/bin

# Install SonarQube Scanner and configure to use system JRE
RUN curl -o sonar-scanner.zip -L https://binaries.sonarsource.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-6.2.1.4610-linux-x64.zip && \
    unzip sonar-scanner.zip && \
    mv sonar-scanner-6.2.1.4610-linux-x64 /opt/sonar-scanner && \
    rm sonar-scanner.zip && \
    # Replace glibc Java with Alpine-compatible system Java
    rm -f /opt/sonar-scanner/jre/bin/java && \
    ln -s /usr/bin/java /opt/sonar-scanner/jre/bin/java && \
    ln -s /opt/sonar-scanner/bin/sonar-scanner /usr/local/bin/sonar-scanner

# Set working directory
WORKDIR /workspace

# Keep container running
CMD ["tail", "-f", "/dev/null"]