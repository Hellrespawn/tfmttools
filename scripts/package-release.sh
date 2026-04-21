#!/bin/sh
set -eu

version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"

if [ -z "$version" ]; then
    echo "Unable to determine workspace version from Cargo.toml" >&2
    exit 1
fi

for required_file in README.md LICENSE CHANGELOG.md; do
    if [ ! -f "$required_file" ]; then
        echo "Missing required release file: ${required_file}" >&2
        exit 1
    fi
done

if [ ! -d examples ]; then
    echo "Missing required release directory: examples" >&2
    exit 1
fi

if ! find examples -type f -name '*.tfmt' | grep -q .; then
    echo "Missing required example template: examples/*.tfmt" >&2
    exit 1
fi

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        target="x86_64-pc-windows-msvc"
        binary="tfmt.exe"
        archive="tfmt-${version}-${target}.zip"
        ;;
    *)
        target="x86_64-unknown-linux-gnu"
        binary="tfmt"
        archive="tfmt-${version}-${target}.tar.gz"
        ;;
esac

checksums="SHA256SUMS-${target}"

cargo build --release

binary_version="$(target/release/${binary} --version)"

case "$binary_version" in
    *"$version"*) ;;
    *)
        echo "Built binary version does not match ${version}: ${binary_version}" >&2
        exit 1
        ;;
esac

dist_dir="dist"
package_dir="${dist_dir}/tfmt-${version}-${target}"

rm -rf "$package_dir"
mkdir -p "$package_dir"

cp "target/release/${binary}" "$package_dir/"
cp README.md "$package_dir/"
cp LICENSE "$package_dir/"
cp CHANGELOG.md "$package_dir/"
cp -R examples "$package_dir/"

mkdir -p "$dist_dir"
rm -f "${dist_dir}/${archive}"
rm -f "${dist_dir}/${checksums}"

case "$archive" in
    *.zip)
        powershell.exe -NoLogo -NoProfile -Command \
            "Compress-Archive -Path '${package_dir}/*' -DestinationPath '${dist_dir}/${archive}' -Force"
        ;;
    *.tar.gz)
        tar -C "$dist_dir" -czf "${dist_dir}/${archive}" \
            "tfmt-${version}-${target}"
        ;;
esac

(
    cd "$dist_dir"
    sha256sum "$archive" > "$checksums"
)

echo "${dist_dir}/${archive}"
echo "${dist_dir}/${checksums}"
