#!/bin/sh
set -eu

command="${1:?missing command}"

if [ -n "${GITHUB_WORKSPACE:-}" ]; then
    cd "${GITHUB_WORKSPACE}"
fi

exec sh -eu -c "${command}"
