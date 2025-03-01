#!/bin/sh

TFMT_LOG=tfmttools=trace DEBUG_DELAY_MS=100 cargo run --features debug -- -c ./test-conf "$@"
