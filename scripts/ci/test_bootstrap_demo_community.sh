#!/usr/bin/env bash
set -euo pipefail

TEST=$1

echo "Bootstrapping demo community and running tests: $TEST"

CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bootstrap_demo_community.py --client $CLIENT_BIN --signer //Bob --test $TEST

cd "$CURRENT_DIR"
