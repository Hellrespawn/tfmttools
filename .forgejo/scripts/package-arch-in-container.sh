#!/bin/sh
set -eu

pacman-key --init
pacman-key --populate archlinux
pacman -Sy --noconfirm --needed archlinux-keyring
pacman -S --noconfirm --needed base-devel shadow
useradd -m builder
chown -R builder:builder "${GITHUB_WORKSPACE}"
cd "${GITHUB_WORKSPACE}"
sudo -u builder sh -eu -c "cd \"${GITHUB_WORKSPACE}\" && sh ./scripts/package-arch.sh"
