#!/bin/sh

RUST_LOG=debug cargo run --bin controller 2>controller-logfile &
PID=$!
BLACKJACK_LOG_LEVEL=blackjack=info blackjack --parallel "$MINIKUBE_CPUS" --timeout-scaling 2 test/blackjack
kill $PID
