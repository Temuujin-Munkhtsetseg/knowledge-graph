# Use a standard Ubuntu image as the base
FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies required for Rust, Node.js, and Python builds
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends curl build-essential pkg-config libssl-dev git

# Switch to a non-root user
USER vscode

# Install mise for the vscode user
RUN curl https://mise.run | sh
ENV PATH="/home/vscode/.local/bin:${PATH}"

# Copy the tool versions configuration
WORKDIR /workspaces/knowledge-graph
COPY --chown=vscode:vscode mise.toml .

# Install the toolchains defined in mise.toml (Rust, Node, Python)
RUN mise install

# Copy package manager files to leverage Docker cache
COPY --chown=vscode:vscode package.json package-lock.json ./

# Install Node.js dependencies
RUN npm install

# Copy the rest of the application code
COPY --chown=vscode:vscode . .

# Set a default command
CMD ["/bin/bash"]