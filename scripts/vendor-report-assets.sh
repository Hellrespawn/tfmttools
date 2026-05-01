#!/usr/bin/env sh
set -eu

HTM_VERSION="3.1.1"
PREACT_VERSION="10.27.2"

curl -L \
  "https://esm.sh/htm@${HTM_VERSION}/es2022/preact/standalone.mjs" -o "crates/test-harness/assets/report/htm-preact-standalone.mjs"

curl -L \
  "https://raw.githubusercontent.com/developit/htm/${HTM_VERSION}/LICENSE" -o "crates/test-harness/assets/report/LICENSE-htm.txt"

curl -L \
  "https://raw.githubusercontent.com/preactjs/preact/${PREACT_VERSION}/LICENSE" -o "crates/test-harness/assets/report/LICENSE-preact.txt"
