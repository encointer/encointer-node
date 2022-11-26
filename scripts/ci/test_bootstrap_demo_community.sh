#!/usr/bin/env bash
set -euo pipefail

echo "Bootstrapping demo community and running tests"

python "$CLIENT_DIR/bootstrap_demo_community.py" --client $CLIENT_BIN --test