#!/bin/sh
#
# Usage: ./.ops/release.sh [Release Type]
#
# Where [Release Type] is one of:
declare -a releaseTypes=("patch" "minor" "major")

# Check arguments.
if [ "$#" -lt 1 ]; then
    echo "please provide a release type. examples:\n"
    echo "  ./.ops/release.sh patch"
    echo "  ./.ops/release.sh minor"
    echo "  ./.ops/release.sh major"
    exit 1
fi

# Extract arguments.
releaseType=$1

# Only allow supported release types.
if [[ ! " ${releaseTypes[*]} " =~ [[:space:]]${releaseType}[[:space:]] ]]; then
    echo "${releaseType} is not one of: ${releaseTypes[*]}"
    exit 1
fi

# Verify workspace.
cargo fmt --check
cargo clippy
cargo test

# Install release tooling if necessary.
echo "installing release tooling..."
cargo install -q cargo-release git-cliff
echo "release tooling installed."

# Run release.
cargo release $releaseType --config ./.ops/release.toml