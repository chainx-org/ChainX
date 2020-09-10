#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

ROOT_CARGO_TOML="../Cargo.toml"

current_version=$(cat "$ROOT_CARGO_TOML" | grep '^version' | cut -f 2 -d '='  | tr -d '[:space:]' | tr -d '"')

echo "Current version: $current_version"
read -p "Bump version to: " next_version
echo ''

./apply_bump_version.py "$next_version"

echo ''
echo "Bump up ChainX to version $next_version successfully!"
