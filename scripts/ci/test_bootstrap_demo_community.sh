#!/usr/bin/env bash
set -euo pipefail

echo "Bootstrapping demo community and running tests"

CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bootstrap_demo_community.py --client $CLIENT_BIN --signer //Bob --test

cd "$CURRENT_DIR"
