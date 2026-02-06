
#podman build --pull --no-cache -t harbor.ethosengine.com/devspaces/udi-plus:latest .
#podman push harbor.ethosengine.com/devspaces/udi-plus:latest

# UBI10 provides GCC 14.2 with GLIBCXX_3.4.32, needed for pre-built Holochain binaries (require 3.4.30)
# RHEL 10 released May 2025, based on Fedora 40, supported until 2035
# See: https://lwn.net/Articles/1021827/
#
# NOTE: Using base-developer-image (not universal-developer-image) because ubi10-latest
# only exists for base. The base image (~800MB) lacks languages/tools that universal has (~8.75GB):
#   - Languages: Java, Node.js, Python, Go, Rust, .NET, PHP, C/C++, Scala
#   - Cloud tools: kubectl, helm, terraform, oc, docker-compose
#   - Build tools: Maven, Gradle
# We install what we need below.
FROM quay.io/devfile/base-developer-image:ubi10-latest

USER root

# Install Node.js (required for npm-based tools like claude-code, gemini-cli)
# Using NodeSource for latest LTS
RUN dnf install -y nodejs npm && dnf clean all

# Install Code Assistants globally
RUN npm install -g @anthropic-ai/claude-code
RUN npm install -g @google/gemini-cli

# Install Java 21 via dnf
RUN dnf install -y java-21-openjdk-headless java-21-openjdk-devel && \
    dnf clean all

# Set Java 21 as the system default for mpc plugin support
ENV JAVA_HOME=/usr/lib/jvm/java-21-openjdk
ENV PATH=/usr/lib/jvm/java-21-openjdk/bin:$PATH

# Verify Java 21 is working
RUN java -version && javac -version

# Install essential CLI tools for Claude Code context management
RUN dnf install -y \
    ncdu \
    fzf \
    screen \
    && dnf clean all

# Install Chrome for Testing (Google's official distribution for automated testing)
# Provides a stable, headless-capable Chrome binary for npm/Karma tests
# Note: wget is already in base image, just need unzip temporarily
RUN dnf install -y unzip && \
    wget -q https://storage.googleapis.com/chrome-for-testing-public/131.0.6778.87/linux64/chrome-linux64.zip && \
    unzip -q chrome-linux64.zip -d /opt/ && \
    rm chrome-linux64.zip && \
    ln -s /opt/chrome-linux64/chrome /usr/local/bin/chrome && \
    ln -s /opt/chrome-linux64/chrome /usr/local/bin/google-chrome && \
    dnf clean all

# Install Chrome runtime dependencies for graphics rendering
RUN dnf install -y \
    libdrm \
    mesa-libgbm \
    && dnf clean all

# Set Chrome binary path for Karma and other testing tools
ENV CHROME_BIN=/usr/local/bin/chrome
ENV CHROME_PATH=/opt/chrome-linux64/chrome

# Create Claude + MCP dirs and make them user-writable
RUN mkdir -p /home/user/.claude \
           /home/user/.cache/sonarqube-mcp \
           /opt/mcp \
 && chown -R user:user /home/user/.claude /home/user/.cache /opt/mcp \
 && chmod -R 775 /home/user/.claude /home/user/.cache /opt/mcp

# Download SonarQube MCP JAR
ARG SONAR_MCP_VERSION=0.0.8.1353
RUN curl -fL -o /opt/mcp/sonarqube-mcp.jar \
  "https://github.com/SonarSource/sonarqube-mcp-server/releases/download/${SONAR_MCP_VERSION}/sonarqube-mcp-server-${SONAR_MCP_VERSION}.jar"

USER user

# Set user environment for Java 21
ENV JAVA_HOME=/usr/lib/jvm/java-21-openjdk
ENV PATH=/usr/lib/jvm/java-21-openjdk/bin:$PATH

# NOTE: VS Code CLI symlink setup
# The Che editor provides 'code-oss' at /checode/checode-linux-libc/ubi9/bin/remote-cli/code-oss
# (path may change with ubi10). Many tools expect 'code' as the command name.
# Since /home is overwritten at Eclipse Che workspace startup, the symlink must be created
# via postStart commands in devfile.yaml files (not here in the Dockerfile).
# ⚠️ REVIEW REQUIRED: With ubi10 base image, verify the code-oss path still works.

