#!/bin/sh

TFMT_LOG=tfmttools=trace cargo run --features debug -- -c ./test-conf "$@"
