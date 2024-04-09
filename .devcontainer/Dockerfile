FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install curl and other dependencies
RUN apt-get update && apt-get install -y curl

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
RUN apt-get install -y nodejs

# Verify that Node.js was installed
RUN node --version
RUN npm --version

RUN apt-get install -y protobuf-compiler