# Verify installation
RUN claude --version || echo "Claude installed but needs auth"
RUN java -version



# podman build --pull --no-cache -t harbor.ethosengine.com/devspaces/rust-nix-dev:latest .
# podman push harbor.ethosengine.com/devspaces/rust-nix-dev:latest

# ============================================================================
# Rust + Holochain Development Container for Eclipse Che/DevSpaces
# ============================================================================
#
# CRITICAL: This image is designed for Eclipse Che/DevSpaces where:
# - PVCs completely REPLACE mounted directories (e.g., ~/.cargo becomes empty)
# - Containers run with arbitrary UIDs (not necessarily 1000)
# - UDI-plus (UBI10) provides DevSpaces integration, Claude Code, Java 21, etc.
#
# Architecture:
# - Rust toolchain installed to /opt/rust (survives PVC mounts on /home/user)
# - Holochain binaries pre-downloaded to /opt/holochain
# - Nix installer cached for optional use (not required for Rust)
#
# Holochain binaries work because UBI10 provides GLIBCXX_3.4.32 (requires 3.4.30)
# ============================================================================

FROM harbor.ethosengine.com/devspaces/udi-plus:latest

USER root

# Install system dependencies for Rust development
RUN dnf install -y \
    xz \
    perl \
    perl-Digest-SHA \
    perl-IPC-Cmd \
    git \
    gcc \
    gcc-c++ \
    make \
    openssl-devel \
    pkg-config \
    && dnf clean all

# ============================================================================
# Rust toolchain (installed to /opt/rust to survive PVC mounts)
# ============================================================================
ENV RUSTUP_HOME=/opt/rust/rustup
ENV CARGO_HOME=/opt/rust/cargo
ENV PATH=/opt/rust/cargo/bin:$PATH

# Create rust directories with proper permissions for arbitrary UID
RUN mkdir -p /opt/rust/rustup /opt/rust/cargo && \
    chown -R user:root /opt/rust && \
    chmod -R g=u /opt/rust

# Install Rust as user
USER user
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --no-modify-path

# Install common Rust components
RUN rustup component add rust-analyzer rustfmt clippy

# Install WASM target for Holochain DNA/zome compilation
RUN rustup target add wasm32-unknown-unknown

# Verify Rust installation
RUN rustc --version && cargo --version && rust-analyzer --version && \
    rustup target list --installed | grep wasm32

USER root

