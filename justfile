set dotenv-load

# Display help information.
help:
  @ just --list

# Open project workspace in VS Code.
code:
  @ code .

# Install tooling for working with The Stack.
[linux]
install-tooling:
  # Install QEMU packages.
  sudo apt-get update
  sudo apt-get install -y qemu-user-static binfmt-support
  sudo update-binfmts --enable qemu-arm
  sudo update-binfmts --display qemu-arm
  # This step will execute the registering scripts.
  docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
  # Testing the emulation environment.
  docker run --rm -t arm64v8/ubuntu uname -m
  @ just _install-tooling-all-platforms

# Install tooling for working with The Stack.
[macos]
install-tooling:
  @ just _install-tooling-all-platforms

_install-tooling-all-platforms:
  # Install bun.
  command -v bun >/dev/null 2>&1 || curl -fsSL https://bun.sh/install | bash
  # Install the zig compiler for cross-compilation.
  command -v zig >/dev/null 2>&1 || bun install --global zig
  # Install cargo-binstall for installing binaries from crates.io.
  command -v cargo-binstall >/dev/null 2>&1 || curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
  # Install the rover CLI tool to manage Apollo supergraphs.
  command -v rover >/dev/null 2>&1 || curl -sSL https://rover.apollo.dev/nix/latest | sh
  # Install cargo-watch for watching Rust files.
  cargo binstall --no-confirm cargo-watch
  # Install cargo-edit for managing dependencies.
  cargo binstall --no-confirm cargo-edit
  # Install cargo-lambda for building Rust Lambda functions.
  cargo binstall --no-confirm cargo-lambda

# Set up all projects.
setup-all:
  @ just setup lambda-directly
  @ just setup lambda-directly-optimized
  @ just setup lambda-with-server

# Setup dependencies and tooling for <project>, e.g. `just setup lambda-directly`.
setup project:
  just _setup-{{project}}

_setup-lambda-directly: (_setup-rust "lambda-directly")

_setup-lambda-directly-optimized: (_setup-rust "lambda-directly-optimized")

_setup-lambda-with-server: (_setup-rust "lambda-with-server")

_setup-rust project:
  #!/usr/bin/env bash
  set -euxo pipefail
  cd {{project}}
  rustup toolchain install stable
  rustup default stable

# Run <project> development server, e.g. `just dev ui-app`.
dev project:
  just _dev-{{project}}

# Alternative: cargo watch -x run
_dev-lambda-directly:
  cd lambda-directly && cargo lambda watch --invoke-port 4010

_dev-lambda-directly-optimized:
  cd lambda-directly-optimized && cargo lambda watch --invoke-port 4020

_dev-lambda-with-server:
  cd lambda-directly && cargo lambda watch --invoke-port 4030

# Invoke the local lambda function for <project>, e.g. `just invoke lambda-directly`.
invoke project:
  just _invoke-{{project}}

_invoke-lambda-directly:
  cargo lambda invoke --invoke-port 4010 --data-ascii '{ "body": "{\"query\":\"{me { name } }\"}" }'

_invoke-lambda-directly-optimized:
  cargo lambda invoke --invoke-port 4020 --data-ascii '{ "body": "{\"query\":\"{me { name } }\"}" }'

_invoke-lambda-with-server:
  cargo lambda invoke --invoke-port 4030 --data-ascii '{ "body": "{\"query\":\"{me { name } }\"}" }'

# Build the bootstrap file in docker for <project>, e.g. `just build lambda-directly-optimized-arm`.
build project:
  @ just _build-{{project}}

_build-lambda-directly:
  _build_generic "lambda-directly"

_build-lambda-with-server:
  _build_generic "lambda-with-server"

_build_generic project:
  #!/usr/bin/env bash
  set -euxo pipefail
  cd {{project}}
  docker build -t {{project}}:lambda .
  export TMP_IMAGE_ID=$(docker create {{project}}:lambda)
  docker cp $TMP_IMAGE_ID:/dist/apollo-router-lambda/target/lambda/apollo-router-lambda/bootstrap bootstrap
  docker rm -v $TMP_IMAGE_ID

_build-lambda-directly-optimized-arm:
  #!/usr/bin/env bash
  set -euxo pipefail
  cd lambda-directly-optimized
  docker buildx build -f Dockerfile-arm --platform linux/arm64 -t lambda-directly-optimized:lambda-arm .
  export TMP_IMAGE_ID=$(docker create --platform linux/arm64 lambda-directly:lambda-arm)
  docker cp $TMP_IMAGE_ID:/dist/apollo-router-lambda/target/lambda/apollo-router-lambda/bootstrap arm/bootstrap
  docker rm -v $TMP_IMAGE_ID

_build-lambda-directly-optimized-x86:
  #!/usr/bin/env bash
  set -euxo pipefail
  cd lambda-directly-optimized
  docker buildx build -f Dockerfile-x86 --platform linux/amd64 -t lambda-directly-optimized:lambda-x86 .
  export TMP_IMAGE_ID=$(docker create --platform linux/amd64 lambda-directly:lambda-x86)
  docker cp $TMP_IMAGE_ID:/dist/apollo-router-lambda/target/lambda/apollo-router-lambda/bootstrap x86/bootstrap
  docker rm -v $TMP_IMAGE_ID
