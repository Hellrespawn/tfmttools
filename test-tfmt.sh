#!/bin/sh

TFMT_LOG=tfmttools=trace DEBUG_DELAY_MS=0 cargo run --features debug -- -c ./test-conf "$@"
