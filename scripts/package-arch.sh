#!/bin/sh
set -eu

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"

if [ -z "$version" ]; then
    echo "Unable to determine workspace version from Cargo.toml" >&2
    exit 1
fi

case "$(uname -s)" in
    Linux)
        ;;
    *)
        echo "Arch package generation is currently supported on Linux only." >&2
        exit 1
        ;;
esac

target="x86_64-unknown-linux-gnu"
archive="dist/tfmt-${version}-${target}.tar.gz"
checksums="SHA256SUMS-${target}"

if [ ! -f "$archive" ]; then
    echo "Missing release archive: ${archive}" >&2
    echo "Run scripts/package-release.sh before scripts/package-arch.sh." >&2
    exit 1
fi

if ! command -v makepkg >/dev/null 2>&1; then
    echo "makepkg is required to build the Arch package." >&2
    exit 1
fi

cp packaging/PKGBUILD dist/PKGBUILD

(
    cd dist
    rm -f tfmt-"${version}"-*.pkg.tar.* tfmt-debug-"${version}"-*.pkg.tar.*
    if [ -f "$checksums" ]; then
        grep -v '\.pkg\.tar\.' "$checksums" > "${checksums}.tmp" || true
        mv "${checksums}.tmp" "$checksums"
    fi
    package_paths="$(
        TFMT_PKGVER="$version" makepkg --packagelist -p PKGBUILD
    )"
    TFMT_PKGVER="$version" makepkg -f --noconfirm -p PKGBUILD
    for package in $package_paths; do
        if [ ! -e "$package" ]; then
            echo "Expected Arch package was not created: $package" >&2
            echo "dist/ contents after makepkg:" >&2
            ls -la >&2
            exit 1
        fi
        sha256sum "$package" >> "$checksums"
    done
)

for package in $(
    cd dist
    TFMT_PKGVER="$version" makepkg --packagelist -p PKGBUILD
); do
    [ -e "$package" ] || continue
    echo "$package"
done
