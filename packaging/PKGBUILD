# Maintainer: Stef Korporaal <stef@skorporaal.com>

function _meta() {
    cargo metadata --format-version=1 --no-deps | jq --raw-output ".packages[0].$1"
}

prgname=tfmt

pkgname=tfmttools
pkgver=$(_meta "version")
pkgrel=1
pkgdesc=$(_meta "description")
url=$(_meta "homepage")
license=("$(_meta "license")")
makedepends=('cargo')
depends=()
arch=('x86_64')
source=()
b2sums=()

prepare() {
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release
}

check() {
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen
}

package() {
    install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/$prgname"
}
