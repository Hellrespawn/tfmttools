#!/usr/bin/env sh
set -eu

curl -L \
  https://esm.sh/htm@3.1.1/es2022/preact/standalone.mjs \
  -o crates/test-harness/assets/report/htm-preact-standalone.mjs

curl -L \
  https://raw.githubusercontent.com/developit/htm/3.1.1/LICENSE \
  -o crates/test-harness/assets/report/LICENSE-htm.txt

curl -L \
  https://raw.githubusercontent.com/preactjs/preact/10.27.2/LICENSE \
  -o crates/test-harness/assets/report/LICENSE-preact.txt
