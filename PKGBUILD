# Maintainer: Stef Korporaal
pkgname=tfmttools
pkgrel=1
pkgdesc="Use \`minijinja\` to rename audio files according to their tags."
arch=('any')
url="https://github.com/Hellrespawn/tfmttools"
license=('BSD 3-Clause')
makedepends=('cargo' 'git')

source=("$pkgname::git+file://$PWD"
	"service"
	"timer"
	"cloudflare-dyndns.conf")

sha256sums=('SKIP'
	'f51bc911fc085cbd28e2e38bf62b4cc5886ed7a4dac2d41cf432bc373a3e897e'
	'720ebde0cd013756bf3ade0eff876ad7d503ea7a13ec00b48d960170dca65673'
	'a814ed5079a20af7ca7471a6bec2dad137ba90806e36b06c3d3fe71dfb7387cd')

prepare() {
	export RUSTUP_TOOLCHAIN=stable
	cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
	export RUSTUP_TOOLCHAIN=stable
	export CARGO_TARGET_DIR=target
	cargo build --frozen --release --all-features
}

bin_name="tfmt"

package() {
	install -Dm755 "$srcdir/target/release/$bin_name" "$pkgdir/usr/bin/$bin_name"
}
