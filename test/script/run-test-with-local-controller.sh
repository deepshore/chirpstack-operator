#!/bin/sh

RUST_LOG=debug cargo run --bin controller 2>controller-logfile &
PID=$!
blackjack $(BLACKJACK_SETTINGS) test/blackjack/user
kill $PID
