#!/bin/sh
set -e

if [ -z "$1" ]; then
    echo "Usage: ./bump <version>"
    echo "Example: ./bump 0.2.0"
    exit 1
fi

V="$1"
echo "Bumping to $V"

sed -i "s/^version = \".*\"/version = \"$V\"/" Cargo.toml
sed -i "s/^version = \".*\"/version = \"$V\"/" crates/python/pyproject.toml
sed -i "s/\"version\": \".*\"/\"version\": \"$V\"/" crates/node/package.json

echo "Cargo.toml:       $(grep '^version' Cargo.toml | head -1)"
echo "pyproject.toml:   $(grep '^version' crates/python/pyproject.toml | head -1)"
echo "package.json:     $(grep '"version"' crates/node/package.json | head -1)"
echo "Done."