# ============================================================================
# Holochain binaries (pre-built, requires UBI10 for GLIBCXX_3.4.30)
# ============================================================================
ARG HOLOCHAIN_VERSION=0.6.0
ARG LAIR_VERSION=0.5.3
RUN mkdir -p /opt/holochain/bin && \
    curl -sL -o /opt/holochain/bin/holochain \
      "https://github.com/holochain/holochain/releases/download/holochain-${HOLOCHAIN_VERSION}/holochain-x86_64-unknown-linux-gnu" && \
    curl -sL -o /opt/holochain/bin/hc \
      "https://github.com/holochain/holochain/releases/download/holochain-${HOLOCHAIN_VERSION}/hc-x86_64-unknown-linux-gnu" && \
    curl -sL -o /opt/holochain/bin/hcterm \
      "https://github.com/holochain/holochain/releases/download/holochain-${HOLOCHAIN_VERSION}/hcterm-x86_64-unknown-linux-gnu" && \
    curl -sL -o /opt/holochain/bin/lair-keystore \
      "https://github.com/holochain/lair/releases/download/lair_keystore-v${LAIR_VERSION}/lair-keystore-v${LAIR_VERSION}-x86_64-unknown-linux-gnu" && \
    chmod +x /opt/holochain/bin/* && \
    chown -R user:root /opt/holochain && \
    chmod -R g=u /opt/holochain

# ============================================================================
# Optional: Nix installer (cached for users who want it)
# ============================================================================
RUN curl -L https://nixos.org/nix/install -o /tmp/nix-installer.sh && \
    chmod +x /tmp/nix-installer.sh

# ============================================================================
# Helper scripts in /home/user/bin
# ============================================================================
RUN mkdir -p /home/user/bin

# Create rust-status helper command
RUN echo '#!/bin/bash' > /home/user/bin/rust-status && \
    echo 'echo "=== Rust Installation ==="' >> /home/user/bin/rust-status && \
    echo 'echo "RUSTUP_HOME: $RUSTUP_HOME"' >> /home/user/bin/rust-status && \
    echo 'echo "CARGO_HOME: $CARGO_HOME"' >> /home/user/bin/rust-status && \
    echo 'echo ""' >> /home/user/bin/rust-status && \
    echo 'if command -v rustc &> /dev/null; then' >> /home/user/bin/rust-status && \
    echo '    rustc --version' >> /home/user/bin/rust-status && \
    echo '    cargo --version' >> /home/user/bin/rust-status && \
    echo '    rustup show' >> /home/user/bin/rust-status && \
    echo 'else' >> /home/user/bin/rust-status && \
    echo '    echo "ERROR: Rust not found in PATH"' >> /home/user/bin/rust-status && \
    echo '    echo "PATH: $PATH"' >> /home/user/bin/rust-status && \
    echo 'fi' >> /home/user/bin/rust-status && \
    echo 'echo ""' >> /home/user/bin/rust-status && \
    echo 'echo "=== Cargo Cache Usage ==="' >> /home/user/bin/rust-status && \
    echo 'du -sh ~/.cargo 2>/dev/null || echo "~/.cargo: empty or not mounted"' >> /home/user/bin/rust-status && \
    echo 'du -sh $CARGO_HOME 2>/dev/null || echo "$CARGO_HOME: not found"' >> /home/user/bin/rust-status && \
    echo 'echo ""' >> /home/user/bin/rust-status && \
    echo 'echo "=== Holochain Binaries ==="' >> /home/user/bin/rust-status && \
    echo 'holochain --version 2>/dev/null || echo "holochain: not found"' >> /home/user/bin/rust-status && \
    echo 'hc --version 2>/dev/null || echo "hc: not found"' >> /home/user/bin/rust-status && \
    echo 'lair-keystore --version 2>/dev/null || echo "lair-keystore: not found"' >> /home/user/bin/rust-status && \
    chmod +x /home/user/bin/rust-status

# Create holo-help helper command
RUN echo '#!/bin/bash' > /home/user/bin/holo-help && \
    echo 'echo "Holochain CLI tools (pre-built binaries in /opt/holochain/bin):"' >> /home/user/bin/holo-help && \
    echo 'echo ""' >> /home/user/bin/holo-help && \
    echo 'echo "  holochain --version    # Conductor version"' >> /home/user/bin/holo-help && \
    echo 'echo "  hc --version           # CLI version"' >> /home/user/bin/holo-help && \
    echo 'echo "  lair-keystore --help   # Key management"' >> /home/user/bin/holo-help && \
    echo 'echo ""' >> /home/user/bin/holo-help && \
    echo 'echo "Scaffolding (P2P Shipyard / DarkSoil):"' >> /home/user/bin/holo-help && \
    echo 'echo "  hc scaffold            # Interactive scaffolding"' >> /home/user/bin/holo-help && \
    echo 'echo "  hc scaffold web-app    # Create new web app"' >> /home/user/bin/holo-help && \
    echo 'echo ""' >> /home/user/bin/holo-help && \
    echo 'echo "Templates: vanilla, lit, svelte, react, vue, headless"' >> /home/user/bin/holo-help && \
    chmod +x /home/user/bin/holo-help

# Create init-nix script for optional Nix installation
RUN echo '#!/bin/bash' > /home/user/bin/init-nix && \
    echo 'set -e' >> /home/user/bin/init-nix && \
    echo '' >> /home/user/bin/init-nix && \
    echo 'echo "Installing Nix package manager (optional)..."' >> /home/user/bin/init-nix && \
    echo 'echo "This installs Nix to /nix for additional packages beyond what'\''s in the container."' >> /home/user/bin/init-nix && \
    echo 'echo ""' >> /home/user/bin/init-nix && \
    echo '' >> /home/user/bin/init-nix && \
    echo 'if [ -f "$HOME/.nix-profile/etc/profile.d/nix.sh" ]; then' >> /home/user/bin/init-nix && \
    echo '    echo "Nix is already installed!"' >> /home/user/bin/init-nix && \
    echo '    source "$HOME/.nix-profile/etc/profile.d/nix.sh"' >> /home/user/bin/init-nix && \
    echo '    nix --version' >> /home/user/bin/init-nix && \
    echo '    exit 0' >> /home/user/bin/init-nix && \
    echo 'fi' >> /home/user/bin/init-nix && \
    echo '' >> /home/user/bin/init-nix && \
    echo '# Use cached installer if available' >> /home/user/bin/init-nix && \
    echo 'if [ -f /tmp/nix-installer.sh ]; then' >> /home/user/bin/init-nix && \
    echo '    bash /tmp/nix-installer.sh --no-daemon' >> /home/user/bin/init-nix && \
    echo 'else' >> /home/user/bin/init-nix && \
    echo '    curl -L https://nixos.org/nix/install | sh -s -- --no-daemon' >> /home/user/bin/init-nix && \
    echo 'fi' >> /home/user/bin/init-nix && \
    echo '' >> /home/user/bin/init-nix && \
    echo '# Configure Nix' >> /home/user/bin/init-nix && \
    echo 'mkdir -p ~/.config/nix' >> /home/user/bin/init-nix && \
    echo 'echo "experimental-features = nix-command flakes" > ~/.config/nix/nix.conf' >> /home/user/bin/init-nix && \
    echo 'echo "sandbox = false" >> ~/.config/nix/nix.conf' >> /home/user/bin/init-nix && \
    echo 'echo "substituters = https://cache.nixos.org https://holochain-ci.cachix.org" >> ~/.config/nix/nix.conf' >> /home/user/bin/init-nix && \
    echo 'echo "trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8=" >> ~/.config/nix/nix.conf' >> /home/user/bin/init-nix && \
    echo 'echo "auto-optimise-store = true" >> ~/.config/nix/nix.conf' >> /home/user/bin/init-nix && \
    echo '' >> /home/user/bin/init-nix && \
    echo 'echo ""' >> /home/user/bin/init-nix && \
    echo 'echo "Nix installed! Source the profile to use it:"' >> /home/user/bin/init-nix && \
    echo 'echo "  source ~/.nix-profile/etc/profile.d/nix.sh"' >> /home/user/bin/init-nix && \
    chmod +x /home/user/bin/init-nix

# Add to bashrc
RUN echo '' >> /home/user/.bashrc && \
    echo '# ============================================================================' >> /home/user/.bashrc && \
    echo '# Rust Development Environment' >> /home/user/.bashrc && \
    echo '# ============================================================================' >> /home/user/.bashrc && \
    echo '' >> /home/user/.bashrc && \
    echo '# Rust toolchain (installed to /opt/rust to survive PVC mounts)' >> /home/user/.bashrc && \
    echo 'export RUSTUP_HOME=/opt/rust/rustup' >> /home/user/.bashrc && \
    echo 'export CARGO_HOME=/opt/rust/cargo' >> /home/user/.bashrc && \
    echo 'export PATH="/opt/rust/cargo/bin:/opt/holochain/bin:/home/user/bin:$PATH"' >> /home/user/.bashrc && \
    echo '' >> /home/user/.bashrc && \
    echo '# Source Nix profile if installed (optional)' >> /home/user/.bashrc && \
    echo 'if [ -f "$HOME/.nix-profile/etc/profile.d/nix.sh" ]; then' >> /home/user/.bashrc && \
    echo '    source "$HOME/.nix-profile/etc/profile.d/nix.sh"' >> /home/user/.bashrc && \
    echo 'fi' >> /home/user/.bashrc

# Fix permissions for arbitrary UID (Che requirement)
RUN chgrp -R 0 /home/user && \
    chmod -R g=u /home/user && \
    chmod -R g+w /home/user

USER 1000

WORKDIR /projects
