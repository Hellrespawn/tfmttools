#!/bin/sh
set -eu

tag="${1:-}"

if [ -z "$tag" ]; then
    tag="${GITHUB_REF_NAME:-}"
fi

if [ -z "$tag" ]; then
    echo "No release tag provided." >&2
    exit 1
fi

if ! printf '%s\n' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Release tag must look like v0.24.0, got: ${tag}" >&2
    exit 1
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"

if [ -z "$version" ]; then
    echo "Unable to determine workspace version from Cargo.toml" >&2
    exit 1
fi

expected_tag="v${version}"

if [ "$tag" != "$expected_tag" ]; then
    echo "Release tag ${tag} does not match workspace version ${version}." >&2
    exit 1
fi

echo "Release tag is valid: ${tag}"
