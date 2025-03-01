#!/bin/sh

TFMT_LOG=tfmttools=trace cargo run -- -c ./test-conf "$@"
